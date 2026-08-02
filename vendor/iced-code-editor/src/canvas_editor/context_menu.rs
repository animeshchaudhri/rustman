//! Right-click context menu for editor actions.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Background, Border, Color, Element, Length, Shadow, Theme, Vector};

use super::Message;
use crate::i18n::Translations;

const MENU_WIDTH: f32 = 224.0;

/// An actionable entry in the editor context menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMenuItem {
    /// Stable action identifier emitted when the item is selected.
    pub id: String,
    /// Text displayed for the item.
    pub label: String,
    /// Optional keyboard shortcut hint displayed beside the label.
    pub shortcut: Option<String>,
    /// Whether the item can be selected.
    pub enabled: bool,
}

impl ContextMenuItem {
    /// Creates an enabled context-menu item without a shortcut hint.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            enabled: true,
        }
    }

    /// Sets the keyboard shortcut hint displayed beside this item.
    #[must_use]
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Sets whether this item can be selected.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// A custom editor context-menu entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMenuEntry {
    /// An actionable menu item.
    Item(ContextMenuItem),
    /// A visual separator between groups of items.
    Separator,
}

impl ContextMenuEntry {
    /// Creates an enabled action entry without a shortcut hint.
    pub fn item(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Item(ContextMenuItem::new(id, label))
    }

    /// Creates a separator entry.
    pub const fn separator() -> Self {
        Self::Separator
    }

    /// Sets the keyboard shortcut hint when this is an action entry.
    #[must_use]
    pub fn with_shortcut(self, shortcut: impl Into<String>) -> Self {
        match self {
            Self::Item(item) => Self::Item(item.with_shortcut(shortcut)),
            Self::Separator => Self::Separator,
        }
    }

    /// Sets whether this entry can be selected when it is an action.
    #[must_use]
    pub fn with_enabled(self, enabled: bool) -> Self {
        match self {
            Self::Item(item) => Self::Item(item.with_enabled(enabled)),
            Self::Separator => Self::Separator,
        }
    }
}

impl From<ContextMenuItem> for ContextMenuEntry {
    fn from(item: ContextMenuItem) -> Self {
        Self::Item(item)
    }
}

#[cfg(target_os = "macos")]
const UNDO_SHORTCUT: &str = "⌘Z";
#[cfg(not(target_os = "macos"))]
const UNDO_SHORTCUT: &str = "Ctrl+Z";

#[cfg(target_os = "macos")]
const REDO_SHORTCUT: &str = "⇧⌘Z";
#[cfg(not(target_os = "macos"))]
const REDO_SHORTCUT: &str = "Ctrl+Y";

#[cfg(target_os = "macos")]
const CUT_SHORTCUT: &str = "⌘X";
#[cfg(not(target_os = "macos"))]
const CUT_SHORTCUT: &str = "Ctrl+X";

#[cfg(target_os = "macos")]
const COPY_SHORTCUT: &str = "⌘C";
#[cfg(not(target_os = "macos"))]
const COPY_SHORTCUT: &str = "Ctrl+C";

#[cfg(target_os = "macos")]
const PASTE_SHORTCUT: &str = "⌘V";
#[cfg(not(target_os = "macos"))]
const PASTE_SHORTCUT: &str = "Ctrl+V";

#[cfg(target_os = "macos")]
const SELECT_ALL_SHORTCUT: &str = "⌘A";
#[cfg(not(target_os = "macos"))]
const SELECT_ALL_SHORTCUT: &str = "Ctrl+A";

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MenuState {
    pub(crate) can_undo: bool,
    pub(crate) can_redo: bool,
    pub(crate) has_selection: bool,
    pub(crate) has_content: bool,
    pub(crate) reveal_in_file_manager_enabled: bool,
}

#[derive(Debug, Clone)]
enum MenuEntry {
    Item { label: String, shortcut: String, message: Option<Message> },
    Separator,
}

impl MenuEntry {
    #[cfg(test)]
    fn label(&self) -> Option<&str> {
        match self {
            Self::Item { label, .. } => Some(label),
            Self::Separator => None,
        }
    }
}

fn custom_entries(entries: &[ContextMenuEntry]) -> Vec<MenuEntry> {
    entries
        .iter()
        .map(|entry| match entry {
            ContextMenuEntry::Item(item) => MenuEntry::Item {
                label: item.label.clone(),
                shortcut: item.shortcut.clone().unwrap_or_default(),
                message: item
                    .enabled
                    .then(|| Message::CustomContextMenuAction(item.id.clone())),
            },
            ContextMenuEntry::Separator => MenuEntry::Separator,
        })
        .collect()
}

fn default_entries(
    state: MenuState,
    translations: &Translations,
) -> Vec<MenuEntry> {
    let mut entries = if state.reveal_in_file_manager_enabled {
        vec![
            MenuEntry::Item {
                label: translations.context_menu_reveal_in_file_manager(),
                shortcut: String::new(),
                message: Some(Message::RevealInFileManager),
            },
            MenuEntry::Separator,
        ]
    } else {
        Vec::new()
    };
    entries.extend([
        MenuEntry::Item {
            label: translations.context_menu_undo(),
            shortcut: UNDO_SHORTCUT.to_string(),
            message: state.can_undo.then_some(Message::Undo),
        },
        MenuEntry::Item {
            label: translations.context_menu_redo(),
            shortcut: REDO_SHORTCUT.to_string(),
            message: state.can_redo.then_some(Message::Redo),
        },
        MenuEntry::Separator,
        MenuEntry::Item {
            label: translations.context_menu_cut(),
            shortcut: CUT_SHORTCUT.to_string(),
            message: state.has_selection.then_some(Message::Cut),
        },
        MenuEntry::Item {
            label: translations.context_menu_copy(),
            shortcut: COPY_SHORTCUT.to_string(),
            message: state.has_selection.then_some(Message::Copy),
        },
        MenuEntry::Item {
            label: translations.context_menu_paste(),
            shortcut: PASTE_SHORTCUT.to_string(),
            message: Some(Message::Paste(String::new())),
        },
        MenuEntry::Separator,
        MenuEntry::Item {
            label: translations.context_menu_select_all(),
            shortcut: SELECT_ALL_SHORTCUT.to_string(),
            message: state.has_content.then_some(Message::SelectAll),
        },
    ]);
    entries
}

fn build_entries(
    custom: &[ContextMenuEntry],
    default_context_menu_enabled: bool,
    state: MenuState,
    translations: &Translations,
) -> Vec<MenuEntry> {
    let mut entries = custom_entries(custom);
    if default_context_menu_enabled {
        if !entries.is_empty() {
            entries.push(MenuEntry::Separator);
        }
        entries.extend(default_entries(state, translations));
    }
    entries
}

/// Builds the context-menu contents.
pub(crate) fn view(
    custom: &[ContextMenuEntry],
    default_context_menu_enabled: bool,
    state: MenuState,
    translations: Translations,
) -> Element<'static, Message> {
    let items = build_entries(
        custom,
        default_context_menu_enabled,
        state,
        &translations,
    )
    .into_iter()
    .map(|entry| match entry {
        MenuEntry::Item { label, shortcut, message } => {
            menu_item(label, shortcut, message)
        }
        MenuEntry::Separator => separator(),
    })
    .collect::<Vec<_>>();

    container(column(items).spacing(1).padding(4))
        .width(Length::Fixed(MENU_WIDTH))
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(Background::Color(
                    palette.background.weak.color,
                )),
                text_color: Some(palette.background.weak.text),
                border: Border {
                    color: palette.background.strong.color,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                shadow: Shadow {
                    color: Color::BLACK.scale_alpha(0.35),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 14.0,
                },
                ..container::Style::default()
            }
        })
        .into()
}

fn menu_item(
    label: String,
    shortcut: String,
    message: Option<Message>,
) -> Element<'static, Message> {
    let enabled = message.is_some();
    let content = row![
        text(label).size(13),
        Space::new().width(Length::Fill),
        text(shortcut).size(12),
    ]
    .align_y(iced::Alignment::Center);

    button(content)
        .width(Length::Fill)
        .padding([6, 9])
        .on_press_maybe(message)
        .style(move |theme: &Theme, status| {
            let palette = theme.extended_palette();
            let text_color = if enabled {
                palette.background.weak.text
            } else {
                palette.background.weak.text.scale_alpha(0.35)
            };
            let background = matches!(
                status,
                button::Status::Hovered | button::Status::Pressed
            )
            .then_some(Background::Color(palette.background.strong.color));

            button::Style {
                background,
                text_color,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        })
        .into()
}

fn separator() -> Element<'static, Message> {
    let line =
        container(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(Background::Color(
                        palette.background.strong.color,
                    )),
                    ..container::Style::default()
                }
            });

    container(line).padding([3, 7]).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContextMenuEntry, ContextMenuItem, Language, Translations};

    #[test]
    fn test_custom_context_menu_action_message_preserves_id() {
        let entries = build_entries(
            &[ContextMenuEntry::Item(ContextMenuItem::new(
                "refactor.extract",
                "Extract function",
            ))],
            false,
            MenuState::default(),
            &Translations::default(),
        );

        assert!(matches!(
            &entries[0],
            MenuEntry::Item {
                message: Some(Message::CustomContextMenuAction(id)),
                ..
            }
                if id == "refactor.extract"
        ));
    }

    #[test]
    fn test_custom_entries_precede_default_entries() {
        let entries = build_entries(
            &[ContextMenuEntry::item("custom.format", "Format document")],
            true,
            MenuState::default(),
            &Translations::default(),
        );

        assert_eq!(entries[0].label(), Some("Format document"));
        assert!(matches!(entries[1], MenuEntry::Separator));
        assert_eq!(entries[2].label(), Some("Undo"));
    }

    #[test]
    fn test_context_menu_uses_selected_language() {
        let translations = Translations::new(Language::ChineseSimplified);
        let entries = default_entries(MenuState::default(), &translations);

        assert_eq!(entries[0].label(), Some("撤消"));
        assert_eq!(entries[1].label(), Some("恢复"));
        assert_eq!(entries[3].label(), Some("剪切"));
        assert_eq!(entries[4].label(), Some("复制"));
        assert_eq!(entries[5].label(), Some("粘贴"));
        assert_eq!(entries[7].label(), Some("选择全部"));

        let custom = custom_entries(&[ContextMenuEntry::item(
            "custom.format",
            "Format document",
        )]);
        assert_eq!(custom[0].label(), Some("Format document"));
    }

    #[test]
    fn test_reveal_in_file_manager_entry_emits_request() {
        let translations = Translations::new(Language::English);
        let entries = build_entries(
            &[],
            true,
            MenuState {
                reveal_in_file_manager_enabled: true,
                ..MenuState::default()
            },
            &translations,
        );

        assert!(matches!(
            &entries[0],
            MenuEntry::Item { label, shortcut, message }
                if label == &translations.context_menu_reveal_in_file_manager()
                    && shortcut.is_empty()
                    && matches!(message, Some(Message::RevealInFileManager))
        ));
        assert!(matches!(entries[1], MenuEntry::Separator));
        assert_eq!(entries[2].label(), Some("Undo"));
    }

    #[test]
    fn test_reveal_in_file_manager_respects_default_menu_toggle() {
        let entries = build_entries(
            &[],
            false,
            MenuState {
                reveal_in_file_manager_enabled: true,
                ..MenuState::default()
            },
            &Translations::default(),
        );

        assert!(entries.is_empty());
    }
}
