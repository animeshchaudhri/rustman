use iced::{
    widget::{button, checkbox, column, container, row, scrollable, text, text_editor, text_input, Space},
    Background, Element, Length,
};

use crate::{domain::request::KeyValue, message::Message, ui::{icons, styles, theme::Palette}};

type ToggleFn = fn(usize) -> Message;
type KeyFn   = fn(usize, String) -> Message;
type RemoveFn = fn(usize) -> Message;

pub fn view<'a>(
    items: &'a [KeyValue],
    on_toggle: ToggleFn,
    on_key: KeyFn,
    on_value: KeyFn,
    on_remove: RemoveFn,
    on_add: Message,
    empty_hint: &'static str,
) -> Element<'a, Message> {
    let header = row![
        container(text("").size(1)).width(24),         // checkbox placeholder
        container(text("").size(1)).width(24),         // spacer
        text("Key").size(10).color(Palette::text_subtle()).width(Length::Fill),
        text("Value").size(10).color(Palette::text_subtle()).width(Length::Fill),
        container(text("").size(1)).width(28),         // remove placeholder
    ]
    .spacing(6)
    .padding(iced::Padding { top: 3.0, right: 10.0, bottom: 3.0, left: 10.0 });

    let header = container(header)
        .style(crate::ui::styles::section_header)
        .width(Length::Fill);

    if items.is_empty() {
        return column![
            header,
            add_row_btn(on_add, empty_hint),
        ]
        .spacing(0)
        .into();
    }

    let rows = items.iter().enumerate().map(|(i, item)| {
        let is_even = i % 2 == 0;
        let bg = if is_even { Palette::row_even() } else { Palette::row_odd() };
        let row_container = container(
            row![
                checkbox(item.enabled)
                    .on_toggle(move |_| on_toggle(i))
                    .size(12)
                    .spacing(0),
                text(format!("{}", i + 1))
                    .size(9)
                    .color(Palette::text_subtle())
                    .font(crate::ui::theme::MONO)
                    .width(20),
                text_input("key", &item.key)
                    .on_input(move |s| on_key(i, s))
                    .size(12)
                    .padding([3, 6])
                    .width(Length::Fill)
                    .style(styles::cell_input),
                text_input("value", &item.value)
                    .on_input(move |s| on_value(i, s))
                    .size(12)
                    .padding([3, 6])
                    .width(Length::Fill)
                    .style(styles::cell_input),
                button(icons::close().size(10).color(Palette::text_subtle()))
                    .on_press(on_remove(i))
                    .style(iced::widget::button::text)
                    .padding([2, 6]),
            ]
            .spacing(4)
            .padding(iced::Padding { top: 2.0, right: 8.0, bottom: 2.0, left: 10.0 })
            .align_y(iced::Alignment::Center),
        )
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(bg)),
            ..Default::default()
        })
        .width(Length::Fill);
        Element::from(row_container)
    });

    let mut col = column![header].spacing(0);
    for r in rows {
        col = col.push(r);
    }
    col = col.push(add_row_btn(on_add, "+ Add"));

    scrollable(col)
        .height(Length::Fill)
        .style(crate::ui::theme::thin_scrollbar)
        .into()
}

fn add_row_btn(msg: Message, label: &'static str) -> Element<'static, Message> {
    let ac = Palette::accent();
    let btn = button(
        row![
            text("+").size(12).color(ac),
            text(format!(" {label}")).size(11).color(Palette::text_muted()),
        ]
        .spacing(2)
        .align_y(iced::Alignment::Center),
    )
    .on_press(msg)
    .style(iced::widget::button::text)
    .padding([5, 12]);
    container(btn)
        .width(Length::Fill)
        .into()
}

pub fn bulk_toggle_bar(on_toggle: Message, label: &str) -> Element<'static, Message> {
    let label = label.to_owned();
    container(
        row![
            Space::new().width(Length::Fill),
            button(text(label).size(10).color(Palette::text_muted()))
                .on_press(on_toggle)
                .style(iced::widget::button::text)
                .padding([2, 8]),
        ]
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .into()
}

pub fn bulk_panel<'a>(
    content: &'a text_editor::Content,
    on_edited: fn(text_editor::Action) -> Message,
    on_toggle: Message,
) -> Element<'a, Message> {
    let toolbar = container(
        row![
            text("Key: Value per line · prefix # to disable")
                .size(10)
                .color(Palette::text_subtle()),
            Space::new().width(Length::Fill),
            button(text("Done").size(10).color(Palette::accent()))
                .on_press(on_toggle)
                .style(iced::widget::button::text)
                .padding([2, 8]),
        ]
        .align_y(iced::Alignment::Center)
        .padding([2, 8]),
    )
    .style(crate::ui::styles::section_header)
    .width(Length::Fill);

    let editor = text_editor(content)
        .on_action(on_edited)
        .height(Length::Fill)
        .font(crate::ui::theme::MONO)
        .style(crate::ui::styles::scripts_editor);

    column![toolbar, editor].spacing(0).height(Length::Fill).into()
}

