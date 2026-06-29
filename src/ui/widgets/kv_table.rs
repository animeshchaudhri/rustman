use iced::{
    widget::{button, column, container, row, text, text_editor, Space},
    Element, Length,
};

use crate::{message::Message, ui::theme::Palette};



pub fn add_row_btn_fn(msg: Message, label: &'static str) -> Element<'static, Message> {
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

