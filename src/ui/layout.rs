use iced::{
    widget::{
        button, column, container, horizontal_rule, row, scrollable, text, vertical_rule, Space,
    },
    Background, Border, Color, Element, Length,
};

use crate::{
    app::AppState,
    message::{Message, PaletteMsg, RequestMsg, RequestTab, ResponseTab, SaveDialogMsg, SidebarMsg, SidebarPanel},
    ui::{request, response, sidebar, theme::Palette},
};


pub fn view(state: &AppState) -> Element<'_, Message> {
    let root = row![left_panel(state), main_area(state)].height(Length::Fill);

    if state.palette_open {
        iced::widget::stack![root, crate::ui::command_palette::view(state)].into()
    } else if state.save_dialog_open {
        iced::widget::stack![root, save_dialog(state)].into()
    } else if state.curl_modal_open {
        iced::widget::stack![root, curl_modal(state)].into()
    } else {
        root.into()
    }
}

// ── Left sidebar ──────────────────────────────────────────────────────────────

fn left_panel(state: &AppState) -> Element<'_, Message> {
    let icon_rail = icon_rail(state);

    let panel_content: Element<Message> = match state.sidebar.panel {
        SidebarPanel::Collections => sidebar::collections::view(state),
        SidebarPanel::History => sidebar::history::view(state),
        SidebarPanel::Environments => sidebar::environments::view(state),
        SidebarPanel::Git => sidebar::collections::view(state), // Git panel hidden — falls back to collections view
        SidebarPanel::Settings => sidebar::settings::view(state),
    };

    container(
        row![
            icon_rail,
            vertical_rule(1),
            container(panel_content).width(Length::Fill).height(Length::Fill),
        ]
        .height(Length::Fill),
    )
    .style(sidebar_style)
    .width(264)
    .height(Length::Fill)
    .into()
}

fn icon_rail(state: &AppState) -> Element<'_, Message> {
    let top_icons: &[(SidebarPanel, &str, &str)] = &[
        (SidebarPanel::Collections, "≡", "Collections"),
        (SidebarPanel::History, "◷", "History"),
        (SidebarPanel::Environments, "◇", "Environments"),
    ];

    let mut top_col = column![].spacing(0);
    for (panel, icon, _label) in top_icons {
        let active = &state.sidebar.panel == panel;
        top_col = top_col.push(icon_btn(icon, panel.clone(), active));
    }

    let settings_active = state.sidebar.panel == SidebarPanel::Settings;
    let settings_btn = icon_btn("⊛", SidebarPanel::Settings, settings_active);

    container(
        column![
            top_col,
            Space::with_height(Length::Fill),
            settings_btn,
        ]
        .padding([4, 0]),
    )
    .width(44)
    .height(Length::Fill)
    .style(icon_rail_style)
    .into()
}

fn icon_btn(icon: &str, panel: SidebarPanel, active: bool) -> Element<'_, Message> {
    let accent_bar = container(Space::new(2, Length::Fill))
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(if active {
                Palette::accent()
            } else {
                Color::TRANSPARENT
            })),
            ..Default::default()
        })
        .height(Length::Fill);

    let icon_el = container(
        text(icon)
            .size(18)
            .color(if active { Palette::accent() } else { Palette::text_subtle() }),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .height(Length::Fill);

    button(
        row![accent_bar, icon_el].height(40),
    )
    .on_press(Message::Sidebar(SidebarMsg::PanelSelected(panel)))
    .style(move |t, s| icon_btn_style(t, s, active))
    .width(44)
    .height(40)
    .padding(0)
    .into()
}

// ── Main area ─────────────────────────────────────────────────────────────────

fn main_area(state: &AppState) -> Element<'_, Message> {
    let tab_bar = multi_tab_bar(state);
    let active_tab = state.tabs.active_tab();
    let url_bar = request::url_bar::view(active_tab);
    let req_tabs = request::tabs::view(active_tab);

    let req_body: Element<Message> = match active_tab.active_request_tab {
        RequestTab::Params => request::params::view(active_tab),
        RequestTab::Headers => request::headers::view(active_tab),
        RequestTab::Body => request::body::view(active_tab),
        RequestTab::Auth => request::auth::view(active_tab),
        RequestTab::Scripts => request::scripts::view(active_tab),
        RequestTab::WebSocket => request::params::view(active_tab), // WebSocket tab not yet wired — falls back to params view
    };

    let req_split = state.panel_split as u16;
    let resp_split = (10 - state.panel_split).max(1) as u16;

    let request_panel = container(
        column![req_tabs, req_body].spacing(0).height(Length::Fill),
    )
    .height(Length::FillPortion(req_split))
    .width(Length::Fill);

    let resp_status: Element<Message> = active_tab
        .response
        .as_ref()
        .map(|r| response::status_bar::view(r))
        .unwrap_or_else(|| Space::new(0, 0).into());

    let resp_tab_bar = response::tabs::view(active_tab);
    let resp_body: Element<Message> = match active_tab.active_response_tab {
        ResponseTab::Body => response::body::view(active_tab),
        ResponseTab::Headers => response::headers::view(active_tab),
        ResponseTab::Cookies => response::cookies::view(active_tab),
        ResponseTab::Tests => response_tests(active_tab),
        ResponseTab::Console => response::console::view(active_tab),
    };

    let response_panel = container(
        column![resp_status, resp_tab_bar, resp_body]
            .spacing(0)
            .height(Length::Fill),
    )
    .style(surface_style)
    .height(Length::FillPortion(resp_split))
    .width(Length::Fill);

    let bottom_bar = status_bar(state);

    container(
        column![
            tab_bar,
            horizontal_rule(1),
            url_bar,
            horizontal_rule(1),
            request_panel,
            horizontal_rule(1),
            response_panel,
            horizontal_rule(1),
            bottom_bar,
        ]
        .spacing(0)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn multi_tab_bar(state: &AppState) -> Element<'_, Message> {
    let tabs = state.tabs.tabs.iter().enumerate().map(|(i, tab)| {
        let active = i == state.tabs.active;
        let title = if tab.url.is_empty() { tab.title.as_str() } else { &tab.url };
        let short_title: String = if title.len() > 20 {
            format!("{}…", &title[..18])
        } else {
            title.to_owned()
        };

        let unsaved_dot = if tab.modified && tab.saved_as.is_some() {
            "● " // modified saved request
        } else if tab.modified {
            "◆ " // new unsaved
        } else {
            ""
        };

        let close_btn = button(text("×").size(10).color(Palette::text_muted()))
            .on_press(Message::Request(RequestMsg::CloseTab(i)))
            .style(iced::widget::button::text)
            .padding([1, 4]);

        let tab_btn = container(
            row![
                button(
                    row![
                        text(tab.method.as_str())
                            .size(9)
                            .color(method_color(&tab.method))
                            .font(iced::Font::MONOSPACE),
                        text(format!("{unsaved_dot}{short_title}")).size(11),
                    ]
                    .spacing(5)
                    .align_y(iced::Alignment::Center),
                )
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

    let new_tab_btn: Element<Message> = button(text("+").size(14).color(Palette::text_muted()))
        .on_press(Message::Request(RequestMsg::NewTab))
        .style(iced::widget::button::text)
        .padding([3, 8])
        .into();

    let mut tab_items: Vec<Element<Message>> = tabs.collect();
    tab_items.push(new_tab_btn);

    let palette_btn = button(text("⌘P").size(10).color(Palette::text_muted()))
        .on_press(Message::Palette(PaletteMsg::Open))
        .style(iced::widget::button::text)
        .padding([3, 8]);

    container(
        row![
            scrollable(row(tab_items).spacing(0))
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new().width(0).scroller_width(0),
                ))
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

fn status_bar(state: &AppState) -> Element<'_, Message> {
    let env_label: String = state
        .environments
        .iter()
        .find(|e| e.is_active)
        .map(|e| format!("  ◈ {}", e.name))
        .unwrap_or_else(|| "  No environment".to_owned());

    let msg = state
        .status_message
        .as_deref()
        .unwrap_or("");

    container(
        row![
            text(msg).size(10).color(Palette::SUCCESS).width(Length::Fill),
            text(env_label).size(10).color(Palette::text_subtle()),
            text("  Ctrl+P  ⌘  ").size(10).color(Palette::text_subtle()),
        ]
        .align_y(iced::Alignment::Center)
        .padding([0, 8]),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Color {
            r: 0.055,
            g: 0.055,
            b: 0.063,
            a: 1.0,
        })),
        ..Default::default()
    })
    .height(22)
    .width(Length::Fill)
    .into()
}

fn response_tests(tab: &crate::state::tabs::RequestTabState) -> Element<'_, Message> {
    let mut col = column![].spacing(4).padding(8);
    if tab.test_results.is_empty() {
        col = col.push(text("No test results.").size(13).color(Palette::text_muted()));
    }
    for r in &tab.test_results {
        let icon = if r.passed { "✓" } else { "✗" };
        let color = if r.passed { Palette::SUCCESS } else { Palette::ERROR };
        col = col.push(
            row![text(icon).size(12).color(color), text(&r.name).size(12)].spacing(6),
        );
    }
    scrollable(col).height(Length::Fill).into()
}

fn save_dialog(state: &AppState) -> Element<'_, Message> {
    use iced::widget::{text_input, Space};

    let title = text("Save Request").size(14).color(Palette::text());

    let name_input = text_input("Request name", &state.save_dialog_name)
        .on_input(|s| Message::SaveDialog(SaveDialogMsg::NameChanged(s)))
        .size(13)
        .padding([8, 10]);

    let mut col_col = column![
        text("Collection").size(11).color(Palette::text_muted()),
    ]
    .spacing(4);

    for col in &state.collections {
        let selected = !state.save_dialog_new_col
            && state.save_dialog_collection_id.as_deref() == Some(&col.id);
        let col_id = col.id.clone();
        col_col = col_col.push(
            button(
                row![
                    container(Space::new(8, 8))
                        .style(move |_| iced::widget::container::Style {
                            background: if selected {
                                Some(Background::Color(Palette::accent()))
                            } else {
                                Some(Background::Color(Palette::border()))
                            },
                            border: Border { radius: 4.0.into(), ..Default::default() },
                            ..Default::default()
                        })
                        .width(8)
                        .height(8),
                    text(&col.name).size(12).color(if selected { Palette::text() } else { Palette::text_muted() }),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::SaveDialog(SaveDialogMsg::CollectionSelected(col_id)))
            .style(move |_t, s| {
                let hovered = matches!(s, iced::widget::button::Status::Hovered);
                iced::widget::button::Style {
                    background: if selected || hovered {
                        Some(Background::Color(Palette::surface_high()))
                    } else {
                        None
                    },
                    border: Border { radius: 4.0.into(), ..Default::default() },
                    text_color: Palette::text(),
                    ..Default::default()
                }
            })
            .padding([5, 8])
            .width(Length::Fill),
        );
    }

    let new_col_selected = state.save_dialog_new_col;
    col_col = col_col.push(
        button(
            row![
                container(Space::new(8, 8))
                    .style(move |_| iced::widget::container::Style {
                        background: if new_col_selected {
                            Some(Background::Color(Palette::accent()))
                        } else {
                            Some(Background::Color(Palette::border()))
                        },
                        border: Border { radius: 4.0.into(), ..Default::default() },
                        ..Default::default()
                    })
                    .width(8)
                    .height(8),
                text("+ New collection")
                    .size(12)
                    .color(if new_col_selected { Palette::accent() } else { Palette::text_muted() }),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::SaveDialog(SaveDialogMsg::ToggleNewCollection))
        .style(move |_t, s| {
            let hovered = matches!(s, iced::widget::button::Status::Hovered);
            iced::widget::button::Style {
                background: if new_col_selected || hovered {
                    Some(Background::Color(Palette::surface_high()))
                } else {
                    None
                },
                border: Border { radius: 4.0.into(), ..Default::default() },
                text_color: Palette::text(),
                ..Default::default()
            }
        })
        .padding([5, 8])
        .width(Length::Fill),
    );

    let new_col_input: Option<iced::Element<'_, Message>> = if state.save_dialog_new_col {
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
        Space::with_width(Length::Fill),
        button(text("Save").size(12).color(Color::WHITE))
            .on_press(Message::SaveDialog(SaveDialogMsg::Confirm))
            .style(|_t, s| iced::widget::button::Style {
                background: Some(Background::Color(match s {
                    iced::widget::button::Status::Hovered => Color { r: Palette::accent().r + 0.06, g: Palette::accent().g + 0.06, b: Palette::accent().b + 0.04, a: 1.0 },
                    _ => Palette::accent(),
                })),
                text_color: Color::WHITE,
                border: Border { radius: 6.0.into(), ..Default::default() },
                ..Default::default()
            })
            .padding([6, 18]),
    ]
    .align_y(iced::Alignment::Center);

    let mut body_col = column![
        title,
        Space::new(0, 8),
        text("Name").size(11).color(Palette::text_muted()),
        name_input,
        Space::new(0, 8),
        col_col,
    ]
    .spacing(4)
    .width(440);

    if let Some(inp) = new_col_input {
        body_col = body_col.push(inp);
    }
    body_col = body_col.push(Space::new(0, 12)).push(btns);

    let inner = container(body_col)

    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface())),
        border: Border { color: Palette::border(), width: 1.0, radius: 10.0.into() },
        shadow: iced::Shadow {
            color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.7 },
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 32.0,
        },
        ..Default::default()
    })
    .padding([20, 24])
    .width(440);

    container(
        column![
            iced::widget::Space::new(Length::Fill, Length::FillPortion(2)),
            row![
                iced::widget::Space::with_width(Length::Fill),
                inner,
                iced::widget::Space::with_width(Length::Fill),
            ],
            iced::widget::Space::new(Length::Fill, Length::FillPortion(3)),
        ]
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.7 })),
        ..Default::default()
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn curl_modal(state: &AppState) -> Element<'_, Message> {
    let title = row![
        text("cURL Command").size(14).color(Palette::text()),
        Space::with_width(Length::Fill),
        button(text("×").size(16).color(Palette::text_muted()))
            .on_press(Message::Request(RequestMsg::CloseCurlModal))
            .style(iced::widget::button::text)
            .padding([2, 6]),
    ]
    .align_y(iced::Alignment::Center);

    let cmd_display = scrollable(
        container(
            text(state.curl_modal_command.clone())
                .font(iced::Font::MONOSPACE)
                .size(11)
                .color(Color { r: 0.7, g: 0.9, b: 0.7, a: 1.0 }),
        )
        .padding([10, 12])
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(200)
    .style(crate::ui::theme::thin_scrollbar);

    let cmd_box = container(cmd_display)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(Color { r: 0.06, g: 0.07, b: 0.06, a: 1.0 })),
            border: Border { color: Palette::border(), width: 1.0, radius: 6.0.into() },
            ..Default::default()
        })
        .width(Length::Fill);

    let copy_btn = button(
        row![
            text("⎘").size(12).color(Color::WHITE),
            text(" Copy to Clipboard").size(12).color(Color::WHITE),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::Request(RequestMsg::CopyCurlToClipboard))
    .style(|_t, s| iced::widget::button::Style {
        background: Some(Background::Color(match s {
            iced::widget::button::Status::Hovered => Color {
                r: Palette::accent().r + 0.06,
                g: Palette::accent().g + 0.06,
                b: Palette::accent().b + 0.04,
                a: 1.0,
            },
            _ => Palette::accent(),
        })),
        text_color: Color::WHITE,
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    })
    .padding([7, 16]);

    let close_btn = button(text("Close").size(12).color(Palette::text_muted()))
        .on_press(Message::Request(RequestMsg::CloseCurlModal))
        .style(iced::widget::button::text)
        .padding([6, 14]);

    let inner = container(
        column![
            title,
            Space::new(0, 10),
            text("Run this command in your terminal:")
                .size(11)
                .color(Palette::text_muted()),
            Space::new(0, 6),
            cmd_box,
            Space::new(0, 12),
            row![close_btn, Space::with_width(Length::Fill), copy_btn]
                .align_y(iced::Alignment::Center),
        ]
        .spacing(0)
        .width(560),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface())),
        border: Border { color: Palette::border(), width: 1.0, radius: 10.0.into() },
        shadow: iced::Shadow {
            color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.7 },
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 32.0,
        },
        ..Default::default()
    })
    .padding([20, 24])
    .width(560);

    container(
        column![
            Space::new(Length::Fill, Length::FillPortion(2)),
            row![
                Space::with_width(Length::Fill),
                inner,
                Space::with_width(Length::Fill),
            ],
            Space::new(Length::Fill, Length::FillPortion(3)),
        ]
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.7 })),
        ..Default::default()
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

// ── Style functions ───────────────────────────────────────────────────────────

fn method_color(method: &crate::domain::request::HttpMethod) -> Color {
    use crate::domain::request::HttpMethod as M;
    match method {
        M::Get => Palette::GET,
        M::Post => Palette::POST,
        M::Put => Palette::PUT,
        M::Patch => Palette::PATCH,
        M::Delete => Palette::DELETE,
        M::Head | M::Options => Palette::HEAD,
    }
}

fn icon_rail_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::background())),
        ..Default::default()
    }
}

fn icon_btn_style(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: if active {
            Some(Background::Color(Palette::surface_high()))
        } else if matches!(status, iced::widget::button::Status::Hovered) {
            Some(Background::Color(Palette::surface()))
        } else {
            None
        },
        border: Border::default(),
        text_color: Palette::text(),
        ..Default::default()
    }
}

fn sidebar_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface())),
        border: Border { color: Palette::border(), width: 1.0, radius: 0.0.into() },
        ..Default::default()
    }
}

fn surface_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::background())),
        ..Default::default()
    }
}

fn tab_bar_container(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface())),
        ..Default::default()
    }
}

fn tab_container_style(_theme: &iced::Theme, active: bool) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: if active {
            Some(Background::Color(Palette::surface_high()))
        } else {
            None
        },
        border: Border {
            color: if active { Palette::border() } else { Color::TRANSPARENT },
            width: if active { 1.0 } else { 0.0 },
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn tab_btn_style(
    _theme: &iced::Theme,
    _status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        text_color: if active { Palette::text() } else { Palette::text_muted() },
        ..Default::default()
    }
}
