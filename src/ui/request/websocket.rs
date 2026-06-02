use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Background, Border, Color, Element, Length,
};

use crate::{
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::{icons, theme::Palette},
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let ws = &tab.ws;

    let connect_btn = if ws.connected {
        button(text("Disconnect").size(12).color(Color::WHITE))
            .on_press(Message::Request(RequestMsg::WsDisconnect))
            .style(|t, s| disconnect_style(t, s))
            .padding([5, 14])
    } else {
        button(text("Connect").size(12).color(Color::WHITE))
            .on_press(Message::Request(RequestMsg::WsConnect))
            .style(|t, s| connect_style(t, s))
            .padding([5, 14])
    };

    let url_row = row![
        text_input("wss://echo.websocket.org", &ws.url)
            .on_input(|s| Message::Request(RequestMsg::WsUrlChanged(s)))
            .size(13)
            .padding([6, 10])
            .width(Length::Fill),
        connect_btn,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .padding([8, 10]);

    let status_color = if ws.connected { Palette::SUCCESS } else { Palette::text_muted() };
    let status_bar = container(
        row![
            icons::dot(status_color),
            text(if ws.connected { "Connected" } else { "Disconnected" })
                .size(11)
                .color(status_color),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .padding([2, 12]);

    let mut feed = column![].spacing(2).padding([4, 8]);
    for msg in &ws.messages {
        let label = if msg.is_outgoing { icons::arrow_right() } else { icons::arrow_left() };
        let color = if msg.is_outgoing { Palette::accent() } else { Palette::SUCCESS };
        feed = feed.push(
            row![
                label.size(11).color(color),
                text(&msg.text).size(12).font(crate::ui::theme::MONO),
            ]
            .spacing(6),
        );
    }

    let send_row = row![
        text_input("Message to send…", &ws.draft)
            .on_input(|s| Message::Request(RequestMsg::WsMessageChanged(s)))
            .size(12)
            .padding([5, 8])
            .width(Length::Fill),
        button(text("Send").size(12))
            .on_press(Message::Request(RequestMsg::WsSend))
            .style(|t, s| connect_style(t, s))
            .padding([5, 12]),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center)
    .padding([6, 8]);

    column![
        url_row,
        status_bar,
        scrollable(feed).height(Length::Fill),
        send_row,
    ]
    .spacing(0)
    .into()
}

fn connect_style(_t: &iced::Theme, s: iced::widget::button::Status) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Background::Color(match s {
            iced::widget::button::Status::Hovered => Color { r: 0.15, g: 0.68, b: 0.44, a: 1.0 },
            _ => Color { r: 0.12, g: 0.60, b: 0.40, a: 1.0 },
        })),
        text_color: Color::WHITE,
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn disconnect_style(_t: &iced::Theme, s: iced::widget::button::Status) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Background::Color(match s {
            iced::widget::button::Status::Hovered => Color { r: 0.80, g: 0.25, b: 0.25, a: 1.0 },
            _ => Color { r: 0.70, g: 0.18, b: 0.18, a: 1.0 },
        })),
        text_color: Color::WHITE,
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}
