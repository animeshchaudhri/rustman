use iced::{
    widget::{button, column, container, image, row, scrollable, text, Space},
    Alignment, Background, Border, Color, Element, Length,
};

use crate::{
    domain::response::HttpResponse,
    message::{Message, ResponseMsg},
    services::spreadsheet::ParsedSheet,
    state::tabs::{PdfPreviewState, RequestTabState, ResponsePreview},
    ui::{icons, theme::{Palette, MONO, TEXT_SM, TEXT_LG, TEXT_XS}},
};

pub fn view(tab: &RequestTabState, spinner_frame: u32) -> Element<'_, Message> {
    if tab.is_loading {
        return loading_view(spinner_frame);
    }

    let Some(resp) = tab.response.as_ref() else {
        return empty_view();
    };

    if let Some(err) = &resp.error {
        return error_view(err);
    }

    if resp.is_binary {
        // An oversized binary download has no bytes kept in memory at all, so
        // there is nothing to build a preview from. Say that plainly rather
        // than falling through to a preview that can never appear.
        if resp.body_stored && resp.binary_data.is_none() {
            return binary_view_with_note(
                resp,
                "This download is too large to preview in the app. \
                 The response completed successfully — only the in-app preview is skipped.",
            );
        }
        return match &tab.response_preview {
            ResponsePreview::Spreadsheet(Ok(sheet)) => spreadsheet_view(sheet),
            ResponsePreview::Spreadsheet(Err(err)) => binary_view_with_note(resp, err),
            ResponsePreview::Pdf(preview) => pdf_view(preview),
            ResponsePreview::None => binary_view(resp),
        };
    }

    if resp.is_html() && !crate::services::webview::creation_failed() {
        return html_view();
    }
    // Either not HTML, or it is but the embedded webview couldn't attach on
    // this platform/configuration (e.g. no child-window support under
    // native Wayland) — fall through to the plain text/source view below,
    // with a banner explaining why (the underlying error used to go only to
    // stderr, which a windowed build has no console for).
    let webview_note: Option<String> = resp
        .is_html()
        .then(crate::services::webview::last_error)
        .flatten();

    let size_note: Element<Message> = match tab.response_truncated_bytes {
        Some(full_len) => text(format!(
            " · showing first {} of {} (full body still copied/saved)",
            human_bytes(tab.response_editor_bytes()),
            human_bytes(full_len),
        ))
        .size(TEXT_XS)
        .color(Palette::WARNING)
        .into(),
        None => Space::new().into(),
    };

    let toolbar = container(
        row![
            text(format!("{} lines", tab.response_viewer_lines)).size(TEXT_XS).color(Palette::text_subtle()),
            size_note,
            Space::new().width(Length::Fill),
            button(
                row![
                    icons::copy().size(11).color(Palette::text_muted()),
                    text("Copy").size(TEXT_SM).color(Palette::text_muted()),
                ]
                .spacing(5)
                .align_y(Alignment::Center),
            )
                .on_press(Message::Response(ResponseMsg::CopyBody))
                .style(|_t, status| {
                    let hovered = matches!(status, iced::widget::button::Status::Hovered);
                    iced::widget::button::Style {
                        background: if hovered { Some(Background::Color(Palette::hover())) } else { None },
                        text_color: Palette::text_muted(),
                        border: Border { radius: 6.0.into(), ..Default::default() },
                        ..Default::default()
                    }
                })
                .padding([4, 8]),
        ]
        .align_y(Alignment::Center)
        .padding([2, 12]),
    )
    .width(Length::Fill);

    let body: Element<Message> = if tab.viewer_processing {
        container(text("Parsing…").size(TEXT_SM).color(Palette::text_subtle()))
            .padding([8, 12])
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        tab.response_editor
            .view()
            .map(|m| Message::Response(ResponseMsg::ViewerEdited(m)))
    };

    let mut panel = column![].spacing(0).height(Length::Fill);
    if let Some(note) = webview_note {
        panel = panel.push(preview_unavailable_banner(&note));
    }
    panel.push(toolbar).push(body).into()
}

/// Shown above the raw-source fallback when the embedded HTML preview could not
/// be created, so the user knows the source view is a fallback and why.
fn preview_unavailable_banner(reason: &str) -> Element<'static, Message> {
    container(
        column![
            text("HTML preview unavailable — showing source")
                .size(TEXT_SM)
                .color(Palette::WARNING),
            text(reason.to_owned()).size(TEXT_XS).color(Palette::text_subtle()),
        ]
        .spacing(2),
    )
    .padding([6, 12])
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(Palette::warning_soft())),
        border: Border {
            color: Palette::WARNING,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Id of the placeholder container reserving screen space for the embedded
/// HTML-preview webview (see `services::webview` and `AppMsg::HtmlPreviewTick`).
/// The container itself renders nothing visible — the native webview is
/// positioned to exactly cover it from outside Iced's own rendering.
pub const HTML_PANEL_ID: iced::widget::Id = iced::widget::Id::new("response-html-panel");

fn html_view() -> Element<'static, Message> {
    container(
        text("Loading preview…").size(TEXT_SM).color(Palette::text_subtle()),
    )
    .id(HTML_PANEL_ID)
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

/// Formats a byte count as a short human-readable size.
fn human_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// A keyboard-key chip: bordered, mono font, slight bottom "keycap" shadow.
fn kbd_chip(label: &str) -> Element<'static, Message> {
    container(
        text(label.to_owned())
            .size(TEXT_XS)
            .color(Palette::text_muted())
            .font(MONO),
    )
    .padding([3, 8])
    .style(|_| container::Style {
        background: Some(Background::Color(Palette::surface_high())),
        border: Border {
            color: Palette::border(),
            width: 1.0,
            radius: 5.0.into(),
        },
        shadow: iced::Shadow {
            color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.4 },
            offset: iced::Vector::new(0.0, 1.5),
            blur_radius: 0.0,
        },
        ..Default::default()
    })
    .into()
}

fn empty_view() -> Element<'static, Message> {
    let modifier = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
    let content = column![
        text("Send a request").size(TEXT_LG).color(Palette::text()).font(crate::ui::theme::UI_FONT_MEDIUM),
        Space::new().height(8),
        row![
            text("Press").size(TEXT_SM).color(Palette::text_subtle()),
            kbd_chip(modifier),
            kbd_chip("Enter"),
            text("or click Send to run the request").size(TEXT_SM).color(Palette::text_subtle()),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    ]
    .align_x(Alignment::Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn error_view(err: &str) -> Element<'static, Message> {
    let content = column![
        text("Request failed").size(TEXT_LG).color(Palette::ERROR).font(crate::ui::theme::UI_FONT_MEDIUM),
        Space::new().height(4),
        text(err.to_owned()).size(TEXT_SM).color(Palette::text_muted()),
    ]
    .max_width(480)
    .align_x(Alignment::Center);

    container(content)
        .padding([24, 24])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn binary_view(resp: &HttpResponse) -> Element<'static, Message> {
    binary_view_with_note(resp, "This response isn't valid text, so it can't be shown in the editor.")
}

fn binary_view_with_note(resp: &HttpResponse, note: &str) -> Element<'static, Message> {
    let content_type = resp
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| "unknown type".to_owned());

    let size_str = if resp.body_size < 1024 {
        format!("{} B", resp.body_size)
    } else if resp.body_size < 1024 * 1024 {
        format!("{:.1} KB", resp.body_size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", resp.body_size as f64 / (1024.0 * 1024.0))
    };

    let content = column![
        text("Binary response").size(TEXT_LG).color(Palette::text()).font(crate::ui::theme::UI_FONT_MEDIUM),
        Space::new().height(4),
        text(format!("{content_type} · {size_str}")).size(TEXT_SM).color(Palette::text_muted()),
        Space::new().height(2),
        text(note.to_owned()).size(TEXT_SM).color(Palette::text_subtle()),
    ]
    .align_x(Alignment::Center);

    container(content)
        .padding([24, 24])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// A simple scrollable grid preview of a parsed spreadsheet's first sheet.
fn spreadsheet_view(sheet: &ParsedSheet) -> Element<'static, Message> {
    let mut rows_col = column![].spacing(0);
    for (row_idx, cells) in sheet.rows.iter().enumerate() {
        let is_header = row_idx == 0;
        let mut row_widget = row![].spacing(0);
        for cell in cells {
            row_widget = row_widget.push(
                container(
                    text(cell.clone())
                        .size(TEXT_SM)
                        .color(if is_header { Palette::text() } else { Palette::text_muted() }),
                )
                .padding([6, 10])
                .width(Length::Fixed(140.0))
                .style(move |_| container::Style {
                    background: if is_header {
                        Some(Background::Color(Palette::surface_high()))
                    } else {
                        None
                    },
                    border: Border { color: Palette::border(), width: 1.0, radius: 0.0.into() },
                    ..Default::default()
                }),
            );
        }
        rows_col = rows_col.push(row_widget);
    }

    let toolbar = container(
        text(format!("Sheet: {} · {} row(s)", sheet.sheet_name, sheet.rows.len()))
            .size(TEXT_XS)
            .color(Palette::text_subtle()),
    )
    .padding([6, 12]);

    column![
        toolbar,
        scrollable(rows_col).direction(scrollable::Direction::Both {
            vertical: scrollable::Scrollbar::default(),
            horizontal: scrollable::Scrollbar::default(),
        })
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .height(Length::Fill)
    .into()
}

/// A rendered PDF page with Prev/Next page navigation.
fn pdf_view(preview: &PdfPreviewState) -> Element<'static, Message> {
    let prev = button(text("< Prev").size(TEXT_SM))
        .on_press_maybe((preview.current_page > 0).then(|| {
            Message::Response(ResponseMsg::PdfPageRequested(preview.current_page - 1))
        }))
        .padding([4, 10]);
    let next = button(text("Next >").size(TEXT_SM))
        .on_press_maybe((preview.current_page + 1 < preview.page_count).then(|| {
            Message::Response(ResponseMsg::PdfPageRequested(preview.current_page + 1))
        }))
        .padding([4, 10]);

    let toolbar = row![
        prev,
        text(format!("Page {} of {}", preview.current_page + 1, preview.page_count.max(1)))
            .size(TEXT_SM)
            .color(Palette::text_muted()),
        next,
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .padding([6, 12]);

    let page: Element<Message> = match &preview.current_image {
        Some(handle) => scrollable(
            container(image(handle.clone()))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(12),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
        None => container(text("Rendering…").size(TEXT_SM).color(Palette::text_subtle()))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
    };

    column![toolbar, page].height(Length::Fill).into()
}

fn loading_view<'a>(spinner_frame: u32) -> Element<'a, Message> {
    let art = spinner_frame_art(spinner_frame);

    container(
        column![
            text(art)
                .size(11)
                .font(MONO)
                .line_height(iced::widget::text::LineHeight::Relative(1.0))
                .color(Palette::accent()),
            Space::new().height(14),
            text("sending request…").size(13).color(Palette::text_muted()),
        ]
        .spacing(0)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

/// Number of pre-rendered spinner frames that are cycled while a request runs.
///
/// `donut` is a little software raytracer: its `theta`/`phi` loops run ~52,000
/// iterations with a `sin_cos` each. It used to be recomputed from scratch on the
/// UI thread for every frame at 30fps, competing for CPU with the very request it
/// was reporting on. The animation depends only on the frame counter, so the
/// frames are rendered once and reused.
const SPINNER_FRAMES: usize = 60;

/// Returns the pre-rendered art for `spinner_frame`, building the set on first
/// use.
fn spinner_frame_art(spinner_frame: u32) -> &'static str {
    use std::sync::OnceLock;
    static FRAMES: OnceLock<Vec<String>> = OnceLock::new();

    let frames = FRAMES.get_or_init(|| {
        (0..SPINNER_FRAMES)
            .map(|i| {
                let f = i as f32;
                donut(f * 0.04, f * 0.023)
            })
            .collect()
    });

    // `SPINNER_FRAMES` is non-zero, so the modulo is always in range.
    frames[(spinner_frame as usize) % SPINNER_FRAMES].as_str()
}

fn donut(a: f32, b: f32) -> String {
    const W: usize = 72;
    const H: usize = 22;
    const K2: f32 = 5.0;
    const R1: f32 = 1.0;
    const R2: f32 = 2.0;
    let kx = W as f32 * 0.27;
    let ky = kx * 0.47;
    let chars = b".,-~:;=!*#$@";

    let mut out = vec![b' '; W * H];
    let mut zbuf = vec![0.0f32; W * H];
    let (sa, ca) = a.sin_cos();
    let (sb, cb) = b.sin_cos();

    let mut theta = 0.0f32;
    while theta < std::f32::consts::TAU {
        let (st, ct) = theta.sin_cos();
        let mut phi = 0.0f32;
        while phi < std::f32::consts::TAU {
            let (sp, cp) = phi.sin_cos();
            let cx = R2 + R1 * ct;
            let cy = R1 * st;
            let x = cx * (cb * cp + sa * sb * sp) - cy * ca * sb;
            let y = cx * (sb * cp - sa * cb * sp) + cy * ca * cb;
            let z = K2 + ca * cx * sp + cy * sa;
            let ooz = 1.0 / z;
            let xp = (W as f32 / 2.0 + kx * ooz * x) as isize;
            let yp = (H as f32 / 2.0 - ky * ooz * y) as isize;
            let lum = cp * ct * sb - ca * ct * sp - sa * st + cb * (ca * st - ct * sa * sp);
            if xp >= 0 && xp < W as isize && yp >= 0 && yp < H as isize {
                let o = xp as usize + yp as usize * W;
                if ooz > zbuf[o] {
                    zbuf[o] = ooz;
                    let li = (lum * 8.0) as isize;
                    out[o] = chars[li.clamp(0, chars.len() as isize - 1) as usize];
                }
            }
            phi += 0.015;
        }
        theta += 0.05;
    }

    let mut s = String::with_capacity(W * H + H);
    for row in 0..H {
        s.push_str(std::str::from_utf8(&out[row * W..row * W + W]).unwrap());
        if row + 1 < H {
            s.push('\n');
        }
    }
    s
}
