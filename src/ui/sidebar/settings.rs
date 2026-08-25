use iced::{
    widget::{button, column, container, image, row, scrollable, svg, text, text_input, Space},
    Background, Border, Color, Element, Length,
};

const LOGO_SVG: &[u8] = include_bytes!("../../../public/rustman-logo.svg");

use crate::{
    app::UpdateState,
    message::{AppMsg, Message, SettingsMsg, UpdateMsg},
    ui::theme::{Palette, ThemeSpec, THEMES, TEXT_SM},
};


pub fn view(state: &crate::app::AppState) -> Element<'_, Message> {
    let avatar_handle = state.profile_avatar.clone();

    let header = build_header();
    let profile = card(build_profile(
        avatar_handle,
        state.github_username.clone(),
        state.github_email.clone(),
        state.github_website.clone(),
    ));
    let git_identity = card(build_git_identity(
        &state.git_user_name,
        &state.git_user_email,
    ));
    let appearance = card(build_appearance(state.theme_idx));
    let layout = card(build_layout(state.horizontal_layout));
    let request_defaults = card(build_request_defaults(&state.default_timeout_text));
    let tls = card(build_tls(state.tls_options));
    let global_scripts = card(build_global_scripts(
        state.global_pre_request_editor.text().len(),
        state.global_test_editor.text().len(),
    ));
    let updates = card(build_updates(&state.update));
    let shortcuts = card(build_shortcuts());
    let footer = build_footer();

    scrollable(
        column![
            header, profile, git_identity, appearance, layout, request_defaults,
            tls, global_scripts, updates, shortcuts, footer
        ]
        .spacing(6)
        .padding(iced::Padding { top: 0.0, right: 8.0, bottom: 8.0, left: 8.0 }),
    )
    .height(Length::Fill)
    .style(crate::ui::theme::hidden_scrollbar)
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
            .style(crate::ui::styles::field_input),
        Space::new().height(6),
        field_label("Email"),
        text_input("you@example.com", email)
            .on_input(|v| Message::Settings(SettingsMsg::GitEmailChanged(v)))
            .size(12)
            .padding([6, 10])
            .style(crate::ui::styles::field_input),
        Space::new().height(4),
        text("Used for git commits. Saves to the active repo's config.")
            .size(9).color(Palette::text_subtle()),
    ]
    .spacing(0)
    .into()
}

fn build_request_defaults(timeout_text: &str) -> Element<'static, Message> {
    column![
        section_label("REQUEST DEFAULTS"),
        field_label("Timeout (ms)"),
        text_input("30000", timeout_text)
            .on_input(|v| Message::Settings(SettingsMsg::DefaultTimeoutChanged(v)))
            .size(12)
            .padding([6, 10])
            .width(160)
            .style(crate::ui::styles::field_input),
        Space::new().height(4),
        text("Applied to every request. 0 falls back to 30000.")
            .size(9).color(Palette::text_subtle()),
    ]
    .spacing(0)
    .into()
}

/// TLS / connection workarounds for endpoints a default client cannot reach
/// (issue #40: works in Chrome/curl, fails in Rustman with `os error 10054`).
fn build_tls(options: crate::services::http::TlsOptions) -> Element<'static, Message> {
    use crate::message::TlsOption;

    column![
        section_label("TLS & CONNECTION"),
        tls_toggle(
            "Send HTTP/1.1 only",
            "Try this first. Rustman offers HTTP/2 in the handshake; some servers              close the connection instead of declining it (Windows: os error 10054).",
            options.http1_only,
            TlsOption::Http1Only,
        ),
        Space::new().height(6),
        tls_toggle(
            "Ignore certificate errors",
            "Accepts self-signed, internal-CA and mismatched-hostname certificates.              Like curl -k — only use it for endpoints you trust.",
            options.accept_invalid_certs,
            TlsOption::AcceptInvalidCerts,
        ),
        Space::new().height(6),
        tls_toggle(
            "Force TLS 1.2",
            "Pin the handshake to TLS 1.2, for servers that fail on 1.3.",
            options.force_tls12,
            TlsOption::ForceTls12,
        ),
        Space::new().height(6),
        tls_toggle(
            "Force TLS 1.3",
            "Pin the handshake to TLS 1.3. Clears the 1.2 pin.",
            options.force_tls13,
            TlsOption::ForceTls13,
        ),
        Space::new().height(4),
        text("Applies to the next request; existing connections are dropped.")
            .size(9)
            .color(Palette::text_subtle()),
    ]
    .spacing(0)
    .into()
}

fn tls_toggle(
    label: &'static str,
    help: &'static str,
    enabled: bool,
    option: crate::message::TlsOption,
) -> Element<'static, Message> {
    column![
        iced::widget::checkbox(enabled)
            .label(label)
            .on_toggle(move |_| Message::Settings(SettingsMsg::TlsOptionToggled(option)))
            .size(15)
            .text_size(12)
            .style(crate::ui::styles::checkbox),
        text(help).size(9).color(Palette::text_subtle()),
    ]
    .spacing(2)
    .into()
}

fn build_global_scripts(pre_request_len: usize, test_len: usize) -> Element<'static, Message> {
    let status = match (pre_request_len > 0, test_len > 0) {
        (false, false) => "Not set up yet".to_owned(),
        (true, false) => "Pre-request configured".to_owned(),
        (false, true) => "Test script configured".to_owned(),
        (true, true) => "All configured".to_owned(),
    };

    column![
        section_label("GLOBAL SCRIPTS"),
        text("Runs before every request's own pre-request/test script — for setup you'd otherwise copy-paste into each request.")
            .size(9).color(Palette::text_subtle()),
        Space::new().height(8),
        row![
            text(status).size(11).color(Palette::text_muted()),
            Space::new().width(Length::Fill),
            button(text("Edit").size(11).color(Palette::text()))
                .on_press(Message::Settings(SettingsMsg::OpenGlobalScriptsModal))
                .style(|_t, s| {
                    let hov = matches!(s, iced::widget::button::Status::Hovered);
                    iced::widget::button::Style {
                        background: Some(Background::Color(if hov {
                            Palette::surface_raised()
                        } else {
                            Palette::surface_high()
                        })),
                        border: Border { color: Palette::border(), width: 1.0, radius: 6.0.into() },
                        text_color: Palette::text(),
                        ..Default::default()
                    }
                })
                .padding([5, 12]),
        ]
        .align_y(iced::Alignment::Center),
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
) -> Element<'static, Message> {
    let accent = Palette::accent();
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

fn build_appearance(theme_idx: usize) -> Element<'static, Message> {
    let mut cards = column![].spacing(4);
    for (i, spec) in THEMES.iter().enumerate() {
        let is_selected = i == theme_idx;
        cards = cards.push(theme_card(spec, is_selected, i));
    }

    column![
        section_label("APPEARANCE"),
        field_label("Theme"),
        cards,
    ]
    .spacing(6)
    .into()
}

fn theme_card(spec: &'static ThemeSpec, is_selected: bool, idx: usize) -> Element<'static, Message> {
    let preview = row![swatch(spec.background), swatch(spec.surface), swatch(spec.accent)].spacing(3);

    let check: Element<'static, Message> = if is_selected {
        text("\u{2713}").size(TEXT_SM).color(spec.accent).into()
    } else {
        Space::new().width(0).into()
    };

    button(
        row![
            preview,
            text(spec.name).size(TEXT_SM).color(Palette::text()),
            Space::new().width(Length::Fill),
            check,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::Settings(SettingsMsg::ThemeChanged(idx)))
    .style(move |_t, s| {
        let hovered = matches!(s, iced::widget::button::Status::Hovered);
        iced::widget::button::Style {
            background: Some(Background::Color(if is_selected {
                Palette::accent_soft()
            } else if hovered {
                Palette::hover()
            } else {
                Palette::surface_high()
            })),
            border: Border {
                color: if is_selected { Palette::accent() } else { Palette::border_subtle() },
                width: 1.0,
                radius: 6.0.into(),
            },
            text_color: Palette::text(),
            ..Default::default()
        }
    })
    .padding([6, 10])
    .width(Length::Fill)
    .into()
}

fn swatch(color: Color) -> Element<'static, Message> {
    container(Space::new())
        .width(14)
        .height(14)
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(color)),
            border: Border { color: Color { a: 0.3, ..color }, width: 1.0, radius: 4.0.into() },
            ..Default::default()
        })
        .into()
}

fn build_layout(horizontal: bool) -> Element<'static, Message> {
    let options = [("Top / Bottom", false), ("Left / Right", true)];

    let mut row_el = row![section_label("LAYOUT")].spacing(6);
    for (label, is_horizontal) in options {
        let selected = horizontal == is_horizontal;
        row_el = row_el.push(
            container(
                button(
                    text(label).size(TEXT_SM).color(if selected {
                        Palette::accent()
                    } else {
                        Palette::text_muted()
                    }),
                )
                .on_press(Message::Settings(SettingsMsg::LayoutDirectionToggled))
                .style(move |_t, s| {
                    use iced::widget::button::Status;
                    iced::widget::button::Style {
                        background: Some(Background::Color(if selected {
                            Palette::accent_soft()
                        } else if matches!(s, Status::Hovered) {
                            Palette::hover()
                        } else {
                            Palette::surface_high()
                        })),
                        border: Border {
                            color: if selected { Palette::accent() } else { Palette::border_subtle() },
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        text_color: if selected { Palette::accent() } else { Palette::text_muted() },
                        ..Default::default()
                    }
                })
                .padding([6, 10])
                .width(Length::Fill),
            )
            .width(Length::FillPortion(1)),
        );
    }
    row_el.into()
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
        (format!("{cmd}F"), "Find"),
        (format!("{cmd}H"), "Find & Replace"),
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
    card_ref(content)
}

/// Like `card`, but for content that borrows from `AppState` (e.g. a code
/// editor's own `view()`) instead of owning everything it displays.
fn card_ref<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
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

