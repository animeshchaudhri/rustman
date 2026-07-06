use iced::{
    widget::{button, column, container, row, text, Space},
    Element, Length,
};

use crate::{
    message::{Message, ResponseMsg},
    state::tabs::RequestTabState,
    ui::theme::{Palette, MONO, TEXT_SM, TEXT_XL, TEXT_XS},
};

pub fn view(tab: &RequestTabState, spinner_frame: u32) -> Element<'_, Message> {
    if tab.is_loading {
        return loading_view(spinner_frame);
    }

    let Some(resp) = tab.response.as_ref() else {
        let accent = Palette::accent();
        let shortcut = if cfg!(target_os = "macos") { "Cmd+Enter" } else { "Ctrl+Enter" };
        let content = column![
            text("⇧").size(32).color(accent),
            Space::new().height(10),
            text("Send a request").size(TEXT_XL).color(Palette::text()).font(crate::ui::theme::UI_FONT_MEDIUM),
            Space::new().height(2),
            text(format!("Press {shortcut} or click Send")).size(TEXT_SM).color(Palette::text_subtle()),
        ]
        .align_x(iced::Alignment::Center);

        return container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    };

    if let Some(err) = &resp.error {
        return error_view(err);
    }

    let toolbar = container(
        row![
            Space::new().width(Length::Fill),
            if tab.viewer_processing {
                text("Parsing…").size(TEXT_XS).color(Palette::text_subtle())
            } else {
                text(format!("{}L", tab.response_viewer_lines)).size(TEXT_XS).color(Palette::text_subtle())
            },
            button(text("Copy").size(TEXT_SM).color(Palette::text_muted()))
                .on_press(Message::Response(ResponseMsg::CopyBody))
                .style(iced::widget::button::text)
                .padding([2, 8]),
        ]
        .align_y(iced::Alignment::Center)
        .padding([4, 6]),
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

    column![toolbar, body]
        .spacing(0)
        .height(Length::Fill)
        .into()
}

fn error_view(err: &str) -> Element<'static, Message> {
    let content = column![
        text("Request failed").size(TEXT_XL).color(Palette::ERROR),
        text(err.to_owned()).size(TEXT_SM).color(Palette::text_muted()),
    ]
    .spacing(8)
    .max_width(480)
    .align_x(iced::Alignment::Center);

    container(content)
        .padding([24, 24])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn loading_view<'a>(spinner_frame: u32) -> Element<'a, Message> {
    let f = spinner_frame as f32;
    let art = donut(f * 0.04, f * 0.023);

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
        .align_x(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
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
