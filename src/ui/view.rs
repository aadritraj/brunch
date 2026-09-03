use freedesktop_desktop_entry::get_languages_from_env;
use iced::{
    Element, Fill, Length,
    widget::{button, column, container, image, row, scrollable, text, text_input},
};

use super::{Launcher, Message, RESULTS_ID, SEARCH_INPUT_ID};
use crate::applications::DesktopAction;
use crate::applications::DesktopEntry;

pub fn render(launcher: &Launcher) -> Element<'_, Message> {
    if launcher.mode == super::ViewMode::Actions {
        return render_actions(launcher);
    }
    let locales = get_languages_from_env();
    let search = text_input("Search applications…", &launcher.query)
        .id(iced::widget::Id::new(SEARCH_INPUT_ID))
        .on_input(Message::QueryChanged)
        .padding(12)
        .width(Fill)
        .style(|theme, status| super::style::search_input(&launcher.style, theme, status));

    let results = launcher
        .matches
        .iter()
        .enumerate()
        .map(|(position, index)| {
            result_row(
                &launcher.scanner.entries()[*index],
                *index,
                position == launcher.selected,
                launcher.selection == Some(*index) && launcher.selection_error.is_some(),
                &locales,
                &launcher.style,
            )
        })
        .collect::<Vec<_>>();
    let content = if results.is_empty() {
        column![
            text("No applications found")
                .size(16)
                .color(launcher.style.muted)
        ]
        .padding(24)
        .align_x(iced::Alignment::Center)
    } else {
        column(results).spacing(launcher.style.gap)
    };
    let error = launcher
        .selection_error
        .as_deref()
        .map(|message| text(message).size(13).color(launcher.style.error));
    let content = container(content);
    let results = scrollable(content)
        .id(iced::widget::Id::new(RESULTS_ID))
        .height(Fill);
    let body = if let Some(error) = error {
        column![search, error, results]
    } else {
        column![search, results]
    }
    .spacing(launcher.style.gap)
    .padding(launcher.style.padding);
    container(
        container(body)
            .style(|theme| super::style::panel(&launcher.style, theme))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .style(|theme| super::style::background(&launcher.style, theme))
    .width(Fill)
    .height(Fill)
    .into()
}

fn result_row<'a>(
    entry: &'a DesktopEntry,
    index: usize,
    selected: bool,
    failed: bool,
    locales: &[String],
    style: &'a crate::userconfig::Style,
) -> Element<'a, Message> {
    let icon: Element<'_, Message> = match super::icons::handle(entry) {
        Some(super::icons::Icon::Raster(handle)) => image(handle).width(36).height(36).into(),
        Some(super::icons::Icon::Svg(handle)) => {
            iced::widget::svg(handle).width(36).height(36).into()
        }
        None => container(text("●").size(22).color(style.accent))
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
                text(comment.into_owned()).size(13).color(style.muted)
            ]
            .spacing(2),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::Select(index))
    .width(Fill)
    .height(style.result_row_height)
    .padding(10)
    .style(super::style::result_button(style, selected, failed))
    .into()
}

fn render_actions(launcher: &Launcher) -> Element<'_, Message> {
    let Some(index) = launcher.matches.get(launcher.selected).copied() else {
        return container(text("[Esc] to go back")).into();
    };
    let entry = &launcher.scanner.entries()[index];
    let locales = get_languages_from_env();
    let icon: Element<'_, Message> = match super::icons::handle(entry) {
        Some(super::icons::Icon::Raster(handle)) => image(handle).width(48).height(48).into(),
        Some(super::icons::Icon::Svg(handle)) => {
            iced::widget::svg(handle).width(48).height(48).into()
        }
        None => container(text("●").size(28).color(launcher.style.accent))
            .width(48)
            .height(48)
            .center_x(48)
            .center_y(48)
            .into(),
    };
    let header = row![
        icon,
        column![
            text(entry.name(&locales).unwrap_or_default().into_owned()).size(20),
            text(entry.comment(&locales).unwrap_or_default().into_owned())
                .size(13)
                .color(launcher.style.muted),
        ]
        .spacing(2),
    ]
    .spacing(14)
    .align_y(iced::Alignment::Center);
    let actions = launcher
        .actions
        .iter()
        .enumerate()
        .map(|(position, action)| {
            action_row(
                action,
                position,
                position == launcher.action_selected,
                &launcher.style,
            )
        })
        .collect::<Vec<_>>();
    let error = launcher
        .selection_error
        .as_deref()
        .map(|message| text(message).size(13).color(launcher.style.error));
    let mut body = column![
        text("[Esc] to go back")
            .size(13)
            .color(launcher.style.muted),
        header,
        text("Actions").size(13).color(launcher.style.muted),
        column(actions).spacing(launcher.style.gap),
    ]
    .spacing(launcher.style.gap)
    .padding(launcher.style.padding);
    if let Some(error) = error {
        body = body.push(error);
    }
    container(
        container(body)
            .style(|theme| super::style::panel(&launcher.style, theme))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .style(|theme| super::style::background(&launcher.style, theme))
    .width(Fill)
    .height(Fill)
    .into()
}

fn action_row<'a>(
    action: &'a DesktopAction,
    index: usize,
    selected: bool,
    style: &'a crate::userconfig::Style,
) -> Element<'a, Message> {
    button(text(action.name.clone()).size(16))
        .on_press(Message::SelectAction(index))
        .width(Fill)
        .height(style.result_row_height)
        .padding(10)
        .style(super::style::result_button(style, selected, false))
        .into()
}
