use ashpd::desktop::{
    CreateSessionOptions,
    global_shortcuts::{GlobalShortcuts, NewShortcut},
};
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use iced::futures::SinkExt;
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::stream;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ShortcutSpec {
    pub start: String,
    pub stop: String,
    pub screenshot: String,
}

#[derive(Debug, Clone)]
pub enum ShortcutEvent {
    Status(String),
    Bound(Vec<(String, String)>),
    Activated(ShortcutAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    Start,
    Stop,
    Screenshot,
}

#[derive(Debug, Clone)]
pub struct ShortcutParseError {
    pub label: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedShortcut {
    ctrl: bool,
    alt: bool,
    shift: bool,
    logo: bool,
    key: ShortcutKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ShortcutKey {
    Character(char),
    Named(Named),
}

pub fn portal_shortcuts(spec: ShortcutSpec) -> BoxStream<'static, ShortcutEvent> {
    stream::channel(100, async move |mut output| {
        let proxy = match GlobalShortcuts::new().await {
            Ok(proxy) => proxy,
            Err(error) => {
                let message = error.to_string();
                let detail = if message.contains("org.freedesktop.portal.GlobalShortcuts") {
                    "Global shortcuts are not exposed by the active portal backend on this desktop. Taffy can still use focused-window shortcuts while its window is focused.".to_string()
                } else {
                    format!("Global shortcuts unavailable on this desktop: {error}")
                };
                let _ = output
                    .send(ShortcutEvent::Status(detail))
                    .await;
                return;
            }
        };

        let session = match proxy.create_session(CreateSessionOptions::default()).await {
            Ok(session) => session,
            Err(error) => {
                let _ = output
                    .send(ShortcutEvent::Status(format!(
                        "Could not create shortcut session: {error}"
                    )))
                    .await;
                return;
            }
        };

        let shortcuts = vec![
            NewShortcut::new("start-recording", "Start recording")
                .preferred_trigger(Some(spec.start.trim())),
            NewShortcut::new("stop-recording", "Stop recording")
                .preferred_trigger(Some(spec.stop.trim())),
            NewShortcut::new("take-screenshot", "Take screenshot")
                .preferred_trigger(Some(spec.screenshot.trim())),
        ];

        let bound = match proxy
            .bind_shortcuts(&session, &shortcuts, None, Default::default())
            .await
        {
            Ok(request) => match request.response() {
                Ok(response) => response,
                Err(error) => {
                    let _ = output
                        .send(ShortcutEvent::Status(format!(
                            "Shortcut binding was denied or cancelled: {error}"
                        )))
                        .await;
                    return;
                }
            },
            Err(error) => {
                let _ = output
                    .send(ShortcutEvent::Status(format!(
                        "Could not request shortcut binding: {error}"
                    )))
                    .await;
                return;
            }
        };

        let descriptions = bound
            .shortcuts()
            .iter()
            .map(|shortcut| {
                (
                    shortcut.id().to_string(),
                    shortcut.trigger_description().to_string(),
                )
            })
            .collect::<Vec<_>>();

        let _ = output.send(ShortcutEvent::Bound(descriptions)).await;

        let mut activated = match proxy.receive_activated().await {
            Ok(stream) => stream,
            Err(error) => {
                let _ = output
                    .send(ShortcutEvent::Status(format!(
                        "Could not listen for shortcut events: {error}"
                    )))
                    .await;
                return;
            }
        };

        while let Some(event) = activated.next().await {
            let action = match event.shortcut_id() {
                "start-recording" => Some(ShortcutAction::Start),
                "stop-recording" => Some(ShortcutAction::Stop),
                "take-screenshot" => Some(ShortcutAction::Screenshot),
                _ => None,
            };

            if let Some(action) = action {
                let _ = output.send(ShortcutEvent::Activated(action)).await;
            }
        }
    })
    .boxed()
}

pub fn action_for_event(spec: &ShortcutSpec, event: &keyboard::Event) -> Option<ShortcutAction> {
    let keyboard::Event::KeyPressed {
        key,
        physical_key,
        modifiers,
        repeat,
        ..
    } = event
    else {
        return None;
    };

    if *repeat {
        return None;
    }

    let start = parse_shortcut(&spec.start).ok();
    let stop = parse_shortcut(&spec.stop).ok();
    let screenshot = parse_shortcut(&spec.screenshot).ok();

    if stop
        .as_ref()
        .is_some_and(|shortcut| shortcut.matches(key, *physical_key, *modifiers))
    {
        Some(ShortcutAction::Stop)
    } else if screenshot
        .as_ref()
        .is_some_and(|shortcut| shortcut.matches(key, *physical_key, *modifiers))
    {
        Some(ShortcutAction::Screenshot)
    } else if start
        .as_ref()
        .is_some_and(|shortcut| shortcut.matches(key, *physical_key, *modifiers))
    {
        Some(ShortcutAction::Start)
    } else {
        None
    }
}

pub fn parse_errors(spec: &ShortcutSpec) -> Vec<ShortcutParseError> {
    let candidates = [
        ("Start", spec.start.as_str()),
        ("Stop", spec.stop.as_str()),
        ("Screenshot", spec.screenshot.as_str()),
    ];

    candidates
        .into_iter()
        .filter_map(|(label, value)| match parse_shortcut(value) {
            Ok(_) => None,
            Err(detail) => Some(ShortcutParseError { label, detail }),
        })
        .collect()
}

impl ParsedShortcut {
    fn matches(
        &self,
        key: &Key,
        physical_key: keyboard::key::Physical,
        modifiers: Modifiers,
    ) -> bool {
        let normalized_modifiers = normalize_modifiers(modifiers);
        if normalized_modifiers.control() != self.ctrl
            || normalized_modifiers.alt() != self.alt
            || normalized_modifiers.shift() != self.shift
            || normalized_modifiers.logo() != self.logo
        {
            return false;
        }

        match &self.key {
            ShortcutKey::Character(expected) => key
                .to_latin(physical_key)
                .map(|actual| actual.eq_ignore_ascii_case(expected))
                .unwrap_or(false),
            ShortcutKey::Named(expected) => key.as_ref() == Key::Named(*expected),
        }
    }
}

fn parse_shortcut(value: &str) -> Result<ParsedShortcut, String> {
    let mut parsed = ParsedShortcut {
        ctrl: false,
        alt: false,
        shift: false,
        logo: false,
        key: ShortcutKey::Named(Named::PrintScreen),
    };
    let mut key = None;

    for raw_part in value.split('+') {
        let part = raw_part.trim();
        if part.is_empty() {
            continue;
        }

        let normalized = part.to_ascii_lowercase();
        match normalized.as_str() {
            "ctrl" | "control" => parsed.ctrl = true,
            "alt" => parsed.alt = true,
            "shift" => parsed.shift = true,
            "super" | "meta" | "logo" | "cmd" | "command" => parsed.logo = true,
            _ => {
                if key.is_some() {
                    return Err(format!("`{value}` contains more than one primary key"));
                }
                key = Some(parse_key(part)?);
            }
        }
    }

    let Some(key) = key else {
        return Err(format!("`{value}` is missing a primary key"));
    };

    parsed.key = key;
    Ok(parsed)
}

fn parse_key(value: &str) -> Result<ShortcutKey, String> {
    let normalized = value.trim().to_ascii_lowercase();

    let key = match normalized.as_str() {
        "print" | "printscr" | "printscreen" | "prtsc" | "sysrq" => {
            ShortcutKey::Named(Named::PrintScreen)
        }
        "space" => ShortcutKey::Named(Named::Space),
        "tab" => ShortcutKey::Named(Named::Tab),
        "enter" | "return" => ShortcutKey::Named(Named::Enter),
        "esc" | "escape" => ShortcutKey::Named(Named::Escape),
        _ => {
            let mut chars = normalized.chars();
            let Some(first) = chars.next() else {
                return Err("Shortcut key cannot be empty".into());
            };

            if chars.next().is_none() && first.is_ascii_alphanumeric() {
                ShortcutKey::Character(first)
            } else {
                return Err(format!(
                    "`{value}` is not a supported shortcut key yet. Use letters, digits, Print, Space, Tab, Enter, or Escape."
                ));
            }
        }
    };

    Ok(key)
}

fn normalize_modifiers(modifiers: Modifiers) -> Modifiers {
    let mut normalized = Modifiers::empty();
    if modifiers.control() {
        normalized |= Modifiers::CTRL;
    }
    if modifiers.alt() {
        normalized |= Modifiers::ALT;
    }
    if modifiers.shift() {
        normalized |= Modifiers::SHIFT;
    }
    if modifiers.logo() {
        normalized |= Modifiers::LOGO;
    }
    normalized
}
