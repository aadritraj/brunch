use std::{
    env, fs,
    path::{Path, PathBuf},
};

use iced::widget::image::Handle;

use crate::applications::DesktopEntry;

pub fn handle(entry: &DesktopEntry) -> Option<Handle> {
    icon_path(entry.icon()?).map(Handle::from_path)
}

fn icon_path(icon: &str) -> Option<PathBuf> {
    let icon = Path::new(icon);
    if icon.is_absolute() {
        return icon.is_file().then(|| icon.to_path_buf());
    }

    find_icon_path(icon, data_roots())
}

fn data_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(data_home) = env::var("XDG_DATA_HOME") {
        roots.push(PathBuf::from(data_home));
    } else if let Ok(home) = env::var("HOME") {
        roots.push(PathBuf::from(home).join(".local/share"));
    }
    roots.extend(
        env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned())
            .split(':')
            .filter(|root| !root.is_empty())
            .map(PathBuf::from),
    );
    roots
}

fn find_icon_path(icon: &Path, data_roots: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    let mut icon_names = vec![icon.to_owned()];
    if icon.extension().is_none() {
        for extension in ["png", "svg", "jpg", "jpeg"] {
            icon_names.push(PathBuf::from(format!("{}.{extension}", icon.display())));
        }
    }
    for root in data_roots {
        if let Some(path) = find_in_directory(&root.join("pixmaps"), &icon_names) {
            return Some(path);
        }

        let icons_root = root.join("icons");
        let mut themes = directories(&icons_root);
        themes.sort();
        for theme in themes {
            if let Some(path) = find_in_directory(&theme, &icon_names) {
                return Some(path);
            }

            let mut sizes = directories(&theme);
            sizes.sort();
            for size in sizes {
                if let Some(path) = find_in_directory(&size.join("apps"), &icon_names) {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn directories(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

fn find_in_directory(directory: &Path, names: &[PathBuf]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| directory.join(name))
        .find(|candidate| candidate.is_file())
}
