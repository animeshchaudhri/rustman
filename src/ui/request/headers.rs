use iced::{
    widget::{button, checkbox, column, container, pick_list, row, scrollable, text, text_input},
    Background, Element, Length,
};

use crate::{
    domain::request::KeyValue,
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::{icons, styles, theme::{Palette, TEXT_SM, TEXT_XS}, widgets::kv_table},
};

const CONTENT_TYPE_SUGGESTIONS: &[&str] = &[
    "application/json",
    "application/xml",
    "text/plain",
    "text/html",
    "text/css",
    "text/javascript",
    "application/javascript",
    "application/x-www-form-urlencoded",
    "multipart/form-data",
    "application/octet-stream",
    "application/pdf",
    "application/zip",
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/svg+xml",
    "audio/mpeg",
    "video/mp4",
    "application/graphql-response+json",
    "application/ld+json",
    "application/merge-patch+json",
    "application/problem+json",
    "application/vnd.api+json",
    "application/yaml",
    "text/csv",
    "text/xml",
    "application/grpc",
    "application/x-protobuf",
    "application/x-ndjson",
];

fn is_content_type_key(key: &str) -> bool {
    key.eq_ignore_ascii_case("content-type")
}

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    match &tab.headers_bulk {
        Some(content) => kv_table::bulk_panel(
            content,
            |a| Message::Request(RequestMsg::HeadersBulkEdited(a)),
            Message::Request(RequestMsg::HeadersBulkToggle),
        ),
        None => header_table(&tab.headers),
    }
}

fn header_table(items: &[KeyValue]) -> Element<'_, Message> {
    let header = row![
        container(text("").size(1)).width(24),
        container(text("").size(1)).width(24),
        text("Key").size(TEXT_SM).color(Palette::text_muted()).width(Length::Fill),
        text("Value").size(TEXT_SM).color(Palette::text_muted()).width(Length::Fill),
        container(text("").size(1)).width(28),
    ]
    .spacing(6)
    .padding(iced::Padding { top: 6.0, right: 10.0, bottom: 6.0, left: 10.0 });

    let header = container(header)
        .style(crate::ui::styles::section_header)
        .width(Length::Fill);

    let footer = || kv_table::table_footer(
        Message::Request(RequestMsg::HeaderAdded),
        "Add header",
        Message::Request(RequestMsg::HeadersBulkToggle),
    );

    if items.is_empty() {
        return column![header, footer()].spacing(0).into();
    }

    let mut col = column![header].spacing(0);

    for (i, item) in items.iter().enumerate() {
        let is_even = i % 2 == 0;
        let bg = if is_even { Palette::row_even() } else { Palette::row_odd() };
        let is_ct = is_content_type_key(&item.key);

        let value_widget: Element<Message> = if is_ct {
            let selected = CONTENT_TYPE_SUGGESTIONS
                .iter()
                .find(|s| **s == item.value)
                .copied();
            let suggestions: Vec<&str> = CONTENT_TYPE_SUGGESTIONS
                .iter()
                .filter(|s| item.value.is_empty() || s.starts_with(&item.value))
                .copied()
                .collect();
            if suggestions.is_empty() {
                text_input("value", &item.value)
                    .on_input(move |s| Message::Request(RequestMsg::HeaderValueChanged(i, s)))
                    .size(TEXT_SM)
                    .padding([5, 6])
                    .width(Length::Fill)
                    .style(styles::cell_input)
                    .into()
            } else {
                let current = selected.unwrap_or("");
                pick_list(
                    suggestions,
                    Some(current),
                    move |s| Message::Request(RequestMsg::HeaderValueChanged(i, s.to_owned())),
                )
                .text_size(TEXT_SM)
                .padding([5, 6])
                .width(Length::Fill)
                .style(styles::pick_list)
                .into()
            }
        } else {
            text_input("value", &item.value)
                .on_input(move |s| Message::Request(RequestMsg::HeaderValueChanged(i, s)))
                .size(TEXT_SM)
                .padding([5, 6])
                .width(Length::Fill)
                .style(styles::cell_input)
                .into()
        };

        let row_el = container(
            row![
                checkbox(item.enabled)
                    .on_toggle(move |_| Message::Request(RequestMsg::HeaderToggled(i)))
                    .size(12)
                    .spacing(0)
                    .style(styles::checkbox),
                text(format!("{}", i + 1))
                    .size(TEXT_XS)
                    .color(Palette::text_subtle())
                    .font(crate::ui::theme::MONO)
                    .width(20),
                text_input("key", &item.key)
                    .on_input(move |s| Message::Request(RequestMsg::HeaderKeyChanged(i, s)))
                    .size(TEXT_SM)
                    .padding([5, 6])
                    .width(Length::Fill)
                    .style(styles::cell_input),
                value_widget,
                button(icons::close().size(TEXT_XS).color(Palette::text_subtle()))
                    .on_press(Message::Request(RequestMsg::HeaderRemoved(i)))
                    .style(iced::widget::button::text)
                    .padding([2, 6]),
            ]
            .spacing(4)
            .padding(iced::Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 10.0 })
            .align_y(iced::Alignment::Center),
        )
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(bg)),
            ..Default::default()
        })
        .width(Length::Fill);

        col = col.push(row_el);
    }

    col = col.push(footer());

    scrollable(col)
        .height(Length::Fill)
        .direction(iced::widget::scrollable::Direction::Vertical(
            crate::ui::theme::grabbable_scrollbar(),
        ))
        .style(crate::ui::theme::thin_scrollbar)
        .into()
}
