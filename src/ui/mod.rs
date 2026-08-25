mod icons;
mod style;
mod view;

use std::sync::{Arc, Mutex};

use iced::keyboard::key::{Key, Named};
use iced::{Element, Subscription, Task, keyboard, widget, window};

use crate::{
    applications::{DesktopEntry, DesktopEntryScanner},
    search,
};

pub const SEARCH_INPUT_ID: &str = "launcher-search";
pub const RESULTS_ID: &str = "launcher-results";
const RESULT_ROW_STEP: f32 = style::RESULT_ROW_HEIGHT + style::GAP;

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    MoveUp,
    MoveDown,
    Select(usize),
    Submit,
    Dismiss,
    WindowEvent(window::Event),
    KeyPressed(Key),
}

pub struct Launcher {
    scanner: DesktopEntryScanner,
    query: String,
    matches: Vec<usize>,
    selected: usize,
    selection: Option<usize>,
    selected_app: Arc<Mutex<Option<String>>>,
}

impl Launcher {
    fn new(selected_app: Arc<Mutex<Option<String>>>) -> (Self, Task<Message>) {
        let mut launcher = Self {
            scanner: DesktopEntryScanner::discover(),
            query: String::new(),
            matches: Vec::new(),
            selected: 0,
            selection: None,
            selected_app,
        };
        launcher.refresh_matches();
        (
            launcher,
            widget::operation::focus(widget::Id::new(SEARCH_INPUT_ID)),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::QueryChanged(query) => {
                self.query = query;
                self.refresh_matches();
                return widget::operation::scroll_to(
                    widget::Id::new(RESULTS_ID),
                    widget::operation::AbsoluteOffset { x: 0.0, y: 0.0 },
                );
            }
            Message::MoveUp => return self.move_selection_and_scroll(-1),
            Message::MoveDown => return self.move_selection_and_scroll(1),
            Message::Select(index) => {
                self.select(index);
                return iced::exit();
            }
            Message::Submit => {
                if let Some(index) = self.matches.get(self.selected).copied() {
                    self.select(index);
                    return iced::exit();
                }
            }
            Message::Dismiss => return iced::exit(),
            Message::WindowEvent(window::Event::Unfocused) => {
                return iced::exit();
            }
            Message::WindowEvent(_) => {}
            Message::KeyPressed(Key::Named(Named::ArrowUp)) => {
                return self.move_selection_and_scroll(-1);
            }
            Message::KeyPressed(Key::Named(Named::ArrowDown)) => {
                return self.move_selection_and_scroll(1);
            }
            Message::KeyPressed(Key::Named(Named::Enter)) => {
                if let Some(index) = self.matches.get(self.selected).copied() {
                    self.select(index);
                    return iced::exit();
                }
            }
            Message::KeyPressed(Key::Named(Named::Escape)) => {
                return iced::exit();
            }
            Message::KeyPressed(_) => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        view::render(self)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            window::events().map(|(_, event)| Message::WindowEvent(event)),
            iced::event::listen_with(|event, _status, _window| {
                if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
                    Some(Message::KeyPressed(key))
                } else {
                    None
                }
            }),
        ])
    }

    pub fn selected(&self) -> Option<&DesktopEntry> {
        self.selection
            .and_then(|index| self.scanner.entries().get(index))
    }

    fn refresh_matches(&mut self) {
        self.matches = search::fuzzy_applications(&self.scanner, &self.query)
            .into_iter()
            .filter_map(|entry| {
                self.scanner
                    .entries()
                    .iter()
                    .position(|candidate| candidate.appid == entry.appid)
            })
            .collect();
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
    }

    fn move_selection_and_scroll(&mut self, direction: isize) -> Task<Message> {
        if self.matches.is_empty() {
            return Task::none();
        }
        let previous = self.selected;
        self.selected = if direction.is_negative() {
            self.selected.saturating_sub(direction.unsigned_abs())
        } else {
            (self.selected + direction as usize).min(self.matches.len() - 1)
        };
        if self.selected == previous {
            return Task::none();
        }
        let offset = self.selected as f32 * RESULT_ROW_STEP;
        widget::operation::scroll_to(
            widget::Id::new(RESULTS_ID),
            widget::operation::AbsoluteOffset { x: 0.0, y: offset },
        )
    }

    fn select(&mut self, index: usize) {
        self.selection = Some(index);
        if let Some(entry) = self.scanner.entries().get(index)
            && let Ok(mut selected_app) = self.selected_app.lock()
        {
            *selected_app = Some(entry.appid.clone());
        }
    }
}

pub fn run() -> Result<Option<String>, iced::Error> {
    let selected_app = Arc::new(Mutex::new(None));
    let app_selected = Arc::clone(&selected_app);
    iced::application(
        move || Launcher::new(Arc::clone(&app_selected)),
        Launcher::update,
        Launcher::view,
    )
    .title("Brunch")
    .subscription(Launcher::subscription)
    .decorations(false)
    .resizable(false)
    .window_size([760.0, 520.0])
    .centered()
    .level(window::Level::AlwaysOnTop)
    .run()
    .map(|_| {
        selected_app
            .lock()
            .ok()
            .and_then(|selection| selection.clone())
    })
}
