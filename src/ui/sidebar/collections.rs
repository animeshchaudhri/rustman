use iced::{
    widget::{button, column, container, horizontal_rule, row, scrollable, text, Space},
    Background, Border, Color, Element, Length,
};

use crate::{
    app::AppState,
    message::{Message, SidebarMsg},
    ui::theme::Palette,
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let action_bar = container(
        row![
            text("Collections").size(11).color(Palette::text_muted()),
            Space::with_width(Length::Fill),
            button(
                row![
                    text("+").size(12).color(Palette::accent()),
                    text(" New").size(11).color(Palette::text_muted()),
                ]
                .spacing(2)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::Sidebar(SidebarMsg::NewCollection))
            .style(|_t, s| iced::widget::button::Style {
                background: if matches!(s, iced::widget::button::Status::Hovered) {
                    Some(Background::Color(Palette::surface_high()))
                } else {
                    None
                },
                text_color: Palette::text_muted(),
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .padding([3, 8]),
        ]
        .align_y(iced::Alignment::Center)
        .padding([6, 8]),
    )
    .width(Length::Fill);

    let mut col = column![action_bar, horizontal_rule(1)].spacing(0);

    for collection in &state.collections {
        let is_expanded = state.sidebar.expanded.contains(&collection.id);
        let arrow = if is_expanded { "▼" } else { "▶" };

        let col_id = collection.id.clone();
        let col_id_del = collection.id.clone();

        let header = row![
            button(
                row![
                    text(arrow).size(9).color(Palette::text_muted()),
                    text(&collection.name).size(12).color(Palette::text()),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::Sidebar(SidebarMsg::CollectionToggled(col_id)))
            .style(|_t, s| iced::widget::button::Style {
                background: if matches!(s, iced::widget::button::Status::Hovered) {
                    Some(Background::Color(Palette::surface_high()))
                } else {
                    None
                },
                text_color: Palette::text(),
                ..Default::default()
            })
            .width(Length::Fill)
            .padding([5, 8]),
            button(text("×").size(12).color(Palette::text_subtle()))
                .on_press(Message::Sidebar(SidebarMsg::DeleteCollection(col_id_del)))
                .style(|_t, s| iced::widget::button::Style {
                    background: if matches!(s, iced::widget::button::Status::Hovered) {
                        Some(Background::Color(Color { r: 0.3, g: 0.05, b: 0.05, a: 1.0 }))
                    } else {
                        None
                    },
                    text_color: if matches!(s, iced::widget::button::Status::Hovered) {
                        Palette::ERROR
                    } else {
                        Palette::text_subtle()
                    },
                    border: Border { radius: 3.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .padding([3, 6]),
        ]
        .align_y(iced::Alignment::Center)
        .width(Length::Fill);

        col = col.push(header);

        if is_expanded {
            let requests = state
                .requests
                .get(&collection.id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            if requests.is_empty() {
                col = col.push(
                    container(
                        text("No requests.").size(11).color(Palette::text_subtle()),
                    )
                    .padding(iced::Padding { top: 2.0, right: 8.0, bottom: 2.0, left: 24.0 }),
                );
            }

            for req in requests {
                let is_selected =
                    state.sidebar.selected_request.as_deref() == Some(req.id.as_str());
                let method_color = method_color(req.method.as_str());
                let req_id = req.id.clone();
                let col_id = req.collection_id.clone();

                let item_row = row![
                    button(
                        row![
                            container(
                                text(req.method.as_str())
                                    .size(9)
                                    .color(method_color)
                                    .font(iced::Font::MONOSPACE),
                            )
                            .width(32),
                            text(&req.name).size(12).color(Palette::text()),
                        ]
                        .spacing(4)
                        .align_y(iced::Alignment::Center),
                    )
                    .on_press(Message::Sidebar(SidebarMsg::RequestOpened(req.clone())))
                    .style(move |t, s| req_item_style(t, s, is_selected))
                    .width(Length::Fill)
                    .padding(iced::Padding { top: 4.0, right: 4.0, bottom: 4.0, left: 24.0 }),
                    button(text("×").size(11).color(Palette::text_subtle()))
                        .on_press(Message::Sidebar(SidebarMsg::DeleteRequest {
                            id: req_id,
                            collection_id: col_id,
                        }))
                        .style(|_t, s| iced::widget::button::Style {
                            background: if matches!(s, iced::widget::button::Status::Hovered) {
                                Some(Background::Color(Color { r: 0.3, g: 0.05, b: 0.05, a: 1.0 }))
                            } else {
                                None
                            },
                            text_color: if matches!(s, iced::widget::button::Status::Hovered) {
                                Palette::ERROR
                            } else {
                                Palette::text_subtle()
                            },
                            border: Border { radius: 3.0.into(), ..Default::default() },
                            ..Default::default()
                        })
                        .padding([2, 5]),
                ]
                .align_y(iced::Alignment::Center)
                .width(Length::Fill);

                col = col.push(item_row);
            }
        }
    }

    if state.collections.is_empty() {
        col = col.push(
            container(
                text("No collections yet.\nClick + New to create one.")
                    .size(12)
                    .color(Palette::text_muted()),
            )
            .padding([12, 8]),
        );
    }

    scrollable(col)
        .height(Length::Fill)
        .style(crate::ui::theme::thin_scrollbar)
        .into()
}

fn method_color(method: &str) -> iced::Color {
    match method {
        "GET" => Palette::GET,
        "POST" => Palette::POST,
        "PUT" => Palette::PUT,
        "PATCH" => Palette::PATCH,
        "DELETE" => Palette::DELETE,
        _ => Palette::HEAD,
    }
}

fn req_item_style(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: if selected {
            Some(Background::Color(Palette::surface_high()))
        } else if matches!(status, iced::widget::button::Status::Hovered) {
            Some(Background::Color(Color { r: 0.14, g: 0.14, b: 0.16, a: 1.0 }))
        } else {
            None
        },
        text_color: Palette::text(),
        ..Default::default()
    }
}
