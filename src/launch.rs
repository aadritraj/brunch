// TODO: handle desktop entries with Terminal=true
use std::{
    io,
    path::PathBuf,
    process::{Command, Stdio},
};

use freedesktop_desktop_entry::DesktopEntry;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum LaunchAction {
    DesktopEntry(DesktopEntryLaunch),
}

impl From<DesktopEntryLaunch> for LaunchAction {
    fn from(entry: DesktopEntryLaunch) -> Self {
        Self::DesktopEntry(entry)
    }
}

#[derive(Debug, Clone)]
pub struct DesktopEntryLaunch {
    pub appid: String,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("could not parse Exec field: {0}")]
    InvalidExec(#[from] freedesktop_desktop_entry::ExecError),
    #[error("launch command is empty")]
    EmptyCommand,
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
            LaunchAction::DesktopEntry(desktop_entry) => {
                let (program, args) = desktop_entry
                    .args
                    .split_first()
                    .ok_or(LaunchError::EmptyCommand)?;
                let mut command = Command::new(program);
                command.args(args);
                command
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                if let Some(path) = &desktop_entry.working_directory {
                    command.current_dir(path);
                }
                command.spawn().map(|_| ()).map_err(LaunchError::from)
            }
        }
    }
}
