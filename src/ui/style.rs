use iced::{
    Background, Border, Color, Theme,
    border::Radius,
    widget::{button, container, text_input},
};

// move to a config file to allow user customi(s)(z)ation
pub const BACKGROUND: Color = Color::from_rgb(0.10, 0.11, 0.13);
pub const SURFACE: Color = Color::from_rgb(0.14, 0.15, 0.18);
pub const SURFACE_SELECTED: Color = Color::from_rgb(0.22, 0.24, 0.29);
pub const TEXT: Color = Color::from_rgb(0.93, 0.94, 0.96);
pub const MUTED: Color = Color::from_rgb(0.62, 0.65, 0.70);
pub const ACCENT: Color = Color::from_rgb(0.45, 0.66, 0.95);
pub const ERROR: Color = Color::from_rgb(0.95, 0.38, 0.38);
pub const SURFACE_ERROR: Color = Color::from_rgb(0.34, 0.16, 0.18);
pub const RADIUS: f32 = 12.0;
pub const RESULT_ROW_HEIGHT: f32 = 56.0;
pub const PADDING: u16 = 20;
pub const GAP: f32 = 10.0;

pub fn background(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BACKGROUND)),
        text_color: Some(TEXT),
        ..Default::default()
    }
}

pub fn panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: Border {
            radius: Radius::new(RADIUS),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn search_input(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = text_input::default(_theme, status);
    style.background = Background::Color(SURFACE);
    style.border.radius = Radius::new(RADIUS);
    style.border.color = ACCENT;
    style.border.width = 1.0;
    style.icon = MUTED;
    style.placeholder = MUTED;
    style.value = TEXT;
    style.selection = ACCENT;
    style
}

pub fn result_button(
    selected: bool,
    failed: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let highlighted = selected || matches!(status, button::Status::Hovered);

        button::Style {
            background: Some(Background::Color(if failed {
                SURFACE_ERROR
            } else if highlighted {
                SURFACE_SELECTED
            } else {
                SURFACE
            })),
            text_color: TEXT,
            border: Border {
                radius: Radius::new(RADIUS),
                ..Default::default()
            },
            shadow: if matches!(status, button::Status::Hovered) {
                iced::Shadow {
                    color: Color::BLACK,
                    offset: iced::Vector::new(0.0, 2.0),
                    blur_radius: 8.0,
                }
            } else {
                Default::default()
            },
            snap: false,
        }
    }
}
