use iced::{
    widget::{button, column, container, horizontal_rule, row, scrollable, text, Space},
    Background, Border, Color, Element, Length,
};

use crate::{
    app::AppState,
    message::{GitMsg, Message},
    ui::theme::Palette,
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let mut col_picker = column![
        text("Repository").size(10).color(Palette::text_muted()),
    ]
    .spacing(2)
    .padding([6, 8]);

    if state.collections.is_empty() {
        col_picker = col_picker.push(
            text("No collections yet").size(11).color(Palette::text_subtle()),
        );
    } else {
        for col in state.collections.iter().take(5) {
            col_picker = col_picker.push(
                container(
                    text(&col.name).size(12).color(Palette::text_muted()),
                )
                .padding([2, 0]),
            );
        }
    }

    let commit_btn = button(
        row![
            text("◉").size(10).color(Palette::accent()),
            text(" Commit all").size(11).color(Palette::text()),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::Git(GitMsg::CommitAll))
    .style(|_t, s| {
        let hovered = matches!(s, iced::widget::button::Status::Hovered);
        iced::widget::button::Style {
            background: if hovered {
                Some(Background::Color(Palette::surface_high()))
            } else {
                Some(Background::Color(Palette::surface_raised()))
            },
            text_color: Palette::text(),
            border: Border {
                color: Palette::border(),
                width: 1.0,
                radius: 5.0.into(),
            },
            ..Default::default()
        }
    })
    .padding([5, 10])
    .width(Length::Fill);

    let action_row = container(
        column![commit_btn].spacing(4).padding([4, 8]),
    )
    .width(Length::Fill);

    let status_section = container(
        column![
            text("Status").size(10).color(Palette::text_muted()),
            if state.collections.is_empty() {
                text("No collections to track").size(11).color(Palette::text_subtle())
            } else {
                text("Collections auto-committed on save").size(11).color(Palette::SUCCESS)
            },
        ]
        .spacing(4),
    )
    .padding([8, 8]);

    let log_header = container(
        row![
            text("Commit Log").size(10).color(Palette::text_muted()),
            Space::with_width(Length::Fill),
            text(format!("{} commits", state.git_log.len()))
                .size(10)
                .color(Palette::text_subtle()),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([6, 8]);

    let mut log_col = column![].spacing(0);

    if state.git_log.is_empty() {
        log_col = log_col.push(
            container(
                text("No commits yet. Click 'Commit all' to save your collections to git.")
                    .size(11)
                    .color(Palette::text_subtle()),
            )
            .padding([8, 8]),
        );
    } else {
        for (i, commit) in state.git_log.iter().take(30).enumerate() {
            let hash_short: String = commit.id.chars().take(7).collect();
            let ts = chrono::DateTime::from_timestamp(commit.timestamp, 0)
                .map(|dt| dt.format("%b %d %H:%M").to_string())
                .unwrap_or_default();
            let msg_owned = commit.message.trim().to_owned();
            let hash_owned = hash_short;
            let ts_owned = ts;
            log_col = log_col.push(
                container(
                    column![
                        text(msg_owned).size(12).color(Palette::text()),
                        row![
                            text(hash_owned)
                                .size(10)
                                .color(Palette::accent())
                                .font(iced::Font::MONOSPACE),
                            text(" · ").size(10).color(Palette::text_subtle()),
                            text(ts_owned).size(10).color(Palette::text_subtle()),
                        ]
                        .align_y(iced::Alignment::Center),
                    ]
                    .spacing(2),
                )
                .style(move |_| iced::widget::container::Style {
                    background: if i % 2 == 0 {
                        Some(Background::Color(Color { r: 0.085, g: 0.085, b: 0.095, a: 1.0 }))
                    } else {
                        None
                    },
                    ..Default::default()
                })
                .padding([6, 8])
                .width(Length::Fill),
            );
        }
    }

    let note = container(
        column![
            text("About").size(10).color(Palette::text_muted()),
            text("Collections are stored as JSON files in a local git repository.")
                .size(10)
                .color(Palette::text_subtle()),
            text("Each save auto-commits the changes.")
                .size(10)
                .color(Palette::text_subtle()),
        ]
        .spacing(3),
    )
    .padding([8, 8]);

    scrollable(
        column![
            col_picker,
            horizontal_rule(1),
            action_row,
            horizontal_rule(1),
            status_section,
            horizontal_rule(1),
            log_header,
            log_col,
            horizontal_rule(1),
            note,
        ]
        .spacing(0),
    )
    .height(Length::Fill)
    .style(crate::ui::theme::thin_scrollbar)
    .into()
}
