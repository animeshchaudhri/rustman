use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Background, Border, Color, Element, Length,
};

use crate::{app::AppState, message::{Message, PaletteMsg, SidebarMsg}, ui::theme::Palette};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let search_input = text_input("Search requests, history, tabs…", &state.palette_query)
        .id("palette-search")
        .on_input(|s| Message::Palette(PaletteMsg::QueryChanged(s)))
        .size(14)
        .padding([12, 10])
        .width(Length::Fill)
        .style(|_t, _s| iced::widget::text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
            icon: Palette::text_muted(),
            placeholder: Palette::text_subtle(),
            value: Palette::text(),
            selection: Color { a: 0.3, ..Palette::accent() },
        });

    let search_row = row![
        container(crate::ui::icons::search().size(15).color(Palette::text_subtle()))
            .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 14.0 }),
        search_input,
    ]
    .align_y(iced::Alignment::Center);

    let mut list = column![].spacing(2);
    for (i, item) in items(state).into_iter().enumerate() {
        let selected = i == state.palette_selected;
        let method_color = method_color(&item.method);
        let row_el = button(
            row![
                container(
                    text(item.method).size(10).color(method_color).font(crate::ui::theme::MONO)
                )
                .width(50),
                column![
                    text(item.name).size(13).color(Palette::text()),
                    text(item.url).size(11).color(Palette::text_muted()),
                ]
                .spacing(1)
                .width(Length::Fill),
                text(item.source).size(10).color(Palette::text_muted()),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .padding([7u16, 12u16]),
        )
        .on_press(Message::Palette(PaletteMsg::ConfirmAt(i)))
        .style(move |t, s| palette_row_style(t, s, selected))
        .width(Length::Fill);
        list = list.push(row_el);
    }

    let inner = column![
        container(search_row).padding([4, 0]),
        container(iced::widget::rule::horizontal(1.0).style(crate::ui::styles::divider)).padding([0, 0]),
        container(scrollable(list).height(Length::Shrink)).padding(6),
    ]
    .spacing(0)
    .width(620);

    let modal = container(inner)
        .style(palette_container)
        .width(620);

    container(
        column![
            iced::widget::Space::new().width(Length::Fill).height(Length::FillPortion(1)),
            row![
                iced::widget::Space::new().width(Length::Fill),
                modal,
                iced::widget::Space::new().width(Length::Fill),
            ],
            iced::widget::Space::new().width(Length::Fill).height(Length::FillPortion(3)),
        ]
    )
    .style(backdrop)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub struct PaletteItem {
    pub method: String,
    pub name: String,
    pub url: String,
    pub source: String,
    pub action: Message,
}

/// Build the palette's filtered result list (open tabs, saved requests, history),
/// capped at the 12 rows the view renders. Shared by the view and the update
/// handler so a selection index always maps to the same action.
pub fn items(state: &AppState) -> Vec<PaletteItem> {
    let query = state.palette_query.to_lowercase();
    let matches = |label: &str| query.is_empty() || label.to_lowercase().contains(&query);
    let mut items: Vec<PaletteItem> = Vec::new();

    for (i, tab) in state.tabs.tabs.iter().enumerate() {
        if i == state.tabs.active {
            continue;
        }
        let label = format!("{} {}", tab.method.as_str(), if tab.url.is_empty() { &tab.title } else { &tab.url });
        if matches(&label) {
            items.push(PaletteItem {
                method: tab.method.as_str().to_owned(),
                name: tab.title.clone(),
                url: tab.url.clone(),
                source: "Open Tab".to_owned(),
                action: Message::Request(crate::message::RequestMsg::SwitchTab(i)),
            });
        }
    }

    for (col_id, reqs) in &state.requests {
        let col_name = state.collections.iter()
            .find(|c| &c.id == col_id)
            .map(|c| c.name.as_str())
            .unwrap_or("Collection");
        for req in reqs {
            let label = format!("{} {} {}", req.method, req.name, req.url);
            if matches(&label) {
                items.push(PaletteItem {
                    method: req.method.as_str().to_owned(),
                    name: req.name.clone(),
                    url: req.url.clone(),
                    source: col_name.to_owned(),
                    action: Message::Sidebar(SidebarMsg::RequestOpened(req.clone())),
                });
            }
        }
    }

    for entry in state.history.iter().take(20) {
        let label = format!("{} {}", entry.method, entry.url);
        if matches(&label) {
            items.push(PaletteItem {
                method: entry.method.clone(),
                name: entry.url.clone(),
                url: entry.url.clone(),
                source: "History".to_owned(),
                action: Message::Sidebar(SidebarMsg::HistoryEntryOpened(entry.clone())),
            });
        }
    }

    items.truncate(12);
    items
}

fn method_color(method: &str) -> Color {
    match method {
        "GET" => Palette::GET,
        "POST" => Palette::POST,
        "PUT" => Palette::PUT,
        "PATCH" => Palette::PATCH,
        "DELETE" => Palette::DELETE,
        _ => Palette::HEAD,
    }
}

fn backdrop(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.7 })),
        ..Default::default()
    }
}

fn palette_container(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface())),
        border: Border {
            color: Palette::border(),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: iced::Shadow {
            color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.6 },
            offset: iced::Vector::new(0.0, 10.0),
            blur_radius: 40.0,
        },
        ..Default::default()
    }
}

fn palette_row_style(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let bg = if selected {
        Some(Background::Color(Palette::accent_soft()))
    } else if matches!(status, iced::widget::button::Status::Hovered) {
        Some(Background::Color(Palette::hover()))
    } else {
        None
    };
    iced::widget::button::Style {
        background: bg,
        text_color: Palette::text(),
        border: Border {
            color: if selected { Color { a: 0.30, ..Palette::accent() } } else { Color::TRANSPARENT },
            width: if selected { 1.0 } else { 0.0 },
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}
