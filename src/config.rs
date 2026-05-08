use std::path::PathBuf;
use std::sync::OnceLock;

use crossterm::event::{KeyCode, KeyModifiers};

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(serde::Serialize, serde::Deserialize)]
struct ConfigRaw {
    #[serde(default)]
    show_dotfiles: bool,
    #[serde(default)]
    show_winhidden: bool,
    #[serde(default = "default_80")]
    popup_width_pct: u16,
    #[serde(default = "default_80")]
    popup_height_pct: u16,
    #[serde(default = "default_5")]
    zoxide_limit: usize,
    #[serde(default = "default_80_grep")]
    grep_debounce_ms: u64,
    #[serde(default = "default_50_find")]
    find_debounce_ms: u64,
    #[serde(default = "default_5")]
    grep_max_depth: usize,
    #[serde(default = "default_priority_roots")]
    priority_roots: Vec<String>,
    #[serde(default = "default_6")]
    max_priority_depth: usize,
    #[serde(default = "default_5")]
    max_secondary_depth: usize,
    #[serde(default = "default_exclude_dirs")]
    exclude_dirs: Vec<String>,
    #[serde(default = "default_exclude_win_dirs")]
    exclude_win_dirs: Vec<String>,
    #[serde(default = "default_exclude_path_globs")]
    exclude_path_globs: Vec<String>,
    #[serde(default)]
    keys: KeybindingsRaw,
}

fn default_80() -> u16 { 80 }
fn default_5() -> usize { 5 }
fn default_6() -> usize { 6 }
fn default_80_grep() -> u64 { 80 }
fn default_50_find() -> u64 { 50 }
fn default_priority_roots() -> Vec<String> { vec!["dev".into(), ".config".into()] }
fn default_exclude_dirs() -> Vec<String> {
    vec![
        "node_modules".into(), ".git".into(), ".cache".into(), "cache".into(),
        "licenses".into(), "vendor".into(), "target".into(), "build".into(),
        "dist".into(), "Modules".into(), "modules".into(), "lib".into(), "platform".into(),
    ]
}
fn default_exclude_win_dirs() -> Vec<String> { vec!["AppData".into(), "ProgramData".into()] }
fn default_exclude_path_globs() -> Vec<String> { vec!["**/go/pkg/mod".into()] }

#[derive(serde::Serialize, serde::Deserialize)]
struct KeybindingsRaw {
    #[serde(default = "default_k_quit")]
    quit: String,
    #[serde(default = "default_k_toggle_dotfiles")]
    toggle_dotfiles: String,
    #[serde(default = "default_k_toggle_winhidden")]
    toggle_winhidden: String,
    #[serde(default = "default_k_open_settings")]
    open_settings: String,
    #[serde(default = "default_k_switch_mode")]
    switch_mode: String,
    #[serde(default = "default_k_open_explorer")]
    open_explorer: String,
    #[serde(default = "default_k_go_home")]
    go_home: String,
}

fn default_k_quit() -> String { "ctrl+c".into() }
fn default_k_toggle_dotfiles() -> String { "ctrl+a".into() }
fn default_k_toggle_winhidden() -> String { "ctrl+w".into() }
fn default_k_open_settings() -> String { "ctrl+e".into() }
fn default_k_switch_mode() -> String { "tab".into() }
fn default_k_open_explorer() -> String { "ctrl+enter".into() }
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
    pub fn match_action(&self, key: &crossterm::event::KeyEvent) -> Option<&'static str> {
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

pub struct Config {
    pub show_dotfiles: bool,
    pub show_winhidden: bool,
    pub popup_width_pct: u16,
    pub popup_height_pct: u16,
    pub zoxide_limit: usize,
    pub grep_debounce_ms: u64,
    pub find_debounce_ms: u64,
    pub grep_max_depth: usize,
    pub priority_roots: Vec<&'static str>,
    pub max_priority_depth: usize,
    pub max_secondary_depth: usize,
    pub exclude_dirs: Vec<&'static str>,
    pub exclude_win_dirs: Vec<&'static str>,
    pub exclude_path_globs: Vec<&'static str>,
    pub keys: Keybindings,
}

impl From<ConfigRaw> for Config {
    fn from(r: ConfigRaw) -> Self {
        Self {
            show_dotfiles: r.show_dotfiles,
            show_winhidden: r.show_winhidden,
            popup_width_pct: r.popup_width_pct,
            popup_height_pct: r.popup_height_pct,
            zoxide_limit: r.zoxide_limit,
            grep_debounce_ms: r.grep_debounce_ms,
            find_debounce_ms: r.find_debounce_ms,
            grep_max_depth: r.grep_max_depth,
            priority_roots: leak(r.priority_roots),
            max_priority_depth: r.max_priority_depth,
            max_secondary_depth: r.max_secondary_depth,
            exclude_dirs: leak(r.exclude_dirs),
            exclude_win_dirs: leak(r.exclude_win_dirs),
            exclude_path_globs: leak(r.exclude_path_globs),
            keys: Keybindings {
                quit: parse_key(&r.keys.quit),
                toggle_dotfiles: parse_key(&r.keys.toggle_dotfiles),
                toggle_winhidden: parse_key(&r.keys.toggle_winhidden),
                open_settings: parse_key(&r.keys.open_settings),
                switch_mode: parse_key(&r.keys.switch_mode),
                open_explorer: parse_key(&r.keys.open_explorer),
                go_home: parse_key(&r.keys.go_home),
            },
        }
    }
}

fn leak(v: Vec<String>) -> Vec<&'static str> {
    v.into_iter().map(|s| s.leak() as &str).collect()
}

const DEFAULT_TOML: &str = include_str!("../config.default.toml");

pub fn init() {
    let cfg_path = config_path();
    let config = match std::fs::read_to_string(&cfg_path) {
        Ok(content) => toml::from_str::<ConfigRaw>(&content)
            .map(Config::from)
            .unwrap_or_else(|e| {
                eprintln!("[cdx] bad config at {}, using defaults: {}", cfg_path.display(), e);
                ConfigRaw::default().into()
            }),
        Err(_) => {
            if let Some(parent) = cfg_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&cfg_path, DEFAULT_TOML);
            ConfigRaw::default().into()
        }
    };
    CONFIG.set(config).ok();
}

fn config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("cdx").join("config.toml")
}

impl Default for ConfigRaw {
    fn default() -> Self {
        Self {
            show_dotfiles: false,
            show_winhidden: false,
            popup_width_pct: 80,
            popup_height_pct: 80,
            zoxide_limit: 5,
            grep_debounce_ms: 80,
            find_debounce_ms: 50,
            grep_max_depth: 5,
            priority_roots: default_priority_roots(),
            max_priority_depth: 6,
            max_secondary_depth: 5,
            exclude_dirs: default_exclude_dirs(),
            exclude_win_dirs: default_exclude_win_dirs(),
            exclude_path_globs: default_exclude_path_globs(),
            keys: KeybindingsRaw::default(),
        }
    }
}

pub fn get() -> &'static Config {
    CONFIG.get().expect("config not initialized — call config::init() first")
}
