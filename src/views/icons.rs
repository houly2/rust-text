use gpui::{IntoElement, SharedString};
use strum::{Display, EnumString};

use super::icon_button::IconButton;

#[derive(Copy, Clone, EnumString, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum Icons {
    Close,
    CharacterSentenceCase,
    RadioButton,
    RadioButtonChecked,
}

impl Icons {
    pub fn path(self) -> impl Into<SharedString> {
        format!("icons/{self}.svg")
    }

    pub fn as_button(self, active: bool) -> impl IntoElement {
        IconButton::new(self).selected(active)
    }
}
