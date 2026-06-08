use iced::{
    widget::{button, container, row, text, Space},
    Background, Border, Color, Element, Length,
};

use crate::{
    app::{AppState, UpdateState},
    message::{Message, UpdateMsg},
    ui::theme::Palette,
};

/// A thin banner shown when an update is available, installing, ready, or failed.
/// Returns `None` for the quiet states (idle / checking / up-to-date).
pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    let (label, actions, color): (String, Vec<Element<Message>>, Color) = match &state.update {
        UpdateState::Available(info) => (
            format!("Rustman v{} is available  ·  you have v{}", info.version, info.current),
            vec![
                action("Update now", Message::Update(UpdateMsg::Install), true),
                action("Dismiss", Message::Update(UpdateMsg::Dismiss), false),
            ],
            Palette::accent(),
        ),
        UpdateState::Installing => (
            "Downloading update…".to_owned(),
            vec![],
            Palette::accent(),
        ),
        UpdateState::Ready(version) => (
            format!("Updated to v{version} — restart to apply"),
            vec![
                action("Restart", Message::Update(UpdateMsg::Restart), true),
                action("Later", Message::Update(UpdateMsg::Dismiss), false),
            ],
            Palette::SUCCESS,
        ),
        UpdateState::Failed(err) => (
            format!("Update failed: {err}"),
            vec![action("Dismiss", Message::Update(UpdateMsg::Dismiss), false)],
            Palette::ERROR,
        ),
        UpdateState::Idle | UpdateState::Checking | UpdateState::UpToDate => return None,
    };

    let mut bar = row![
        text(label).size(12).color(Palette::text()),
        Space::new().width(Length::Fill),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);
    for a in actions {
        bar = bar.push(a);
    }

    let tint = Color { a: 0.16, ..color };
    Some(
        container(bar)
            .style(move |_| container::Style {
                background: Some(Background::Color(tint)),
                border: Border { color, width: 0.0, radius: 0.0.into() },
                ..Default::default()
            })
            .padding([6, 12])
            .width(Length::Fill)
            .into(),
    )
}

fn action(label: &str, msg: Message, primary: bool) -> Element<'static, Message> {
    let label = label.to_owned();
    button(
        text(label)
            .size(11)
            .color(if primary { Color::WHITE } else { Palette::text_muted() }),
    )
    .on_press(msg)
    .style(move |_t, status| {
        let hovered = matches!(status, iced::widget::button::Status::Hovered);
        let bg = if primary {
            let a = Palette::accent();
            Some(Background::Color(if hovered {
                Color { r: a.r + 0.06, g: a.g + 0.06, b: a.b + 0.04, a: 1.0 }
            } else {
                a
            }))
        } else if hovered {
            Some(Background::Color(Palette::surface_high()))
        } else {
            None
        };
        iced::widget::button::Style {
            background: bg,
            text_color: if primary { Color::WHITE } else { Palette::text_muted() },
            border: Border { radius: 5.0.into(), ..Default::default() },
            ..Default::default()
        }
    })
    .padding([3, 10])
    .into()
}
