use iced::{
    widget::{button, checkbox, column, container, row, scrollable, text, text_input},
    Background, Border, Color, Element, Length,
};

use crate::{domain::request::KeyValue, message::Message, ui::theme::Palette};

type ToggleFn = fn(usize) -> Message;
type KeyFn = fn(usize, String) -> Message;
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
    let header = container(
        row![
            container(text("#").size(9).color(Palette::text_subtle())).width(24),
            container(text("").size(1)).width(24),
            text("Key").size(10).color(Palette::text_muted()).width(Length::Fill),
            text("Value").size(10).color(Palette::text_muted()).width(Length::Fill),
            container(text("").size(1)).width(28),
        ]
        .spacing(6)
        .padding(iced::Padding { top: 4.0, right: 12.0, bottom: 4.0, left: 10.0 }),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Color { r: 0.10, g: 0.10, b: 0.10, a: 1.0 })),
        ..Default::default()
    })
    .width(Length::Fill);

    if items.is_empty() {
        let empty = column![
            header,
            container(
                button(
                    row![
                        text("+").size(13).color(Palette::accent()),
                        text(format!(" {empty_hint}")).size(12).color(Palette::text_muted()),
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(4),
                )
                .on_press(on_add)
                .style(iced::widget::button::text)
                .padding([10, 14]),
            )
            .width(Length::Fill),
        ]
        .spacing(0);
        return empty.into();
    }

    let rows = items.iter().enumerate().map(|(i, item)| {
        let is_even = i % 2 == 0;
        let row_el: Element<Message> = container(
            row![
                container(
                    text(format!("{}", i + 1))
                        .size(9)
                        .color(Palette::text_subtle())
                        .font(iced::Font::MONOSPACE),
                )
                .width(24),
                checkbox("", item.enabled)
                    .on_toggle(move |_| on_toggle(i))
                    .size(13)
                    .spacing(0),
                text_input("key", &item.key)
                    .on_input(move |s| on_key(i, s))
                    .size(12)
                    .padding([4, 6])
                    .width(Length::Fill)
                    .style(cell_input_style),
                text_input("value", &item.value)
                    .on_input(move |s| on_value(i, s))
                    .size(12)
                    .padding([4, 6])
                    .width(Length::Fill)
                    .style(cell_input_style),
                button(text("✕").size(10).color(Palette::text_muted()))
                    .on_press(on_remove(i))
                    .style(iced::widget::button::text)
                    .padding([3, 6]),
            ]
            .spacing(6)
            .padding(iced::Padding { top: 2.0, right: 8.0, bottom: 2.0, left: 10.0 })
            .align_y(iced::Alignment::Center),
        )
        .style(move |_| iced::widget::container::Style {
            background: if is_even {
                Some(Background::Color(Color { r: 0.095, g: 0.095, b: 0.095, a: 1.0 }))
            } else {
                None
            },
            ..Default::default()
        })
        .width(Length::Fill)
        .into();
        row_el
    });

    let mut col = column![header].spacing(0);
    for r in rows {
        col = col.push(r);
    }
    col = col.push(
        container(
            button(
                row![
                    text("+").size(13).color(Palette::accent()),
                    text(" Add row").size(11).color(Palette::text_muted()),
                ]
                .align_y(iced::Alignment::Center)
                .spacing(4),
            )
            .on_press(on_add)
            .style(iced::widget::button::text)
            .padding([6, 14]),
        )
        .width(Length::Fill),
    );

    scrollable(col)
        .height(Length::Fill)
        .style(crate::ui::theme::thin_scrollbar)
        .into()
}

fn cell_input_style(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    iced::widget::text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: match status {
                iced::widget::text_input::Status::Focused => {
                    Color { r: 0.37, g: 0.58, b: 0.95, a: 0.6 }
                }
                iced::widget::text_input::Status::Hovered => {
                    Color { r: 0.25, g: 0.25, b: 0.25, a: 1.0 }
                }
                _ => Color::TRANSPARENT,
            },
            width: 1.0,
            radius: 3.0.into(),
        },
        icon: Palette::text_muted(),
        placeholder: Color { r: 0.35, g: 0.35, b: 0.35, a: 1.0 },
        value: Palette::text(),
        selection: Color { r: 0.37, g: 0.58, b: 0.95, a: 0.3 },
    }
}
