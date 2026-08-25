use freedesktop_desktop_entry::get_languages_from_env;
use iced::{
    Element, Fill, Length,
    widget::{button, column, container, image, row, scrollable, text, text_input},
};

use super::{Launcher, Message, RESULTS_ID, SEARCH_INPUT_ID};
use crate::applications::DesktopEntry;

pub fn render(launcher: &Launcher) -> Element<'_, Message> {
    let locales = get_languages_from_env();
    let search = text_input("Search applications…", &launcher.query)
        .id(iced::widget::Id::new(SEARCH_INPUT_ID))
        .on_input(Message::QueryChanged)
        .on_submit(Message::Submit)
        .padding(12)
        .width(Fill)
        .style(super::style::search_input);

    let results = launcher
        .matches
        .iter()
        .enumerate()
        .map(|(position, index)| {
            result_row(
                &launcher.scanner.entries()[*index],
                *index,
                position == launcher.selected,
                &locales,
            )
        })
        .collect::<Vec<_>>();
    let content = if results.is_empty() {
        column![
            text("No applications found")
                .size(16)
                .color(super::style::MUTED)
        ]
        .padding(24)
        .align_x(iced::Alignment::Center)
    } else {
        column(results).spacing(super::style::GAP)
    };
    let content = container(content)
        .padding(iced::Padding::default().bottom(super::style::RESULTS_BOTTOM_PADDING));
    let results = scrollable(content)
        .id(iced::widget::Id::new(RESULTS_ID))
        .height(Fill);
    let body = column![search, results]
        .spacing(super::style::GAP)
        .padding(super::style::PADDING);
    container(
        container(body)
            .style(super::style::panel)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .style(super::style::background)
    .width(Fill)
    .height(Fill)
    .into()
}

fn result_row<'a>(
    entry: &'a DesktopEntry,
    index: usize,
    selected: bool,
    locales: &[String],
) -> Element<'a, Message> {
    let icon: Element<'_, Message> = match super::icons::handle(entry) {
        Some(handle) => image(handle).width(36).height(36).into(),
        None => container(text("●").size(22).color(super::style::ACCENT))
            .width(36)
            .height(36)
            .center_x(36)
            .center_y(36)
            .into(),
    };
    let name = entry.name(locales).unwrap_or_default();
    let comment = entry.comment(locales).unwrap_or_default();
    button(
        row![
            icon,
            column![
                text(name.into_owned()).size(16),
                text(comment.into_owned())
                    .size(13)
                    .color(super::style::MUTED)
            ]
            .spacing(2)
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::Select(index))
    .width(Fill)
    .height(super::style::RESULT_ROW_HEIGHT)
    .padding(10)
    .style(super::style::result_button(selected))
    .into()
}
