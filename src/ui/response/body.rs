use std::collections::HashSet;

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

// ── Syntax colours ────────────────────────────────────────────────────────────

const STRING_COLOR: Color = Color { r: 0.56, g: 0.86, b: 0.50, a: 1.0 };
const NUMBER_COLOR: Color = Color { r: 0.95, g: 0.78, b: 0.42, a: 1.0 };
const BOOL_COLOR: Color = Color { r: 0.47, g: 0.73, b: 0.98, a: 1.0 };
const NULL_COLOR: Color = Color { r: 0.90, g: 0.52, b: 0.52, a: 1.0 };
const KEY_COLOR: Color = Color { r: 0.72, g: 0.61, b: 0.97, a: 1.0 };
const BRACE_COLOR: Color = Color { r: 0.65, g: 0.65, b: 0.70, a: 1.0 };

const INDENT_W: f32 = 16.0;
const TOGGLE_W: f32 = 14.0;
const MAX_RENDER_LINES: usize = 5_000;
const MAX_STRING_CHARS: usize = 120;

// ── Flat line representation ──────────────────────────────────────────────────

struct FlatLine {
    depth: usize,
    key: Option<String>,
    kind: LineKind,
    trailing_comma: bool,
}

enum LineKind {
    Open { path: String, is_array: bool, is_collapsed: bool, child_count: usize },
    Close { is_array: bool },
    Scalar { display: String, raw: String, color: Color },
}

// ── JSON → flat line list ─────────────────────────────────────────────────────

fn flatten(
    value: &serde_json::Value,
    path: String,
    depth: usize,
    key: Option<String>,
    trailing_comma: bool,
    collapsed: &HashSet<String>,
    out: &mut Vec<FlatLine>,
) -> bool {
    if out.len() >= MAX_RENDER_LINES * 2 {
        return false;
    }
    match value {
        serde_json::Value::Object(map) => {
            let is_collapsed = collapsed.contains(&path);
            let child_count = map.len();
            out.push(FlatLine {
                depth,
                key,
                kind: LineKind::Open { path: path.clone(), is_array: false, is_collapsed, child_count },
                trailing_comma: if is_collapsed { trailing_comma } else { false },
            });
            if !is_collapsed {
                let n = map.len();
                for (i, (k, v)) in map.iter().enumerate() {
                    if !flatten(v, format!("{}/{}", path, k), depth + 1, Some(k.clone()), i + 1 < n, collapsed, out) {
                        return false;
                    }
                }
                out.push(FlatLine { depth, key: None, kind: LineKind::Close { is_array: false }, trailing_comma });
            }
        }
        serde_json::Value::Array(arr) => {
            let is_collapsed = collapsed.contains(&path);
            let child_count = arr.len();
            out.push(FlatLine {
                depth,
                key,
                kind: LineKind::Open { path: path.clone(), is_array: true, is_collapsed, child_count },
                trailing_comma: if is_collapsed { trailing_comma } else { false },
            });
            if !is_collapsed {
                let n = arr.len();
                for (i, v) in arr.iter().enumerate() {
                    if !flatten(v, format!("{}/{}", path, i), depth + 1, None, i + 1 < n, collapsed, out) {
                        return false;
                    }
                }
                out.push(FlatLine { depth, key: None, kind: LineKind::Close { is_array: true }, trailing_comma });
            }
        }
        serde_json::Value::String(s) => {
            let single_line = s.replace(['\n', '\r', '\t'], " ");
            let escaped = single_line.replace('\\', "\\\\").replace('"', "\\\"");
            let char_count = escaped.chars().count();
            let display = if char_count > MAX_STRING_CHARS {
                format!("\"{}…\"", escaped.chars().take(MAX_STRING_CHARS).collect::<String>())
            } else {
                format!("\"{}\"", escaped)
            };
            out.push(FlatLine {
                depth,
                key,
                kind: LineKind::Scalar { display, raw: s.clone(), color: STRING_COLOR },
                trailing_comma,
            });
        }
        serde_json::Value::Number(n) => {
            let s = n.to_string();
            out.push(FlatLine {
                depth,
                key,
                kind: LineKind::Scalar { display: s.clone(), raw: s, color: NUMBER_COLOR },
                trailing_comma,
            });
        }
        serde_json::Value::Bool(b) => {
            let s = if *b { "true" } else { "false" };
            out.push(FlatLine {
                depth,
                key,
                kind: LineKind::Scalar { display: s.into(), raw: s.into(), color: BOOL_COLOR },
                trailing_comma,
            });
        }
        serde_json::Value::Null => {
            out.push(FlatLine {
                depth,
                key,
                kind: LineKind::Scalar { display: "null".into(), raw: "null".into(), color: NULL_COLOR },
                trailing_comma,
            });
        }
    }
    true
}

// ── Render a single flat line ─────────────────────────────────────────────────

fn render_line_numbered(row_n: usize, line: &FlatLine) -> Element<'static, Message> {
    let gutter = container(
        text(format!("{row_n}"))
            .size(10)
            .color(Palette::text_subtle())
            .font(iced::Font::MONOSPACE),
    )
    .width(40)
    .padding(iced::Padding { top: 1.0, right: 4.0, bottom: 1.0, left: 4.0 });

    row![gutter, render_line(line)]
        .align_y(iced::Alignment::Center)
        .into()
}

fn render_line(line: &FlatLine) -> Element<'static, Message> {
    let indent = Space::with_width(line.depth as f32 * INDENT_W);

    let toggle: Element<Message> = match &line.kind {
        LineKind::Open { path, is_collapsed, .. } => {
            let icon = if *is_collapsed { "▶" } else { "▼" };
            let p = path.clone();
            button(text(icon).size(9).color(if *is_collapsed { Palette::accent() } else { BRACE_COLOR }))
                .on_press(Message::Response(ResponseMsg::ToggleJsonNode(p)))
                .style(|_t, _s| iced::widget::button::Style { background: None, ..Default::default() })
                .padding(0)
                .width(TOGGLE_W)
                .into()
        }
        _ => Space::with_width(TOGGLE_W).into(),
    };

    let value: Element<Message> = match &line.kind {
        LineKind::Open { path, is_array, is_collapsed, child_count } => {
            if *is_collapsed {
                let (open_b, close_b) = if *is_array { ('[', ']') } else { ('{', '}') };
                let noun = if *is_array {
                    if *child_count == 1 { "item" } else { "items" }
                } else {
                    if *child_count == 1 { "key" } else { "keys" }
                };
                let summary = format!("{} {} {} {}", open_b, child_count, noun, close_b);
                let p = path.clone();
                button(text(summary).size(11).color(Palette::text_subtle()).font(iced::Font::MONOSPACE))
                    .on_press(Message::Response(ResponseMsg::ToggleJsonNode(p)))
                    .style(|_t, s| {
                        let hov = matches!(s, iced::widget::button::Status::Hovered);
                        iced::widget::button::Style {
                            background: if hov { Some(Background::Color(Color { r: 0.15, g: 0.15, b: 0.20, a: 1.0 })) } else { None },
                            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 3.0.into() },
                            text_color: Palette::text_subtle(),
                            ..Default::default()
                        }
                    })
                    .padding([0, 4])
                    .into()
            } else {
                text(if *is_array { "[" } else { "{" })
                    .size(11).color(BRACE_COLOR).font(iced::Font::MONOSPACE).into()
            }
        }
        LineKind::Close { is_array } => {
            text(if *is_array { "]" } else { "}" })
                .size(11).color(BRACE_COLOR).font(iced::Font::MONOSPACE).into()
        }
        LineKind::Scalar { display, raw: _, color } => {
            let c = *color;
            let ac = Palette::accent();
            // on_input with Noop makes the field focusable/selectable while ignoring edits.
            iced::widget::text_input("", display)
                .on_input(|_| Message::App(crate::message::AppMsg::Noop))
                .size(11)
                .font(iced::Font::MONOSPACE)
                .padding([0, 2])
                .style(move |_t, s| iced::widget::text_input::Style {
                    background: Background::Color(match s {
                        iced::widget::text_input::Status::Focused => Color { r: 0.14, g: 0.14, b: 0.19, a: 1.0 },
                        iced::widget::text_input::Status::Hovered => Color { r: 0.11, g: 0.11, b: 0.15, a: 1.0 },
                        _ => Color::TRANSPARENT,
                    }),
                    border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 2.0.into() },
                    icon: c,
                    placeholder: Palette::text_subtle(),
                    value: c,
                    selection: Color { r: ac.r, g: ac.g, b: ac.b, a: 0.35 },
                })
                .into()
        }
    };

    let mut r = row![indent, toggle];
    if let Some(k) = &line.key {
        r = r
            .push(text(format!("\"{}\"", k)).size(11).color(KEY_COLOR).font(iced::Font::MONOSPACE))
            .push(text(": ").size(11).color(BRACE_COLOR).font(iced::Font::MONOSPACE));
    }
    r = r.push(value);
    if line.trailing_comma {
        r = r.push(text(",").size(11).color(BRACE_COLOR).font(iced::Font::MONOSPACE));
    }

    container(r.align_y(iced::Alignment::Center).spacing(0))
        .padding(iced::Padding { top: 1.0, right: 8.0, bottom: 1.0, left: 6.0 })
        .width(Length::Shrink)
        .into()
}

// ── Main view ─────────────────────────────────────────────────────────────────

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

    let is_json = tab.parsed_json.is_some() && !tab.json_raw_mode;

    // ── Toolbar ───────────────────────────────────────────────────────────────

    let toggle_label = if tab.json_raw_mode { "Tree" } else { "Raw" };
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
        if tab.json_raw_mode {
            let body_text = r.body.as_str();
            Some(body_text.to_lowercase().match_indices(&q).count())
        } else {
            None
        }
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

    let body_content: Element<'_, Message> = if is_json {
        let json = tab.parsed_json.as_ref().unwrap();
        let mut lines: Vec<FlatLine> = Vec::new();
        flatten(json, "root".to_string(), 0, None, false, &tab.json_collapsed, &mut lines);

        let total = lines.len();
        let filtered_lines: Vec<FlatLine> = if tab.search_query.is_empty() {
            lines
        } else {
            let q = tab.search_query.to_lowercase();
            lines
                .into_iter()
                .filter(|l| {
                    let key_match = l.key.as_deref().map(|k| k.to_lowercase().contains(&q)).unwrap_or(false);
                    let val_match = match &l.kind {
                        LineKind::Scalar { display, .. } => display.to_lowercase().contains(&q),
                        LineKind::Open { path, .. } => path.to_lowercase().contains(&q),
                        _ => false,
                    };
                    key_match || val_match
                })
                .collect()
        };

        let filtered_total = filtered_lines.len();
        let truncated = filtered_total > MAX_RENDER_LINES;
        let render_slice = if truncated { &filtered_lines[..MAX_RENDER_LINES] } else { &filtered_lines[..] };

        if total == 0 {
            return container(text("Empty response").size(11).color(Palette::text_subtle()))
                .padding([8, 6])
                .into();
        }

        let mut col = column![].spacing(0);
        for (row_n, line) in render_slice.iter().enumerate() {
            col = col.push(render_line_numbered(row_n + 1, line));
        }
        if truncated {
            col = col.push(
                container(
                    text(format!("… {} more nodes (collapse some to show more)", filtered_total - MAX_RENDER_LINES))
                        .size(10).color(Palette::text_subtle()).font(iced::Font::MONOSPACE),
                )
                .padding(iced::Padding { top: 6.0, right: 8.0, bottom: 6.0, left: 24.0 }),
            );
        }

        scrollable(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(iced::widget::scrollable::Direction::Both {
                vertical: iced::widget::scrollable::Scrollbar::new().width(8).scroller_width(6),
                horizontal: iced::widget::scrollable::Scrollbar::new().width(8).scroller_width(6),
            })
            .style(crate::ui::theme::thin_scrollbar)
            .into()
    } else if tab.viewer_processing {
        // Content build is in progress; show the raw body string immediately
        // via a plain `text` widget. `width(Shrink)` is required for the
        // horizontal scrollable to work correctly.
        let body_str = r.body.as_str();
        let bg = Color { r: 0.07, g: 0.07, b: 0.08, a: 1.0 };
        container(
            scrollable(
                container(
                    text(body_str)
                        .size(11)
                        .font(iced::Font::MONOSPACE)
                        .color(Palette::text()),
                )
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
        // text_editor handles its own scrollbar; a separate line-number gutter
        // is omitted because there is no way to sync it with the internal scroll position.
        let highlight_enabled = tab.parsed_json.is_some() && line_count <= 3_000;
        let bg = Color { r: 0.07, g: 0.07, b: 0.08, a: 1.0 };
        let ac = Palette::accent();

        text_editor(&tab.response_viewer)
            .on_action(|a| Message::Response(ResponseMsg::ViewerAction(a)))
            .height(Length::Fill)
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
            )
            .into()
    };

    let mut col = column![toolbar];
    if let Some(bar) = search_bar {
        col = col.push(bar);
    }
    col.push(body_content).spacing(0).height(Length::Fill).into()
}
