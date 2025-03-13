use gpui::*;
use prelude::FluentBuilder;

use crate::{
    settings_manager::CurrentSettings,
    theme_manager::{ActiveTheme, Theme},
    views::icons::Icons,
};

use super::modal_manager::ModalView;

actions!(modal, [Up, Down, Select, Close]);

pub struct ThemeSelector {
    focus_handle: FocusHandle,
    themes: Vec<Theme>,
    selection_idx: usize,
    selection_length: usize,
}

impl ThemeSelector {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        cx.bind_keys([
            KeyBinding::new("up", Up, None),
            KeyBinding::new("down", Down, None),
            KeyBinding::new("enter", Select, None),
            KeyBinding::new("escape", Close, None),
        ]);

        Self {
            focus_handle: cx.focus_handle(),
            themes: cx.themes().clone().into_vec(),
            selection_idx: 0,
            selection_length: cx.themes().len(),
        }
    }

    fn up(&mut self, _: &Up, cx: &mut ViewContext<Self>) {
        if self.selection_idx == 0 {
            self.selection_idx = self.selection_length - 1;
        } else {
            self.selection_idx -= 1;
        }
        cx.notify();
    }

    fn down(&mut self, _: &Down, cx: &mut ViewContext<Self>) {
        if self.selection_idx == self.selection_length - 1 {
            self.selection_idx = 0;
        } else {
            self.selection_idx += 1;
        }
        cx.notify();
    }

    fn select(&mut self, _: &Select, cx: &mut ViewContext<Self>) {
        let Some(new_theme) = self
            .themes
            .iter()
            .enumerate()
            .find(|(idx, _)| *idx == self.selection_idx)
        else {
            return;
        };

        cx.change_theme(new_theme.1);
        cx.refresh();
    }

    fn close(&mut self, _: &Close, cx: &mut ViewContext<Self>) {
        cx.emit(DismissEvent);
    }
}

impl Render for ThemeSelector {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .p_2()
            .rounded_md()
            .font_family(cx.settings().font_family)
            .line_height(px(28.))
            .text_size(px(18.))
            .key_context("ThemeSelector")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select))
            .on_action(cx.listener(Self::close))
            .bg(cx.theme().background)
            .text_color(cx.theme().editor_text)
            .children(self.themes.iter().enumerate().map(|(idx, theme)| {
                div()
                    .p_1()
                    .rounded_md()
                    .when(idx == self.selection_idx, |el| {
                        el.bg(cx.theme().editor_background)
                    })
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .items_center()
                            .child(
                                svg()
                                    .size(px(12.))
                                    .flex_none()
                                    .path(if cx.theme().id == theme.id {
                                        Icons::RadioButtonChecked.path()
                                    } else {
                                        Icons::RadioButton.path()
                                    })
                                    .text_color(cx.theme().editor_text),
                            )
                            .child(theme.id.to_string())
                            .child(match theme.mode {
                                crate::theme_manager::ThemeMode::Light => "Light",
                                crate::theme_manager::ThemeMode::Dark => "Dark",
                            }),
                    )
            }))
    }
}

impl ModalView for ThemeSelector {}
impl EventEmitter<DismissEvent> for ThemeSelector {}

impl FocusableView for ThemeSelector {
    fn focus_handle(&self, _: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}
