use std::{fs, path::Path};

use iced::Color;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read config file: {0}")]
    Read(#[source] std::io::Error),
    #[error("could not write config file: {0}")]
    Write(#[source] std::io::Error),
    #[error("could not parse config file: {0}")]
    Parse(#[source] toml::de::Error),
    #[error("invalid style property: {0}")]
    InvalidStyle(String),
}

#[derive(Debug, Clone, Copy)]
pub struct UserConfig {
    pub style: Style,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            style: Style::default(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawUserConfig {
    style: StyleOverrides,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct StyleOverrides {
    pub background: Option<String>,
    pub surface: Option<String>,
    pub surface_selected: Option<String>,
    pub text: Option<String>,
    pub muted: Option<String>,
    pub accent: Option<String>,
    pub error: Option<String>,
    pub surface_error: Option<String>,
    pub radius: Option<f32>,
    pub result_row_height: Option<f32>,
    pub padding: Option<u16>,
    pub gap: Option<f32>,
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

impl Default for Style {
    fn default() -> Self {
        /*
        default values for styling, is used if config failed to load or parse
        also serialised into toml for the user config by default
        so technically changing this is a breaking change for old users
        but, i would argue that retaining what the user was used to is more important than new defaults
        */
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

impl UserConfig {
    pub fn write_defaults(path: &Path) -> Result<(), ConfigError> {
        use std::io::Write;
        let mut file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(()),
            Err(error) => return Err(ConfigError::Write(error)),
        };
        file.write_all(default_config().as_bytes())
            .map_err(ConfigError::Write)
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let source = fs::read_to_string(path).map_err(ConfigError::Read)?;
        let config: RawUserConfig = toml::from_str(&source).map_err(ConfigError::Parse)?;
        Ok(Self {
            style: config.style.resolve()?,
        })
    }
}

#[derive(Serialize)]
struct DefaultConfig {
    style: DefaultStyle,
}

#[derive(Serialize)]
struct DefaultStyle {
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

fn default_config() -> String {
    let style = UserConfig::default().style;
    toml::to_string_pretty(&DefaultConfig {
        style: DefaultStyle {
            background: color_hex(style.background),
            surface: color_hex(style.surface),
            surface_selected: color_hex(style.surface_selected),
            text: color_hex(style.text),
            muted: color_hex(style.muted),
            accent: color_hex(style.accent),
            error: color_hex(style.error),
            surface_error: color_hex(style.surface_error),
            radius: style.radius,
            result_row_height: style.result_row_height,
            padding: style.padding,
            gap: style.gap,
        },
    })
    .expect("default config should always serialize")
}

fn color_hex(color: Color) -> String {
    // stinky
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
        (color.a * 255.0).round() as u8
    )
}

impl StyleOverrides {
    fn resolve(&self) -> Result<Style, ConfigError> {
        let mut style = Style::default();
        for (target, value) in [
            (&mut style.background, &self.background),
            (&mut style.surface, &self.surface),
            (&mut style.surface_selected, &self.surface_selected),
            (&mut style.text, &self.text),
            (&mut style.muted, &self.muted),
            (&mut style.accent, &self.accent),
            (&mut style.error, &self.error),
            (&mut style.surface_error, &self.surface_error),
        ] {
            if let Some(value) = value {
                *target = parse_color(value)?;
            }
        }
        if let Some(value) = self.radius {
            validate_nonnegative("radius", value)?;
            style.radius = value;
        }
        if let Some(value) = self.result_row_height {
            validate_nonnegative("result_row_height", value)?;
            style.result_row_height = value;
        }
        if let Some(value) = self.padding {
            style.padding = value;
        }
        if let Some(value) = self.gap {
            validate_nonnegative("gap", value)?;
            style.gap = value;
        }
        Ok(style)
    }
}

fn validate_nonnegative(name: &str, value: f32) -> Result<(), ConfigError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(ConfigError::InvalidStyle(format!(
            "{name} must be finite and non-negative"
        )))
    }
}

fn parse_color(value: &str) -> Result<Color, ConfigError> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    let (hex, alpha) = match hex.len() {
        3 => (hex.chars().flat_map(|c| [c, c]).collect::<String>(), 255),
        6 => (hex.to_owned(), 255),
        8 => (
            hex[..6].to_owned(),
            u8::from_str_radix(&hex[6..], 16)
                .map_err(|_| ConfigError::InvalidStyle(format!("invalid color {value:?}")))?,
        ),
        _ => {
            return Err(ConfigError::InvalidStyle(format!(
                "invalid color {value:?}"
            )));
        }
    };
    let component = |start| {
        u8::from_str_radix(&hex[start..start + 2], 16)
            .map_err(|_| ConfigError::InvalidStyle(format!("invalid color {value:?}")))
    };
    Ok(Color::from_rgba8(
        component(0)?,
        component(2)?,
        component(4)?,
        alpha as u8 as f32 / 255.0,
    ))
}
