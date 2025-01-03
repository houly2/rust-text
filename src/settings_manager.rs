use gpui::{AppContext, Global};
use smallvec::SmallVec;

use crate::theme_manager::{Theme, ThemeManager};

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
    fn themes(&self) -> &SmallVec<[Theme; 2]>;
    fn change_theme(&mut self, new_theme: &Theme);
}

impl CurrentSettings for AppContext {
    fn settings(&self) -> &Settings {
        &self.global::<SettingsManager>().settings
    }

    fn themes(&self) -> &SmallVec<[Theme; 2]> {
        &self.global::<ThemeManager>().themes()
    }

    fn change_theme(&mut self, new_theme: &Theme) {
        let _ = &self.global_mut::<ThemeManager>().set_theme(new_theme);
    }
}
