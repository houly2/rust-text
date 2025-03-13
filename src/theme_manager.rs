use std::sync::Arc;

use gpui::{rgb, rgba, AppContext, Global, Rgba};
use smallvec::{smallvec, SmallVec};

#[derive(Clone)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Clone)]
pub struct Theme {
    pub id: String,
    pub mode: ThemeMode,
    pub background: Rgba,
    pub editor_text: Rgba,
    pub editor_background: Rgba,
    pub cursor: Rgba,
    pub selection_bg: Rgba,
    pub hover_bg: Rgba,
    pub scroll_bar_bg: Rgba,
    pub scroll_bar_border: Rgba,
    pub scroll_bar_handle_bg: Rgba,
    pub scroll_bar_cursor_highlight: Rgba,
    pub error: Rgba,
}

#[derive(Clone)]
pub struct ThemeManager {
    active_theme: Arc<Theme>,
    themes: SmallVec<[Theme; 2]>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let default_theme = Theme {
            id: "Default".into(),
            mode: ThemeMode::Dark,
            background: rgb(0x1e1e2e),
            editor_text: rgb(0xcdd6f4),
            editor_background: rgb(0x1a1a29),
            cursor: rgb(0xcdd6f4),
            selection_bg: rgba(0x7f849c64),
            hover_bg: rgb(0x000000),
            scroll_bar_bg: rgba(0x14142033),
            scroll_bar_border: rgba(0x2e2e4d66),
            scroll_bar_handle_bg: rgba(0xaeaecd44),
            scroll_bar_cursor_highlight: rgba(0x89b4fadd),
            error: rgb(0xf38ba8),
        };

        let other_theme = Theme {
            id: "Other theme".into(),
            mode: ThemeMode::Light,
            background: rgb(0x9ca0b0),
            editor_text: rgb(0x343648),
            editor_background: rgb(0xeff1f5),
            cursor: rgb(0xe64553),
            selection_bg: rgba(0x7c7f93aa),
            hover_bg: rgb(0x7c7f93),
            scroll_bar_bg: rgb(0xccd0da),
            scroll_bar_border: rgb(0x9ca0b0),
            scroll_bar_handle_bg: rgb(0x5c5f77),
            scroll_bar_cursor_highlight: rgb(0xd20f39),
            error: rgb(0xd20f39),
        };

        Self {
            active_theme: Arc::new(default_theme.clone()),
            themes: smallvec![default_theme, other_theme],
        }
    }

    pub fn themes(&self) -> &SmallVec<[Theme; 2]> {
        &self.themes
    }

    pub fn set_theme(&mut self, new_theme: &Theme) {
        self.active_theme = Arc::new(new_theme.clone());
    }
}

impl Global for ThemeManager {}

pub trait ActiveTheme {
    fn theme(&self) -> &Arc<Theme>;
}

impl ActiveTheme for AppContext {
    fn theme(&self) -> &Arc<Theme> {
        &self.global::<ThemeManager>().active_theme
    }
}
