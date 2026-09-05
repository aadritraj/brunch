use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::OsString,
    fs,
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

// icon paths are resolved once at scan time; this only wraps the resolved
// path in an iced handle and never touches the filesystem
pub fn icon(path: Option<&Path>) -> Option<Icon> {
    let path = path?;
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("svg") => Some(Icon::Svg(svg::Handle::from_path(path))),
        Some("png" | "jpg" | "jpeg") => Some(Icon::Raster(image::Handle::from_path(path))),
        _ => None,
    }
}

pub struct IconResolver {
    data_roots: Vec<PathBuf>,
    listings: HashMap<PathBuf, Listing>,
}

impl Default for IconResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl IconResolver {
    pub fn new() -> Self {
        Self::from_data_roots(data_roots())
    }

    pub fn from_data_roots(data_roots: Vec<PathBuf>) -> Self {
        Self {
            data_roots,
            listings: HashMap::new(),
        }
    }

    pub fn resolve(&mut self, entry: &DesktopEntry) -> Option<PathBuf> {
        let icon = entry.icon()?;
        let icon = Path::new(icon);
        if icon.is_absolute() {
            return icon.is_file().then(|| icon.to_path_buf());
        }
        let data_roots = self.data_roots.clone();
        self.find_icon_path(icon, &data_roots)
            .or_else(|| self.find_flatpak_icon_path(entry, icon))
    }

    fn find_icon_path(&mut self, icon: &Path, data_roots: &[PathBuf]) -> Option<PathBuf> {
        let mut icon_names = vec![icon.to_owned()];
        if !has_supported_extension(icon) {
            for extension in ["png", "svg", "jpg", "jpeg"] {
                icon_names.push(PathBuf::from(format!("{}.{extension}", icon.display())));
            }
        }
        for root in data_roots {
            if let Some(path) = self.find_in_directory(&root.join("pixmaps"), &icon_names) {
                return Some(path);
            }

            let icons_root = root.join("icons");
            let mut themes = self.directories(&icons_root);
            themes.sort();
            for theme in themes {
                if let Some(path) = self.find_in_directory(&theme, &icon_names) {
                    return Some(path);
                }

                let mut sizes = self.directories(&theme);
                sizes.sort();
                for size in sizes {
                    if let Some(path) = self.find_in_directory(&size.join("apps"), &icon_names) {
                        return Some(path);
                    }
                }
            }
        }
        None
    }

    fn find_flatpak_icon_path(&mut self, entry: &DesktopEntry, icon: &Path) -> Option<PathBuf> {
        let export_root = entry.path.ancestors().find(|ancestor| {
            ancestor.file_name().and_then(|name| name.to_str()) == Some("exports")
        })?;
        let flatpak_root = export_root.parent()?;
        if flatpak_root.file_name().and_then(|name| name.to_str()) != Some("flatpak") {
            return None;
        }

        let appid = flatpak_appid(&entry.path)?;
        let files_root = flatpak_root.join("app").join(appid);
        let mut deployments = Vec::new();
        for architecture in self.directories(&files_root) {
            for branch in self.directories(&architecture) {
                let active = branch.join("active");
                if active.is_dir() {
                    deployments.push(active.join("files/share"));
                }
            }
        }
        deployments.sort();
        deployments
            .iter()
            .find_map(|root| self.find_icon_path(icon, std::slice::from_ref(root)))
    }

    fn listing(&mut self, path: &Path) -> &Listing {
        self.listings
            .entry(path.to_path_buf())
            .or_insert_with(|| Listing::read(path))
    }

    fn directories(&mut self, path: &Path) -> Vec<PathBuf> {
        self.listing(path).directories.clone()
    }

    fn find_in_directory(&mut self, directory: &Path, names: &[PathBuf]) -> Option<PathBuf> {
        let files = &self.listing(directory).files;
        names
            .iter()
            .find(|name| files.contains(name.as_os_str()))
            .map(|name| directory.join(name))
    }
}

struct Listing {
    directories: Vec<PathBuf>,
    files: HashSet<OsString>,
}

impl Listing {
    fn read(path: &Path) -> Self {
        let mut listing = Self {
            directories: Vec::new(),
            files: HashSet::new(),
        };
        let Ok(children) = fs::read_dir(path) else {
            return listing;
        };
        for child in children.flatten() {
            let path = child.path();
            // metadata follows symlinks, matching the is_dir/is_file checks
            // this cache replaces
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            if metadata.is_dir() {
                listing.directories.push(path);
            } else if metadata.is_file() {
                listing.files.insert(child.file_name());
            }
        }
        listing
    }
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
