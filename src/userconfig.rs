use std::{
    fs, io,
    path::{Path, PathBuf},
};

use iced::Color;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: ron::error::SpannedError,
    },
    #[error("invalid style property {property}: {reason}")]
    InvalidStyle {
        property: &'static str,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Config {}

#[derive(Debug, Clone, Copy)]
pub struct UserConfig {
    pub config: Config,
    pub style: Style,
}

#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub background: Color,
    pub surface: Color,
    pub surface_selected: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub error: Color,
    pub surface_error: Color,
    pub radius: f32,
    pub result_row_height: f32,
    pub padding: u16,
    pub gap: f32,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            config: Config::default(),
            style: Style::default(),
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: Color::from_rgb(0.10, 0.11, 0.13),
            surface: Color::from_rgb(0.14, 0.15, 0.18),
            surface_selected: Color::from_rgb(0.22, 0.24, 0.29),
            text: Color::from_rgb(0.93, 0.94, 0.96),
            muted: Color::from_rgb(0.62, 0.65, 0.70),
            accent: Color::from_rgb(0.45, 0.66, 0.95),
            error: Color::from_rgb(0.95, 0.38, 0.38),
            surface_error: Color::from_rgb(0.34, 0.16, 0.18),
            radius: 12.0,
            result_row_height: 56.0,
            padding: 20,
            gap: 10.0,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct StyleFile {
    background: Option<String>,
    surface: Option<String>,
    surface_selected: Option<String>,
    text: Option<String>,
    muted: Option<String>,
    accent: Option<String>,
    error: Option<String>,
    surface_error: Option<String>,
    radius: Option<f32>,
    result_row_height: Option<f32>,
    padding: Option<u16>,
    gap: Option<f32>,
}

#[derive(Debug, Serialize)]
struct StyleDefaults {
    background: String,
    surface: String,
    surface_selected: String,
    text: String,
    muted: String,
    accent: String,
    error: String,
    surface_error: String,
    radius: f32,
    result_row_height: f32,
    padding: u16,
    gap: f32,
}

impl UserConfig {
    pub fn load(config_path: &Path, style_path: &Path) -> Self {
        let config = match load_ron(config_path) {
            Ok(Some(config)) => config,
            Ok(None) => Config::default(),
            Err(error) => {
                eprintln!("warning: {error}; using default config");
                Config::default()
            }
        };
        let style = match load_style(style_path) {
            Ok(style) => style,
            Err(error) => {
                eprintln!("warning: {error}; using default style");
                Style::default()
            }
        };
        Self { config, style }
    }

    pub fn write_defaults(config_path: &Path, style_path: &Path) -> Result<(), ConfigError> {
        create_default(config_path, &Config::default())?;
        create_default(style_path, &StyleDefaults::from(Style::default()))
    }
}

fn load_ron<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, ConfigError> {
    let source = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.to_owned(),
                source,
            });
        }
    };
    if source.trim().is_empty() {
        return Ok(None);
    }
    ron::from_str(&source)
        .map(Some)
        .map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source,
        })
}

fn load_style(path: &Path) -> Result<Style, ConfigError> {
    let options =
        ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(Style::default()),
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.to_owned(),
                source,
            });
        }
    };
    if source.trim().is_empty() {
        return Ok(Style::default());
    }
    let file: StyleFile = options
        .from_str(&source)
        .map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source,
        })?;
    file.resolve()
}

fn create_default<T: Serialize>(path: &Path, value: &T) -> Result<(), ConfigError> {
    let text = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::new())
        .expect("defaults serialize");
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            io::Write::write_all(&mut file, text.as_bytes()).map_err(|source| ConfigError::Write {
                path: path.to_owned(),
                source,
            })
        }
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(source) => Err(ConfigError::Write {
            path: path.to_owned(),
            source,
        }),
    }
}

impl StyleFile {
    fn resolve(self) -> Result<Style, ConfigError> {
        let mut style = Style::default();
        apply_color(
            &mut style.background,
            "background",
            self.background.as_deref(),
        )?;
        apply_color(&mut style.surface, "surface", self.surface.as_deref())?;
        apply_color(
            &mut style.surface_selected,
            "surface_selected",
            self.surface_selected.as_deref(),
        )?;
        apply_color(&mut style.text, "text", self.text.as_deref())?;
        apply_color(&mut style.muted, "muted", self.muted.as_deref())?;
        apply_color(&mut style.accent, "accent", self.accent.as_deref())?;
        apply_color(&mut style.error, "error", self.error.as_deref())?;
        apply_color(
            &mut style.surface_error,
            "surface_error",
            self.surface_error.as_deref(),
        )?;
        for (name, value, target) in [
            ("radius", self.radius, &mut style.radius),
            (
                "result_row_height",
                self.result_row_height,
                &mut style.result_row_height,
            ),
            ("gap", self.gap, &mut style.gap),
        ] {
            if let Some(value) = value {
                if !value.is_finite() || value < 0.0 {
                    return Err(ConfigError::InvalidStyle {
                        property: name,
                        reason: "must be finite and non-negative".into(),
                    });
                }
                *target = value;
            }
        }
        if let Some(value) = self.padding {
            style.padding = value;
        }
        Ok(style)
    }
}

fn apply_color(
    target: &mut Color,
    property: &'static str,
    value: Option<&str>,
) -> Result<(), ConfigError> {
    if let Some(value) = value {
        *target = parse_color(property, value)?;
    }
    Ok(())
}

impl From<Style> for StyleDefaults {
    fn from(style: Style) -> Self {
        Self {
            background: hex(style.background),
            surface: hex(style.surface),
            surface_selected: hex(style.surface_selected),
            text: hex(style.text),
            muted: hex(style.muted),
            accent: hex(style.accent),
            error: hex(style.error),
            surface_error: hex(style.surface_error),
            radius: style.radius,
            result_row_height: style.result_row_height,
            padding: style.padding,
            gap: style.gap,
        }
    }
}

fn hex(color: Color) -> String {
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
        (color.a * 255.0).round() as u8
    )
}

fn parse_color(property: &'static str, value: &str) -> Result<Color, ConfigError> {
    let value = value.trim().trim_start_matches('#');
    let expanded = match value.len() {
        3 => value.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 | 8 => value.to_owned(),
        _ => return Err(invalid_color(property, value)),
    };
    let alpha = if expanded.len() == 8 {
        byte(property, &expanded[6..])?
    } else {
        255
    };
    Ok(Color::from_rgba8(
        byte(property, &expanded[0..2])?,
        byte(property, &expanded[2..4])?,
        byte(property, &expanded[4..6])?,
        f32::from(alpha) / 255.0,
    ))
}

fn byte(property: &'static str, value: &str) -> Result<u8, ConfigError> {
    u8::from_str_radix(value, 16).map_err(|_| invalid_color(property, value))
}
fn invalid_color(property: &'static str, value: &str) -> ConfigError {
    ConfigError::InvalidStyle {
        property,
        reason: format!("invalid hex color {value:?}"),
    }
}
