mod capture;
mod config;
mod shortcuts;

use capture::{ActiveRecording, CaptureOutcome};
use config::{AppConfig, CaptureKind, CaptureSource};
use futures_util::stream::BoxStream;
use iced::{
    Alignment, Element, Length, Size, Subscription, Task, Theme, application, keyboard, time,
    widget::{
        button, button as button_widget, checkbox, column, container, pick_list, row, scrollable,
        slider, text, text_input,
    },
    window,
};
use shortcuts::{ShortcutAction, ShortcutEvent, ShortcutSpec};
use std::time::{Duration, Instant};

fn main() -> iced::Result {
    application(Taffy::default, update, view)
        .title("Taffy")
        .theme(app_theme)
        .subscription(subscription)
        .window(window::Settings {
            size: Size::new(560.0, 520.0),
            minimizable: false,
            icon: load_window_icon(),
            platform_specific: window::settings::PlatformSpecific {
                application_id: "taffy".into(),
                ..window::settings::PlatformSpecific::default()
            },
            ..window::Settings::default()
        })
        .run()
}

fn load_window_icon() -> Option<window::Icon> {
    window::icon::from_file_data(include_bytes!("../icon.png"), None).ok()
}

fn app_theme(_: &Taffy) -> Theme {
    Theme::TokyoNight
}

#[derive(Debug)]
struct Taffy {
    config: AppConfig,
    status: String,
    shortcut_status: String,
    is_busy: bool,
    active_recording: Option<ActiveRecording>,
    recording_started_at: Option<Instant>,
    recording_elapsed: Duration,
    show_menu: bool,
    show_preferences: bool,
    show_shortcuts: bool,
    start_shortcut_value: String,
    stop_shortcut_value: String,
    screenshot_shortcut_value: String,
    screenshot_directory_value: String,
    gif_directory_value: String,
    video_directory_value: String,
    shortcut_revision: u64,
    applied_shortcuts: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
enum Message {
    CaptureKindChanged(CaptureKind),
    CaptureSourceChanged(CaptureSource),
    FrameRateChanged(u32),
    StartDelayChanged(u32),
    StopDelayChanged(u32),
    ShowPointerChanged(bool),
    StartShortcutChanged(String),
    StopShortcutChanged(String),
    ScreenshotShortcutChanged(String),
    ScreenshotDirectoryChanged(String),
    GifDirectoryChanged(String),
    VideoDirectoryChanged(String),
    ToggleMenu,
    TogglePreferences,
    ToggleShortcuts,
    ApplyShortcutsPressed,
    StartPressed,
    StopPressed,
    KeyboardEvent(keyboard::Event),
    Tick(Instant),
    CaptureReady(std::result::Result<CaptureOutcome, String>),
    CaptureStopped(std::result::Result<std::path::PathBuf, String>),
    ShortcutEvent(ShortcutEvent),
}

impl Taffy {
    fn default() -> (Self, Task<Message>) {
        let config = config::load().unwrap_or_else(|_| AppConfig::default());
        (
            Self {
                start_shortcut_value: config.start_shortcut.clone(),
                stop_shortcut_value: config.stop_shortcut.clone(),
                screenshot_shortcut_value: config.screenshot_shortcut.clone(),
                screenshot_directory_value: config.screenshot_directory.display().to_string(),
                gif_directory_value: config.gif_directory.display().to_string(),
                video_directory_value: config.video_directory.display().to_string(),
                config,
                status: "Ready to capture".into(),
                shortcut_status: "Focused-window shortcuts are available whenever Taffy is active."
                    .into(),
                is_busy: false,
                active_recording: None,
                recording_started_at: None,
                recording_elapsed: Duration::ZERO,
                show_menu: false,
                show_preferences: false,
                show_shortcuts: false,
                shortcut_revision: 0,
                applied_shortcuts: Vec::new(),
            },
            Task::none(),
        )
    }

    fn persist(&mut self) {
        self.config.start_shortcut = self.start_shortcut_value.clone();
        self.config.stop_shortcut = self.stop_shortcut_value.clone();
        self.config.screenshot_shortcut = self.screenshot_shortcut_value.clone();
        self.config.screenshot_directory = self.screenshot_directory_value.trim().into();
        self.config.gif_directory = self.gif_directory_value.trim().into();
        self.config.video_directory = self.video_directory_value.trim().into();

        if let Err(error) = config::save(&self.config) {
            self.status = format!("Could not save settings: {error}");
        }
    }
}

fn subscription(app: &Taffy) -> Subscription<Message> {
    let shortcut_spec = ShortcutSpec {
        start: app.config.start_shortcut.clone(),
        stop: app.config.stop_shortcut.clone(),
        screenshot: app.config.screenshot_shortcut.clone(),
    };

    let mut subscriptions = vec![
        Subscription::run_with((app.shortcut_revision, shortcut_spec), shortcut_stream)
            .map(Message::ShortcutEvent),
        keyboard::listen().map(Message::KeyboardEvent),
    ];

    if app.active_recording.is_some() {
        subscriptions.push(time::every(Duration::from_millis(250)).map(Message::Tick));
    }

    Subscription::batch(subscriptions)
}

fn shortcut_stream(data: &(u64, ShortcutSpec)) -> BoxStream<'static, ShortcutEvent> {
    shortcuts::portal_shortcuts(data.1.clone())
}

fn update(app: &mut Taffy, message: Message) -> Task<Message> {
    match message {
        Message::CaptureKindChanged(kind) => {
            app.config.capture_kind = kind;
            app.persist();
            Task::none()
        }
        Message::CaptureSourceChanged(source) => {
            app.config.capture_source = source;
            app.persist();
            Task::none()
        }
        Message::FrameRateChanged(fps) => {
            app.config.frame_rate = fps;
            app.persist();
            Task::none()
        }
        Message::StartDelayChanged(delay) => {
            app.config.start_delay_secs = delay;
            app.persist();
            Task::none()
        }
        Message::StopDelayChanged(delay) => {
            app.config.stop_delay_secs = delay;
            app.persist();
            Task::none()
        }
        Message::ShowPointerChanged(value) => {
            app.config.show_pointer = value;
            app.persist();
            Task::none()
        }
        Message::StartShortcutChanged(value) => {
            app.start_shortcut_value = value;
            app.persist();
            Task::none()
        }
        Message::StopShortcutChanged(value) => {
            app.stop_shortcut_value = value;
            app.persist();
            Task::none()
        }
        Message::ScreenshotShortcutChanged(value) => {
            app.screenshot_shortcut_value = value;
            app.persist();
            Task::none()
        }
        Message::ScreenshotDirectoryChanged(value) => {
            app.screenshot_directory_value = value;
            app.persist();
            Task::none()
        }
        Message::GifDirectoryChanged(value) => {
            app.gif_directory_value = value;
            app.persist();
            Task::none()
        }
        Message::VideoDirectoryChanged(value) => {
            app.video_directory_value = value;
            app.persist();
            Task::none()
        }
        Message::ToggleMenu => {
            app.show_menu = !app.show_menu;
            Task::none()
        }
        Message::TogglePreferences => {
            app.show_preferences = !app.show_preferences;
            app.show_menu = false;
            if app.show_preferences {
                app.show_shortcuts = false;
            }
            Task::none()
        }
        Message::ToggleShortcuts => {
            app.show_shortcuts = !app.show_shortcuts;
            app.show_menu = false;
            if app.show_shortcuts {
                app.show_preferences = false;
            }
            Task::none()
        }
        Message::ApplyShortcutsPressed => {
            app.persist();
            app.shortcut_revision = app.shortcut_revision.wrapping_add(1);
            app.shortcut_status = "Refreshing global shortcuts…".into();
            Task::none()
        }
        Message::StartPressed => start_capture_task(app, app.config.capture_kind),
        Message::StopPressed => stop_capture_task(app),
        Message::KeyboardEvent(event) => handle_keyboard_event(app, event),
        Message::Tick(now) => {
            if let Some(started) = app.recording_started_at {
                app.recording_elapsed = now.saturating_duration_since(started);
            }
            Task::none()
        }
        Message::CaptureReady(result) => {
            app.is_busy = false;
            match result {
                Ok(CaptureOutcome::Finished(path)) => {
                    app.status = format!("Saved to {}", path.display());
                }
                Ok(CaptureOutcome::Recording(recording)) => {
                    let label = match recording.capture_kind {
                        CaptureKind::Gif => "GIF",
                        CaptureKind::Video => "video",
                        CaptureKind::Screenshot => "capture",
                    };
                    app.status = format!(
                        "Recording {label} to {}. Use your stop shortcut or the Stop button when you are done.",
                        recording.output_path.display()
                    );
                    app.active_recording = Some(recording);
                    app.recording_started_at = Some(Instant::now());
                    app.recording_elapsed = Duration::ZERO;
                }
                Err(error) => {
                    app.status = format!("Capture failed: {error}");
                    app.recording_started_at = None;
                    app.recording_elapsed = Duration::ZERO;
                }
            }
            Task::none()
        }
        Message::CaptureStopped(result) => {
            app.is_busy = false;
            app.recording_started_at = None;
            app.recording_elapsed = Duration::ZERO;
            match result {
                Ok(path) => app.status = format!("Saved to {}", path.display()),
                Err(error) => app.status = format!("Stop failed: {error}"),
            }
            Task::none()
        }
        Message::ShortcutEvent(event) => match event {
            ShortcutEvent::Status(status) => {
                app.shortcut_status = status;
                Task::none()
            }
            ShortcutEvent::Bound(shortcuts) => {
                app.applied_shortcuts = shortcuts;
                if !app.applied_shortcuts.is_empty() {
                    app.shortcut_status = "Global shortcuts are active.".into();
                } else {
                    app.shortcut_status =
                        "The desktop did not report any assigned global shortcuts.".into();
                }
                Task::none()
            }
            ShortcutEvent::Activated(action) => trigger_shortcut_action(app, action, "global"),
        },
    }
}

fn start_capture_task(app: &mut Taffy, capture_kind: CaptureKind) -> Task<Message> {
    if app.is_busy || (app.active_recording.is_some() && capture_kind != CaptureKind::Screenshot) {
        return Task::none();
    }

    app.is_busy = true;
    app.show_menu = false;

    let mut config = app.config.clone();
    config.capture_kind = capture_kind;

    app.status = match config.capture_kind {
        CaptureKind::Screenshot => "Preparing screenshot…".into(),
        CaptureKind::Gif => "Preparing GIF capture…".into(),
        CaptureKind::Video => "Preparing video capture…".into(),
    };

    Task::perform(
        async move {
            capture::begin_capture(config)
                .await
                .map_err(|e| format!("{e:#}"))
        },
        Message::CaptureReady,
    )
}

fn stop_capture_task(app: &mut Taffy) -> Task<Message> {
    let Some(recording) = app.active_recording.take() else {
        return Task::none();
    };

    app.is_busy = true;
    app.status = "Stopping capture…".into();

    Task::perform(
        async move {
            capture::stop_capture(recording)
                .await
                .map_err(|e| format!("{e:#}"))
        },
        Message::CaptureStopped,
    )
}

fn handle_keyboard_event(app: &mut Taffy, event: keyboard::Event) -> Task<Message> {
    let shortcut_spec = ShortcutSpec {
        start: app.config.start_shortcut.clone(),
        stop: app.config.stop_shortcut.clone(),
        screenshot: app.config.screenshot_shortcut.clone(),
    };

    let Some(action) = shortcuts::action_for_event(&shortcut_spec, &event) else {
        return Task::none();
    };

    trigger_shortcut_action(app, action, "focused")
}

fn trigger_shortcut_action(
    app: &mut Taffy,
    action: ShortcutAction,
    origin: &'static str,
) -> Task<Message> {
    match action {
        ShortcutAction::Start => {
            if app.config.capture_kind == CaptureKind::Screenshot {
                app.status = format!(
                    "The {origin} record shortcut is idle while Screenshot mode is selected. Use the screenshot shortcut instead."
                );
                Task::none()
            } else {
                start_capture_task(app, app.config.capture_kind)
            }
        }
        ShortcutAction::Stop => stop_capture_task(app),
        ShortcutAction::Screenshot => start_capture_task(app, CaptureKind::Screenshot),
    }
}

fn format_elapsed(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let minutes = seconds / 60;
    let remaining_seconds = seconds % 60;
    format!("{minutes:02}:{remaining_seconds:02}")
}

fn view(app: &Taffy) -> Element<'_, Message> {
    let title_row = row![
        button("≡").on_press(Message::ToggleMenu).padding([8, 12]),
        text("Taffy").size(28),
        text(if app.active_recording.is_some() {
            format!("REC {}", format_elapsed(app.recording_elapsed))
        } else {
            "Ready".into()
        })
        .size(14),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let whole_screen_selected = app.config.capture_source == CaptureSource::WholeScreen;
    let selection_selected = app.config.capture_source == CaptureSource::Interactive;

    let selection_row = row![
        button("Whole Screen")
            .style(if whole_screen_selected {
                button_widget::primary
            } else {
                button_widget::secondary
            })
            .on_press(Message::CaptureSourceChanged(CaptureSource::WholeScreen))
            .padding([14, 18]),
        button("Selection")
            .style(if selection_selected {
                button_widget::primary
            } else {
                button_widget::secondary
            })
            .on_press(Message::CaptureSourceChanged(CaptureSource::Interactive))
            .padding([14, 18]),
    ]
    .spacing(10);

    let quick_toggles = row![
        checkbox(app.config.show_pointer)
            .label("Show pointer")
            .on_toggle(Message::ShowPointerChanged),
        text(format!("{} fps", app.config.frame_rate)).size(14),
        text(format!("{}s delay", app.config.start_delay_secs)).size(14),
    ]
    .spacing(18)
    .align_y(Alignment::Center);

    let action_row = if app.active_recording.is_some() {
        row![
            button("Stop")
                .on_press_maybe((!app.is_busy).then_some(Message::StopPressed))
                .padding([12, 26])
        ]
        .spacing(12)
    } else if app.config.capture_kind == CaptureKind::Screenshot {
        row![]
    } else {
        row![
            button("Record")
                .on_press_maybe((!app.is_busy).then_some(Message::StartPressed))
                .padding([12, 26])
        ]
        .spacing(12)
    };

    let shortcut_hint = if app.config.capture_kind == CaptureKind::Screenshot {
        text(format!(
            "Use screenshot shortcut: {}",
            app.config.screenshot_shortcut
        ))
        .size(14)
    } else if app.active_recording.is_some() {
        text(format!("Use stop shortcut: {}", app.config.stop_shortcut)).size(14)
    } else {
        text(format!(
            "Use record shortcut: {}",
            app.config.start_shortcut
        ))
        .size(14)
    };

    let format_row = row![
        text("Format").width(Length::Fixed(96.0)),
        pick_list(
            &CaptureKind::ALL[..],
            Some(app.config.capture_kind),
            Message::CaptureKindChanged
        )
        .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let recording_banner: Element<'_, Message> = if app.active_recording.is_some() {
        container(
            row![
                text("Recording").size(16),
                text(format_elapsed(app.recording_elapsed)).size(16),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .padding([10, 14])
        .width(Length::Fill)
        .into()
    } else {
        container(text("")).into()
    };

    let status_block = column![text(&app.status).size(15), shortcut_status_view(app),].spacing(6);

    let mut content = column![
        title_row,
        recording_banner,
        format_row,
        selection_row,
        quick_toggles,
        action_row,
        shortcut_hint,
        status_block,
    ]
    .spacing(16);

    if app.show_menu {
        content = content.push(menu_view());
    }

    if app.show_preferences {
        content = content.push(preferences_view(app));
    }

    if app.show_shortcuts {
        content = content.push(shortcuts_view(app));
    }

    content = content.push(help_view());

    let scrolled = scrollable(content.padding(20).max_width(520))
        .width(Length::Fill)
        .height(Length::Fill);

    container(scrolled)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

fn shortcut_status_view(app: &Taffy) -> Element<'_, Message> {
    let parse_errors = shortcuts::parse_errors(&ShortcutSpec {
        start: app.config.start_shortcut.clone(),
        stop: app.config.stop_shortcut.clone(),
        screenshot: app.config.screenshot_shortcut.clone(),
    });

    let mut lines = vec![app.shortcut_status.clone()];

    if !app.applied_shortcuts.is_empty() {
        lines.push(
            app.applied_shortcuts
                .iter()
                .map(|(id, trigger)| format!("{id}: {trigger}"))
                .collect::<Vec<_>>()
                .join(" | "),
        );
    }

    if !parse_errors.is_empty() {
        lines.push(
            parse_errors
                .iter()
                .map(|error| format!("{} shortcut: {}", error.label, error.detail))
                .collect::<Vec<_>>()
                .join(" | "),
        );
    }

    lines.push("If the desktop ignores global shortcuts, Taffy only sees shortcuts while its own window is focused.".into());
    lines.push("On COSMIC right now, screenshots should be taken another way and recordings should generally be started and stopped from the Taffy UI.".into());

    text(lines.join("\n")).size(13).into()
}

fn menu_view<'a>() -> Element<'a, Message> {
    container(
        column![
            button("Preferences").on_press(Message::TogglePreferences),
            button("Keyboard Shortcuts").on_press(Message::ToggleShortcuts),
        ]
        .spacing(8),
    )
    .padding(12)
    .into()
}

fn preferences_view(app: &Taffy) -> Element<'_, Message> {
    let view = column![
        text("Preferences").size(22),
        row![
            text("Frame rate").width(Length::Fixed(140.0)),
            slider(1..=60, app.config.frame_rate, Message::FrameRateChanged).width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        row![
            text("Start delay").width(Length::Fixed(140.0)),
            slider(
                0..=10,
                app.config.start_delay_secs,
                Message::StartDelayChanged
            )
            .width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        row![
            text("Stop delay").width(Length::Fixed(140.0)),
            slider(
                0..=10,
                app.config.stop_delay_secs,
                Message::StopDelayChanged
            )
            .width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        text("Save folders").size(18),
        text_input("Screenshot folder", &app.screenshot_directory_value)
            .on_input(Message::ScreenshotDirectoryChanged)
            .padding(10),
        text_input("GIF folder", &app.gif_directory_value)
            .on_input(Message::GifDirectoryChanged)
            .padding(10),
        text_input("Video folder", &app.video_directory_value)
            .on_input(Message::VideoDirectoryChanged)
            .padding(10),
    ]
    .spacing(12);

    container(view).padding(14).into()
}

fn shortcuts_view(app: &Taffy) -> Element<'_, Message> {
    let view = column![
        text("Keyboard shortcuts").size(22),
        text("Use letters, digits, or keys like Print, Space, Tab, Enter, and Escape. Press Apply after changing portal shortcuts.")
            .size(13),
        text("On COSMIC today, these only work while the Taffy window is focused.")
            .size(13),
        text_input("Start recording shortcut", &app.start_shortcut_value)
            .on_input(Message::StartShortcutChanged)
            .padding(12),
        text_input("Stop recording shortcut", &app.stop_shortcut_value)
            .on_input(Message::StopShortcutChanged)
            .padding(12),
        text_input("Screenshot shortcut", &app.screenshot_shortcut_value)
            .on_input(Message::ScreenshotShortcutChanged)
            .padding(12),
        button("Apply Global Shortcuts")
            .on_press(Message::ApplyShortcutsPressed)
            .padding([10, 18]),
    ]
    .spacing(12);

    container(view).padding(14).into()
}

fn help_view<'a>() -> Element<'a, Message> {
    text(
        "On your current COSMIC portal backend, ScreenCast and Screenshot are available but Global Shortcuts are not. That means Taffy only sees shortcuts while its own window is focused, so COSMIC users should treat recording as a start-and-stop-from-the-UI workflow for now and use another screenshot method when the UI needs to stay out of the shot. Selection mode currently uses `slurp` for region picking after the share flow.",
    )
    .size(13)
    .into()
}
