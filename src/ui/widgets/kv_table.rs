use iced::{
    widget::{button, column, container, row, text, text_editor, Space},
    Background, Border, Element, Length,
};

use crate::{
    message::Message,
    ui::theme::{Palette, TEXT_SM},
};


pub fn empty_state(message: &str) -> Element<'static, Message> {
    container(text(message.to_owned()).size(TEXT_SM).color(Palette::text_subtle()))
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

pub fn table_footer(add_msg: Message, add_label: &'static str, bulk_msg: Message) -> Element<'static, Message> {
    let ac = Palette::accent();
    let add_btn = button(
        row![
            text("+").size(TEXT_SM).color(ac),
            text(format!(" {add_label}")).size(TEXT_SM).color(Palette::text_muted()),
        ]
        .spacing(2)
        .align_y(iced::Alignment::Center),
    )
    .on_press(add_msg)
    .style(|_t, status| {
        let hovered = matches!(status, iced::widget::button::Status::Hovered);
        iced::widget::button::Style {
            background: if hovered { Some(Background::Color(Palette::hover())) } else { None },
            text_color: Palette::text_muted(),
            border: Border { color: Palette::border_subtle(), width: 1.0, radius: 6.0.into() },
            ..Default::default()
        }
    })
    .padding([7, 12]);

    let bulk_btn = button(text("Bulk Edit").size(TEXT_SM).color(Palette::text_muted()))
        .on_press(bulk_msg)
        .style(iced::widget::button::text)
        .padding([7, 8]);

    container(row![add_btn, Space::new().width(Length::Fill), bulk_btn].align_y(iced::Alignment::Center))
        .padding(iced::Padding { top: 8.0, right: 10.0, bottom: 10.0, left: 10.0 })
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
                .size(TEXT_SM)
                .color(Palette::text_subtle()),
            Space::new().width(Length::Fill),
            button(text("Done").size(TEXT_SM).color(Palette::accent()))
                .on_press(on_toggle)
                .style(iced::widget::button::text)
                .padding([4, 8]),
        ]
        .align_y(iced::Alignment::Center)
        .padding([4, 8]),
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

