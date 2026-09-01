use std::{
    env, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use freedesktop_desktop_entry::DesktopEntry;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum LaunchAction {
    DesktopEntry(DesktopEntryLaunch),
    DesktopAction(DesktopActionLaunch),
}

impl From<DesktopEntryLaunch> for LaunchAction {
    fn from(entry: DesktopEntryLaunch) -> Self {
        Self::DesktopEntry(entry)
    }
}

impl From<DesktopActionLaunch> for LaunchAction {
    fn from(action: DesktopActionLaunch) -> Self {
        Self::DesktopAction(action)
    }
}

#[derive(Debug, Clone)]
pub struct DesktopEntryLaunch {
    pub appid: String,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub terminal: bool,
}

#[derive(Debug, Clone)]
pub struct DesktopActionLaunch {
    pub appid: String,
    pub action_id: String,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub terminal: bool,
}

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("could not parse Exec field: {0}")]
    InvalidExec(#[from] freedesktop_desktop_entry::ExecError),
    #[error("launch command is empty")]
    EmptyCommand,
    #[error("no terminal emulator is available")]
    NoTerminal,
    #[error("could not spawn application: {0}")]
    Spawn(#[from] io::Error),
}

impl DesktopEntryLaunch {
    pub fn from_entry(entry: &DesktopEntry) -> Result<Self, LaunchError> {
        let appid = entry.id().to_owned();
        let args = entry.parse_exec()?;
        let working_directory = entry.path().map(PathBuf::from);

        Ok(Self {
            appid,
            args,
            working_directory,
            terminal: entry.terminal(),
        })
    }
}

impl DesktopActionLaunch {
    pub fn from_entry(entry: &DesktopEntry, action_id: &str) -> Result<Self, LaunchError> {
        let appid = entry.id().to_owned();
        let args = entry.parse_exec_action(action_id)?;
        let working_directory = entry.path().map(PathBuf::from);

        Ok(Self {
            appid,
            action_id: action_id.to_owned(),
            args,
            working_directory,
            terminal: entry.terminal(),
        })
    }
}

pub trait ActionExecutor {
    fn execute(&self, action: &LaunchAction) -> Result<(), LaunchError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemExecutor;

impl ActionExecutor for SystemExecutor {
    fn execute(&self, action: &LaunchAction) -> Result<(), LaunchError> {
        match action {
            LaunchAction::DesktopEntry(desktop_entry) => Self::spawn(
                &desktop_entry.args,
                desktop_entry.working_directory.as_deref(),
                desktop_entry.terminal,
            ),
            LaunchAction::DesktopAction(desktop_action) => Self::spawn(
                &desktop_action.args,
                desktop_action.working_directory.as_deref(),
                desktop_action.terminal,
            ),
        }
    }
}

impl SystemExecutor {
    fn spawn(
        args: &[String],
        working_directory: Option<&Path>,
        terminal: bool,
    ) -> Result<(), LaunchError> {
        let (program, args) = args.split_first().ok_or(LaunchError::EmptyCommand)?;
        let mut command = if terminal {
            let terminal = resolve_terminal().ok_or(LaunchError::NoTerminal)?;
            let mut command = Command::new(terminal.executable);
            command.args(terminal.prefix);
            command.arg(program).args(args);
            command
        } else {
            let mut command = Command::new(program);
            command.args(args);
            command
        };
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // gnome-terminal does not respect this! they opt to have a --working-directory flag instead
        // i will not bother.
        if let Some(path) = working_directory {
            command.current_dir(path);
        }
        command.spawn().map(|_| ()).map_err(LaunchError::from)
    }
}

#[derive(Debug)]
struct TerminalCommand {
    executable: PathBuf,
    prefix: Vec<String>,
}

fn resolve_terminal() -> Option<TerminalCommand> {
    let mut candidates = env::var_os("TERMINAL")
        .map(PathBuf::from)
        .into_iter()
        .chain(std::iter::once(PathBuf::from("x-terminal-emulator")))
        .chain(
            [
                "ghostty",
                "kitty",
                "alacritty",
                "konsole",
                "gnome-terminal",
                "xfce4-terminal",
            ]
            .into_iter()
            .map(PathBuf::from),
        );

    candidates.find_map(|candidate| {
        let executable = find_executable(&candidate)?;
        let name = candidate.file_name()?.to_str()?;
        let prefix = terminal_prefix(name)
            .iter()
            .map(|arg| (*arg).to_owned())
            .collect();
        Some(TerminalCommand { executable, prefix })
    })
}

fn find_executable(candidate: &Path) -> Option<PathBuf> {
    if candidate.is_absolute() || candidate.components().count() > 1 {
        return candidate.is_file().then(|| candidate.to_owned());
    }

    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|directory| directory.join(candidate))
        .find(|path| path.is_file())
}

// please standardise this
fn terminal_prefix(name: &str) -> &'static [&'static str] {
    match name {
        "kitty" | "gnome-terminal" => &["--"],
        _ => &["-e"],
    }
}
