use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::{
    collections::HashSet,
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
        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        let desktops = freedesktop_desktop_entry::current_desktop().unwrap_or_default();
        let path_dirs: Vec<PathBuf> = env::var_os("PATH")
            .map(|v| env::split_paths(&v).collect())
            .unwrap_or_default();

        for directory in directories.into_iter().map(Into::into) {
            let mut paths = Vec::new();
            collect_desktop_files(&directory, &mut paths);
            for path in paths {
                let Some(id) = desktop_id(&directory, &path) else {
                    continue;
                };
                if seen.contains(&id) {
                    continue;
                }
                let Ok(entry) = DesktopEntry::from_path(&path, None::<&[&str]>) else {
                    continue;
                };
                seen.insert(id);
                if entry.type_() != Some("Application")
                    || entry.hidden()
                    || entry.no_display()
                    || !should_show_entry(&entry, &desktops, &path_dirs)
                {
                    continue;
                }
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

fn collect_desktop_files(directory: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(root_metadata) = fs::metadata(directory) else {
        return;
    };
    if !root_metadata.is_dir() {
        return;
    }
    let mut visited = HashSet::new();
    visited.insert((root_metadata.dev(), root_metadata.ino()));

    // subdirectories are pushed in reverse so they pop in sorted order
    let mut stack = vec![directory.to_owned()];
    while let Some(directory) = stack.pop() {
        let Ok(children) = fs::read_dir(&directory) else {
            continue;
        };
        let mut children: Vec<_> = children.flatten().collect();
        children.sort_by_key(|child| child.file_name());

        for child in children.iter().rev() {
            let path = child.path();
            let Ok(file_type) = child.file_type() else {
                continue;
            };
            let is_directory = file_type.is_dir()
                || (file_type.is_symlink()
                    && fs::metadata(&path).is_ok_and(|metadata| metadata.is_dir()));
            if !is_directory {
                continue;
            }
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            if visited.insert((metadata.dev(), metadata.ino())) {
                stack.push(path);
            }
        }

        for child in &children {
            let path = child.path();
            if path.extension().and_then(|extension| extension.to_str()) == Some("desktop") {
                paths.push(path);
            }
        }
    }
}

// dedup key: the path relative to the applications directory with '/' replaced
// by '-'. unlike the spec's id, the .desktop suffix is kept here — it is only
// used for dedup, the crate strips it when deriving appid
fn desktop_id(directory: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(directory).ok()?;
    let mut id = String::new();
    for (position, component) in relative.components().enumerate() {
        if position > 0 {
            id.push('-');
        }
        id.push_str(component.as_os_str().to_str()?);
    }
    Some(id)
}

fn should_show_entry(entry: &DesktopEntry, desktops: &[String], path_dirs: &[PathBuf]) -> bool {
    if !desktops.is_empty()
        && let Some(only_show_in) = entry.only_show_in()
        && !only_show_in
            .iter()
            .filter(|desktop| !desktop.is_empty())
            .any(|desktop| {
                desktops
                    .iter()
                    .any(|current| current.eq_ignore_ascii_case(desktop))
            })
    {
        return false;
    }

    if !desktops.is_empty()
        && entry.not_show_in().is_some_and(|not_show_in| {
            not_show_in
                .iter()
                .filter(|desktop| !desktop.is_empty())
                .any(|desktop| {
                    desktops
                        .iter()
                        .any(|current| current.eq_ignore_ascii_case(desktop))
                })
        })
    {
        return false;
    }

    entry
        .try_exec()
        .is_none_or(|command| is_executable(command, path_dirs))
}

fn is_executable(command: &str, path_dirs: &[PathBuf]) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return false;
    }

    let exe = if trimmed.starts_with('"') || trimmed.starts_with('\'') {
        let quote = trimmed.as_bytes()[0] as char;
        trimmed[1..]
            .find(quote)
            .map(|end| trimmed[1..][..end].trim())
    } else {
        None
    }
    .unwrap_or_else(|| {
        trimmed
            .split_ascii_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(|c| c == '"' || c == '\'')
    });

    if exe.is_empty() {
        return false;
    }

    let path = Path::new(exe);
    if path.is_absolute() || exe.contains('/') {
        return is_executable_file(path);
    }

    path_dirs
        .iter()
        .any(|directory| is_executable_file(&directory.join(exe)))
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
