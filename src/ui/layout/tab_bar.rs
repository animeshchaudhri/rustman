use iced::{
    widget::{button, container, row, scrollable, text},
    Element, Length,
};

use crate::{
    app::AppState,
    message::{Message, PaletteMsg, RequestMsg},
    ui::{icons, theme::{Palette, MONO}},
};

use super::style::{method_color, tab_bar_container, tab_btn_style, tab_container_style};

pub(super) fn multi_tab_bar(state: &AppState) -> Element<'_, Message> {
    let tabs = state.tabs.tabs.iter().enumerate().map(|(i, tab)| {
        let active = i == state.tabs.active;
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

        let close_btn = button(icons::close().size(10).color(Palette::text_muted()))
            .on_press(Message::Request(RequestMsg::CloseTab(i)))
            .style(iced::widget::button::text)
            .padding([1, 4]);

        let mut label_row = row![
            text(tab.method.as_str())
                .size(9)
                .color(method_color(&tab.method))
                .font(MONO),
        ]
        .spacing(5)
        .align_y(iced::Alignment::Center);
        if let Some(c) = modified_dot {
            label_row = label_row.push(icons::dot(c));
        }
        label_row = label_row.push(text(short_title).size(11));

        let tab_btn = container(
            row![
                button(label_row)
                    .on_press(Message::Request(RequestMsg::SwitchTab(i)))
                    .style(move |t, s| tab_btn_style(t, s, active))
                    .padding([5, 8]),
                close_btn,
            ]
            .spacing(0)
            .align_y(iced::Alignment::Center),
        )
        .style(move |t| tab_container_style(t, active));

        Element::from(tab_btn)
    });

    let mut tab_items: Vec<Element<Message>> = tabs.collect();
    tab_items.push(
        button(text("+").size(14).color(Palette::text_muted()))
            .on_press(Message::Request(RequestMsg::NewTab))
            .style(iced::widget::button::text)
            .padding([3, 8])
            .into(),
    );

    let palette_btn = button(icons::search().size(13).color(Palette::text_muted()))
        .on_press(Message::Palette(PaletteMsg::Open))
        .style(iced::widget::button::text)
        .padding([3, 8]);

    container(
        row![
            scrollable(row(tab_items).spacing(0).width(Length::Shrink))
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new().width(4).scroller_width(3),
                ))
                .style(crate::ui::theme::thin_scrollbar)
                .width(Length::Fill),
            palette_btn,
        ]
        .align_y(iced::Alignment::Center)
        .height(32),
    )
    .style(tab_bar_container)
    .width(Length::Fill)
    .into()
}
