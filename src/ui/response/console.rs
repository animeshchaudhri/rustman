use iced::{
    widget::{column, scrollable, text},
    Element, Length,
};

use crate::{domain::response::ConsoleLevel, message::Message, state::tabs::RequestTabState};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let mut col = column![].spacing(2).padding(8);
    for entry in &tab.console {
        let prefix = match entry.level {
            ConsoleLevel::Log => "[LOG]",
            ConsoleLevel::Info => "[INFO]",
            ConsoleLevel::Warn => "[WARN]",
            ConsoleLevel::Error => "[ERR]",
        };
        col = col.push(
            text(format!("{prefix} {}", entry.message))
                .size(12)
                .font(crate::ui::theme::MONO),
        );
    }
    if tab.console.is_empty() {
        col = col.push(text("No console output.").size(13));
    }
    scrollable(col).width(Length::Fill).height(Length::Fill).into()
}
