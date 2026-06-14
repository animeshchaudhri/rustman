use iced::{
    widget::{button, container, pick_list, row, text, text_input},
    Background, Border, Color, Element, Length,
};

use crate::{
    domain::request::HttpMethod,
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::{theme::Palette, icons},
};


pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let methods: Vec<&str> = HttpMethod::all().iter().map(|m| m.as_str()).collect();

    let color = method_color(&tab.method);

    let method_picker = pick_list(
        methods,
        Some(tab.method.as_str()),
        |m| Message::Request(RequestMsg::MethodChanged(m.to_owned())),
    )
    .width(100)
    .text_size(13)
    .padding([6, 8])
    .style(move |_theme, _status| iced::widget::pick_list::Style {
        text_color: color,
        placeholder_color: Palette::text_subtle(),
        handle_color: Palette::text_muted(),
        background: Background::Color(Palette::surface_high()),
        border: Border { color: Palette::border(), width: 1.0, radius: 6.0.into() },
    });

    let url_bar = text_input(
        "https://api.example.com/path  —  or paste a cURL command",
        &tab.url,
    )
    .on_input(|v| {
        let trimmed = v.trim_start().to_lowercase();
        if trimmed.starts_with("curl ") || trimmed.starts_with("curl\t") {
            Message::Request(RequestMsg::ImportCurl(v.trim_start().to_owned()))
        } else if crate::services::import::httpie::is_httpie_command(&v) {
            Message::Request(RequestMsg::ImportHttpie(v.trim_start().to_owned()))
        } else {
            Message::Request(RequestMsg::UrlChanged(v))
        }
    })
    .size(13)
    .padding([7, 12])
    .width(Length::Fill)
    .style(url_input_style);

    let curl_btn = button(icons::curl().size(11).color(Palette::text_subtle()))
        .on_press(Message::Request(RequestMsg::ExportCurl))
        .style(iced::widget::button::text)
        .padding([5, 6]);

  

    let ws_mode = tab.is_websocket();

    let action_btn: Element<Message> = if ws_mode {
        ws_action_button(tab)
    } else if tab.is_loading {
        button(row![text("Abort").size(12)].align_y(iced::Alignment::Center).spacing(4))
            .on_press(Message::Request(RequestMsg::Abort))
            .style(abort_button)
            .padding([5, 14])
            .into()
    } else {
        button(row![text("Send").size(12)].align_y(iced::Alignment::Center).spacing(4))
            .on_press(Message::Request(RequestMsg::Send))
            .style(send_button)
            .padding([5, 14])
            .into()
    };

    let mut bar = row![].spacing(6).align_y(iced::Alignment::Center).padding([6, 10]);
    if ws_mode {
        bar = bar.push(ws_badge());
    } else {
        bar = bar.push(method_picker);
    }
    bar = bar.push(url_bar).push(action_btn);
    if !ws_mode {
        bar = bar.push(curl_btn);
    }

    container(bar)
        .style(url_bar_container)
        .width(Length::Fill)
        .into()
}

fn ws_action_button(tab: &RequestTabState) -> Element<'static, Message> {
    if tab.ws.connected {
        button(text("Disconnect").size(12))
            .on_press(Message::Request(RequestMsg::WsDisconnect))
            .style(abort_button)
            .padding([5, 14])
            .into()
    } else if tab.ws.connecting {
        button(text("Connecting…").size(12)).style(send_button).padding([5, 14]).into()
    } else {
        button(text("Connect").size(12))
            .on_press(Message::Request(RequestMsg::WsConnect))
            .style(send_button)
            .padding([5, 14])
            .into()
    }
}

fn ws_badge() -> Element<'static, Message> {
    container(text("WS").size(12).color(Color::WHITE))
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(Palette::accent())),
            border: Border { radius: 6.0.into(), ..Default::default() },
            ..Default::default()
        })
        .padding([6, 10])
        .into()
}

fn method_color(method: &HttpMethod) -> Color {
    match method {
        HttpMethod::Get => Palette::GET,
        HttpMethod::Post => Palette::POST,
        HttpMethod::Put => Palette::PUT,
        HttpMethod::Patch => Palette::PATCH,
        HttpMethod::Delete => Palette::DELETE,
        HttpMethod::Head | HttpMethod::Options => Palette::HEAD,
    }
}

fn url_input_style(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    iced::widget::text_input::Style {
        background: Background::Color(Palette::surface_high()),
        border: Border {
            color: match status {
                iced::widget::text_input::Status::Focused { .. } => Palette::accent(),
                iced::widget::text_input::Status::Hovered => Palette::border(),
                _ => Palette::border_subtle(),
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

fn url_bar_container(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::chrome())),
        border: Border {
            color: Palette::border_subtle(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn send_button(_theme: &iced::Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let base_bg = Palette::accent();
    let hover_bg = Color {
        r: (base_bg.r + 0.06).min(1.0),
        g: (base_bg.g + 0.06).min(1.0),
        b: (base_bg.b + 0.04).min(1.0),
        a: 1.0,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(match status {
            iced::widget::button::Status::Hovered => hover_bg,
            _ => base_bg,
        })),
        text_color: Color::WHITE,
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn abort_button(_theme: &iced::Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let base_bg = Color { r: 0.75, g: 0.20, b: 0.20, a: 1.0 };
    let hover_bg = Color { r: 0.85, g: 0.25, b: 0.25, a: 1.0 };
    iced::widget::button::Style {
        background: Some(Background::Color(match status {
            iced::widget::button::Status::Hovered => hover_bg,
            _ => base_bg,
        })),
        text_color: Color::WHITE,
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}
