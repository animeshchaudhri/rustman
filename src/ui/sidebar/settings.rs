use iced::{
    widget::{button, column, container, image, row, scrollable, svg, text, text_input, Space},
    Background, Border, Color, Element, Length,
};

const LOGO_SVG: &[u8] = include_bytes!("../../../public/rustman-logo.svg");

use crate::{
    app::UpdateState,
    message::{AppMsg, Message, SettingsMsg, UpdateMsg},
    ui::theme::{Palette, ACCENT_PALETTE},
};


pub fn view(state: &crate::app::AppState) -> Element<'_, Message> {
    let avatar_handle = state.profile_avatar.clone();

    let header = build_header();
    let profile = card(build_profile(
        avatar_handle,
        state.github_username.clone(),
        state.github_email.clone(),
        state.github_website.clone(),
        state.accent_idx,
    ));
    let git_identity = card(build_git_identity(
        &state.git_user_name,
        &state.git_user_email,
    ));
    let appearance = card(build_appearance(state.accent_idx));
    let updates = card(build_updates(&state.update));
    let shortcuts = card(build_shortcuts());
    let footer = build_footer();

    scrollable(
        column![header, profile, git_identity, appearance, updates, shortcuts, footer]
            .spacing(8)
            .padding(iced::Padding { top: 0.0, right: 8.0, bottom: 20.0, left: 8.0 }),
    )
    .height(Length::Fill)
    .style(crate::ui::theme::thin_scrollbar)
    .into()
}

fn build_git_identity(name: &str, email: &str) -> Element<'static, Message> {
    column![
        section_label("GIT IDENTITY"),
        field_label("Name"),
        text_input("Your Name", name)
            .on_input(|v| Message::Settings(SettingsMsg::GitNameChanged(v)))
            .size(12)
            .padding([6, 10])
            .style(|_theme, status| {
                let accent = Palette::accent();
                iced::widget::text_input::Style {
                    background: Background::Color(Palette::surface_high()),
                    border: Border {
                        color: match status {
                            iced::widget::text_input::Status::Focused { .. } => accent,
                            iced::widget::text_input::Status::Hovered => Palette::border(),
                            _ => Palette::border_subtle(),
                        },
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    icon: Palette::text_muted(),
                    placeholder: Palette::text_subtle(),
                    value: Palette::text(),
                    selection: iced::Color { r: accent.r, g: accent.g, b: accent.b, a: 0.25 },
                }
            }),
        Space::new().height(6),
        field_label("Email"),
        text_input("you@example.com", email)
            .on_input(|v| Message::Settings(SettingsMsg::GitEmailChanged(v)))
            .size(12)
            .padding([6, 10])
            .style(|_theme, status| {
                let accent = Palette::accent();
                iced::widget::text_input::Style {
                    background: Background::Color(Palette::surface_high()),
                    border: Border {
                        color: match status {
                            iced::widget::text_input::Status::Focused { .. } => accent,
                            iced::widget::text_input::Status::Hovered => Palette::border(),
                            _ => Palette::border_subtle(),
                        },
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    icon: Palette::text_muted(),
                    placeholder: Palette::text_subtle(),
                    value: Palette::text(),
                    selection: iced::Color { r: accent.r, g: accent.g, b: accent.b, a: 0.25 },
                }
            }),
        Space::new().height(4),
        text("Used for git commits. Saves to the active repo's config.")
            .size(9).color(Palette::text_subtle()),
    ]
    .spacing(0)
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
                    text("Rustman").size(13).color(Palette::text()).font(crate::ui::theme::MONO),
                    Space::new().width(5),
                    container(text(format!("v{}", crate::services::update::current_version())).size(9).color(Palette::text_muted()).font(crate::ui::theme::MONO))
                        .style(|_| iced::widget::container::Style {
                            background: Some(Background::Color(Palette::surface_raised())),
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
    avatar_handle: Option<image::Handle>,
    github_username: String,
    github_email: String,
    github_website: String,
    accent_idx: usize,
) -> Element<'static, Message> {
    let accent = ACCENT_PALETTE.get(accent_idx).copied().unwrap_or(Palette::accent());
    let accent_dim = Color { r: accent.r * 0.62, g: accent.g * 0.62, b: accent.b * 0.63, a: 1.0 };

    let avatar: Element<'static, Message> = if let Some(handle) = avatar_handle {
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
        section_label("MADE BY"),
        row![
            button(avatar)
                .on_press(Message::App(AppMsg::OpenUrl(github_url.clone())))
                .style(|_t, _s| iced::widget::button::Style { background: None, ..Default::default() })
                .padding(0),
            text(github_username.clone()).size(13).color(Palette::text()),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
        text("Socials").size(10).color(Palette::text_subtle()),
        full_link_row("GitHub", format!("github.com/{}", github_username), github_url, accent),
        full_link_row("Site", github_website.clone(), website_url, accent),
        full_link_row("Email", github_email.clone(), email_url, accent),
    ]
    .spacing(6)
    .into()
}

fn build_appearance(accent_idx: usize) -> Element<'static, Message> {

    let mut swatches = row![].spacing(5);
    for (i, &color) in ACCENT_PALETTE.iter().enumerate() {
        let is_selected = i == accent_idx;
        swatches = swatches.push(
            button(Space::new().width(0))
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
        field_label("Accent"),
        swatches,
    ]
    .spacing(6)
    .into()
}

fn build_updates(update: &UpdateState) -> Element<'static, Message> {
    let (status, status_color) = match update {
        UpdateState::Idle => ("Checked automatically on launch".to_owned(), Palette::text_subtle()),
        UpdateState::Checking => ("Checking…".to_owned(), Palette::text_muted()),
        UpdateState::Available(info) => (format!("v{} available", info.version), Palette::accent()),
        UpdateState::Installing => ("Downloading update…".to_owned(), Palette::text_muted()),
        UpdateState::Ready(v) => (format!("v{v} installed — restart to apply"), Palette::SUCCESS),
        UpdateState::UpToDate => ("You're on the latest version".to_owned(), Palette::SUCCESS),
        UpdateState::Failed(e) => (format!("Check failed: {e}"), Palette::ERROR),
    };
    let busy = matches!(update, UpdateState::Checking | UpdateState::Installing);

    let mut check = button(text("Check for updates").size(11).color(Palette::text()))
        .style(|_t, status| {
            let hov = matches!(status, iced::widget::button::Status::Hovered);
            iced::widget::button::Style {
                background: Some(Background::Color(if hov { Palette::surface_raised() } else { Palette::surface_high() })),
                border: Border { color: Palette::border(), width: 1.0, radius: 6.0.into() },
                text_color: Palette::text(),
                ..Default::default()
            }
        })
        .padding([6, 10]);
    if !busy {
        check = check.on_press(Message::Update(UpdateMsg::Check));
    }

    column![
        section_label("UPDATES"),
        text(status).size(11).color(status_color),
        check,
    ]
    .spacing(6)
    .into()
}

fn build_shortcuts() -> Element<'static, Message> {
    let cmd = if cfg!(target_os = "macos") { "⌘" } else { "Ctrl+" };
    let alt = if cfg!(target_os = "macos") { "⌥" } else { "Alt+" };
    let pairs: Vec<(String, &str)> = vec![
        (format!("{cmd}T"), "New tab"),
        (format!("{cmd}W"), "Close tab"),
        (format!("{cmd}S"), "Save"),
        (format!("{cmd}P"), "Palette"),
        (format!("{cmd}F"), "Search"),
        (format!("{cmd}E"), "Export cURL"),
        (format!("{cmd}Enter"), "Send"),
        (format!("{alt}1-9"), "Switch tab"),
        ("Esc".to_owned(), "Close dialog"),
        ("↑/↓".to_owned(), "Navigate"),
    ];

    let mut grid = column![section_label("KEYBOARD SHORTCUTS")].spacing(4);
    for chunk in pairs.chunks(2) {
        let mut r = row![].spacing(4);
        for (key, desc) in chunk {
            r = r.push(
                container(
                    row![
                        container(
                            text(key.clone()).size(10).color(Palette::text()).font(crate::ui::theme::MONO),
                        )
                        .style(|_| iced::widget::container::Style {
                            background: Some(Background::Color(Palette::surface_raised())),
                            border: Border { color: Palette::border(), width: 1.0, radius: 4.0.into() },
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
        text("Rustman · purely in Rust + iced")
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
        text(label).size(9).color(Palette::text_subtle()).font(crate::ui::theme::MONO),
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
        container(text(label).size(9).color(Palette::text_subtle()).font(crate::ui::theme::MONO))
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

