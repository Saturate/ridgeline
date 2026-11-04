use std::path::PathBuf;

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Global, px};
use serde::{Deserialize, Serialize};

use crate::theme::InstalledThemes;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    pub const ALL: [Appearance; 3] = [Appearance::System, Appearance::Light, Appearance::Dark];

    pub fn label(self) -> &'static str {
        match self {
            Appearance::System => "System",
            Appearance::Light => "Light",
            Appearance::Dark => "Dark",
        }
    }

    pub fn from_label(label: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|a| a.label() == label)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub appearance: Appearance,
    pub dark_theme: Option<String>,
    pub light_theme: Option<String>,
    pub ui_font_size: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance: Appearance::System,
            dark_theme: None,
            light_theme: None,
            ui_font_size: 14.,
        }
    }
}

impl Global for Settings {}

impl Settings {
    pub fn get(cx: &App) -> &Settings {
        cx.global::<Settings>()
    }

    fn path() -> Option<PathBuf> {
        Some(
            dirs::config_dir()?
                .join("ridgeline")
                .join("ui-settings.json"),
        )
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|path| std::fs::read_to_string(&path).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save(&self) {
        let Some(path) = Self::path() else {
            return;
        };
        let Ok(json) = serde_json::to_vec_pretty(self) else {
            return;
        };
        if let Some(dir) = path.parent() {
            if std::fs::create_dir_all(dir).is_ok() {
                let tmp = path.with_extension("json.tmp");
                if std::fs::write(&tmp, json).is_ok() {
                    let _ = std::fs::rename(tmp, path);
                }
            }
        }
    }

    pub fn dark_theme_name(&self, cx: &App) -> String {
        self.dark_theme
            .clone()
            .unwrap_or_else(|| cx.global::<InstalledThemes>().zed_dark.clone())
    }

    pub fn light_theme_name(&self, cx: &App) -> String {
        self.light_theme
            .clone()
            .unwrap_or_else(|| cx.global::<InstalledThemes>().zed_light.clone())
    }

    pub fn update(cx: &mut App, change: impl FnOnce(&mut Settings)) {
        change(cx.global_mut::<Settings>());
        Settings::get(cx).save();
        Settings::apply(cx);
    }

    pub fn apply(cx: &mut App) {
        let settings = Settings::get(cx).clone();
        crate::theme::select(
            &settings.dark_theme_name(cx),
            &settings.light_theme_name(cx),
            cx,
        );
        match settings.appearance {
            Appearance::System => Theme::sync_system_appearance(None, cx),
            Appearance::Light => Theme::change(ThemeMode::Light, None, cx),
            Appearance::Dark => Theme::change(ThemeMode::Dark, None, cx),
        }
        Theme::global_mut(cx).font_size = px(settings.ui_font_size as f32);
        cx.refresh_windows();
    }
}

