use gpui::{AppContext, Global};

use crate::theme_manager::ThemeManager;

#[derive(Clone)]
pub struct Settings {
    pub font_family: &'static str,
}

#[derive(Clone)]
pub struct SettingsManager {
    settings: Settings,
    _theme_manager: ThemeManager,
}

impl SettingsManager {
    pub fn new(cx: &mut AppContext) -> Self {
        let theme_manager = ThemeManager::new();
        cx.set_global::<ThemeManager>(theme_manager.clone());

        let this = Self {
            settings: Settings {
                font_family: "Iosevka",
            },
            _theme_manager: theme_manager,
        };

        cx.set_global::<SettingsManager>(this.clone());

        this
    }
}

impl Global for SettingsManager {}

pub trait CurrentSettings {
    fn settings(&self) -> &Settings;
}

impl CurrentSettings for AppContext {
    fn settings(&self) -> &Settings {
        &self.global::<SettingsManager>().settings
    }
}
