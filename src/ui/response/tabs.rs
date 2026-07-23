use iced::{
    widget::{container, row},
    Background, Border, Element, Length,
};

use crate::{
    message::{Message, ResponseMsg, ResponseTab},
    state::tabs::RequestTabState,
    ui::{request::tabs::pill_tab, theme::Palette},
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let tabs: Vec<(ResponseTab, &str)> = vec![
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
                None,
                active,
                Message::Response(ResponseMsg::TabSelected(t.clone())),
            )
        })
        .collect();

    container(
        container(row(btns).spacing(2).padding(3))
            .style(|_| iced::widget::container::Style {
                background: Some(Background::Color(Palette::background())),
                border: Border {
                    color: Palette::border_subtle(),
                    width: 1.0,
                    radius: 9.0.into(),
                },
                ..Default::default()
            }),
    )
    .padding([8, 12])
    .width(Length::Fill)
    .into()
}
