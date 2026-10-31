//! External overlay control: visibility override and the command channel.

use std::sync::Mutex;

/// Manual override of the layer-driven overlay visibility.
///
/// The override lasts until it is changed again and is never persisted, so a
/// read-only declarative configuration stays authoritative.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum OverlayOverride {
    /// Visibility follows held layers, the visible-layer mask, and the timeout.
    #[default]
    Auto,
    /// Visible while a keyboard is connected, regardless of layers.
    Shown,
    /// Hidden, except while a window needs the overlay.
    Hidden,
}

impl OverlayOverride {
    /// Resolves a command against the currently effective visibility.
    ///
    /// `Toggle` flips the overlay as the user sees it, so it needs no client
    /// state of its own.
    pub fn apply(self, command: OverlayCommand, currently_visible: bool) -> Self {
        match command {
            OverlayCommand::Toggle if currently_visible => Self::Hidden,
            OverlayCommand::Toggle => Self::Shown,
            OverlayCommand::Show => Self::Shown,
            OverlayCommand::Hide => Self::Hidden,
            OverlayCommand::Auto => Self::Auto,
            OverlayCommand::Settings => self,
        }
    }
}

/// A control request arriving from outside the UI loop.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OverlayCommand {
    Toggle,
    Show,
    Hide,
    Auto,
    Settings,
}

impl OverlayCommand {
    /// Wire and command-line spelling of the command.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Toggle => "toggle",
            Self::Show => "show",
            Self::Hide => "hide",
            Self::Auto => "auto",
            Self::Settings => "settings",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "toggle" => Some(Self::Toggle),
            "show" => Some(Self::Show),
            "hide" => Some(Self::Hide),
            "auto" => Some(Self::Auto),
            "settings" => Some(Self::Settings),
            _ => None,
        }
    }
}

/// Commands handed to the UI loop by the desktop runner.
#[derive(Default)]
pub struct CommandQueue(Mutex<Vec<OverlayCommand>>);

impl CommandQueue {
    pub fn push(&self, command: OverlayCommand) {
        if let Ok(mut pending) = self.0.lock() {
            pending.push(command);
        }
    }

    /// Removes and returns every command received since the last drain.
    pub fn drain(&self) -> Vec<OverlayCommand> {
        self.0
            .lock()
            .map(|mut pending| std::mem::take(&mut *pending))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_spelling_round_trips() {
        for command in [
            OverlayCommand::Toggle,
            OverlayCommand::Show,
            OverlayCommand::Hide,
            OverlayCommand::Auto,
            OverlayCommand::Settings,
        ] {
            assert_eq!(OverlayCommand::parse(command.as_str()), Some(command));
        }
    }

    #[test]
    fn unknown_commands_are_rejected() {
        assert_eq!(OverlayCommand::parse("explode"), None);
        assert_eq!(OverlayCommand::parse(""), None);
    }

    #[test]
    fn toggle_flips_the_effective_visibility() {
        assert_eq!(
            OverlayOverride::Auto.apply(OverlayCommand::Toggle, true),
            OverlayOverride::Hidden
        );
        assert_eq!(
            OverlayOverride::Auto.apply(OverlayCommand::Toggle, false),
            OverlayOverride::Shown
        );
        assert_eq!(
            OverlayOverride::Shown.apply(OverlayCommand::Toggle, true),
            OverlayOverride::Hidden
        );
        assert_eq!(
            OverlayOverride::Hidden.apply(OverlayCommand::Toggle, false),
            OverlayOverride::Shown
        );
    }

    #[test]
    fn explicit_commands_set_the_override() {
        assert_eq!(
            OverlayOverride::Auto.apply(OverlayCommand::Show, false),
            OverlayOverride::Shown
        );
        assert_eq!(
            OverlayOverride::Shown.apply(OverlayCommand::Hide, true),
            OverlayOverride::Hidden
        );
        assert_eq!(
            OverlayOverride::Hidden.apply(OverlayCommand::Auto, false),
            OverlayOverride::Auto
        );
    }

    #[test]
    fn settings_request_leaves_the_override_alone() {
        assert_eq!(
            OverlayOverride::Hidden.apply(OverlayCommand::Settings, false),
            OverlayOverride::Hidden
        );
    }

    #[test]
    fn draining_empties_the_queue() {
        let queue = CommandQueue::default();
        queue.push(OverlayCommand::Show);
        queue.push(OverlayCommand::Hide);
        assert_eq!(queue.drain(), [OverlayCommand::Show, OverlayCommand::Hide]);
        assert!(queue.drain().is_empty());
    }
}
