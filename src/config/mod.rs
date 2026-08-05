pub mod keybindings;

use std::path::PathBuf;
use std::sync::OnceLock;

use keybindings::{Keybindings, KeybindingsRaw};

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ToolEntry {
    pub name: String,
    pub command: String,
}

fn default_tool_selector() -> Vec<ToolEntry> {
    vec![
        ToolEntry { name: "yazi".into(), command: "yazi".into() },
        ToolEntry { name: "nvim".into(), command: "nvim".into() },
        ToolEntry { name: "opencode".into(), command: "opencode".into() },
        ToolEntry { name: "lazygit".into(), command: "lazygit".into() },
    ]
}

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
    #[serde(default = "default_tool_selector")]
    tool_selector: Vec<ToolEntry>,
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
    pub tool_selector: Vec<ToolEntry>,
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
            keys: Keybindings::from_raw(&r.keys),
            tool_selector: r.tool_selector,
        }
    }
}

fn leak(v: Vec<String>) -> Vec<&'static str> {
    v.into_iter().map(|s| s.leak() as &str).collect()
}

const DEFAULT_TOML: &str = include_str!("../../config.default.toml");

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
            tool_selector: default_tool_selector(),
        }
    }
}

pub fn get() -> &'static Config {
    CONFIG.get().expect("config not initialized — call config::init() first")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_config_defaults() {
        init();
        let cfg = get();
        assert_eq!(cfg.find_debounce_ms, 50);
        assert_eq!(cfg.grep_debounce_ms, 80);
        assert_eq!(cfg.grep_max_depth, 5);
        assert_eq!(cfg.max_secondary_depth, 5);
        assert_eq!(cfg.max_priority_depth, 6);
        assert_eq!(cfg.zoxide_limit, 5);
        assert_eq!(cfg.popup_width_pct, 80);
        assert_eq!(cfg.popup_height_pct, 80);
        assert!(!cfg.show_dotfiles);
        assert!(!cfg.show_winhidden);
    }

    #[test]
    fn test_config_exclude_dirs() {
        init();
        let cfg = get();
        assert!(cfg.exclude_dirs.contains(&"node_modules"));
        assert!(cfg.exclude_dirs.contains(&".git"));
        assert!(cfg.exclude_dirs.contains(&".cache"));
        assert!(cfg.exclude_dirs.contains(&"target"));
        assert!(cfg.exclude_dirs.contains(&"build"));
        assert!(cfg.exclude_dirs.contains(&"dist"));
        assert!(cfg.exclude_dirs.contains(&"vendor"));
    }

    #[test]
    fn test_config_exclude_win_dirs() {
        init();
        let cfg = get();
        assert!(cfg.exclude_win_dirs.contains(&"AppData"));
        assert!(cfg.exclude_win_dirs.contains(&"ProgramData"));
    }

    #[test]
    fn test_config_exclude_path_globs() {
        init();
        let cfg = get();
        assert!(cfg.exclude_path_globs.contains(&"**/go/pkg/mod"));
    }

    #[test]
    fn test_config_priority_roots() {
        init();
        let cfg = get();
        assert!(cfg.priority_roots.contains(&"dev"));
        assert!(cfg.priority_roots.contains(&".config"));
    }

    #[test]
    fn test_config_keybinding_parsing() {
        let raw = KeybindingsRaw::default();
        let kb = Keybindings::from_raw(&raw);

        assert_eq!(kb.quit.code, KeyCode::Char('c'));
        assert!(kb.quit.modifiers.contains(KeyModifiers::CONTROL));

        assert_eq!(kb.toggle_dotfiles.code, KeyCode::Char('h'));
        assert!(kb.toggle_dotfiles.modifiers.contains(KeyModifiers::CONTROL));

        assert_eq!(kb.toggle_winhidden.code, KeyCode::Char('w'));
        assert!(kb.toggle_winhidden.modifiers.contains(KeyModifiers::CONTROL));

        assert_eq!(kb.open_settings.code, KeyCode::Char('e'));
        assert!(kb.open_settings.modifiers.contains(KeyModifiers::CONTROL));

        assert_eq!(kb.switch_mode.code, KeyCode::Tab);
        assert_eq!(kb.switch_mode.modifiers, KeyModifiers::NONE);

        assert_eq!(kb.open_explorer.code, KeyCode::Char('o'));
        assert!(kb.open_explorer.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn test_config_custom_keybinding_parsing() {
        let raw = KeybindingsRaw {
            quit: "ctrl+shift+q".into(),
            ..KeybindingsRaw::default()
        };
        let kb = Keybindings::from_raw(&raw);
        assert_eq!(kb.quit.code, KeyCode::Char('q'));
        assert!(kb.quit.modifiers.contains(KeyModifiers::CONTROL));
        assert!(kb.quit.modifiers.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn test_config_special_key_parsing() {
        let raw = KeybindingsRaw {
            switch_mode: "ctrl+tab".into(),
            ..KeybindingsRaw::default()
        };
        let kb = Keybindings::from_raw(&raw);
        assert_eq!(kb.switch_mode.code, KeyCode::Tab);
        assert!(kb.switch_mode.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn test_config_match_action() {
        init();
        let cfg = get();
        let quit_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(cfg.keys.match_action(&quit_key), Some("quit"));

        let tab_key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(cfg.keys.match_action(&tab_key), Some("switch_mode"));

        let unknown = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
        assert_eq!(cfg.keys.match_action(&unknown), None);
    }

    #[test]
    fn test_tool_selector_defaults() {
        init();
        let cfg = get();
        assert_eq!(cfg.tool_selector.len(), 4);
        assert_eq!(cfg.tool_selector[0].name, "yazi");
        assert_eq!(cfg.tool_selector[0].command, "yazi");
        assert_eq!(cfg.tool_selector[1].name, "nvim");
        assert_eq!(cfg.tool_selector[2].name, "opencode");
        assert_eq!(cfg.tool_selector[3].name, "lazygit");
    }

    #[test]
    fn test_config_raw_default_values() {
        let raw = ConfigRaw::default();
        assert!(!raw.show_dotfiles);
        assert!(!raw.show_winhidden);
        assert_eq!(raw.popup_width_pct, 80);
        assert_eq!(raw.popup_height_pct, 80);
        assert_eq!(raw.zoxide_limit, 5);
        assert_eq!(raw.grep_debounce_ms, 80);
        assert_eq!(raw.find_debounce_ms, 50);
        assert_eq!(raw.grep_max_depth, 5);
        assert_eq!(raw.max_secondary_depth, 5);
    }

    #[test]
    fn test_config_toml_parsing() {
        let toml_str = r#"
            show_dotfiles = true
            find_debounce_ms = 100
            grep_debounce_ms = 200
            grep_max_depth = 10
            zoxide_limit = 3
            [keys]
            quit = "ctrl+q"
        "#;
        let raw: ConfigRaw = toml::from_str(toml_str).unwrap();
        assert!(raw.show_dotfiles);
        assert_eq!(raw.find_debounce_ms, 100);
        assert_eq!(raw.grep_debounce_ms, 200);
        assert_eq!(raw.grep_max_depth, 10);
        assert_eq!(raw.zoxide_limit, 3);
        assert_eq!(raw.keys.quit, "ctrl+q");
    }

    #[test]
    fn test_config_toml_partial_override() {
        let toml_str = r#"
            find_debounce_ms = 150
        "#;
        let raw: ConfigRaw = toml::from_str(toml_str).unwrap();
        assert_eq!(raw.find_debounce_ms, 150);
        // Defaults preserved
        assert_eq!(raw.grep_debounce_ms, 80);
        assert!(!raw.show_dotfiles);
    }

    #[test]
    fn test_config_invalid_toml_fallback() {
        let result = toml::from_str::<ConfigRaw>("this is not valid toml {{{");
        assert!(result.is_err());
    }
}
