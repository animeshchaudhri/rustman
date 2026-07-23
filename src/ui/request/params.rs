use iced::{
    widget::{button, checkbox, column, container, row, scrollable, text, text_input},
    Background, Element, Length,
};

use crate::{
    domain::environment::substitute,
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::{icons, theme::{Palette, TEXT_SM, TEXT_XS}, widgets::kv_table},
};

pub fn view<'a>(
    tab: &'a RequestTabState,
    env: Option<&'a crate::domain::environment::AppEnvironment>,
) -> Element<'a, Message> {
    match &tab.params_bulk {
        Some(content) => kv_table::bulk_panel(
            content,
            |a| Message::Request(RequestMsg::ParamsBulkEdited(a)),
            Message::Request(RequestMsg::ParamsBulkToggle),
        ),
        None => param_table(&tab.params, env),
    }
}

fn param_table<'a>(
    items: &'a [crate::domain::request::KeyValue],
    env: Option<&'a crate::domain::environment::AppEnvironment>,
) -> Element<'a, Message> {
    let has_env = env.is_some() && items.iter().any(|p| p.key.contains("{{") || p.value.contains("{{"));

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
        Message::Request(RequestMsg::ParamAdded),
        "Add param",
        Message::Request(RequestMsg::ParamsBulkToggle),
    );

    if items.is_empty() {
        return column![header, footer()].spacing(0).into();
    }

    let mut col = column![header].spacing(0);

    for (i, item) in items.iter().enumerate() {
        let is_even = i % 2 == 0;
        let bg = if is_even { Palette::row_even() } else { Palette::row_odd() };

        let row_el = container(
            row![
                checkbox(item.enabled)
                    .on_toggle(move |_| Message::Request(RequestMsg::ParamToggled(i)))
                    .size(12)
                    .spacing(0)
                    .style(crate::ui::styles::checkbox),
                text(format!("{}", i + 1))
                    .size(TEXT_XS)
                    .color(Palette::text_subtle())
                    .font(crate::ui::theme::MONO)
                    .width(20),
                text_input("key", &item.key)
                    .on_input(move |s| Message::Request(RequestMsg::ParamKeyChanged(i, s)))
                    .size(TEXT_SM)
                    .padding([5, 6])
                    .width(Length::Fill)
                    .style(crate::ui::styles::cell_input),
                text_input("value", &item.value)
                    .on_input(move |s| Message::Request(RequestMsg::ParamValueChanged(i, s)))
                    .size(TEXT_SM)
                    .padding([5, 6])
                    .width(Length::Fill)
                    .style(crate::ui::styles::cell_input),
                button(icons::close().size(TEXT_XS).color(Palette::text_subtle()))
                    .on_press(Message::Request(RequestMsg::ParamRemoved(i)))
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

        if has_env && (item.key.contains("{{") || item.value.contains("{{")) {
            let expanded_key = substitute(&item.key, env);
            let expanded_val = substitute(&item.value, env);
            let preview = format!("↳ {}: {}", expanded_key, expanded_val);
            col = col.push(
                container(
                    text(preview).size(9).color(Palette::accent()).font(crate::ui::theme::MONO),
                )
                .padding(iced::Padding { top: 0.0, right: 8.0, bottom: 2.0, left: 70.0 })
                .width(Length::Fill),
            );
        }
    }

    col = col.push(footer());

    scrollable(col)
        .height(Length::Fill)
        .style(crate::ui::theme::thin_scrollbar)
        .into()
}
