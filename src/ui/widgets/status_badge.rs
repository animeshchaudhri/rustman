use iced::{widget::text, Element};
use crate::{message::Message, ui::theme::Palette};

pub fn view(status: u16) -> Element<'static, Message> {
    let color = match status {
        200..=299 => Palette::SUCCESS,
        300..=399 => Palette::WARNING,
        400..=599 => Palette::ERROR,
        _ => Palette::text_muted(),
    };
    text(status.to_string()).size(13).color(color).font(iced::Font::MONOSPACE).into()
}
