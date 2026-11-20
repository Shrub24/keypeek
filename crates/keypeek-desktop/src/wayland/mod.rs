// Drives the egui UI directly on a smithay-client-toolkit event loop. The overlay
// is a wlr-layer-shell overlay surface so it stays above other windows and
// click-through without WM cooperation (winit/eframe can't do this on Wayland);
// other egui viewports, such as the settings window, are ordinary xdg toplevels.

mod egl;
mod input;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;
use web_time::Instant;

use smithay_client_toolkit::reexports::protocols::wp::{
    fractional_scale::v1::client::{
        wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        wp_fractional_scale_v1::{self, WpFractionalScaleV1},
    },
    viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_dispatch2, delegate_registry,
    output::{OutputHandler, OutputState},
    reexports::calloop::{ping::make_ping, EventLoop},
    reexports::calloop_wayland_source::WaylandSource,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        xdg::{
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell,
        },
        WaylandSurface,
    },
};
use wayland_client::{
    delegate_noop,
    globals::registry_queue_init,
    protocol::{
        wl_keyboard::WlKeyboard, wl_output::WlOutput, wl_pointer::WlPointer, wl_seat::WlSeat,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, QueueHandle,
};

use keypeek_presentation::{
    add_phosphor_to_fonts, settings::MonitorSelection, CommandQueue, DesktopOverlayApp,
    OverlayHost, SettingsStore,
};
use keypeek_protocol::{DiscoveredDevice, UiWake};

use egl::{EglContext, EglWindow};
use input::InputState;

/// Records the egui UI's host requests during one frame, applied after `ui()` returns
/// (we can't touch the Wayland objects while the app borrow is live).
#[derive(Default)]
struct WaylandHost {
    passthrough: Option<bool>,
}

impl OverlayHost for WaylandHost {
    fn set_passthrough(&mut self, enabled: bool) {
        self.passthrough = Some(enabled);
    }
}

/// The GL context and painter every surface is drawn with; created with the
/// first surface that gets a size.
struct Gpu {
    egl: EglContext,
    /// Loaded once a surface has made the context current.
    render: Option<Render>,
}

struct Render {
    gl: std::sync::Arc<egui_glow::glow::Context>,
    painter: egui_glow::Painter,
}

/// Fractional scaling needs both protocols: the scale event and a viewport to map
/// the scaled buffer back to the logical size. Without them, integer scale.
#[derive(Clone)]
struct Scaling {
    viewporter: Option<WpViewporter>,
    fractional_manager: Option<WpFractionalScaleManagerV1>,
}

/// One drawable `wl_surface`: its EGL window, logical size, scale, and input.
struct Canvas {
    surface: WlSurface,
    egl: Option<EglWindow>,
    viewport: Option<WpViewport>,
    fractional: Option<WpFractionalScaleV1>,
    /// Size in logical points and the (possibly fractional) scale.
    width: i32,
    height: i32,
    scale: f64,
    configured: bool,
    swap_interval: i32,
    input: InputState,
}

impl Canvas {
    fn new(
        surface: WlSurface,
        scaling: &Scaling,
        qh: &QueueHandle<WaylandApp>,
        swap_interval: i32,
    ) -> Self {
        let (viewport, fractional) = match (&scaling.viewporter, &scaling.fractional_manager) {
            (Some(viewporter), Some(manager)) => (
                Some(viewporter.get_viewport(&surface, qh, ())),
                Some(manager.get_fractional_scale(&surface, qh, ())),
            ),
            _ => (None, None),
        };
        Self {
            surface,
            egl: None,
            viewport,
            fractional,
            width: 0,
            height: 0,
            scale: 1.0,
            configured: false,
            swap_interval,
            input: InputState::default(),
        }
    }

    fn size_px(&self) -> [u32; 2] {
        [
            (self.width as f64 * self.scale).round().max(1.0) as u32,
            (self.height as f64 * self.scale).round().max(1.0) as u32,
        ]
    }

    /// Sizes the EGL window to the surface, creating it (and the shared GL state)
    /// on first use.
    fn resize_gl(&mut self, conn: &Connection, gpu: &mut Option<Gpu>) {
        if let Err(e) = self.try_resize_gl(conn, gpu) {
            eprintln!("KeyPeek: {e}");
        }
    }

    fn try_resize_gl(&mut self, conn: &Connection, gpu: &mut Option<Gpu>) -> Result<(), String> {
        let [w, h] = self.size_px();
        if let Some(viewport) = &self.viewport {
            // Buffer scale stays 1; the viewport maps the scaled buffer to the
            // logical surface size.
            viewport.set_destination(self.width.max(1), self.height.max(1));
        } else {
            self.surface.set_buffer_scale(self.scale as i32);
        }

        if let Some(egl) = &self.egl {
            egl.resize(w as i32, h as i32);
            return Ok(());
        }

        if gpu.is_none() {
            *gpu = Some(Gpu {
                egl: EglContext::new(conn)?,
                render: None,
            });
        }
        let Some(gpu) = gpu.as_mut() else {
            return Ok(());
        };
        self.egl = Some(EglWindow::new(
            &gpu.egl,
            &self.surface,
            w as i32,
            h as i32,
            self.swap_interval,
        )?);
        // GL loading and the painter need a current context, which only exists
        // once a surface does.
        if gpu.render.is_none() {
            let gl = gpu.egl.load_gl();
            let painter = egui_glow::Painter::new(gl.clone(), "", None, false)
                .map_err(|e| format!("egui_glow painter init failed: {e}"))?;
            gpu.render = Some(Render { gl, painter });
        }
        Ok(())
    }

    /// Paints one frame; returns `false`, leaving `textures` unapplied, when the
    /// surface cannot be drawn yet.
    fn paint(
        &self,
        gpu: &mut Gpu,
        clear: [f32; 4],
        primitives: &[egui::ClippedPrimitive],
        pixels_per_point: f32,
        textures: &mut egui::TexturesDelta,
    ) -> bool {
        let (Some(egl), Some(render)) = (&self.egl, gpu.render.as_mut()) else {
            return false;
        };
        if egl.make_current(&gpu.egl).is_err() {
            return false;
        }
        let size = self.size_px();
        egui_glow::painter::clear(&render.gl, size, clear);
        render
            .painter
            .paint_and_update_textures(size, pixels_per_point, primitives, textures);
        let _ = egl.swap_buffers();
        true
    }

    /// Drains this surface's input for an egui pass of viewport `id`.
    fn take_input(&mut self, id: egui::ViewportId, info: egui::ViewportInfo) -> egui::RawInput {
        let mut raw_input = self.input.take_raw_input((self.width, self.height));
        raw_input.viewport_id = id;
        raw_input.viewports = std::iter::once((id, info)).collect();
        raw_input
    }

    fn viewport_info(&self) -> egui::ViewportInfo {
        egui::ViewportInfo {
            native_pixels_per_point: Some(self.scale as f32),
            ..Default::default()
        }
    }

    /// Releases what this canvas attached to its surface. The surface itself
    /// belongs to its role object, which must be destroyed first.
    fn release(&mut self) {
        self.egl = None;
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
        if let Some(fractional) = self.fractional.take() {
            fractional.destroy();
        }
    }
}

/// The overlay's layer surface on its output.
struct Overlay {
    layer: LayerSurface,
    canvas: Canvas,
    /// First frame with a buffer committed; the compositor only sends
    /// `preferred_scale` and a configure after that.
    primed: bool,
}

/// A native window showing one egui viewport.
struct AppWindow {
    window: Window,
    canvas: Canvas,
    default_size: (i32, i32),
    close_requested: bool,
}

impl Drop for AppWindow {
    fn drop(&mut self) {
        // The EGL window goes first; dropping `window` then destroys the toplevel
        // before its surface, as xdg-shell requires.
        self.canvas.release();
    }
}

/// State the immediate-viewport renderer shares with the event loop. egui calls
/// the renderer in the middle of the root pass, while the app is borrowed.
#[derive(Default)]
struct Shared {
    gpu: Option<Gpu>,
    windows: HashMap<egui::ViewportId, AppWindow>,
    /// Texture updates from passes that could not paint; applied with the next paint.
    pending_textures: egui::TexturesDelta,
}

struct WaylandApp {
    registry_state: RegistryState,
    output_state: OutputState,
    seat_state: SeatState,
    compositor_state: CompositorState,
    shell: LayerShell,
    xdg_shell: XdgShell,
    qh: QueueHandle<WaylandApp>,
    scaling: Scaling,

    /// `None` until the first surface is bound; retargeting replaces it.
    overlay: Option<Overlay>,
    /// Output the overlay is bound to; `None` lets the compositor choose.
    applied_output: Option<String>,
    /// Input region applied to the overlay; empty means click-through.
    applied_input: Option<Vec<egui::Rect>>,
    interactive: bool,
    keyboard: Option<WlKeyboard>,
    keyboard_focus: Option<WlSurface>,
    pointer: Option<WlPointer>,

    egui_ctx: egui::Context,
    app: DesktopOverlayApp,
    _tray: super::tray::Tray,
    shared: Rc<RefCell<Shared>>,

    needs_redraw: bool,
    exit: bool,
    repaint_at: Option<Instant>,
}

pub fn run(
    settings_store: std::sync::Arc<dyn SettingsStore>,
    devices: Vec<DiscoveredDevice>,
    open_settings: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init::<WaylandApp>(&conn)?;
    let qh = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh)?;
    let shell = LayerShell::bind(&globals, &qh)?;
    let xdg_shell = XdgShell::bind(&globals, &qh)?;

    let scaling = Scaling {
        viewporter: globals
            .bind::<WpViewporter, WaylandApp, _>(&qh, 1..=1, ())
            .ok(),
        fractional_manager: globals
            .bind::<WpFractionalScaleManagerV1, WaylandApp, _>(&qh, 1..=1, ())
            .ok(),
    };

    // calloop loop, plus a ping the worker threads use to request repaints.
    let mut event_loop: EventLoop<WaylandApp> = EventLoop::try_new()?;
    let loop_handle = event_loop.handle();

    let (ping, ping_source) = make_ping()?;
    loop_handle
        .insert_source(ping_source, |_, _, app: &mut WaylandApp| {
            app.needs_redraw = true;
        })
        .map_err(|e| format!("failed to insert repaint ping source: {e}"))?;

    let egui_ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    add_phosphor_to_fonts(&mut fonts);
    egui_ctx.set_fonts(fonts);
    egui_ctx.set_embed_viewports(false);
    let shared = Rc::new(RefCell::new(Shared::default()));
    install_viewport_renderer(shared.clone());

    let ui_wake = UiWake::new(std::sync::Arc::new(move || ping.ping()));
    let commands = std::sync::Arc::new(CommandQueue::default());
    if let Err(e) = super::ipc::spawn(commands.clone(), ui_wake.clone()) {
        eprintln!("KeyPeek: overlay control unavailable: {e}");
    }
    let tray_icon = super::tray::create_tray_icon(commands.clone(), ui_wake.clone());
    let app = DesktopOverlayApp::new(
        commands,
        ui_wake,
        settings_store,
        super::session_state_store(),
        devices,
        open_settings,
    );

    let mut state = WaylandApp {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),
        compositor_state,
        shell,
        xdg_shell,
        qh,
        scaling,
        overlay: None,
        applied_output: None,
        applied_input: None,
        interactive: false,
        keyboard: None,
        keyboard_focus: None,
        pointer: None,
        egui_ctx,
        app,
        _tray: tray_icon,
        shared,
        needs_redraw: false,
        exit: false,
        repaint_at: None,
    };

    // One roundtrip so output names are known before the surface is bound; the
    // overlay follows the configured output from its first frame.
    event_queue.roundtrip(&mut state)?;
    state.update_available_monitors();
    let initial = target_output_name(&state.app.settings.active.monitor).map(str::to_string);
    state.bind_overlay(initial.as_deref());

    WaylandSource::new(conn.clone(), event_queue)
        .insert(loop_handle.clone())
        .map_err(|e| format!("failed to insert Wayland event source: {e}"))?;

    while !state.exit {
        // Sleep until the next scheduled repaint; wake immediately when a redraw is
        // already pending, and block indefinitely (until an event) when idle.
        let timeout = if state.needs_redraw && state.overlay_configured() {
            Some(Duration::ZERO)
        } else {
            state
                .repaint_at
                .map(|at| at.saturating_duration_since(Instant::now()))
        };
        event_loop.dispatch(timeout, &mut state)?;

        if let Some(at) = state.repaint_at {
            if Instant::now() >= at {
                state.repaint_at = None;
                state.needs_redraw = true;
            }
        }
        if state.needs_redraw && state.overlay_configured() {
            state.draw();
        }

        // A new monitor choice applies to the running overlay, not just the next start.
        let target = target_output_name(&state.app.settings.active.monitor).map(str::to_string);
        if target.as_deref() != state.applied_output.as_deref() {
            state.bind_overlay(target.as_deref());
        }
    }

    Ok(())
}

/// Lets egui open immediate viewports as native windows: each pass runs with the
/// window's own input and is painted onto its surface with the shared painter.
fn install_viewport_renderer(shared: Rc<RefCell<Shared>>) {
    egui::Context::set_immediate_viewport_renderer(move |ctx, viewport| {
        let id = viewport.ids.this;
        let input = {
            let mut shared = shared.borrow_mut();
            match shared.windows.get_mut(&id) {
                Some(window) => {
                    let mut info = window.canvas.viewport_info();
                    if std::mem::take(&mut window.close_requested) {
                        info.events.push(egui::ViewportEvent::Close);
                    }
                    window.canvas.take_input(id, info)
                }
                // Not open yet: the host opens it after this pass. egui still
                // needs the pass to run.
                None => egui::RawInput {
                    viewport_id: id,
                    viewports: std::iter::once((id, egui::ViewportInfo::default())).collect(),
                    ..Default::default()
                },
            }
        };

        // No borrow may be held here: the UI can show further viewports.
        let mut viewport_ui_cb = viewport.viewport_ui_cb;
        let output = ctx.run_ui(input, |ui| viewport_ui_cb(ui));
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let clear = egui::Rgba::from(ctx.global_style().visuals.panel_fill).to_array();

        let mut shared = shared.borrow_mut();
        let Shared {
            gpu,
            windows,
            pending_textures,
        } = &mut *shared;
        let mut textures = std::mem::take(pending_textures);
        textures.append(output.textures_delta);
        let painted = match (gpu.as_mut(), windows.get(&id)) {
            (Some(gpu), Some(window)) if window.canvas.configured => window.canvas.paint(
                gpu,
                clear,
                &primitives,
                output.pixels_per_point,
                &mut textures,
            ),
            _ => false,
        };
        if !painted {
            *pending_textures = textures;
        }
    });
}

/// Output name the overlay should be bound to, or `None` to let the compositor choose.
fn target_output_name(selection: &MonitorSelection) -> Option<&str> {
    match selection {
        MonitorSelection::Primary => None,
        MonitorSelection::Named(name) => Some(name.as_str()),
    }
}

/// Screen areas of everything egui shows on the overlay, the only places it
/// should take input.
fn interactive_rects(ctx: &egui::Context) -> Vec<egui::Rect> {
    ctx.memory(|mem| {
        let mut rects: Vec<egui::Rect> = mem
            .areas()
            .visible_layer_ids()
            .into_iter()
            .filter(|layer| layer.order != egui::Order::Background)
            .filter_map(|layer| mem.area_rect(layer.id))
            .collect();
        rects.sort_by(|a, b| {
            a.min
                .x
                .total_cmp(&b.min.x)
                .then(a.min.y.total_cmp(&b.min.y))
        });
        rects
    })
}

impl WaylandApp {
    fn overlay_configured(&self) -> bool {
        self.overlay.as_ref().is_some_and(|o| o.canvas.configured)
    }

    fn output_for_name(&self, name: &str) -> Option<WlOutput> {
        self.output_state.outputs().find(|output| {
            self.output_state
                .info(output)
                .and_then(|info| info.name)
                .is_some_and(|output_name| output_name == name)
        })
    }

    /// Output names the compositor reports, for the settings window.
    fn update_available_monitors(&mut self) {
        let names = self.output_names();
        self.app.set_available_monitors(names);
    }

    fn output_names(&self) -> Vec<String> {
        self.output_state
            .outputs()
            .filter_map(|output| self.output_state.info(&output).and_then(|info| info.name))
            .collect()
    }

    /// Binds the overlay to `name`, replacing its layer surface.
    ///
    /// The output of a layer surface is fixed when it is created, so following a
    /// new setting means replacing the surface. `None` lets the compositor choose.
    /// The GL context and painter survive, so textures egui already sent remain.
    fn bind_overlay(&mut self, name: Option<&str>) {
        if let Some(mut overlay) = self.overlay.take() {
            overlay.canvas.release();
            // Dropping the layer destroys the role object, then the surface.
            drop(overlay);
        }

        let output = name.and_then(|name| self.output_for_name(name));
        let surface = self.compositor_state.create_surface(&self.qh);
        let mut canvas = Canvas::new(surface.clone(), &self.scaling, &self.qh, 1);
        if let Some(info) = output
            .as_ref()
            .and_then(|output| self.output_state.info(output))
        {
            if info.scale_factor > 0 {
                canvas.scale = info.scale_factor as f64;
            }
        }

        // Above everything and covering the whole output; the input region limits
        // where it takes clicks.
        let layer = self.shell.create_layer_surface(
            &self.qh,
            surface,
            Layer::Overlay,
            Some("keypeek"),
            output.as_ref(),
        );
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.set_size(0, 0); // 0,0 + all-edge anchors => compositor sizes it to the output
        layer.commit();

        self.overlay = Some(Overlay {
            layer,
            canvas,
            primed: false,
        });
        self.applied_output = name.map(str::to_string);
        self.needs_redraw = true;
        self.update_available_monitors();

        // A new surface accepts input by default, and the overlay starts hidden. Go
        // click-through immediately and let the next frame re-apply what the UI needs,
        // so an invisible overlay never swallows clicks across the output.
        self.applied_input = None;
        self.interactive = false;
        self.apply_input(Vec::new());
        self.app.forget_passthrough();
    }

    fn draw(&mut self) {
        self.needs_redraw = false;
        let Some(overlay) = self.overlay.as_mut() else {
            return;
        };

        // Commit one empty frame first: the compositor only sends preferred scale
        // and a configure for the surface once it has a buffer to map.
        if !overlay.primed {
            let mut shared = self.shared.borrow_mut();
            if let Some(gpu) = shared.gpu.as_mut() {
                overlay.primed = overlay.canvas.paint(
                    gpu,
                    [0.0; 4],
                    &[],
                    1.0,
                    &mut egui::TexturesDelta::default(),
                );
            }
            self.needs_redraw = true;
            return;
        }

        let info = overlay.canvas.viewport_info();
        let raw_input = overlay.canvas.take_input(egui::ViewportId::ROOT, info);

        let ctx = self.egui_ctx.clone();
        let mut host = WaylandHost::default();
        ctx.begin_pass(raw_input);
        self.app.ui(&ctx, &mut host);
        let rects = interactive_rects(&ctx);
        let mut full_output = ctx.end_pass();

        let primitives = ctx.tessellate(
            std::mem::take(&mut full_output.shapes),
            full_output.pixels_per_point,
        );
        {
            let mut shared = self.shared.borrow_mut();
            let Shared {
                gpu,
                pending_textures,
                ..
            } = &mut *shared;
            let mut textures = std::mem::take(pending_textures);
            textures.append(std::mem::take(&mut full_output.textures_delta));
            let painted = match (gpu.as_mut(), self.overlay.as_ref()) {
                (Some(gpu), Some(overlay)) => overlay.canvas.paint(
                    gpu,
                    [0.0; 4],
                    &primitives,
                    full_output.pixels_per_point,
                    &mut textures,
                ),
                _ => false,
            };
            if !painted {
                *pending_textures = textures;
            }
        }

        // Honor egui's requested repaint cadence (e.g. the overlay fade-out
        // animation) for every viewport, since all of them are drawn in this pass.
        if let Some(delay) = full_output
            .viewport_output
            .values()
            .map(|viewport| viewport.repaint_delay)
            .min()
        {
            if delay.is_zero() {
                self.needs_redraw = true;
            } else if delay < Duration::from_secs(24 * 60 * 60) {
                self.repaint_at = Some(Instant::now() + delay);
            }
        }

        self.sync_windows(&full_output.viewport_output);

        if let Some(passthrough) = host.passthrough {
            self.interactive = !passthrough;
        }
        self.apply_input(if self.interactive { rects } else { Vec::new() });
    }

    /// Opens a window for each viewport egui showed this frame and closes the
    /// windows of viewports it no longer shows.
    fn sync_windows(
        &mut self,
        viewports: &std::collections::BTreeMap<egui::ViewportId, egui::ViewportOutput>,
    ) {
        let mut shared = self.shared.borrow_mut();
        shared.windows.retain(|id, _| viewports.contains_key(id));

        for (id, viewport) in viewports {
            if *id == egui::ViewportId::ROOT || shared.windows.contains_key(id) {
                continue;
            }
            let builder = &viewport.builder;
            let default_size = builder
                .inner_size
                .map_or((450, 700), |size| (size.x as i32, size.y as i32));

            let surface = self.compositor_state.create_surface(&self.qh);
            let canvas = Canvas::new(surface.clone(), &self.scaling, &self.qh, 0);
            let window =
                self.xdg_shell
                    .create_window(surface, WindowDecorations::RequestServer, &self.qh);
            if let Some(title) = &builder.title {
                window.set_title(title.clone());
            }
            if let Some(app_id) = &builder.app_id {
                window.set_app_id(app_id.clone());
            }
            if let Some(min) = builder.min_inner_size {
                window.set_min_size(Some((min.x as u32, min.y as u32)));
            }
            window.commit();

            shared.windows.insert(
                *id,
                AppWindow {
                    window,
                    canvas,
                    default_size,
                    close_requested: false,
                },
            );
        }
    }

    /// Limits the overlay's input to `rects`; empty makes it click-through.
    fn apply_input(&mut self, rects: Vec<egui::Rect>) {
        if self.applied_input.as_ref() == Some(&rects) {
            return;
        }
        let Some(overlay) = self.overlay.as_ref() else {
            return;
        };
        let surface = overlay.layer.wl_surface();
        if let Ok(region) = Region::new(&self.compositor_state) {
            for rect in &rects {
                let min = rect.min.floor();
                let max = rect.max.ceil();
                region.add(
                    min.x as i32,
                    min.y as i32,
                    (max.x - min.x) as i32,
                    (max.y - min.y) as i32,
                );
            }
            surface.set_input_region(Some(region.wl_region()));
        }
        overlay
            .layer
            .set_keyboard_interactivity(if rects.is_empty() {
                KeyboardInteractivity::None
            } else {
                KeyboardInteractivity::OnDemand
            });
        surface.commit();
        self.applied_input = Some(rects);
    }

    /// Runs `f` on the canvas `pred` selects, with the shared GPU state.
    fn with_canvas(
        &mut self,
        pred: impl Fn(&Canvas) -> bool,
        f: impl FnOnce(&mut Canvas, &mut Option<Gpu>),
    ) {
        let mut shared = self.shared.borrow_mut();
        let Shared { gpu, windows, .. } = &mut *shared;
        if let Some(overlay) = self.overlay.as_mut().filter(|o| pred(&o.canvas)) {
            f(&mut overlay.canvas, gpu);
        } else if let Some(window) = windows.values_mut().find(|w| pred(&w.canvas)) {
            f(&mut window.canvas, gpu);
        }
        self.needs_redraw = true;
    }

    fn with_focused_input(&mut self, f: impl FnOnce(&mut InputState)) {
        let Some(focus) = self.keyboard_focus.clone() else {
            return;
        };
        self.with_canvas(|c| c.surface == focus, |canvas, _| f(&mut canvas.input));
    }
}

impl CompositorHandler for WaylandApp {
    fn scale_factor_changed(
        &mut self,
        conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &WlSurface,
        new_factor: i32,
    ) {
        let new_factor = new_factor as f64;
        self.with_canvas(
            |c| c.surface == *surface,
            |canvas, gpu| {
                // With fractional scaling active, the wp-fractional-scale event is
                // authoritative.
                if canvas.viewport.is_some() || new_factor <= 0.0 || new_factor == canvas.scale {
                    return;
                }
                canvas.scale = new_factor;
                if canvas.configured {
                    canvas.resize_gl(conn, gpu);
                }
            },
        );
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: wayland_client::protocol::wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlSurface, _time: u32) {}

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: &WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: &WlOutput,
    ) {
    }
}

impl LayerShellHandler for WaylandApp {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let mut shared = self.shared.borrow_mut();
        let Some(overlay) = self.overlay.as_mut() else {
            return;
        };
        let canvas = &mut overlay.canvas;
        let (mut w, mut h) = configure.new_size;
        // 0 means "pick your own size"; fall back to a sane default until an output
        // reports real dimensions.
        if w == 0 {
            w = if canvas.width > 0 {
                canvas.width as u32
            } else {
                1280
            };
        }
        if h == 0 {
            h = if canvas.height > 0 {
                canvas.height as u32
            } else {
                720
            };
        }
        canvas.width = w as i32;
        canvas.height = h as i32;

        canvas.resize_gl(conn, &mut shared.gpu);
        canvas.configured = true;
        self.needs_redraw = true;
    }
}

impl WindowHandler for WaylandApp {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &Window) {
        if let Some(app_window) = self
            .shared
            .borrow_mut()
            .windows
            .values_mut()
            .find(|w| w.window == *window)
        {
            app_window.close_requested = true;
        }
        self.needs_redraw = true;
    }

    fn configure(
        &mut self,
        conn: &Connection,
        _: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        let mut shared = self.shared.borrow_mut();
        let Shared { gpu, windows, .. } = &mut *shared;
        let Some(app_window) = windows.values_mut().find(|w| w.window == *window) else {
            return;
        };
        let (default_w, default_h) = app_window.default_size;
        let canvas = &mut app_window.canvas;
        canvas.width = configure.new_size.0.map_or(default_w, |w| w.get() as i32);
        canvas.height = configure.new_size.1.map_or(default_h, |h| h.get() as i32);
        canvas.resize_gl(conn, gpu);
        canvas.configured = true;
        self.needs_redraw = true;
    }
}

impl SeatHandler for WaylandApp {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            if let Ok(kbd) = self.seat_state.get_keyboard(qh, &seat, None) {
                self.keyboard = Some(kbd);
            }
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            if let Ok(ptr) = self.seat_state.get_pointer(qh, &seat) {
                self.pointer = Some(ptr);
            }
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(kbd) = self.keyboard.take() {
                kbd.release();
            }
        }
        if capability == Capability::Pointer {
            if let Some(ptr) = self.pointer.take() {
                ptr.release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
}

impl KeyboardHandler for WaylandApp {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        surface: &WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
        self.keyboard_focus = Some(surface.clone());
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        surface: &WlSurface,
        _: u32,
    ) {
        if self.keyboard_focus.as_ref() == Some(surface) {
            self.keyboard_focus = None;
        }
    }

    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.with_focused_input(|input| input.key(event.keysym, event.utf8.as_deref(), true));
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.with_focused_input(|input| input.key(event.keysym, event.utf8.as_deref(), false));
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        modifiers: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
        self.with_focused_input(|input| input.set_modifiers(modifiers));
    }

    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }
}

impl PointerHandler for WaylandApp {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let pos = egui::pos2(event.position.0 as f32, event.position.1 as f32);
            self.with_canvas(
                |c| c.surface == event.surface,
                |canvas, _| {
                    let input = &mut canvas.input;
                    match event.kind {
                        PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                            input.pointer_moved(pos);
                        }
                        PointerEventKind::Leave { .. } => input.pointer_left(),
                        PointerEventKind::Press { button, .. } => {
                            if let Some(b) = map_button(button) {
                                input.pointer_moved(pos);
                                input.pointer_button(b, true);
                            }
                        }
                        PointerEventKind::Release { button, .. } => {
                            if let Some(b) = map_button(button) {
                                input.pointer_button(b, false);
                            }
                        }
                        PointerEventKind::Axis {
                            horizontal,
                            vertical,
                            ..
                        } => {
                            // Wayland axis is positive-down; egui scroll is positive-up.
                            let delta =
                                egui::vec2(-horizontal.absolute as f32, -vertical.absolute as f32);
                            input.scroll(delta);
                        }
                    }
                },
            );
        }
    }
}

/// Linux evdev button codes -> egui pointer buttons.
fn map_button(code: u32) -> Option<egui::PointerButton> {
    match code {
        0x110 => Some(egui::PointerButton::Primary), // BTN_LEFT
        0x111 => Some(egui::PointerButton::Secondary), // BTN_RIGHT
        0x112 => Some(egui::PointerButton::Middle),  // BTN_MIDDLE
        _ => None,
    }
}

impl OutputHandler for WaylandApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {
        self.update_available_monitors();
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {
        self.update_available_monitors();
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {
        self.update_available_monitors();
    }
}

impl Dispatch<WpFractionalScaleV1, ()> for WaylandApp {
    fn event(
        state: &mut Self,
        proxy: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _: &(),
        conn: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // The compositor reports the preferred scale in 120ths.
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            let new_scale = scale as f64 / 120.0;
            state.with_canvas(
                |c| c.fractional.as_ref() == Some(proxy),
                |canvas, gpu| {
                    if new_scale > 0.0 && new_scale != canvas.scale {
                        canvas.scale = new_scale;
                        if canvas.configured {
                            canvas.resize_gl(conn, gpu);
                        }
                    }
                },
            );
        }
    }
}

delegate_noop!(WaylandApp: ignore WpFractionalScaleManagerV1);
delegate_noop!(WaylandApp: ignore WpViewporter);
delegate_noop!(WaylandApp: ignore WpViewport);

impl ProvidesRegistryState for WaylandApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

delegate_registry!(WaylandApp);
delegate_dispatch2!(WaylandApp);
