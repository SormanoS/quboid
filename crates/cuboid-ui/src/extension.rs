use cuboid_core::Language;
use egui::Ui;

use crate::UiIntent;

/// A label available in both interface languages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalizedText {
    pub italian: &'static str,
    pub english: &'static str,
}

impl LocalizedText {
    pub const fn new(italian: &'static str, english: &'static str) -> Self {
        Self { italian, english }
    }

    pub const fn resolve(self, language: Language) -> &'static str {
        match language {
            Language::Italian => self.italian,
            Language::English => self.english,
        }
    }
}

/// The shapes the navigation rail can draw. Pages pick one; Base draws it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PageIcon {
    /// Two panels side by side.
    #[default]
    Panels,
    /// A key row.
    Keys,
    /// A four-cell grid.
    Grid,
    /// Two stacked cards.
    Stack,
    /// A dial.
    Dial,
}

/// A page contributed by a host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtensionPage {
    /// Stable identifier, also used as the egui id salt.
    pub id: &'static str,
    pub icon: PageIcon,
    pub label: LocalizedText,
    pub title: LocalizedText,
    pub description: LocalizedText,
}

/// What a hosted page changed while it was drawn.
///
/// A continuous edit reports [`ExtensionChange::Live`] while it lasts and
/// [`ExtensionChange::Persist`] once: only the write to disk waits for the end
/// of the interaction.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExtensionChange {
    /// Nothing changed.
    #[default]
    Unchanged,
    /// The settings changed but must not be written yet.
    Live,
    /// The settings changed and should be written now.
    Persist,
}

impl ExtensionChange {
    /// The stronger of two changes reported while drawing the same page.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        self.max(other)
    }

    /// Whether anything changed at all.
    pub const fn changed(self) -> bool {
        !matches!(self, Self::Unchanged)
    }

    /// Whether the change should be written now.
    pub const fn persists(self) -> bool {
        matches!(self, Self::Persist)
    }
}

impl From<bool> for ExtensionChange {
    /// A change reported by a widget that is edited in one step.
    fn from(changed: bool) -> Self {
        if changed {
            Self::Persist
        } else {
            Self::Unchanged
        }
    }
}

/// Pages contributed to the settings window by a host.
pub trait UiExtension {
    /// The pages this extension contributes, in navigation order.
    fn pages(&self) -> &[ExtensionPage];

    /// Draws `page`, reporting what the host's own settings did.
    fn show(
        &mut self,
        page: &str,
        ui: &mut Ui,
        language: Language,
        intents: &mut Vec<UiIntent>,
    ) -> ExtensionChange;
}
