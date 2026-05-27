use iced::{
    widget::{button, column, container, row, scrollable, svg, text, text_editor, text_input, Space},
    Background, Border, Color, Element, Length,
};

use crate::{
    message::{Message, ResponseMsg},
    state::tabs::RequestTabState,
    ui::{theme::Palette, widgets::json_highlighter::{JsonHighlighter, JsonHighlightSettings}},
};

const NYAN_SVG: &[u8] = include_bytes!("../../../public/The Nyan Cat.svg");
const PALLBEARERS_SVG: &[u8] = include_bytes!("../../../public/Dancing Pallbearers.svg");

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    if tab.is_loading {
        let pick_nyan = tab.id.bytes().fold(0u8, |acc, b| acc.wrapping_add(b)) % 2 == 0;
        let anim_bytes = if pick_nyan { NYAN_SVG } else { PALLBEARERS_SVG };
        let handle = svg::Handle::from_memory(anim_bytes);
        return container(
            column![
                svg(handle).width(220).height(160),
                text("Sending request…").size(12).color(Palette::text_subtle()),
            ]
            .spacing(12)
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into();
    }

    if tab.response.is_none() {
        return container(
            text("Send a request to see the response.").size(13).color(Palette::text_muted()),
        )
        .padding([24, 16])
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    let r = tab.response.as_ref().unwrap();

    // ── Toolbar ───────────────────────────────────────────────────────────────

    let toggle_label = if tab.json_raw_mode { "Pretty" } else { "Raw" };
    let toggle_color = Palette::text_muted();
    let line_count = tab.response_viewer_lines;

    let toolbar = container(
        row![
            Space::with_width(Length::Fill),
            if tab.viewer_processing {
                text("Parsing…").size(10).color(Palette::text_subtle())
            } else if line_count > 0 {
                text(format!("{}L", line_count)).size(10).color(Palette::text_subtle())
            } else {
                text("").size(10)
            },
            Space::with_width(6),
            button(text("Copy").size(11).color(Palette::text_muted()))
                .on_press(Message::Response(ResponseMsg::CopyBody))
                .style(iced::widget::button::text)
                .padding([2, 6]),
            button(text("Format").size(11).color(Palette::text_muted()))
                .on_press(Message::Response(ResponseMsg::FormatBody))
                .style(iced::widget::button::text)
                .padding([2, 6]),
            if tab.parsed_json.is_some() || tab.json_raw_mode {
                button(text(toggle_label).size(11).color(toggle_color))
                    .on_press(Message::Response(ResponseMsg::ToggleJsonRaw))
                    .style(iced::widget::button::text)
                    .padding([2, 6])
            } else {
                button(text("").size(11))
                    .style(iced::widget::button::text)
                    .padding([2, 6])
            },
            button(
                text(if tab.search_visible { "Search ✓" } else { "Search" })
                    .size(11)
                    .color(if tab.search_visible { Palette::accent() } else { Palette::text_muted() }),
            )
            .on_press(Message::Response(ResponseMsg::ToggleSearch))
            .style(iced::widget::button::text)
            .padding([2, 6]),
        ]
        .spacing(0)
        .align_y(iced::Alignment::Center)
        .padding([3, 6]),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Color { r: 0.07, g: 0.07, b: 0.08, a: 1.0 })),
        ..Default::default()
    })
    .width(Length::Fill);

    let match_count: Option<usize> = if tab.search_visible && !tab.search_query.is_empty() {
        let q = tab.search_query.to_lowercase();
        let body_text = r.body.as_str();
        Some(body_text.to_lowercase().match_indices(&q).count())
    } else {
        None
    };

    let search_bar = if tab.search_visible {
        let match_label: String = match match_count {
            Some(n) if !tab.search_query.is_empty() => format!("{n} match{}", if n == 1 { "" } else { "es" }),
            _ => String::new(),
        };
        Some(
            container(
                row![
                    text_input("Search…", &tab.search_query)
                        .id(iced::widget::text_input::Id::new("response-search"))
                        .on_input(|s| Message::Response(ResponseMsg::SearchChanged(s)))
                        .size(12)
                        .padding([4, 10])
                        .width(Length::Fill)
                        .style(|_t, s| iced::widget::text_input::Style {
                            background: Background::Color(Color { r: 0.10, g: 0.10, b: 0.12, a: 1.0 }),
                            border: Border {
                                color: match s {
                                    iced::widget::text_input::Status::Focused => Palette::accent(),
                                    _ => Palette::border_subtle(),
                                },
                                width: 1.0,
                                radius: 4.0.into(),
                            },
                            icon: Palette::text_muted(),
                            placeholder: Palette::text_subtle(),
                            value: Palette::text(),
                            selection: Color { r: Palette::accent().r, g: Palette::accent().g, b: Palette::accent().b, a: 0.3 },
                        }),
                    text(match_label).size(10).color(Palette::text_subtle()),
                    if !tab.search_query.is_empty() {
                        button(text("×").size(14).color(Palette::text_muted()))
                            .on_press(Message::Response(ResponseMsg::SearchChanged(String::new())))
                            .style(iced::widget::button::text)
                            .padding([2, 6])
                    } else {
                        button(text("×").size(14).color(Color::TRANSPARENT))
                            .style(iced::widget::button::text)
                            .padding([2, 6])
                    },
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center)
                .padding([5, 8]),
            )
            .style(|_| iced::widget::container::Style {
                background: Some(Background::Color(Color { r: 0.09, g: 0.09, b: 0.11, a: 1.0 })),
                ..Default::default()
            })
            .width(Length::Fill),
        )
    } else {
        None
    };

    let bg = Color { r: 0.07, g: 0.07, b: 0.08, a: 1.0 };
    let gutter_bg = Color { r: 0.06, g: 0.06, b: 0.07, a: 1.0 };

    let body_content: Element<'_, Message> = if tab.viewer_processing {
        // Background parse in progress — show raw body immediately (non-interactive).
        let body_str = r.body.as_str();
        container(
            scrollable(
                container(text(body_str).size(11).font(iced::Font::MONOSPACE).color(Palette::text()))
                    .padding([4, 8])
                    .width(Length::Shrink),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(iced::widget::scrollable::Direction::Both {
                vertical: iced::widget::scrollable::Scrollbar::new().width(8).scroller_width(6),
                horizontal: iced::widget::scrollable::Scrollbar::new().width(8).scroller_width(6),
            })
            .style(crate::ui::theme::thin_scrollbar),
        )
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(bg)),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        // text_editor at Shrink height lets the outer scrollable own all scrolling.
        let highlight_enabled = tab.parsed_json.is_some() && line_count <= 3_000;
        let ac = Palette::accent();
        let num_lines = line_count.max(1);

        let gutter = container(
            iced::widget::column(
                (1..=num_lines)
                    .map(|n| {
                        container(
                            text(format!("{}", n))
                                .size(11)
                                .color(Palette::text_subtle())
                                .font(iced::Font::MONOSPACE),
                        )
                        .padding(iced::Padding { top: 2.0, right: 8.0, bottom: 2.0, left: 6.0 })
                        .align_x(iced::Alignment::End)
                        .width(Length::Fixed(44.0))
                        .into()
                    })
                    .collect::<Vec<_>>(),
            )
            .spacing(0),
        )
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(gutter_bg)),
            ..Default::default()
        });

        let editor = text_editor(&tab.response_viewer)
            .on_action(|a| Message::Response(ResponseMsg::ViewerAction(a)))
            .height(Length::Shrink)
            .font(iced::Font::MONOSPACE)
            .size(11)
            .style(move |_theme, _status| iced::widget::text_editor::Style {
                background: Background::Color(bg),
                border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
                icon: Palette::text_muted(),
                placeholder: Palette::text_subtle(),
                value: Palette::text(),
                selection: Color { r: ac.r, g: ac.g, b: ac.b, a: 0.25 },
            })
            .highlight_with::<JsonHighlighter>(
                JsonHighlightSettings { enabled: highlight_enabled },
                |h, _theme| iced::advanced::text::highlighter::Format {
                    color: Some(h.color()),
                    font: None,
                },
            );

        container(
            // The text_editor handles horizontal scroll internally.
            scrollable(
                row![gutter, editor],
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(8).scroller_width(6),
            ))
            .style(crate::ui::theme::thin_scrollbar),
        )
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(bg)),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    };

    let mut col = column![toolbar];
    if let Some(bar) = search_bar {
        col = col.push(bar);
    }
    col.push(body_content).spacing(0).height(Length::Fill).into()
}
