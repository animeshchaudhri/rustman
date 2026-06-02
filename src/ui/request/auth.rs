use iced::{
    widget::{column, pick_list, row, text, text_input},
    Element, Length,
};

use crate::{
    domain::request::AuthType,
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::styles,
};

const AUTH_TYPES: &[&str] = &["none", "bearer", "basic", "apikey", "jwt-user", "cookie"];

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let type_picker = row![
        text("Auth type").size(12).width(80),
        pick_list(
            AUTH_TYPES,
            Some(tab.auth_type.as_str()),
            |s| Message::Request(RequestMsg::AuthTypeChanged(s.to_owned()))
        )
        .text_size(12)
        .width(140),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .padding([8, 12]);

    let fields: Element<Message> = match &tab.auth_type {
        AuthType::Bearer => column![
            labeled_input("Token", &tab.bearer_token, |s| {
                Message::Request(RequestMsg::BearerTokenChanged(s))
            }),
        ]
        .spacing(6)
        .padding([4, 12])
        .into(),

        AuthType::Basic => column![
            labeled_input("Username", &tab.basic_user, |s| {
                Message::Request(RequestMsg::BasicUserChanged(s))
            }),
            labeled_input("Password", &tab.basic_pass, |s| {
                Message::Request(RequestMsg::BasicPassChanged(s))
            }),
        ]
        .spacing(6)
        .padding([4, 12])
        .into(),

        AuthType::ApiKey => column![
            labeled_input("Key name", &tab.api_key_name, |s| {
                Message::Request(RequestMsg::ApiKeyNameChanged(s))
            }),
            labeled_input("Value", &tab.api_key_value, |s| {
                Message::Request(RequestMsg::ApiKeyValueChanged(s))
            }),
            row![
                text("Location").size(12).width(80),
                pick_list(
                    &["header", "query"][..],
                    Some(tab.api_key_location.as_str()),
                    |s| Message::Request(RequestMsg::ApiKeyLocationChanged(s.to_owned()))
                )
                .text_size(12)
                .width(120),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(6)
        .padding([4, 12])
        .into(),

        AuthType::Cookie => column![
            labeled_input("Cookie string", &tab.cookie_string, |s| {
                Message::Request(RequestMsg::CookieStringChanged(s))
            }),
        ]
        .spacing(6)
        .padding([4, 12])
        .into(),

        AuthType::JwtUser => column![
            labeled_input("Subject / sub", &tab.jwt_subject, |s| {
                Message::Request(RequestMsg::JwtSubjectChanged(s))
            }),
            labeled_input("Secret", &tab.jwt_secret, |s| {
                Message::Request(RequestMsg::JwtSecretChanged(s))
            }),
            row![
                text("Algorithm").size(12).width(80),
                pick_list(
                    &["HS256"][..],
                    Some(tab.jwt_algo.as_str()),
                    |s| Message::Request(RequestMsg::JwtAlgoChanged(s.to_owned()))
                )
                .text_size(12)
                .width(100),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                iced::widget::Space::new().width(80),
                text("Token is generated per-request (exp = now + 1h)").size(10)
                    .color(crate::ui::theme::Palette::text_subtle()),
            ]
            .spacing(8),
        ]
        .spacing(6)
        .padding([4, 12])
        .into(),

        AuthType::None => {
            iced::widget::Space::new().into()
        }
    };

    column![type_picker, fields].spacing(0).into()
}

fn labeled_input<'a>(
    label: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(label).size(12).width(80),
        text_input(label, value)
            .on_input(on_input)
            .size(12)
            .padding([4, 8])
            .width(Length::Fill)
            .style(styles::field_input),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}
