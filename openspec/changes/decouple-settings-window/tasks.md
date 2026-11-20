## 1. Settings viewport

- [x] 1.1 Show settings and its dialogs through `show_viewport_immediate`
- [x] 1.2 Remove the shade and `OverlayHost::request_close`

## 2. Wayland host

- [x] 2.1 Share one EGL context and painter across surfaces
- [x] 2.2 Open egui viewports as xdg toplevels with their own input and scale
- [x] 2.3 Limit the overlay input region to its egui areas
- [x] 2.4 Verify on umbriel with `WAYLAND_DEBUG`: settings is a toplevel, a monitor change rebuilds only the layer surface, no protocol errors

## 3. Gate

- [x] 3.1 `nix flake check`
