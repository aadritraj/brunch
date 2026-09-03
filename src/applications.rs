use std::os::unix::fs::PermissionsExt;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

pub use freedesktop_desktop_entry::DesktopEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopAction {
    pub id: String,
    pub name: String,
}

pub fn actions_for_entry<L: AsRef<str>>(entry: &DesktopEntry, locales: &[L]) -> Vec<DesktopAction> {
    entry
        .actions()
        .into_iter()
        .flatten()
        .filter_map(|id| {
            let name = entry.action_name(id, locales)?;
            let exec = entry.action_exec(id)?;
            if name.trim().is_empty() || exec.trim().is_empty() {
                return None;
            }
            Some(DesktopAction {
                id: id.to_owned(),
                name: name.into_owned(),
            })
        })
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct DesktopEntryScanner {
    entries: Vec<DesktopEntry>,
}

impl DesktopEntryScanner {
    pub fn discover() -> Self {
        Self::from_directories(freedesktop_desktop_entry::default_paths())
    }

    pub fn from_directories<I, P>(directories: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        let mut seen = HashMap::new();
        let mut entries = Vec::new();

        for directory in directories.into_iter().map(Into::into) {
            let Ok(files) = fs::read_dir(directory) else {
                continue;
            };
            let mut files = files.flatten().map(|file| file.path()).collect::<Vec<_>>();
            files.sort();
            for path in files {
                if path.extension().and_then(|extension| extension.to_str()) != Some("desktop") {
                    continue;
                }
                let Some(id) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                if seen.contains_key(id) {
                    continue;
                }
                let Ok(entry) = DesktopEntry::from_path(&path, None::<&[&str]>) else {
                    continue;
                };
                if entry.type_() != Some("Application")
                    || entry.hidden()
                    || entry.no_display()
                    || !should_show_entry(&entry)
                {
                    continue;
                }
                seen.insert(id.to_owned(), entries.len());
                entries.push(entry);
            }
        }

        Self { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn entries(&self) -> &[DesktopEntry] {
        &self.entries
    }

    pub fn search(&self, query: &str) -> Vec<&DesktopEntry> {
        let query = query.trim().to_lowercase();
        let locales: &[&str] = &[];
        self.entries
            .iter()
            .filter(|entry| {
                query.is_empty()
                    || entry
                        .name(locales)
                        .is_some_and(|name| name.to_lowercase().contains(&query))
                    || entry
                        .comment(locales)
                        .is_some_and(|comment| comment.to_lowercase().contains(&query))
                    || entry.keywords(locales).is_some_and(|keywords| {
                        keywords
                            .iter()
                            .any(|keyword| keyword.to_lowercase().contains(&query))
                    })
            })
            .collect()
    }
}

fn should_show_entry(entry: &DesktopEntry) -> bool {
    let desktops = freedesktop_desktop_entry::current_desktop().unwrap_or_default();

    if let Some(only_show_in) = entry.only_show_in()
        && !only_show_in.iter().any(|desktop| {
            desktops
                .iter()
                .any(|current| current.eq_ignore_ascii_case(desktop))
        })
    {
        return false;
    }

    if entry.not_show_in().is_some_and(|not_show_in| {
        not_show_in.iter().any(|desktop| {
            desktops
                .iter()
                .any(|current| current.eq_ignore_ascii_case(desktop))
        })
    }) {
        return false;
    }

    entry.try_exec().is_none_or(is_executable)
}

fn is_executable(command: &str) -> bool {
    let path = Path::new(command);
    if path.is_absolute() || command.contains('/') {
        return is_executable_file(path);
    }

    env::var_os("PATH").is_some_and(|path_var| {
        env::split_paths(&path_var).any(|directory| is_executable_file(&directory.join(command)))
    })
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    metadata.permissions().mode() & 0o111 != 0
}
