use iced::{
    widget::{button, column, container, image, row, scrollable, svg, text, Space},
    Background, Border, Color, Element, Length,
};

const LOGO_SVG: &[u8] = include_bytes!("../../../public/rustman-logo.svg");

use crate::{
    message::{AppMsg, ImportMsg, Message, SettingsMsg},
    ui::theme::{Palette, ACCENT_PALETTE},
};


pub fn view(state: &crate::app::AppState) -> Element<'_, Message> {
    let avatar_bytes = state.profile_avatar.clone();
    let collections: Vec<(String, String)> = state
        .collections
        .iter()
        .map(|c| (c.id.clone(), c.name.clone()))
        .collect();

    let header = build_header();
    let profile = card(build_profile(
        avatar_bytes,
        state.github_username.clone(),
        state.github_email.clone(),
        state.github_website.clone(),
        state.accent_idx,
    ));
    let appearance = card(build_appearance(state.accent_idx, state.theme_is_dark));
    let data = card(build_data_section(collections));
    let shortcuts = card(build_shortcuts());
    let footer = build_footer();

    scrollable(
        column![header, profile, appearance, data, shortcuts, footer]
            .spacing(8)
            .padding(iced::Padding { top: 0.0, right: 8.0, bottom: 20.0, left: 8.0 }),
    )
    .height(Length::Fill)
    .style(crate::ui::theme::thin_scrollbar)
    .into()
}

// ── Sections ──────────────────────────────────────────────────────────────────

fn build_header() -> Element<'static, Message> {
    let logo_handle = svg::Handle::from_memory(LOGO_SVG);
    container(
        row![
            svg(logo_handle).width(36).height(36),
            column![
                row![
                    text("Rustman").size(13).color(Palette::text()).font(iced::Font::MONOSPACE),
                    Space::with_width(5),
                    container(text("v0.3.1").size(9).color(Palette::text_muted()).font(iced::Font::MONOSPACE))
                        .style(|_| iced::widget::container::Style {
                            background: Some(Background::Color(Color { r: 0.16, g: 0.16, b: 0.20, a: 1.0 })),
                            border: Border { color: Palette::border(), width: 1.0, radius: 4.0.into() },
                            ..Default::default()
                        })
                        .padding([2, 5]),
                ]
                .align_y(iced::Alignment::Center),
                text("Native API Testing Client").size(9).color(Palette::text_subtle()),
            ]
            .spacing(2),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding(iced::Padding { top: 12.0, right: 12.0, bottom: 6.0, left: 12.0 })
    .into()
}

fn build_profile(
    avatar_bytes: Option<Vec<u8>>,
    github_username: String,
    github_email: String,
    github_website: String,
    accent_idx: usize,
) -> Element<'static, Message> {
    let accent = ACCENT_PALETTE.get(accent_idx).copied().unwrap_or(Palette::accent());
    let accent_dim = Color { r: accent.r * 0.62, g: accent.g * 0.62, b: accent.b * 0.63, a: 1.0 };

    let avatar: Element<'static, Message> = if let Some(bytes) = avatar_bytes {
        let handle = image::Handle::from_bytes(bytes);
        container(image(handle).width(36).height(36).content_fit(iced::ContentFit::Cover))
            .style(move |_| iced::widget::container::Style {
                border: Border { color: accent, width: 2.0, radius: 18.0.into() },
                ..Default::default()
            })
            .width(36)
            .height(36)
            .into()
    } else {
        let initials: String = github_username
            .split(|c: char| !c.is_alphanumeric())
            .filter(|p| !p.is_empty())
            .take(2)
            .map(|p| p.chars().next().unwrap_or('A').to_uppercase().next().unwrap_or('A'))
            .collect();
        let display = if initials.is_empty() { "AC".to_string() } else { initials };
        container(text(display).size(13).color(accent))
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(accent_dim)),
                border: Border { color: accent, width: 1.5, radius: 18.0.into() },
                ..Default::default()
            })
            .width(36)
            .height(36)
            .center_x(36)
            .center_y(36)
            .into()
    };

    let github_url = format!("https://github.com/{}", github_username);
    let website_url = if github_website.starts_with("http") {
        github_website.clone()
    } else {
        format!("https://{}", github_website)
    };
    let email_url = format!("mailto:{}", github_email);

    column![
        section_label("PROFILE"),
        row![
            button(avatar)
                .on_press(Message::App(AppMsg::OpenUrl(github_url.clone())))
                .style(|_t, _s| iced::widget::button::Style { background: None, ..Default::default() })
                .padding(0),
            text(github_username.clone()).size(13).color(Palette::text()),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
        text("Made by").size(10).color(Palette::text_subtle()),
        full_link_row("GitHub", format!("github.com/{}", github_username), github_url, accent),
        full_link_row("Site", github_website.clone(), website_url, accent),
        full_link_row("Email", github_email.clone(), email_url, accent),
    ]
    .spacing(6)
    .into()
}

fn build_appearance(accent_idx: usize, theme_is_dark: bool) -> Element<'static, Message> {
    let sh = Palette::surface_high();
    let sr = Palette::surface_raised();
    let bd = Palette::border();
    let ac = Palette::accent();
    let tm = Palette::text_muted();

    let dark_style = move |_t: &iced::Theme, s: iced::widget::button::Status| {
        let hov = matches!(s, iced::widget::button::Status::Hovered);
        iced::widget::button::Style {
            background: Some(Background::Color(if theme_is_dark || hov { sr } else { sh })),
            border: Border {
                color: if theme_is_dark { ac } else { bd },
                width: 1.0,
                radius: 5.0.into(),
            },
            text_color: if theme_is_dark { ac } else { tm },
            ..Default::default()
        }
    };
    let light_style = move |_t: &iced::Theme, s: iced::widget::button::Status| {
        let hov = matches!(s, iced::widget::button::Status::Hovered);
        iced::widget::button::Style {
            background: Some(Background::Color(if !theme_is_dark || hov { sr } else { sh })),
            border: Border {
                color: if !theme_is_dark { ac } else { bd },
                width: 1.0,
                radius: 5.0.into(),
            },
            text_color: if !theme_is_dark { ac } else { tm },
            ..Default::default()
        }
    };

    let theme_row = row![
        button(
            text("🌑 Dark").size(11)
                .color(if theme_is_dark { Palette::accent() } else { Palette::text_muted() })
                .width(Length::Fill)
                .align_x(iced::Alignment::Center),
        )
        .on_press(Message::Settings(SettingsMsg::ThemeDark))
        .style(dark_style)
        .padding([6, 8])
        .width(Length::Fill),
        button(
            text("☀ Light").size(11)
                .color(if !theme_is_dark { Palette::accent() } else { Palette::text_muted() })
                .width(Length::Fill)
                .align_x(iced::Alignment::Center),
        )
        .on_press(Message::Settings(SettingsMsg::ThemeLight))
        .style(light_style)
        .padding([6, 8])
        .width(Length::Fill),
    ]
    .spacing(6);

    let mut swatches = row![].spacing(5);
    for (i, &color) in ACCENT_PALETTE.iter().enumerate() {
        let is_selected = i == accent_idx;
        swatches = swatches.push(
            button(Space::with_width(0))
                .on_press(Message::Settings(SettingsMsg::AccentChanged(i)))
                .style(move |_t, s| {
                    let hov = matches!(s, iced::widget::button::Status::Hovered);
                    iced::widget::button::Style {
                        background: Some(Background::Color(color)),
                        border: Border {
                            color: if is_selected || hov { Color::WHITE } else {
                                Color { r: color.r * 0.5, g: color.g * 0.5, b: color.b * 0.5, a: 1.0 }
                            },
                            width: if is_selected { 2.5 } else { 1.0 },
                            radius: 12.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .width(22)
                .height(22)
                .padding(0),
        );
    }

    column![
        section_label("APPEARANCE"),
        field_label("Theme"),
        theme_row,
        field_label("Accent"),
        swatches,
    ]
    .spacing(6)
    .into()
}

fn build_data_section(collections: Vec<(String, String)>) -> Element<'static, Message> {
    let import_row = row![
        action_button("↓  Postman", Message::Import(ImportMsg::OpenPostmanDialog)),
        action_button("↓  OpenAPI", Message::Import(ImportMsg::OpenOpenApiDialog)),
    ]
    .spacing(6);

    let mut col = column![section_label("DATA MANAGEMENT"), import_row].spacing(8);

    if !collections.is_empty() {
        let mut list = column![
            container(text("Collections").size(10).color(Palette::text_subtle()))
                .padding(iced::Padding { top: 4.0, right: 0.0, bottom: 2.0, left: 0.0 }),
        ]
        .spacing(2);

        for (id, name) in collections {
            list = list.push(
                container(
                    row![
                        text(name).size(11).color(Palette::text()).width(Length::Fill),
                        button(text("↑ Export").size(10).color(Palette::accent()))
                            .on_press(Message::Import(ImportMsg::ExportCollection(id)))
                            .style(|_t, status| {
                                let hov = matches!(status, iced::widget::button::Status::Hovered);
                                iced::widget::button::Style {
                                    background: Some(Background::Color(if hov { Palette::accent_dim() } else { Color::TRANSPARENT })),
                                    border: Border { color: Palette::accent(), width: 1.0, radius: 4.0.into() },
                                    text_color: Palette::accent(),
                                    ..Default::default()
                                }
                            })
                            .padding([2, 6]),
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .style(|_| iced::widget::container::Style {
                    background: Some(Background::Color(Color { r: 0.12, g: 0.12, b: 0.15, a: 1.0 })),
                    border: Border { color: Palette::border_subtle(), width: 1.0, radius: 5.0.into() },
                    ..Default::default()
                })
                .padding([5, 8])
                .width(Length::Fill),
            );
        }
        col = col.push(list);
    }

    col.into()
}

fn build_shortcuts() -> Element<'static, Message> {
    const PAIRS: &[(&str, &str)] = &[
        ("Ctrl+T", "New tab"),
        ("Ctrl+W", "Close tab"),
        ("Ctrl+S", "Save"),
        ("Ctrl+P", "Palette"),
        ("Ctrl+F", "Search"),
        ("Ctrl+E", "Export cURL"),
        ("Ctrl+↵", "Send"),
        ("Alt+1-9", "Switch tab"),
        ("Esc", "Close dialog"),
        ("↑ ↓", "Navigate"),
    ];

    let mut grid = column![section_label("KEYBOARD SHORTCUTS")].spacing(4);
    for chunk in PAIRS.chunks(2) {
        let mut r = row![].spacing(4);
        for (key, desc) in chunk {
            r = r.push(
                container(
                    row![
                        container(
                            text(*key).size(10).color(Palette::text()).font(iced::Font::MONOSPACE),
                        )
                        .style(|_| iced::widget::container::Style {
                            background: Some(Background::Color(Color { r: 0.17, g: 0.17, b: 0.20, a: 1.0 })),
                            border: Border { color: Palette::border(), width: 1.0, radius: 3.0.into() },
                            ..Default::default()
                        })
                        .padding([2, 5]),
                        text(*desc).size(10).color(Palette::text_muted()),
                    ]
                    .spacing(5)
                    .align_y(iced::Alignment::Center),
                )
                .width(Length::FillPortion(1))
                .padding([1, 0]),
            );
        }
        grid = grid.push(r);
    }
    grid.into()
}

fn build_footer() -> Element<'static, Message> {
    container(
        text("Rustman · purely in Rust + iced  🦀")
            .size(9)
            .color(Palette::text_subtle()),
    )
    .padding(iced::Padding { top: 4.0, right: 12.0, bottom: 8.0, left: 12.0 })
    .into()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn card(content: impl Into<Element<'static, Message>>) -> Element<'static, Message> {
    let bg = Palette::surface();
    let bd = Palette::border_subtle();
    container(content)
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(bg)),
            border: Border { color: bd, width: 1.0, radius: 8.0.into() },
            ..Default::default()
        })
        .padding(12)
        .width(Length::Fill)
        .into()
}

fn section_label(label: &'static str) -> Element<'static, Message> {
    container(
        text(label).size(9).color(Palette::text_subtle()).font(iced::Font::MONOSPACE),
    )
    .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 2.0, left: 0.0 })
    .into()
}

fn field_label(label: &'static str) -> Element<'static, Message> {
    text(label).size(10).color(Palette::text_subtle()).into()
}

/// A full-width row: label chip + clickable link text, guaranteed to not overflow.
fn full_link_row(label: &'static str, display: String, url: String, accent: Color) -> Element<'static, Message> {
    let accent_dim = Color { r: accent.r * 0.62, g: accent.g * 0.62, b: accent.b * 0.63, a: 1.0 };
    let chip_bg = Palette::surface_high();
    let chip_bd = Palette::border_subtle();
    row![
        container(text(label).size(9).color(Palette::text_subtle()).font(iced::Font::MONOSPACE))
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(chip_bg)),
                border: Border { color: chip_bd, width: 1.0, radius: 3.0.into() },
                ..Default::default()
            })
            .padding([2, 6]),
        button(text(display).size(11).color(accent))
            .on_press(Message::App(AppMsg::OpenUrl(url)))
            .style(move |_t, s| {
                let hov = matches!(s, iced::widget::button::Status::Hovered);
                iced::widget::button::Style {
                    background: if hov { Some(Background::Color(accent_dim)) } else { None },
                    border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 3.0.into() },
                    text_color: accent,
                    ..Default::default()
                }
            })
            .padding([1, 4]),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn action_button(label: &'static str, msg: Message) -> Element<'static, Message> {
    let bg_normal = Palette::surface_high();
    let bg_hover = Palette::surface_raised();
    let bd = Palette::border();
    let fg = Palette::text();
    button(
        text(label).size(11).color(fg).width(Length::Fill).align_x(iced::Alignment::Center),
    )
    .on_press(msg)
    .style(move |_t, status| {
        let hov = matches!(status, iced::widget::button::Status::Hovered);
        iced::widget::button::Style {
            background: Some(Background::Color(if hov { bg_hover } else { bg_normal })),
            border: Border { color: bd, width: 1.0, radius: 6.0.into() },
            text_color: fg,
            ..Default::default()
        }
    })
    .padding([7, 10])
    .width(Length::Fill)
    .into()
}
