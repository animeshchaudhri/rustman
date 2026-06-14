use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Space},
    Background, Border, Color, Element, Length,
};

use crate::{
    app::AppState,
    message::{GitMsg, Message, RepoSummary},
    services::repos::LOCAL_ID,
    ui::theme::{Palette, MONO},
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let branch = state
        .git_status
        .as_ref()
        .map(|s| s.branch.clone())
        .unwrap_or_else(|| "main".to_owned());

    let mut root = column![title_bar(), commit_box(state, &branch)].spacing(0);

    if state.git_busy {
        root = root.push(
            container(text("Working…").size(11).color(Palette::accent()))
                .padding([3, 14])
                .width(Length::Fill),
        );
    }

    root = root
        .push(divider())
        .push(repositories_group(state))
        .push(divider())
        .push(changes_group(state))
        .push(sync_row(state))
        .push(divider())
        .push(branches_group(state))
        .push(divider())
        .push(history_group(state))
        .push(divider())
        .push(add_repo_row(state));

    scrollable(root)
        .height(Length::Fill)
        .style(crate::ui::theme::thin_scrollbar)
        .into()
}

fn title_bar() -> Element<'static, Message> {
    container(
        row![
            text("SOURCE CONTROL").size(11).color(Palette::text_muted()),
            Space::new().width(Length::Fill),
            tool_button("⟳", Message::Git(GitMsg::Refresh)),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([8, 12])
    .width(Length::Fill)
    .into()
}

fn commit_box<'a>(state: &'a AppState, branch: &str) -> Element<'a, Message> {
    let input = text_input("Message (Enter to commit)", &state.git_commit_message)
        .on_input(|v| Message::Git(GitMsg::CommitMessageChanged(v)))
        .on_submit(Message::Git(GitMsg::Commit))
        .size(12)
        .padding([8, 10])
        .style(scm_input_style);

    let commit = button(
        text(format!("✓  Commit to {branch}"))
            .size(12)
            .color(Color::WHITE)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .on_press(Message::Git(GitMsg::Commit))
    .style(primary_style)
    .padding([7, 12])
    .width(Length::Fill);

    column![input, commit].spacing(6).padding([6, 12]).width(Length::Fill).into()
}

fn repositories_group(state: &AppState) -> Element<'_, Message> {
    let mut col = column![group_header("Repositories", Some(state.git_repos.len()))]
        .spacing(1)
        .padding([6, 6]);

    for repo in &state.git_repos {
        let active = repo.id == state.git_active_repo;
        let summary = summary_for(state, &repo.id);
        let branch = summary.map(|s| s.branch.clone()).unwrap_or_else(|| "—".to_owned());
        let sync = summary
            .filter(|s| s.ahead > 0 || s.behind > 0)
            .map(|s| format!("↑{} ↓{}", s.ahead, s.behind))
            .unwrap_or_default();
        let changes = summary.map(|s| s.changes).unwrap_or(0);

        let mut line = row![
            text(if active { "●" } else { "○" })
                .size(10)
                .color(if active { Palette::accent() } else { Palette::text_subtle() }),
            text(&repo.name).size(12).color(Palette::text()),
            text(branch).size(10).color(Palette::text_subtle()).font(MONO),
            Space::new().width(Length::Fill),
            text(sync).size(10).color(Palette::accent()).font(MONO),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);
        if changes > 0 {
            line = line.push(count_badge(changes));
        }

        let select = button(line)
            .on_press(Message::Git(GitMsg::SelectRepo(repo.id.clone())))
            .style(move |_t, status| repo_row_style(status, active))
            .padding([5, 8])
            .width(Length::Fill);

        let mut entry = row![select].spacing(2).align_y(iced::Alignment::Center);
        if repo.id != LOCAL_ID {
            entry = entry.push(tool_button("✕", Message::Git(GitMsg::RemoveRepo(repo.id.clone()))));
        }
        col = col.push(entry);
    }

    col.width(Length::Fill).into()
}

fn changes_group(state: &AppState) -> Element<'_, Message> {
    let changes = state.git_status.as_ref().map(|s| s.changes.as_slice()).unwrap_or(&[]);

    let header = row![
        group_header("Changes", Some(changes.len())),
        Space::new().width(Length::Fill),
        tool_text(if state.git_diff.is_some() { "Hide diff" } else { "Diff" }, Message::Git(GitMsg::ToggleDiff)),
    ]
    .align_y(iced::Alignment::Center);

    let mut col = column![header].spacing(1).padding([6, 6]);

    if changes.is_empty() {
        col = col.push(
            container(text("No changes").size(11).color(Palette::text_subtle())).padding([2, 10]),
        );
    } else {
        for change in changes.iter().take(80) {
            col = col.push(
                container(
                    row![
                        text(pretty_name(state, &change.path)).size(12).color(Palette::text_muted()),
                        Space::new().width(Length::Fill),
                        text(badge(&change.state)).size(11).color(state_color(&change.state)).font(MONO),
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
                )
                .padding([3, 10])
                .width(Length::Fill),
            );
        }
    }

    if let Some(diff) = &state.git_diff {
        col = col.push(
            container(
                scrollable(text(diff.clone()).size(10).font(MONO).color(Palette::text_muted()))
                    .height(180)
                    .style(crate::ui::theme::thin_scrollbar),
            )
            .padding(8)
            .style(|_| container::Style {
                background: Some(Background::Color(Palette::background())),
                border: Border { color: Palette::border(), width: 1.0, radius: 6.0.into() },
                ..Default::default()
            }),
        );
    }

    col.width(Length::Fill).into()
}

fn sync_row(state: &AppState) -> Element<'_, Message> {
    let has_remote = state.git_status.as_ref().and_then(|s| s.remote_url.as_ref()).is_some();
    let (ahead, behind) = state.git_status.as_ref().map(|s| (s.ahead, s.behind)).unwrap_or((0, 0));

    let mut col = column![].spacing(6).padding([6, 12]).width(Length::Fill);

    if has_remote {
        col = col
            .push(
                row![
                    text("origin").size(10).color(Palette::text_subtle()).font(MONO),
                    Space::new().width(Length::Fill),
                    text(format!("↑{ahead} ↓{behind}")).size(10).color(Palette::accent()).font(MONO),
                ]
                .align_y(iced::Alignment::Center),
            )
            .push(
                row![
                    ghost_button("⟱ Pull", Message::Git(GitMsg::Pull)),
                    ghost_button("Fetch", Message::Git(GitMsg::Fetch)),
                    primary_button("⟰ Push", Message::Git(GitMsg::Push)),
                ]
                .spacing(6),
            )
            .push(
                text("Push/Pull use your system git — same SSH keys / login as your terminal.")
                    .size(9)
                    .color(Palette::text_subtle()),
            );
    } else {
        let remote = text_input("Remote URL (https:// or git@…)", &state.git_remote_input)
            .on_input(|v| Message::Git(GitMsg::RemoteUrlChanged(v)))
            .on_submit(Message::Git(GitMsg::SetRemote))
            .size(11)
            .padding([6, 10])
            .style(scm_input_style);
        col = col
            .push(
                text("Create the repo on any git host (GitHub, GitLab, Azure DevOps, self-hosted…), then add its URL:")
                    .size(9)
                    .color(Palette::text_subtle()),
            )
            .push(remote)
            .push(primary_button("Add remote", Message::Git(GitMsg::SetRemote)));
    }

    col.into()
}

fn branches_group(state: &AppState) -> Element<'_, Message> {
    let mut col = column![group_header("Branches", None)].spacing(1).padding([6, 6]);

    for branch in state.git_branches.iter().take(20) {
        let name = branch.name.clone();
        col = col.push(
            button(
                row![
                    text(if branch.is_head { "●" } else { "○" })
                        .size(10)
                        .color(if branch.is_head { Palette::accent() } else { Palette::text_subtle() }),
                    text(&branch.name).size(11).color(Palette::text_muted()),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::Git(GitMsg::SwitchBranch(name)))
            .style(|_t, status| repo_row_style(status, false))
            .padding([4, 8])
            .width(Length::Fill),
        );
    }

    let new_branch = text_input("New branch…", &state.git_new_branch)
        .on_input(|v| Message::Git(GitMsg::NewBranchNameChanged(v)))
        .on_submit(Message::Git(GitMsg::CreateBranch))
        .size(11)
        .padding([6, 10])
        .style(scm_input_style);

    col = col.push(container(new_branch).padding([2, 6]));
    col.width(Length::Fill).into()
}

fn history_group(state: &AppState) -> Element<'_, Message> {
    let mut col = column![group_header("History", Some(state.git_log.len()))]
        .spacing(0)
        .padding([6, 6]);

    if state.git_log.is_empty() {
        col = col.push(
            container(text("No commits yet").size(11).color(Palette::text_subtle())).padding([2, 10]),
        );
        return col.width(Length::Fill).into();
    }

    for commit in state.git_log.iter().take(40) {
        let hash: String = commit.id.chars().take(7).collect();
        let when = chrono::DateTime::from_timestamp(commit.timestamp, 0)
            .map(|dt| dt.format("%b %d %H:%M").to_string())
            .unwrap_or_default();
        let id = commit.id.clone();
        col = col.push(
            container(
                row![
                    column![
                        text(commit.message.trim().to_owned()).size(12).color(Palette::text()),
                        row![
                            text(hash).size(10).color(Palette::accent()).font(MONO),
                            text(" · ").size(10).color(Palette::text_subtle()),
                            text(when).size(10).color(Palette::text_subtle()),
                        ]
                        .align_y(iced::Alignment::Center),
                    ]
                    .spacing(2),
                    Space::new().width(Length::Fill),
                    tool_text("Restore", Message::Git(GitMsg::AskRestore(id))),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            )
            .padding([5, 10])
            .width(Length::Fill),
        );
    }

    col.width(Length::Fill).into()
}

fn add_repo_row(state: &AppState) -> Element<'_, Message> {
    let clone = text_input("Clone URL (https:// or git@…)", &state.git_clone_url)
        .on_input(|v| Message::Git(GitMsg::CloneUrlChanged(v)))
        .on_submit(Message::Git(GitMsg::CloneRepo))
        .size(11)
        .padding([6, 10])
        .style(scm_input_style);

    column![
        group_header("Add repository", None),
        clone,
        row![
            ghost_button("Clone", Message::Git(GitMsg::CloneRepo)),
            ghost_button("Open folder…", Message::Git(GitMsg::OpenFolder)),
        ]
        .spacing(6),
    ]
    .spacing(6)
    .padding([6, 12])
    .width(Length::Fill)
    .into()
}

fn summary_for<'a>(state: &'a AppState, id: &str) -> Option<&'a RepoSummary> {
    state.git_repo_summaries.iter().find(|s| s.id == id)
}

fn pretty_name(state: &AppState, path: &str) -> String {
    let id = path.trim_end_matches(".json");
    state
        .collections
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| path.to_owned())
}

fn group_header(label: &str, count: Option<usize>) -> Element<'static, Message> {
    let mut head = row![text(label.to_uppercase()).size(10).color(Palette::text_muted())]
        .spacing(6)
        .align_y(iced::Alignment::Center);
    if let Some(n) = count {
        head = head.push(count_badge(n));
    }
    container(head).padding([2, 6]).into()
}

fn count_badge(n: usize) -> Element<'static, Message> {
    container(text(n.to_string()).size(9).color(Palette::text()))
        .style(|_| container::Style {
            background: Some(Background::Color(Palette::surface_high())),
            border: Border { radius: 8.0.into(), ..Default::default() },
            ..Default::default()
        })
        .padding([1, 6])
        .into()
}

fn divider() -> Element<'static, Message> {
    container(iced::widget::rule::horizontal(1.0).style(crate::ui::styles::divider)).padding([3, 0]).into()
}

fn badge(state: &str) -> String {
    match state {
        "new" => "U".to_owned(),
        "deleted" => "D".to_owned(),
        "renamed" => "R".to_owned(),
        _ => "M".to_owned(),
    }
}

fn state_color(state: &str) -> Color {
    match state {
        "new" => Palette::SUCCESS,
        "deleted" => Palette::ERROR,
        _ => Palette::WARNING,
    }
}

fn scm_input_style(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    iced::widget::text_input::Style {
        background: Background::Color(Palette::surface_high()),
        border: Border {
            color: match status {
                iced::widget::text_input::Status::Focused { .. } => Palette::accent(),
                _ => Palette::border_subtle(),
            },
            width: 1.0,
            radius: 5.0.into(),
        },
        icon: Palette::text_muted(),
        placeholder: Palette::text_subtle(),
        value: Palette::text(),
        selection: Color { r: Palette::accent().r, g: Palette::accent().g, b: Palette::accent().b, a: 0.3 },
    }
}

fn primary_style(_t: &iced::Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let accent = Palette::accent();
    let bg = if matches!(status, iced::widget::button::Status::Hovered) {
        Color { r: (accent.r + 0.06).min(1.0), g: (accent.g + 0.06).min(1.0), b: (accent.b + 0.04).min(1.0), a: 1.0 }
    } else {
        accent
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border { radius: 5.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn primary_button(label: &str, msg: Message) -> Element<'static, Message> {
    button(text(label.to_owned()).size(11).color(Color::WHITE))
        .on_press(msg)
        .style(primary_style)
        .padding([6, 12])
        .into()
}

fn ghost_button(label: &str, msg: Message) -> Element<'static, Message> {
    button(text(label.to_owned()).size(11).color(Palette::text()))
        .on_press(msg)
        .style(|_t, status| {
            let hovered = matches!(status, iced::widget::button::Status::Hovered);
            iced::widget::button::Style {
                background: Some(Background::Color(if hovered {
                    Palette::surface_high()
                } else {
                    Palette::surface_raised()
                })),
                text_color: Palette::text(),
                border: Border { color: Palette::border(), width: 1.0, radius: 5.0.into() },
                ..Default::default()
            }
        })
        .padding([5, 10])
        .into()
}

fn tool_button(glyph: &str, msg: Message) -> Element<'static, Message> {
    button(text(glyph.to_owned()).size(13).color(Palette::text_muted()))
        .on_press(msg)
        .style(iced::widget::button::text)
        .padding([2, 6])
        .into()
}

fn tool_text(label: &str, msg: Message) -> Element<'static, Message> {
    button(text(label.to_owned()).size(10).color(Palette::accent()))
        .on_press(msg)
        .style(iced::widget::button::text)
        .padding([2, 6])
        .into()
}

fn repo_row_style(status: iced::widget::button::Status, active: bool) -> iced::widget::button::Style {
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    iced::widget::button::Style {
        background: if active {
            Some(Background::Color(Palette::surface_high()))
        } else if hovered {
            Some(Background::Color(Palette::surface_raised()))
        } else {
            None
        },
        text_color: Palette::text(),
        border: Border {
            color: if active { Palette::accent() } else { Color::TRANSPARENT },
            width: if active { 1.0 } else { 0.0 },
            radius: 5.0.into(),
        },
        ..Default::default()
    }
}
