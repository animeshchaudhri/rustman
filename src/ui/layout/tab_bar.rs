use iced::{
    widget::{button, container, mouse_area, row, scrollable, text, Text},
    Background, Border, Element, Length,
};

use crate::{
    app::AppState,
    message::{Message, PaletteMsg, RequestMsg},
    ui::{icons, theme::{Palette, MONO, TEXT_SM, TEXT_XS}},
};

use super::style::{method_color, tab_bar_container, tab_container_style};


fn tab_icon_btn(icon: Text<'static>, msg: Message, side: f32) -> Element<'static, Message> {
    button(container(icon).center_x(Length::Fill).center_y(Length::Fill))
        .on_press(msg)
        .style(|_t, status| {
            let hovered = matches!(status, iced::widget::button::Status::Hovered);
            iced::widget::button::Style {
                background: if hovered { Some(Background::Color(Palette::hover())) } else { None },
                text_color: Palette::text_muted(),
                border: Border { radius: 5.0.into(), ..Default::default() },
                ..Default::default()
            }
        })
        .width(side)
        .height(side)
        .padding(0)
        .into()
}

fn method_chip(method: &crate::domain::request::HttpMethod) -> Element<'static, Message> {
    let color = method_color(method);
    container(text(method.as_str()).size(9).color(color).font(MONO))
        .padding([1, 4])
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(iced::Color { a: 0.16, ..color })),
            border: Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

pub(super) fn multi_tab_bar(state: &AppState) -> Element<'_, Message> {
    let tabs = state.tabs.tabs.iter().enumerate().map(|(i, tab)| {
        let active = i == state.tabs.active;
        let dragging = state.dragging_tab == Some(i);
        let title = if tab.url.is_empty() { tab.title.as_str() } else { &tab.url };
        let short_title: String = if title.len() > 20 {
            format!("{}…", &title[..18])
        } else {
            title.to_owned()
        };

        let modified_dot = if tab.modified && tab.saved_as.is_some() {
            Some(Palette::accent())
        } else if tab.modified {
            Some(Palette::WARNING)
        } else {
            None
        };

        let close_btn = tab_icon_btn(
            icons::close().size(TEXT_XS).color(Palette::text_muted()),
            Message::Request(RequestMsg::CloseTab(i)),
            20.0,
        );

        let title_color = if active { Palette::text() } else { Palette::text_muted() };
        let mut label_row = row![method_chip(&tab.method)].spacing(6).align_y(iced::Alignment::Center);
        if let Some(c) = modified_dot {
            label_row = label_row.push(icons::dot(c));
        }
        label_row = label_row.push(text(short_title).size(TEXT_SM).color(title_color));


        let tab_content = container(
            row![
                container(label_row).padding([9, 12]),
                close_btn,
            ]
            .spacing(2)
            .align_y(iced::Alignment::Center)
            .padding(iced::Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 0.0 }),
        )
        .style(move |t| tab_container_style(t, active, dragging));

        let tab_btn = mouse_area(tab_content)
            .on_press(Message::Request(RequestMsg::TabDragStart(i)))
            .on_release(Message::Request(RequestMsg::TabDragEnd))
            .on_enter(Message::Request(RequestMsg::TabDragOver(i)));

        Element::from(tab_btn)
    });

    let mut tab_items: Vec<Element<Message>> = tabs.collect();
    tab_items.push(tab_icon_btn(
        text("+").size(16).color(Palette::text_muted()),
        Message::Request(RequestMsg::NewTab),
        28.0,
    ));

    let palette_btn = button(icons::search().size(13).color(Palette::text_muted()))
        .on_press(Message::Palette(PaletteMsg::Open))
        .style(iced::widget::button::text)
        .padding([3, 8]);

    let bar = container(
        row![
            scrollable(row(tab_items).spacing(0).width(Length::Shrink))
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new().width(3).scroller_width(3).margin(0),
                ))
                .style(crate::ui::theme::tab_scrollbar)
                .width(Length::Fill),
            palette_btn,
        ]
        .align_y(iced::Alignment::Center)
        .height(32),
    )
    .style(tab_bar_container)
    .width(Length::Fill);

    mouse_area(bar).on_release(Message::Request(RequestMsg::TabDragEnd)).into()
}
