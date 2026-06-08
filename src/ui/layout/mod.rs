use iced::{
    widget::{button, column, container, row, Space, Text},
    Background, Color, Element, Length,
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
    } else if state.save_dialog_open {
        iced::widget::stack![root, dialogs::save_dialog(state)].into()
    } else if state.curl_modal_open {
        iced::widget::stack![root, dialogs::curl_modal(state)].into()
    } else {
        root
    }
}

fn left_panel(state: &AppState) -> Element<'_, Message> {
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
    let top_icons: [(SidebarPanel, fn() -> Text<'static>); 3] = [
        (SidebarPanel::Collections, icons::collections),
        (SidebarPanel::History, icons::history),
        (SidebarPanel::Environments, icons::environments),
    ];

    let mut top_col = column![].spacing(0);
    for (panel, icon) in top_icons {
        let active = state.sidebar.panel == panel;
        top_col = top_col.push(icon_btn(icon(), panel, active));
    }

    let settings_active = state.sidebar.panel == SidebarPanel::Settings;
    container(
        column![
            top_col,
            Space::new().height(Length::Fill),
            icon_btn(icons::settings(), SidebarPanel::Settings, settings_active),
        ]
        .padding([0, 0]),
    )
    .width(44)
    .height(Length::Fill)
    .style(icon_rail_style)
    .into()
}

fn icon_btn(icon: Text<'static>, panel: SidebarPanel, active: bool) -> Element<'static, Message> {
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

    button(row![accent_bar, icon_el].height(40))
        .on_press(Message::Sidebar(SidebarMsg::PanelSelected(panel)))
        .style(move |t, s| icon_btn_style(t, s, active))
        .width(44)
        .height(40)
        .padding(0)
        .into()
}

fn main_area(state: &AppState) -> Element<'_, Message> {
    let active_tab = state.tabs.active_tab();
    let url_bar = request::url_bar::view(active_tab);
    let req_tabs = request::tabs::view(active_tab);

    let req_body: Element<Message> = match active_tab.active_request_tab {
        RequestTab::Params    => request::params::view(active_tab),
        RequestTab::Headers   => request::headers::view(active_tab),
        RequestTab::Body      => request::body::view(active_tab),
        RequestTab::Auth      => request::auth::view(active_tab),
        RequestTab::Scripts   => request::scripts::view(active_tab),
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
        ResponseTab::Body    => response::body::view(active_tab),
        ResponseTab::Headers => response::headers::view(active_tab),
        ResponseTab::Cookies => response::cookies::view(active_tab),
        ResponseTab::Tests   => status_bar::response_tests(active_tab),
        ResponseTab::Console => response::console::view(active_tab),
    };

    let response_panel = container(
        column![resp_status, response::tabs::view(active_tab), resp_body]
            .spacing(0)
            .height(Length::Fill),
    )
    .style(surface_style)
    .height(Length::FillPortion(resp_split))
    .width(Length::Fill)
    .padding(2);

    // One guard over the whole editor so Cmd/Ctrl+Z works in the URL bar, headers,
    // params and auth fields alike. Inactive while a code editor is focused so the
    // body/response editors keep their own undo. Guard delegates layout, so this
    // doesn't change the panel proportions.
    let editors_focused = active_tab.body_editor.has_keyboard_focus()
        || active_tab.response_editor.has_keyboard_focus();

    container(
        crate::ui::widgets::key_guard::key_guard(
            column![
                tab_bar::multi_tab_bar(state),
                iced::widget::rule::horizontal(1.0),
                url_bar,
                iced::widget::rule::horizontal(1.0),
                request_panel,
                iced::widget::rule::horizontal(1.0),
                response_panel,
                iced::widget::rule::horizontal(1.0),
                status_bar::status_bar(state),
            ]
            .spacing(0)
            .height(Length::Fill),
            !editors_focused,
            Message::Request(RequestMsg::Undo),
            Message::Request(RequestMsg::Redo),
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
