//! Interface to request window changes from the host platform.

/// Interface provided by the window host to control overlay passthrough.
pub trait OverlayHost {
    fn set_passthrough(&mut self, enabled: bool);
}
