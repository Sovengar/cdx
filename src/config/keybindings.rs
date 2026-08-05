use crossterm::event::{KeyCode, KeyModifiers, KeyEvent};

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct KeybindingsRaw {
    #[serde(default = "default_k_quit")]
    pub(crate) quit: String,
    #[serde(default = "default_k_toggle_dotfiles")]
    pub(crate) toggle_dotfiles: String,
    #[serde(default = "default_k_toggle_winhidden")]
    pub(crate) toggle_winhidden: String,
    #[serde(default = "default_k_open_settings")]
    pub(crate) open_settings: String,
    #[serde(default = "default_k_switch_mode")]
    pub(crate) switch_mode: String,
    #[serde(default = "default_k_open_explorer")]
    pub(crate) open_explorer: String,
    #[serde(default = "default_k_go_home")]
    pub(crate) go_home: String,
}

fn default_k_quit() -> String { "ctrl+c".into() }
fn default_k_toggle_dotfiles() -> String { "ctrl+h".into() }
fn default_k_toggle_winhidden() -> String { "ctrl+w".into() }
fn default_k_open_settings() -> String { "ctrl+e".into() }
fn default_k_switch_mode() -> String { "tab".into() }
fn default_k_open_explorer() -> String { "ctrl+o".into() }
fn default_k_go_home() -> String { "".into() }

impl Default for KeybindingsRaw {
    fn default() -> Self {
        Self {
            quit: default_k_quit(),
            toggle_dotfiles: default_k_toggle_dotfiles(),
            toggle_winhidden: default_k_toggle_winhidden(),
            open_settings: default_k_open_settings(),
            switch_mode: default_k_switch_mode(),
            open_explorer: default_k_open_explorer(),
            go_home: default_k_go_home(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBind {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

pub struct Keybindings {
    pub quit: KeyBind,
    pub toggle_dotfiles: KeyBind,
    pub toggle_winhidden: KeyBind,
    pub open_settings: KeyBind,
    pub switch_mode: KeyBind,
    pub open_explorer: KeyBind,
    pub go_home: KeyBind,
}

impl Keybindings {
    pub fn match_action(&self, key: &KeyEvent) -> Option<&'static str> {
        macro_rules! check {
            ($name:ident) => {
                if key.code == self.$name.code && key.modifiers == self.$name.modifiers {
                    return Some(stringify!($name));
                }
            }
        }
        check!(quit);
        check!(toggle_dotfiles);
        check!(toggle_winhidden);
        check!(open_settings);
        check!(switch_mode);
        check!(open_explorer);
        check!(go_home);
        None
    }

    pub(crate) fn from_raw(raw: &KeybindingsRaw) -> Self {
        Self {
            quit: parse_key(&raw.quit),
            toggle_dotfiles: parse_key(&raw.toggle_dotfiles),
            toggle_winhidden: parse_key(&raw.toggle_winhidden),
            open_settings: parse_key(&raw.open_settings),
            switch_mode: parse_key(&raw.switch_mode),
            open_explorer: parse_key(&raw.open_explorer),
            go_home: parse_key(&raw.go_home),
        }
    }
}

fn parse_key(s: &str) -> KeyBind {
    let s = s.trim().to_lowercase();
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    let mut modifiers = KeyModifiers::NONE;
    let key_name = parts.last().copied().unwrap_or("");
    for part in parts.iter().rev().skip(1) {
        match *part {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "super" | "win" | "cmd" => modifiers |= KeyModifiers::SUPER,
            "hyper" => modifiers |= KeyModifiers::HYPER,
            "meta" => modifiers |= KeyModifiers::META,
            _ => {}
        }
    }
    let code = match key_name {
        "tab" => KeyCode::Tab,
        "enter" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "space" => KeyCode::Char(' '),
        "backspace" | "bs" => KeyCode::Backspace,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" => KeyCode::PageDown,
        "insert" | "ins" => KeyCode::Insert,
        "delete" | "del" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "null" => KeyCode::Null,
        _ if key_name.len() == 1 => {
            let c = key_name.chars().next().unwrap();
            KeyCode::Char(c)
        }
        _ => KeyCode::Null,
    };
    KeyBind { code, modifiers }
}
