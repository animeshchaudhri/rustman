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
        background: Background::Color(Color { r: 0.09, g: 0.09, b: 0.11, a: 1.0 }),
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

  

    let send_btn = if tab.is_loading {
        button(
            row![text("Abort").size(12)]
                .align_y(iced::Alignment::Center)
                .spacing(4),
        )
        .on_press(Message::Request(RequestMsg::Abort))
        .style(abort_button)
        .padding([5, 14])
    } else {
        button(
            row![text("Send").size(12)]
                .align_y(iced::Alignment::Center)
                .spacing(4),
        )
        .on_press(Message::Request(RequestMsg::Send))
        .style(send_button)
        .padding([5, 14])
    };

    container(
        row![method_picker, url_bar,send_btn, curl_btn]
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .padding([6, 10]),
    )
    .style(url_bar_container)
    .width(Length::Fill)
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
        background: Some(Background::Color(Palette::surface())),
        border: Border {
            color: Palette::border_subtle(),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn send_button(_theme: &iced::Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let base_bg = Palette::accent();
    let hover_bg = Color { r: base_bg.r + 0.06, g: base_bg.g + 0.06, b: base_bg.b + 0.04, a: 1.0 };
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
