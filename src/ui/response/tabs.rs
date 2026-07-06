use iced::{widget::{container, row}, Background, Border, Element, Length};

use crate::{
    message::{Message, ResponseMsg, ResponseTab},
    state::tabs::RequestTabState,
    ui::{request::tabs::pill_tab, theme::Palette},
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let tabs = [
        (ResponseTab::Body, "Body"),
        (ResponseTab::Headers, "Headers"),
        (ResponseTab::Cookies, "Cookies"),
    ];

    let btns: Vec<Element<Message>> = tabs
        .iter()
        .map(|(t, label)| {
            let active = &tab.active_response_tab == t;
            pill_tab(
                label,
                None,
                active,
                Message::Response(ResponseMsg::TabSelected(t.clone())),
            )
        })
        .collect();

    container(
        row(btns).spacing(4).padding([6, 10]),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface())),
        border: Border {
            color: Palette::border_subtle(),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}
