use iced::{widget::text, Element};
use crate::{domain::request::HttpMethod, message::Message, ui::theme::Palette};

pub fn view(method: &HttpMethod) -> Element<'static, Message> {
    let color = match method {
        HttpMethod::Get => Palette::GET,
        HttpMethod::Post => Palette::POST,
        HttpMethod::Put => Palette::PUT,
        HttpMethod::Patch => Palette::PATCH,
        HttpMethod::Delete => Palette::DELETE,
        HttpMethod::Head | HttpMethod::Options => Palette::HEAD,
    };
    text(method.as_str())
        .size(11)
        .color(color)
        .font(crate::ui::theme::MONO)
        .into()
}
