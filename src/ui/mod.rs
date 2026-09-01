mod icons;
mod style;
mod view;

use freedesktop_desktop_entry::get_languages_from_env;
use iced::keyboard::key::{Key, Named};
use iced::{Element, Subscription, Task, keyboard, widget, window};

use crate::{
    applications::{DesktopAction, DesktopEntry, DesktopEntryScanner, actions_for_entry},
    directories::AppDirectories,
    history::History,
    launch::{ActionExecutor, DesktopActionLaunch, DesktopEntryLaunch, SystemExecutor},
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
    SelectAction(usize),
    OpenActions,
    Dismiss,
    WindowEvent(window::Event),
    KeyPressed(Key),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Applications,
    Actions,
}

pub struct Launcher {
    scanner: DesktopEntryScanner,
    query: String,
    matches: Vec<usize>,
    selected: usize,
    selection: Option<usize>,
    selection_error: Option<String>,
    mode: ViewMode,
    actions: Vec<DesktopAction>,
    action_selected: usize,
    history: History,
    executor: SystemExecutor,
}

impl Launcher {
    fn new() -> (Self, Task<Message>) {
        let directories = AppDirectories::initialize().ok();
        let mut launcher = Self {
            scanner: DesktopEntryScanner::discover(),
            query: String::new(),
            matches: Vec::new(),
            selected: 0,
            selection: None,
            selection_error: None,
            mode: ViewMode::Applications,
            actions: Vec::new(),
            action_selected: 0,
            history: History::load(directories.map(|directories| directories.history_path())),
            executor: SystemExecutor,
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
            Message::MoveUp => {
                return match self.mode {
                    ViewMode::Applications => self.move_selection_and_scroll(-1),
                    ViewMode::Actions => {
                        self.action_selected = self.action_selected.saturating_sub(1);
                        Task::none()
                    }
                };
            }
            Message::MoveDown => {
                return match self.mode {
                    ViewMode::Applications => self.move_selection_and_scroll(1),
                    ViewMode::Actions => {
                        if !self.actions.is_empty() {
                            self.action_selected =
                                (self.action_selected + 1).min(self.actions.len() - 1);
                        }
                        Task::none()
                    }
                };
            }
            Message::Select(index) => {
                if self.mode != ViewMode::Applications {
                    return Task::none();
                }
                if self.select(index) {
                    return iced::exit();
                }
            }
            Message::SelectAction(index) => {
                if self.mode == ViewMode::Actions && self.select_action(index) {
                    return iced::exit();
                }
            }
            Message::OpenActions => self.open_actions(),
            Message::Dismiss => return iced::exit(),
            Message::WindowEvent(window::Event::Unfocused) => {
                return iced::exit();
            }
            Message::WindowEvent(_) => {}
            Message::KeyPressed(Key::Named(Named::ArrowUp)) => {
                return match self.mode {
                    ViewMode::Applications => self.move_selection_and_scroll(-1),
                    ViewMode::Actions => {
                        self.action_selected = self.action_selected.saturating_sub(1);
                        Task::none()
                    }
                };
            }
            Message::KeyPressed(Key::Named(Named::ArrowDown)) => {
                return match self.mode {
                    ViewMode::Applications => self.move_selection_and_scroll(1),
                    ViewMode::Actions => {
                        if !self.actions.is_empty() {
                            self.action_selected =
                                (self.action_selected + 1).min(self.actions.len() - 1);
                        }
                        Task::none()
                    }
                };
            }
            Message::KeyPressed(Key::Named(Named::Enter)) => match self.mode {
                ViewMode::Applications => {
                    if let Some(index) = self.matches.get(self.selected).copied()
                        && self.select(index)
                    {
                        return iced::exit();
                    }
                }
                ViewMode::Actions => {
                    if self.select_action(self.action_selected) {
                        return iced::exit();
                    }
                }
            },
            Message::KeyPressed(Key::Named(Named::Tab)) => {
                if self.mode == ViewMode::Applications {
                    self.open_actions();
                }
            }
            Message::KeyPressed(Key::Named(Named::Escape)) => {
                if self.mode == ViewMode::Actions {
                    self.mode = ViewMode::Applications;
                    self.selection_error = None;
                } else {
                    return iced::exit();
                }
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
        self.matches = search::fuzzy_applications(&self.scanner, &self.query, &self.history)
            .into_iter()
            .map(|result| result.index)
            .collect();
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
        self.selection_error = None;
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

    fn select(&mut self, index: usize) -> bool {
        self.selection = Some(index);
        self.selection_error = None;

        let Some(entry) = self.scanner.entries().get(index) else {
            self.selection_error = Some("Selected application is no longer available".into());
            return false;
        };
        match DesktopEntryLaunch::from_entry(entry) {
            Ok(action) => match self.executor.execute(&action.into()) {
                Ok(()) => {
                    self.history.record_launch(&entry.appid);
                    if let Err(error) = self.history.persist() {
                        eprintln!("warning: could not persist launch history: {error}");
                    }
                    true
                }
                Err(error) => {
                    self.selection_error = Some(error.to_string());
                    false
                }
            },
            Err(error) => {
                self.selection_error = Some(error.to_string());
                false
            }
        }
    }

    fn open_actions(&mut self) {
        let Some(index) = self.matches.get(self.selected).copied() else {
            return;
        };
        let Some(entry) = self.scanner.entries().get(index) else {
            return;
        };
        let locales = get_languages_from_env();
        let actions = actions_for_entry(entry, &locales);
        if actions.is_empty() {
            return;
        }
        self.actions = actions;
        self.action_selected = 0;
        self.selection_error = None;
        self.mode = ViewMode::Actions;
    }

    fn select_action(&mut self, index: usize) -> bool {
        let Some(parent_index) = self.matches.get(self.selected).copied() else {
            return false;
        };
        let Some(action) = self.actions.get(index) else {
            return false;
        };
        let Some(entry) = self.scanner.entries().get(parent_index) else {
            self.selection_error = Some("Selected application is no longer available".into());
            return false;
        };
        self.selection_error = None;
        match DesktopActionLaunch::from_entry(entry, &action.id) {
            Ok(launch) => match self.executor.execute(&launch.into()) {
                Ok(()) => {
                    self.history.record_launch(&entry.appid);
                    if let Err(error) = self.history.persist() {
                        eprintln!("warning: could not persist launch history: {error}");
                    }
                    true
                }
                Err(error) => {
                    self.selection_error = Some(error.to_string());
                    false
                }
            },
            Err(error) => {
                self.selection_error = Some(error.to_string());
                false
            }
        }
    }
}

pub fn run() -> Result<(), iced::Error> {
    iced::application(Launcher::new, Launcher::update, Launcher::view)
        .title("Brunch")
        .subscription(Launcher::subscription)
        .decorations(false)
        .resizable(false)
        .window_size([760.0, 520.0])
        .centered()
        .level(window::Level::AlwaysOnTop)
        .run()
}
