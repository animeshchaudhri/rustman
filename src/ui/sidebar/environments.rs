use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Background, Border, Color, Element, Length,
};

use crate::{
    app::AppState,
    message::{Message, SidebarMsg},
    ui::{icons, theme::{Palette, TEXT_LG, TEXT_SM}, widgets::kv_table},
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let action_bar = container(
        row![
            text("Environments").size(TEXT_LG).color(Palette::text()).font(crate::ui::theme::UI_FONT_MEDIUM),
            iced::widget::Space::new().width(Length::Fill),
            button(text("+ New").size(TEXT_SM).color(Palette::accent()))
                .on_press(Message::Sidebar(SidebarMsg::EnvironmentCreated))
                .style(iced::widget::button::text)
                .padding([2, 6]),
        ]
        .align_y(iced::Alignment::Center)
        .padding([10, 10]),
    )
    .width(Length::Fill);

    if state.environments.is_empty() {
        return column![
            action_bar,
            iced::widget::rule::horizontal(1.0).style(crate::ui::styles::divider),
            kv_table::empty_state("No environments yet.\nClick + New to create one."),
        ]
        .spacing(0)
        .height(Length::Fill)
        .into();
    }

    let mut col = column![action_bar, iced::widget::rule::horizontal(1.0).style(crate::ui::styles::divider)].spacing(0);

    for env in &state.environments {
        let env_id = env.id.clone();
        let env_id_act = env.id.clone();
        let env_id_del = env.id.clone();
        let env_id_edit = env.id.clone();
        let is_active = env.is_active;
        let is_editing = state.sidebar.env_editing.as_deref() == Some(&env.id);

        let activate_color = if is_active { Palette::SUCCESS } else { Palette::text_muted() };
        let edit_color = if is_editing { Palette::accent() } else { Palette::text_muted() };

        let name_row = container(
            row![
                text_input("Name", &env.name)
                    .on_input({
                        let eid = env.id.clone();
                        move |s| Message::Sidebar(SidebarMsg::EnvironmentNameChanged(eid.clone(), s))
                    })
                    .size(12)
                    .padding([3, 6])
                    .width(Length::Fill)
                    .style(name_input_style),
                button(
                    row![
                        icons::dot(activate_color),
                        text(if is_active { "Active" } else { "Use" }).size(10).color(activate_color),
                    ]
                    .spacing(5)
                    .align_y(iced::Alignment::Center),
                )
                    .on_press(Message::Sidebar(SidebarMsg::EnvironmentSelected(env_id_act)))
                    .style(iced::widget::button::text)
                    .padding([2, 4]),
                button(icons::edit().size(11).color(edit_color))
                    .on_press(Message::Sidebar(SidebarMsg::EnvironmentToggleEdit(env_id_edit)))
                    .style(iced::widget::button::text)
                    .padding([2, 4]),
                button(icons::close().size(10).color(Palette::text_muted()))
                    .on_press(Message::Sidebar(SidebarMsg::EnvironmentDeleted(env_id_del)))
                    .style(iced::widget::button::text)
                    .padding([2, 4]),
            ]
            .spacing(2)
            .align_y(iced::Alignment::Center)
            .padding([5, 8]),
        )
        .width(Length::Fill);

        col = col.push(name_row);

        if is_editing {
            col = col.push(var_editor(env_id, &state.sidebar.env_edit_rows));
        } else if is_active && !env.variables.is_empty() {
            let mut preview = column![].spacing(0);
            let mut sorted: Vec<(&String, &String)> = env.variables.iter().collect();
            sorted.sort();
            for (k, v) in sorted.into_iter().take(4) {
                let short_v = if v.len() > 20 { format!("{}…", &v[..18]) } else { v.clone() };
                preview = preview.push(
                    container(
                        row![
                            text(format!("{{{{{k}}}}}")).size(9).color(Palette::accent()).width(90),
                            text(short_v).size(9).color(Palette::text_muted()),
                        ]
                        .spacing(4),
                    )
                    .padding(iced::Padding { top: 1.0, right: 8.0, bottom: 1.0, left: 16.0 }),
                );
            }
            if env.variables.len() > 4 {
                preview = preview.push(
                    container(
                        text(format!("  +{} more…", env.variables.len() - 4))
                            .size(9)
                            .color(Palette::text_subtle()),
                    )
                    .padding([1, 16]),
                );
            }
            col = col.push(preview);
        }

        col = col.push(iced::widget::rule::horizontal(1.0).style(crate::ui::styles::divider));
    }

    scrollable(col)
        .height(Length::Fill)
        .direction(iced::widget::scrollable::Direction::Vertical(
            crate::ui::theme::grabbable_scrollbar(),
        ))
        .style(crate::ui::theme::thin_scrollbar)
        .into()
}

fn var_editor<'a>(
    env_id: String,
    rows: &'a [(String, String)],
) -> Element<'a, Message> {
    let header = container(
        row![
            text("Key").size(9).color(Palette::text_subtle()).width(Length::Fill),
            text("Value").size(9).color(Palette::text_subtle()).width(Length::Fill),
            iced::widget::Space::new().width(20),
        ]
        .padding(iced::Padding { top: 3.0, right: 8.0, bottom: 3.0, left: 16.0 }),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface_high())),
        ..Default::default()
    })
    .width(Length::Fill);

    let mut var_col = column![header].spacing(0);

    for (i, (k, v)) in rows.iter().enumerate() {
        let eid_k = env_id.clone();
        let eid_v = env_id.clone();
        let eid_r = env_id.clone();
        let key_str = k.clone();
        let val_str = v.clone();
        let is_even = i % 2 == 0;

        let row_el = container(
            row![
                text_input("KEY", &key_str)
                    .on_input(move |s| {
                        Message::Sidebar(SidebarMsg::EnvironmentVarKeyChanged(eid_k.clone(), i, s))
                    })
                    .size(11)
                    .padding([3, 4])
                    .width(Length::Fill)
                    .style(crate::ui::styles::cell_input),
                text_input("value", &val_str)
                    .on_input(move |s| {
                        Message::Sidebar(SidebarMsg::EnvironmentVarValueChanged(eid_v.clone(), i, s))
                    })
                    .size(11)
                    .padding([3, 4])
                    .width(Length::Fill)
                    .style(crate::ui::styles::cell_input),
                button(icons::close().size(9).color(Palette::text_subtle()))
                    .on_press(Message::Sidebar(SidebarMsg::EnvironmentVarRemoved(eid_r, i)))
                    .style(iced::widget::button::text)
                    .padding([2, 4]),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .padding(iced::Padding { top: 2.0, right: 6.0, bottom: 2.0, left: 16.0 }),
        )
        .style(move |_| iced::widget::container::Style {
            background: if is_even {
                Some(Background::Color(Palette::row_odd()))
            } else {
                None
            },
            ..Default::default()
        })
        .width(Length::Fill);

        var_col = var_col.push(row_el);
    }

    let add_btn = container(
        button(
            row![
                text("+").size(12).color(Palette::accent()),
                text(" Add variable").size(10).color(Palette::text_muted()),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::Sidebar(SidebarMsg::EnvironmentVarAdded(env_id)))
        .style(iced::widget::button::text)
        .padding([4, 14]),
    )
    .width(Length::Fill);

    var_col = var_col.push(add_btn);

    container(var_col)
        .style(|_| iced::widget::container::Style {
            border: Border {
                color: Palette::border_subtle(),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
}

fn name_input_style(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    iced::widget::text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: match status {
                iced::widget::text_input::Status::Focused { .. } => Palette::accent(),
                _ => Color::TRANSPARENT,
            },
            width: 1.0,
            radius: 6.0.into(),
        },
        icon: Palette::text_muted(),
        placeholder: Palette::text_subtle(),
        value: Palette::text(),
        selection: Color { r: Palette::accent().r, g: Palette::accent().g, b: Palette::accent().b, a: 0.3 },
    }
}
