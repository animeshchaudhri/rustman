use iced::{
    widget::{button, column, container, row, text, tooltip, Space, Text},
    Background, Border, Color, Element, Length,
};

use crate::{
    app::AppState,
    message::{Message, RequestMsg, RequestTab, ResponseTab, SidebarMsg, SidebarPanel},
    ui::{icons, request, response, sidebar, theme::Palette},
};

mod dialogs;
mod status_bar;
mod style;
mod tab_bar;

use style::{icon_btn_style, icon_rail_style, sidebar_style, surface_style};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let main = row![left_panel(state), main_area(state)].height(Length::Fill);
    let root: Element<Message> = match crate::ui::update_banner::view(state) {
        Some(banner) => column![banner, main].into(),
        None => main.into(),
    };

    if state.palette_open {
        iced::widget::stack![root, crate::ui::command_palette::view(state)].into()
    } else if state.close_confirm_tab.is_some() {
        iced::widget::stack![root, dialogs::close_confirm_dialog(state)].into()
    } else if state.git_restore_confirm.is_some() {
        iced::widget::stack![root, dialogs::restore_confirm_dialog(state)].into()
    } else if state.export_dialog_collection.is_some() {
        iced::widget::stack![root, dialogs::export_dialog(state)].into()
    } else if state.save_dialog_open {
        iced::widget::stack![root, dialogs::save_dialog(state)].into()
    } else if state.curl_modal_open {
        iced::widget::stack![root, dialogs::curl_modal(state)].into()
    } else {
        root
    }
}

fn left_panel(state: &AppState) -> Element<'_, Message> {
    if state.sidebar.collapsed {
        return container(icon_rail(state))
            .style(sidebar_style)
            .width(48)
            .height(Length::Fill)
            .into();
    }

    let panel_content: Element<Message> = match state.sidebar.panel {
        SidebarPanel::Collections => sidebar::collections::view(state),
        SidebarPanel::History    => sidebar::history::view(state),
        SidebarPanel::Environments => sidebar::environments::view(state),
        SidebarPanel::Git        => sidebar::git::view(state),
        SidebarPanel::Settings   => sidebar::settings::view(state),
    };

    container(
        row![
            icon_rail(state),
            iced::widget::rule::vertical(0.0),
            container(panel_content).width(Length::Fill).height(Length::Fill),
        ]
        .height(Length::Fill),
    )
    .style(sidebar_style)
    .width(264)
    .height(Length::Fill)
    .into()
}

fn icon_rail(state: &AppState) -> Element<'_, Message> {
    let top_icons: [(SidebarPanel, fn() -> Text<'static>, &'static str); 4] = [
        (SidebarPanel::Collections, icons::collections, "Collections"),
        (SidebarPanel::History, icons::history, "History"),
        (SidebarPanel::Environments, icons::environments, "Environments"),
        (SidebarPanel::Git, icons::git_branch, "Source Control"),
    ];

    let mut top_col = column![].spacing(2);
    for (panel, icon, label) in top_icons {
        let active = state.sidebar.panel == panel;
        top_col = top_col.push(icon_btn(icon(), panel, active, label));
    }

    let settings_active = state.sidebar.panel == SidebarPanel::Settings;
    let collapsed = state.sidebar.collapsed;
    container(
        column![
            top_col,
            Space::new().height(Length::Fill),
            collapse_toggle_btn(collapsed),
            icon_btn(icons::settings(), SidebarPanel::Settings, settings_active, "Settings"),
        ]
        .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 6.0, left: 0.0 })
        .spacing(2),
    )
    .width(48)
    .height(Length::Fill)
    .style(icon_rail_style)
    .into()
}

fn collapse_toggle_btn(collapsed: bool) -> Element<'static, Message> {
    let icon = if collapsed { icons::arrow_right() } else { icons::arrow_left() };
    let label = if collapsed { "Expand sidebar" } else { "Collapse sidebar" };

    let btn = button(
        container(icon.size(14).color(Palette::text_subtle()))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .on_press(Message::Sidebar(SidebarMsg::ToggleCollapsed))
    .style(|_t, s| iced::widget::button::Style {
        background: if matches!(s, iced::widget::button::Status::Hovered) {
            Some(Background::Color(Palette::hover()))
        } else {
            None
        },
        text_color: Palette::text_subtle(),
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    })
    .width(48)
    .height(28)
    .padding(0);

    tooltip(
        btn,
        container(text(label).size(crate::ui::theme::TEXT_SM).color(Palette::text()))
            .padding([4, 8])
            .style(|_| iced::widget::container::Style {
                background: Some(Background::Color(Palette::surface_raised())),
                border: Border { color: Palette::border(), width: 1.0, radius: 6.0.into() },
                ..Default::default()
            }),
        tooltip::Position::Right,
    )
    .gap(8)
    .into()
}

fn icon_btn(
    icon: Text<'static>,
    panel: SidebarPanel,
    active: bool,
    label: &'static str,
) -> Element<'static, Message> {
    let accent_bar = container(Space::new().width(2).height(Length::Fill))
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(if active { Palette::accent() } else { Color::TRANSPARENT })),
            ..Default::default()
        })
        .height(Length::Fill);

    let icon_el = container(
        icon.size(18).color(if active { Palette::accent() } else { Palette::text_subtle() }),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .height(Length::Fill);

    let btn = button(row![accent_bar, icon_el].height(42))
        .on_press(Message::Sidebar(SidebarMsg::PanelSelected(panel)))
        .style(move |t, s| icon_btn_style(t, s, active))
        .width(48)
        .height(42)
        .padding(0);

    tooltip(
        btn,
        container(text(label).size(crate::ui::theme::TEXT_SM).color(Palette::text()))
            .padding([4, 8])
            .style(|_| iced::widget::container::Style {
                background: Some(Background::Color(Palette::surface_raised())),
                border: Border { color: Palette::border(), width: 1.0, radius: 6.0.into() },
                ..Default::default()
            }),
        tooltip::Position::Right,
    )
    .gap(8)
    .into()
}

fn main_area(state: &AppState) -> Element<'_, Message> {
    let active_tab = state.tabs.active_tab();
    let active_env = state.environments.iter().find(|e| e.is_active);
    let url_bar = request::url_bar::view(active_tab, active_env);
    let req_tabs = request::tabs::view(active_tab);

    let req_body: Element<Message> = match active_tab.active_request_tab {
        RequestTab::Params    => request::params::view(active_tab, active_env),
        RequestTab::Headers   => request::headers::view(active_tab),
        RequestTab::Body      => request::body::view(active_tab),
        RequestTab::Auth      => request::auth::view(active_tab),
        RequestTab::Scripts   => request::scripts::view(active_tab),
        RequestTab::Settings  => request::settings::view(active_tab),
        RequestTab::WebSocket => request::websocket::view(active_tab),
    };

    let req_split  = state.panel_split as u16;
    let resp_split = (10 - state.panel_split).max(1) as u16;

    let request_panel = container(
        column![req_tabs, req_body].spacing(0).height(Length::Fill),
    )
    .height(Length::FillPortion(req_split))
    .style(surface_content_style)
    .width(Length::Fill);

    let resp_status: Element<Message> = active_tab
        .response.as_ref()
        .map(|r| response::status_bar::view(r))
        .unwrap_or_else(|| Space::new().into());

    let resp_body: Element<Message> = match active_tab.active_response_tab {
        ResponseTab::Body    => response::body::view(active_tab, state.spinner_frame),
        ResponseTab::Headers => response::headers::view(active_tab),
        ResponseTab::Cookies => response::cookies::view(active_tab),
    };

    let response_panel = container(
        column![resp_status, response::tabs::view(active_tab), resp_body]
            .spacing(0)
            .height(Length::Fill),
    )
    .style(surface_style)
    .height(Length::FillPortion(resp_split))
    .width(Length::Fill);

    let middle: Element<Message> = if active_tab.is_websocket() {
        container(request::websocket::view(active_tab))
            .style(surface_content_style)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        column![
            request_panel,
            iced::widget::rule::horizontal(1.0).style(crate::ui::styles::divider),
            response_panel,
        ]
        .spacing(0)
        .height(Length::Fill)
        .into()
    };

    let editors_focused = active_tab.body_editor.has_keyboard_focus()
        || active_tab.response_editor.has_keyboard_focus();

    container(
        crate::ui::widgets::key_guard::key_guard(
            column![
                tab_bar::multi_tab_bar(state),
                iced::widget::rule::horizontal(1.0).style(crate::ui::styles::divider),
                url_bar,
                iced::widget::rule::horizontal(1.0).style(crate::ui::styles::divider),
                middle,
                iced::widget::rule::horizontal(1.0).style(crate::ui::styles::divider),
                status_bar::status_bar(state),
            ]
            .spacing(0)
            .height(Length::Fill),
            !editors_focused,
            Message::Request(RequestMsg::Undo),
            Message::Request(RequestMsg::Redo),
            Message::Request(RequestMsg::Send),
        ),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn surface_content_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface())),
        ..Default::default()
    }
}
