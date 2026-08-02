use iced::{
    widget::{button, container, row, text},
    Alignment, Background, Border, Color, Element, Length,
};

use crate::{
    message::{Message, RequestMsg, RequestTab},
    state::tabs::RequestTabState,
    ui::{icons, theme::{Palette, TEXT_SM, TEXT_XS}},
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let tabs = [
        (RequestTab::Params, "Params"),
        (RequestTab::Headers, "Headers"),
        (RequestTab::Body, "Body"),
        (RequestTab::Auth, "Auth"),
        (RequestTab::Scripts, "Scripts"),
    ];

    let btns: Vec<Element<Message>> = tabs
        .iter()
        .map(|(t, label)| {
            let active = &tab.active_request_tab == t;
            let (badge, dot): (Option<usize>, Option<Color>) = match t {
                RequestTab::Params => {
                    let n = tab.params.iter().filter(|p| p.enabled && !p.key.is_empty()).count();
                    (if n > 0 { Some(n) } else { None }, None)
                }
                RequestTab::Headers => {
                    let n = tab.headers.iter().filter(|h| h.enabled && !h.key.is_empty()).count();
                    (if n > 0 { Some(n) } else { None }, None)
                }
                RequestTab::Auth => {
                    let has_auth = !tab.bearer_token.is_empty()
                        || !tab.basic_user.is_empty()
                        || !tab.basic_pass.is_empty()
                        || !tab.api_key_name.is_empty()
                        || !tab.api_key_value.is_empty()
                        || !tab.cookie_string.is_empty()
                        || !tab.jwt_secret.is_empty()
                        || !tab.jwt_subject.is_empty();
                    (None, if has_auth { Some(Palette::accent()) } else { None })
                }
                _ => (None, None),
            };
            pill_tab(label, badge, dot, active, Message::Request(RequestMsg::TabSelected(t.clone())))
        })
        .collect();

    // Segmented control on a recessed track.
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

pub fn pill_tab<'a>(label: &str, badge: Option<usize>, dot: Option<Color>, active: bool, msg: Message) -> Element<'a, Message> {
    let mut content = row![].align_y(Alignment::Center).spacing(6);
    if let Some(color) = dot {
        content = content.push(icons::dot(color));
    }
    content = content.push(
        text(label.to_owned())
            .size(TEXT_SM)
            .color(if active { Palette::text() } else { Palette::text_muted() })
            .font(if active { crate::ui::theme::UI_FONT_MEDIUM } else { crate::ui::theme::UI_FONT }),
    );
    if let Some(n) = badge {
        content = content.push(
            container(text(n.to_string()).size(TEXT_XS).color(if active { Palette::accent() } else { Palette::text_subtle() }))
                .padding([1, 6])
                .style(move |_| iced::widget::container::Style {
                    background: Some(Background::Color(if active { Palette::accent_soft() } else { Palette::surface_high() })),
                    border: Border { radius: 100.0.into(), ..Default::default() },
                    ..Default::default()
                }),
        );
    }

    button(content)
        .on_press(msg)
        .style(move |_, status| pill_tab_style(status, active))
        .padding([6, 12])
        .into()
}

fn pill_tab_style(
    status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    iced::widget::button::Style {
        background: if active {
            Some(Background::Color(Palette::surface_raised()))
        } else if hovered {
            Some(Background::Color(Palette::hover()))
        } else {
            None
        },
        text_color: if active { Palette::text() } else { Palette::text_muted() },
        border: Border {
            color: if active { Palette::border() } else { Color::TRANSPARENT },
            width: if active { 1.0 } else { 0.0 },
            radius: 7.0.into(),
        },
        shadow: if active {
            iced::Shadow { color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.25 }, offset: iced::Vector::new(0.0, 1.0), blur_radius: 4.0 }
        } else {
            iced::Shadow::default()
        },
        ..Default::default()
    }
}
