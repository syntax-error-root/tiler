use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub render: RenderConfig,
    #[serde(default)]
    pub keybinds: KeybindConfig,
}

#[derive(Deserialize, Clone)]
pub struct RenderConfig {
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_cursor_style")]
    pub cursor_style: String,
    #[serde(default = "default_true")]
    pub cursor_blink: bool,
    #[serde(default = "default_scrollback")]
    pub scrollback_lines: usize,
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    #[serde(default = "default_window_height")]
    pub window_height: u32,
    #[serde(default = "default_bg_color")]
    pub bg_color: (u8, u8, u8),
    #[serde(default = "default_fg_color")]
    pub fg_color: (u8, u8, u8),
    #[serde(default = "default_tab_bar")]
    pub tab_bar_height: usize,
}

#[derive(Deserialize, Clone)]
pub struct KeybindConfig {
    #[serde(default = "default_prefix")]
    pub prefix: String,
    #[serde(default = "default_split_h")]
    pub split_horizontal: String,
    #[serde(default = "default_split_v")]
    pub split_vertical: String,
    #[serde(default = "default_new_tab")]
    pub new_tab: String,
    #[serde(default = "default_close_tab")]
    pub close_tab: String,
    #[serde(default = "default_next_tab")]
    pub next_tab: String,
    #[serde(default = "default_prev_tab")]
    pub prev_tab: String,
}

fn default_font_family() -> String { "mono".to_string() }
fn default_font_size() -> f32 { 14.0 }
fn default_cursor_style() -> String { "block".to_string() }
fn default_true() -> bool { true }
fn default_scrollback() -> usize { 10000 }
fn default_window_width() -> u32 { 1024 }
fn default_window_height() -> u32 { 768 }
fn default_bg_color() -> (u8, u8, u8) { (30, 30, 30) }
fn default_fg_color() -> (u8, u8, u8) { (220, 220, 220) }
fn default_tab_bar() -> usize { 1 }
fn default_prefix() -> String { "CtrlA".to_string() }
fn default_split_h() -> String { "h".to_string() }
fn default_split_v() -> String { "v".to_string() }
fn default_new_tab() -> String { "t".to_string() }
fn default_close_tab() -> String { "w".to_string() }
fn default_next_tab() -> String { "n".to_string() }
fn default_prev_tab() -> String { "p".to_string() }

impl Default for Config {
    fn default() -> Self {
        Config {
            render: RenderConfig::default(),
            keybinds: KeybindConfig::default(),
        }
    }
}

impl Default for RenderConfig {
    fn default() -> Self {
        RenderConfig {
            font_family: default_font_family(),
            font_size: default_font_size(),
            cursor_style: default_cursor_style(),
            cursor_blink: default_true(),
            scrollback_lines: default_scrollback(),
            window_width: default_window_width(),
            window_height: default_window_height(),
            bg_color: default_bg_color(),
            fg_color: default_fg_color(),
            tab_bar_height: default_tab_bar(),
        }
    }
}

impl Default for KeybindConfig {
    fn default() -> Self {
        KeybindConfig {
            prefix: default_prefix(),
            split_horizontal: default_split_h(),
            split_vertical: default_split_v(),
            new_tab: default_new_tab(),
            close_tab: default_close_tab(),
            next_tab: default_next_tab(),
            prev_tab: default_prev_tab(),
        }
    }
}

pub fn load_config() -> Config {
    let config_path = directories::ProjectDirs::from("", "", "term-tiler")
        .map(|d| d.config_dir().join("config.toml"));

    if let Some(path) = config_path {
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            return toml::from_str(&content).unwrap_or_default();
        }
    }

    Config::default()
}

pub fn resolve_font_path(font_family: &str) -> Option<PathBuf> {
    let path = PathBuf::from(font_family);
    if path.is_absolute() && path.exists() {
        return Some(path);
    }

    let search_dirs = [
        "/usr/share/fonts/",
        "/usr/local/share/fonts/",
        &format!(
            "{}/.local/share/fonts/",
            std::env::var("HOME").unwrap_or_default()
        ),
        &format!("{}/.fonts/", std::env::var("HOME").unwrap_or_default()),
    ];

    let extensions = [".ttf", ".otf"];

    for dir in &search_dirs {
        let dir_path = PathBuf::from(dir);
        if !dir_path.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let result @ Some(_) = search_dir_recursive(&path, font_family, &extensions) {
                        return result;
                    }
                } else if path.is_file() {
                    if let result @ Some(_) = matches_font(&path, font_family, &extensions) {
                        return result;
                    }
                }
            }
        }
    }

    None
}

fn search_dir_recursive(dir: &std::path::Path, font_family: &str, extensions: &[&str]) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let result @ Some(_) = search_dir_recursive(&path, font_family, extensions) {
                    return result;
                }
            } else if path.is_file() {
                if let result @ Some(_) = matches_font(&path, font_family, extensions) {
                    return result;
                }
            }
        }
    }
    None
}

fn matches_font(path: &std::path::Path, font_family: &str, extensions: &[&str]) -> Option<PathBuf> {
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    let family_lower = font_family.to_lowercase();
    if name.contains(&family_lower) {
        if let Some(ext) = path.extension() {
            if extensions.contains(&ext.to_string_lossy().as_ref()) {
                return Some(path.to_path_buf());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.render.font_size, 14.0);
        assert_eq!(config.render.scrollback_lines, 10000);
        assert_eq!(config.render.cursor_style, "block");
        assert!(config.render.cursor_blink);
        assert_eq!(config.keybinds.prefix, "CtrlA");
    }

    #[test]
    fn test_partial_toml_parsing() {
        let toml = r#"
[render]
font_size = 18.0
scrollback_lines = 5000
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.render.font_size, 18.0);
        assert_eq!(config.render.scrollback_lines, 5000);
        // Defaults for unspecified fields
        assert_eq!(config.render.font_family, "mono");
        assert_eq!(config.render.cursor_style, "block");
    }

    #[test]
    fn test_empty_toml_parsing() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.render.font_size, 14.0);
    }

    #[test]
    fn test_full_toml_parsing() {
        let toml = r#"
[render]
font_family = "JetBrains Mono"
font_size = 16.0
cursor_style = "bar"
cursor_blink = false
scrollback_lines = 20000
window_width = 1200
window_height = 900

[render.colors]  # This will be ignored gracefully
background = [30, 30, 30]
foreground = [220, 220, 220]

[keybinds]
prefix = "CtrlA"
split_horizontal = "h"
split_vertical = "v"
new_tab = "t"
close_tab = "w"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.render.font_family, "JetBrains Mono");
        assert_eq!(config.render.font_size, 16.0);
        assert_eq!(config.render.cursor_style, "bar");
        assert!(!config.render.cursor_blink);
    }
}
