use crate::userconfig::Style;
use iced::{
    Background, Border, Color, Theme,
    border::Radius,
    widget::{button, container, text_input},
};

pub fn background(style: &Style, _theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(style.background)),
        text_color: Some(style.text),
        ..Default::default()
    }
}
pub fn panel(style: &Style, _theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(style.surface)),
        border: Border {
            radius: Radius::new(style.radius),
            ..Default::default()
        },
        ..Default::default()
    }
}
pub fn search_input(style: &Style, theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut result = text_input::default(theme, status);
    result.background = Background::Color(style.surface);
    result.border.radius = Radius::new(style.radius);
    result.border.color = style.accent;
    result.border.width = 1.0;
    result.icon = style.muted;
    result.placeholder = style.muted;
    result.value = style.text;
    result.selection = style.accent;
    result
}
pub fn result_button(
    style: &Style,
    selected: bool,
    failed: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    let surface = style.surface;
    let selected_surface = style.surface_selected;
    let error_surface = style.surface_error;
    let text = style.text;
    let radius = style.radius;
    move |_theme, status| {
        let highlighted = selected || matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(Background::Color(if failed {
                error_surface
            } else if highlighted {
                selected_surface
            } else {
                surface
            })),
            text_color: text,
            border: Border {
                radius: Radius::new(radius),
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
