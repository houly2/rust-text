use std::sync::Arc;

use gpui::{rgb, rgba, AppContext, Global, Rgba};
use smallvec::{smallvec, SmallVec};

#[derive(Clone)]
pub struct Theme {
    pub _id: String,
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
}

#[derive(Clone)]
pub struct ThemeManager {
    active_theme: Arc<Theme>,
    _themes: SmallVec<[Theme; 1]>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let default_theme = Theme {
            _id: "Default".into(),
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
        };

        Self {
            active_theme: Arc::new(default_theme.clone()),
            _themes: smallvec![default_theme],
        }
    }
}

pub trait ActiveTheme {
    fn theme(&self) -> &Arc<Theme>;
}

impl ActiveTheme for AppContext {
    fn theme(&self) -> &Arc<Theme> {
        &self.global::<ThemeManager>().active_theme
    }
}

impl Global for ThemeManager {}
