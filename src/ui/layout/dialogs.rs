use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Space},
    Background, Border, Color, Element, Length,
};

use crate::{
    app::AppState,
    message::{Message, RequestMsg, SaveDialogMsg},
    ui::{icons, theme::{Palette, MONO}},
};

pub(super) fn save_dialog(state: &AppState) -> Element<'_, Message> {
    let name_input = text_input("Request name", &state.save_dialog_name)
        .on_input(|s| Message::SaveDialog(SaveDialogMsg::NameChanged(s)))
        .size(13)
        .padding([8, 10]);

    let mut col_col = column![text("Collection").size(11).color(Palette::text_muted())].spacing(4);

    for col in &state.collections {
        let selected = !state.save_dialog_new_col
            && state.save_dialog_collection_id.as_deref() == Some(&col.id);
        let col_id = col.id.clone();
        col_col = col_col.push(collection_radio_btn(&col.name, selected, move || {
            Message::SaveDialog(SaveDialogMsg::CollectionSelected(col_id.clone()))
        }));
    }

    let new_col_selected = state.save_dialog_new_col;
    col_col = col_col.push(collection_radio_btn(
        "+ New collection",
        new_col_selected,
        || Message::SaveDialog(SaveDialogMsg::ToggleNewCollection),
    ));

    let new_col_input: Option<Element<'_, Message>> = if state.save_dialog_new_col {
        Some(
            text_input("Collection name…", &state.save_dialog_new_col_name)
                .on_input(|s| Message::SaveDialog(SaveDialogMsg::NewCollectionNameChanged(s)))
                .size(12)
                .padding([6, 10])
                .into(),
        )
    } else {
        None
    };

    let btns = row![
        button(text("Cancel").size(12).color(Palette::text_muted()))
            .on_press(Message::SaveDialog(SaveDialogMsg::Close))
            .style(iced::widget::button::text)
            .padding([6, 14]),
        Space::new().width(Length::Fill),
        button(text("Save").size(12).color(Color::WHITE))
            .on_press(Message::SaveDialog(SaveDialogMsg::Confirm))
            .style(accent_btn_style)
            .padding([6, 18]),
    ]
    .align_y(iced::Alignment::Center);

    let mut body_col = column![
        text("Save Request").size(14).color(Palette::text()),
        Space::new().height(8),
        text("Name").size(11).color(Palette::text_muted()),
        name_input,
        Space::new().height(8),
        col_col,
    ]
    .spacing(4)
    .width(440);

    if let Some(inp) = new_col_input { body_col = body_col.push(inp); }
    body_col = body_col.push(Space::new().height(12)).push(btns);

    modal_overlay(container(body_col).style(modal_card_style).padding([20, 24]).width(440))
}

pub(super) fn curl_modal(state: &AppState) -> Element<'_, Message> {
    let title = row![
        text("cURL Command").size(14).color(Palette::text()),
        Space::new().width(Length::Fill),
        button(icons::close().size(16).color(Palette::text_muted()))
            .on_press(Message::Request(RequestMsg::CloseCurlModal))
            .style(iced::widget::button::text)
            .padding([2, 6]),
    ]
    .align_y(iced::Alignment::Center);

    let cmd_box = container(
        scrollable(
            container(
                text(state.curl_modal_command.clone())
                    .font(MONO)
                    .size(11)
                    .color(Color { r: 0.7, g: 0.9, b: 0.7, a: 1.0 }),
            )
            .padding([10, 12])
            .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(200)
        .style(crate::ui::theme::thin_scrollbar),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Palette::background())),
        border: Border { color: Palette::border(), width: 1.0, radius: 6.0.into() },
        ..Default::default()
    })
    .width(Length::Fill);

    let inner = container(
        column![
            title,
            Space::new().height(10),
            text("Run this command in your terminal:").size(11).color(Palette::text_muted()),
            Space::new().height(6),
            cmd_box,
            Space::new().height(12),
            row![
                button(text("Close").size(12).color(Palette::text_muted()))
                    .on_press(Message::Request(RequestMsg::CloseCurlModal))
                    .style(iced::widget::button::text)
                    .padding([6, 14]),
                Space::new().width(Length::Fill),
                button(
                    row![
                        icons::copy().size(12).color(Color::WHITE),
                        text("Copy").size(12).color(Color::WHITE),
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
                )
                .on_press(Message::Request(RequestMsg::CopyCurlToClipboard))
                .style(accent_btn_style)
                .padding([7, 16]),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .spacing(0)
        .width(560),
    )
    .style(modal_card_style)
    .padding([20, 24])
    .width(560);

    modal_overlay(inner)
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn collection_radio_btn<'a>(
    label: &'a str,
    selected: bool,
    on_press: impl Fn() -> Message + 'a,
) -> Element<'a, Message> {
    button(
        row![
            container(Space::new().width(8).height(8))
                .style(move |_| iced::widget::container::Style {
                    background: Some(Background::Color(if selected { Palette::accent() } else { Palette::border() })),
                    border: Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .width(8)
                .height(8),
            text(label)
                .size(12)
                .color(if selected { Palette::accent() } else { Palette::text_muted() }),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .on_press(on_press())
    .style(move |_t, s| iced::widget::button::Style {
        background: if selected || matches!(s, iced::widget::button::Status::Hovered) {
            Some(Background::Color(Palette::surface_high()))
        } else {
            None
        },
        border: Border { radius: 4.0.into(), ..Default::default() },
        text_color: Palette::text(),
        ..Default::default()
    })
    .padding([5, 8])
    .width(Length::Fill)
    .into()
}

fn modal_overlay<'a>(inner: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(
        column![
            Space::new().width(Length::Fill).height(Length::FillPortion(2)),
            row![
                Space::new().width(Length::Fill),
                inner.into(),
                Space::new().width(Length::Fill),
            ],
            Space::new().width(Length::Fill).height(Length::FillPortion(3)),
        ],
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.70 })),
        ..Default::default()
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn modal_card_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface())),
        border: Border { color: Palette::border(), width: 1.0, radius: 10.0.into() },
        shadow: iced::Shadow {
            color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.7 },
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 32.0,
        },
        ..Default::default()
    }
}

fn accent_btn_style(_t: &iced::Theme, s: iced::widget::button::Status) -> iced::widget::button::Style {
    let accent = Palette::accent();
    let bg = if matches!(s, iced::widget::button::Status::Hovered) {
        Color { r: (accent.r + 0.06).min(1.0), g: (accent.g + 0.06).min(1.0), b: (accent.b + 0.04).min(1.0), a: 1.0 }
    } else {
        accent
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}
