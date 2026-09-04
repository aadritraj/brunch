use std::{
    env, fs,
    path::{Path, PathBuf},
};

use iced::{
    widget::image::Handle,
    widget::{image, svg},
};

use crate::applications::DesktopEntry;

pub enum Icon {
    Raster(Handle),
    Svg(svg::Handle),
}

pub fn handle(entry: &DesktopEntry) -> Option<Icon> {
    let icon = entry.icon()?;
    let path = icon_path(entry, icon)?;
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("svg") => Some(Icon::Svg(svg::Handle::from_path(path))),
        Some("png" | "jpg" | "jpeg") => Some(Icon::Raster(image::Handle::from_path(path))),
        _ => None,
    }
}

fn icon_path(entry: &DesktopEntry, icon: &str) -> Option<PathBuf> {
    let icon = Path::new(icon);
    if icon.is_absolute() {
        return icon.is_file().then(|| icon.to_path_buf());
    }

    let path = find_icon_path(icon, data_roots());
    path.or_else(|| find_flatpak_icon_path(entry, icon))
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
    if !has_supported_extension(icon) {
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

fn find_flatpak_icon_path(entry: &DesktopEntry, icon: &Path) -> Option<PathBuf> {
    let export_root = entry
        .path
        .ancestors()
        .find(|ancestor| ancestor.file_name().and_then(|name| name.to_str()) == Some("exports"))?;
    let flatpak_root = export_root.parent()?;
    if flatpak_root.file_name().and_then(|name| name.to_str()) != Some("flatpak") {
        return None;
    }

    let appid = flatpak_appid(&entry.path)?;
    let files_root = flatpak_root.join("app").join(appid);
    let mut deployments = Vec::new();
    for architecture in directories(&files_root) {
        for branch in directories(&architecture) {
            let active = branch.join("active");
            if active.is_dir() {
                deployments.push(active.join("files/share"));
            }
        }
    }
    deployments.sort();
    deployments
        .into_iter()
        .find_map(|root| find_icon_path(icon, [root]))
}

// the deployment directory is named after the first path component below the
// applications directory; the crate's appid can differ for nested entries
fn flatpak_appid(path: &Path) -> Option<String> {
    let applications_root = path.ancestors().find(|ancestor| {
        ancestor.file_name().and_then(|name| name.to_str()) == Some("applications")
    })?;
    let mut component = path
        .strip_prefix(applications_root)
        .ok()?
        .components()
        .next()?
        .as_os_str()
        .to_str()?
        .to_owned();
    if let Some(stem) = component.strip_suffix(".desktop") {
        component = stem.to_owned();
    }
    Some(component)
}

fn has_supported_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("png" | "svg" | "jpg" | "jpeg" | "svgz")
    )
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
