use std::ffi::c_void;
use std::sync::Arc;

use egui_glow::glow;
use khronos_egl as egl;
use wayland_client::{protocol::wl_surface::WlSurface, Connection, Proxy};
use wayland_egl::WlEglSurface;

type Egl = egl::Instance<egl::Static>;

/// The one EGL context every surface is drawn with.
///
/// egui sends each texture once, so all surfaces must share a context (and a
/// painter): a context created later would never receive the font atlas.
pub struct EglContext {
    egl: Egl,
    display: egl::Display,
    config: egl::Config,
    context: egl::Context,
}

impl EglContext {
    pub fn new(conn: &Connection) -> Result<Self, String> {
        let egl = egl::Instance::new(egl::Static);

        // The EGL display is derived from the Wayland display pointer.
        let display_ptr = conn.backend().display_ptr() as *mut c_void;
        let display = unsafe { egl.get_display(display_ptr) }.ok_or("eglGetDisplay failed")?;
        egl.initialize(display)
            .map_err(|e| format!("eglInitialize failed: {e}"))?;
        egl.bind_api(egl::OPENGL_ES_API)
            .map_err(|e| format!("eglBindAPI failed: {e}"))?;

        let config_attribs = [
            egl::SURFACE_TYPE,
            egl::WINDOW_BIT,
            egl::RENDERABLE_TYPE,
            egl::OPENGL_ES2_BIT,
            egl::RED_SIZE,
            8,
            egl::GREEN_SIZE,
            8,
            egl::BLUE_SIZE,
            8,
            egl::ALPHA_SIZE,
            8,
            egl::NONE,
        ];
        let config = egl
            .choose_first_config(display, &config_attribs)
            .map_err(|e| format!("eglChooseConfig failed: {e}"))?
            .ok_or("no matching EGL config (need ARGB for transparency)")?;

        let context_attribs = [egl::CONTEXT_CLIENT_VERSION, 3, egl::NONE];
        let context = egl
            .create_context(display, config, None, &context_attribs)
            .map_err(|e| format!("eglCreateContext failed: {e}"))?;

        Ok(Self {
            egl,
            display,
            config,
            context,
        })
    }

    /// Loads the GL functions; needs the context current, which takes a surface.
    pub fn load_gl(&self) -> Arc<glow::Context> {
        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                self.egl
                    .get_proc_address(name)
                    .map_or(std::ptr::null(), |p| p as *const c_void)
            })
        };
        Arc::new(gl)
    }
}

/// The EGL window of one `wl_surface`, drawn with the shared [`EglContext`].
pub struct EglWindow {
    egl: Egl,
    display: egl::Display,
    surface: egl::Surface,
    wl_egl_surface: WlEglSurface,
}

impl EglWindow {
    /// `swap_interval` 0 keeps a hidden surface from blocking the shared draw loop
    /// on a frame callback that never comes.
    pub fn new(
        ctx: &EglContext,
        wl_surface: &WlSurface,
        width: i32,
        height: i32,
        swap_interval: i32,
    ) -> Result<Self, String> {
        let wl_egl_surface = WlEglSurface::new(wl_surface.id(), width.max(1), height.max(1))
            .map_err(|e| format!("WlEglSurface::new failed: {e}"))?;

        let surface = unsafe {
            ctx.egl.create_window_surface(
                ctx.display,
                ctx.config,
                wl_egl_surface.ptr() as egl::NativeWindowType,
                None,
            )
        }
        .map_err(|e| format!("eglCreateWindowSurface failed: {e}"))?;

        let window = Self {
            egl: egl::Instance::new(egl::Static),
            display: ctx.display,
            surface,
            wl_egl_surface,
        };
        window.make_current(ctx)?;
        let _ = ctx.egl.swap_interval(ctx.display, swap_interval);
        Ok(window)
    }

    pub fn make_current(&self, ctx: &EglContext) -> Result<(), String> {
        ctx.egl
            .make_current(
                ctx.display,
                Some(self.surface),
                Some(self.surface),
                Some(ctx.context),
            )
            .map_err(|e| format!("eglMakeCurrent failed: {e}"))
    }

    pub fn resize(&self, width: i32, height: i32) {
        self.wl_egl_surface
            .resize(width.max(1), height.max(1), 0, 0);
    }

    pub fn swap_buffers(&self) -> Result<(), String> {
        self.egl
            .swap_buffers(self.display, self.surface)
            .map_err(|e| format!("eglSwapBuffers failed: {e}"))
    }
}

impl Drop for EglWindow {
    fn drop(&mut self) {
        // Release the EGL surface before its wl_egl_window (dropped next) goes away.
        let _ = self.egl.destroy_surface(self.display, self.surface);
    }
}
