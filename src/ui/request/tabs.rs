use iced::{
    widget::{button, container, row, text},
    Background, Border, Color, Element, Length,
};

use crate::{
    message::{Message, RequestMsg, RequestTab},
    state::tabs::RequestTabState,
    ui::theme::Palette,
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let tabs = [
        (RequestTab::Params, "Params"),
        (RequestTab::Headers, "Headers"),
        (RequestTab::Body, "Body"),
        (RequestTab::Auth, "Auth"),
        (RequestTab::Scripts, "Scripts"),
        (RequestTab::Settings, "Settings"),
    ];

    let btns: Vec<Element<Message>> = tabs
        .iter()
        .map(|(t, label)| {
            let active = &tab.active_request_tab == t;
            let badge: Option<usize> = match t {
                RequestTab::Params => {
                    let n = tab.params.iter().filter(|p| p.enabled && !p.key.is_empty()).count();
                    if n > 0 { Some(n) } else { None }
                }
                RequestTab::Headers => {
                    let n = tab.headers.iter().filter(|h| h.enabled && !h.key.is_empty()).count();
                    if n > 0 { Some(n) } else { None }
                }
                _ => None,
            };
            pill_tab(label, badge, active, Message::Request(RequestMsg::TabSelected(t.clone())))
        })
        .collect();

    container(
        row(btns).spacing(2).padding([4, 8]),
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

pub fn pill_tab<'a>(label: &str, badge: Option<usize>, active: bool, msg: Message) -> Element<'a, Message> {
    let label_owned = label.to_owned();
    let badge_text = badge.map(|n| format!(" {n}")).unwrap_or_default();
    let content = row![
        text(format!("{label_owned}{badge_text}"))
            .size(11)
            .color(if active { Palette::text() } else { Palette::text_muted() }),
    ]
    .align_y(iced::Alignment::Center);

    button(content)
        .on_press(msg)
        .style(move |_, status| pill_tab_style(status, active))
        .padding([4, 10])
        .into()
}

fn pill_tab_style(
    status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    iced::widget::button::Style {
        background: if active {
            Some(Background::Color(Palette::surface_high()))
        } else if hovered {
            Some(Background::Color(Color { r: 0.13, g: 0.13, b: 0.15, a: 1.0 }))
        } else {
            None
        },
        text_color: if active { Palette::text() } else { Palette::text_muted() },
        border: Border {
            color: if active { Palette::border() } else { Color::TRANSPARENT },
            width: if active { 1.0 } else { 0.0 },
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}
