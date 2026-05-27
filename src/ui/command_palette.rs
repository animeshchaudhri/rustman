use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Background, Border, Color, Element, Length,
};

use crate::{app::AppState, message::{Message, PaletteMsg, SidebarMsg}, ui::theme::Palette};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let query = state.palette_query.to_lowercase();

    let mut items: Vec<PaletteItem> = Vec::new();

    for (i, tab) in state.tabs.tabs.iter().enumerate() {
        if i == state.tabs.active {
            continue;
        }
        let label = format!("{} {}", tab.method.as_str(), if tab.url.is_empty() { &tab.title } else { &tab.url });
        if query.is_empty() || label.to_lowercase().contains(&query) {
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
            if query.is_empty() || label.to_lowercase().contains(&query) {
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
        if query.is_empty() || label.to_lowercase().contains(&query) {
            items.push(PaletteItem {
                method: entry.method.clone(),
                name: entry.url.clone(),
                url: entry.url.clone(),
                source: "History".to_owned(),
                action: Message::Sidebar(SidebarMsg::HistoryEntryOpened(entry.clone())),
            });
        }
    }

    let search_input = text_input("Search requests, history, tabs…", &state.palette_query)
        .id(iced::widget::text_input::Id::new("palette-search"))
        .on_input(|s| Message::Palette(PaletteMsg::QueryChanged(s)))
        .size(14)
        .padding([10, 14])
        .width(Length::Fill);

    let mut list = column![].spacing(0);
    for (i, item) in items.into_iter().take(12).enumerate() {
        let selected = i == state.palette_selected;
        let method_color = method_color(&item.method);
        let method = item.method;
        let name = item.name;
        let url = item.url;
        let source = item.source;
        let action = item.action;
        let row_el = button(
            row![
                container(
                    text(method).size(10).color(method_color).font(iced::Font::MONOSPACE)
                )
                .width(50),
                column![
                    text(name).size(13).color(Palette::text()),
                    text(url).size(11).color(Palette::text_muted()),
                ]
                .spacing(1)
                .width(Length::Fill),
                text(source).size(10).color(Palette::text_muted()),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .padding([6u16, 12u16]),
        )
        .on_press(action)
        .style(move |t, s| palette_row_style(t, s, selected))
        .width(Length::Fill);
        list = list.push(row_el);
    }

    let inner = column![
        search_input,
        container(iced::widget::horizontal_rule(1)).padding([0, 0]),
        scrollable(list).height(Length::Shrink),
    ]
    .spacing(0)
    .width(620);

    let modal = container(inner)
        .style(palette_container)
        .width(620);

    container(
        column![
            iced::widget::Space::new(Length::Fill, Length::FillPortion(1)),
            row![
                iced::widget::Space::with_width(Length::Fill),
                modal,
                iced::widget::Space::with_width(Length::Fill),
            ],
            iced::widget::Space::new(Length::Fill, Length::FillPortion(3)),
        ]
    )
    .style(backdrop)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

struct PaletteItem {
    method: String,
    name: String,
    url: String,
    source: String,
    action: Message,
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
            radius: 10.0.into(),
        },
        shadow: iced::Shadow {
            color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.6 },
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 32.0,
        },
        ..Default::default()
    }
}

fn palette_row_style(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let bg = if selected || matches!(status, iced::widget::button::Status::Hovered) {
        Some(Background::Color(Palette::surface_high()))
    } else {
        None
    };
    iced::widget::button::Style {
        background: bg,
        text_color: Palette::text(),
        border: Border::default(),
        ..Default::default()
    }
}
