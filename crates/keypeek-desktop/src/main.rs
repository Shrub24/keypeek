#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use keypeek_desktop::RunOptions;
use keypeek_presentation::{
    FileSettingsStore, OverlayCommand, ReadOnlySettingsStore, SettingsStore,
};
use keypeek_protocol::{discover_devices, os_layout, scan_all_hid};

/// What this invocation does: run the overlay, drive a running instance, or
/// configure one (starting it when none is running).
#[derive(Debug, PartialEq, Eq)]
enum Invocation {
    Run {
        config: Option<PathBuf>,
        open_settings: bool,
    },
    Control(OverlayCommand),
    AttachOrStart {
        config: Option<PathBuf>,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum Action {
    /// `--settings`: configure the running instance, or start one.
    Attach,
    /// Hotkey commands: they never start an instance.
    Control(OverlayCommand),
}

const USAGE: &str = "usage: keypeek [--config <path>] [--no-settings]\n       \
                     keypeek --settings [--config <path>]\n       \
                     keypeek --toggle|--show|--hide|--auto";

/// Parses the command line.
///
/// Exits with status 2 on an unknown argument, a missing value, two conflicting
/// actions, or a hotkey command combined with `--config`, so a typo never
/// silently starts KeyPeek with the wrong configuration.
fn parse_args() -> Invocation {
    parse(std::env::args().skip(1))
}

fn parse<I: Iterator<Item = String>>(args: I) -> Invocation {
    let mut args = args;
    let mut config = None;
    let mut action: Option<Action> = None;
    let mut no_settings = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => match args.next() {
                Some(path) => config = Some(PathBuf::from(path)),
                None => usage_error("--config requires a file path"),
            },
            "--no-settings" => no_settings = true,
            "--settings" => set_action(&mut action, Action::Attach),
            "--toggle" => set_action(&mut action, Action::Control(OverlayCommand::Toggle)),
            "--show" => set_action(&mut action, Action::Control(OverlayCommand::Show)),
            "--hide" => set_action(&mut action, Action::Control(OverlayCommand::Hide)),
            "--auto" => set_action(&mut action, Action::Control(OverlayCommand::Auto)),
            other => usage_error(&format!("unrecognized argument `{other}`")),
        }
    }

    if no_settings && action.is_some() {
        usage_error("--no-settings only applies when starting KeyPeek");
    }

    match action {
        // Attaching uses whatever configuration the running instance has; the
        // given one applies only if an instance has to be started.
        Some(Action::Attach) => Invocation::AttachOrStart { config },
        Some(Action::Control(command)) => {
            if config.is_some() {
                usage_error("--config cannot be combined with a control command");
            }
            Invocation::Control(command)
        }
        None => Invocation::Run {
            config,
            open_settings: !no_settings,
        },
    }
}

fn set_action(slot: &mut Option<Action>, action: Action) {
    if slot.replace(action).is_some() {
        usage_error("only one of --settings, --toggle, --show, --hide, --auto may be given");
    }
}

fn usage_error(message: &str) -> ! {
    eprintln!("KeyPeek: {message}\n{USAGE}");
    std::process::exit(2);
}

#[cfg(unix)]
fn send_control(command: OverlayCommand) -> Result<(), std::io::Error> {
    keypeek_desktop::ipc::send(command)
}

#[cfg(not(unix))]
fn send_control(_command: OverlayCommand) -> Result<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "overlay control is only available on Unix systems",
    ))
}

fn main() {
    let (config, open_settings) = match parse_args() {
        Invocation::Control(command) => {
            if let Err(e) = send_control(command) {
                eprintln!(
                    "KeyPeek: cannot send `{}` to {}: {e}",
                    command.as_str(),
                    socket_path_display()
                );
                std::process::exit(1);
            }
            return;
        }
        Invocation::AttachOrStart { config } => {
            // A launcher click configures what runs; only start when nothing does.
            if send_control(OverlayCommand::Settings).is_ok() {
                return;
            }
            (config, true)
        }
        Invocation::Run {
            config,
            open_settings,
        } => (config, open_settings),
    };

    if let Err(e) = run(config, open_settings) {
        eprintln!("KeyPeek: {e}");
        std::process::exit(1);
    }
}

#[cfg(unix)]
fn socket_path_display() -> String {
    keypeek_desktop::ipc::socket_path().display().to_string()
}

#[cfg(not(unix))]
fn socket_path_display() -> String {
    String::from("the control socket")
}

fn run(config: Option<PathBuf>, open_settings: bool) -> Result<(), Box<dyn std::error::Error>> {
    os_layout::init();
    let settings_store: Arc<dyn SettingsStore> = match config {
        Some(path) => Arc::new(ReadOnlySettingsStore::new(path)),
        None => Arc::new(FileSettingsStore::default()),
    };
    let available_devices = discover_devices(scan_all_hid());
    keypeek_desktop::run(
        settings_store,
        available_devices,
        RunOptions {
            force_x11: false,
            open_settings,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invocation(args: &[&str]) -> Invocation {
        parse(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn a_bare_invocation_runs_with_settings_open() {
        assert_eq!(
            invocation(&[]),
            Invocation::Run {
                config: None,
                open_settings: true
            }
        );
    }

    #[test]
    fn no_settings_suppresses_the_window() {
        assert_eq!(
            invocation(&["--no-settings"]),
            Invocation::Run {
                config: None,
                open_settings: false
            }
        );
    }

    #[test]
    fn settings_attaches_and_may_carry_a_configuration() {
        assert_eq!(
            invocation(&["--settings"]),
            Invocation::AttachOrStart { config: None }
        );
        assert_eq!(
            invocation(&["--config", "/etc/keypeek.ini", "--settings"]),
            Invocation::AttachOrStart {
                config: Some(PathBuf::from("/etc/keypeek.ini"))
            }
        );
    }

    #[test]
    fn hotkey_commands_parse() {
        for (flag, command) in [
            ("--toggle", OverlayCommand::Toggle),
            ("--show", OverlayCommand::Show),
            ("--hide", OverlayCommand::Hide),
            ("--auto", OverlayCommand::Auto),
        ] {
            assert_eq!(invocation(&[flag]), Invocation::Control(command), "{flag}");
        }
    }
}
