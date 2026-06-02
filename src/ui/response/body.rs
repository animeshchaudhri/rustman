use iced::{
    widget::{button, column, container, row, text, Space},
    Element, Length,
};

use crate::{
    message::{Message, ResponseMsg},
    state::tabs::RequestTabState,
    ui::theme::Palette,
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    if tab.is_loading {
        return loading_view();
    }

    if tab.response.is_none() {
        return container(
            text("Send a request to see the response.").size(13).color(Palette::text_muted()),
        )
        .padding([24, 16])
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    let toolbar = container(
        row![
            Space::new().width(Length::Fill),
            if tab.viewer_processing {
                text("Parsing…").size(10).color(Palette::text_subtle())
            } else {
                text(format!("{}L", tab.response_viewer_lines)).size(10).color(Palette::text_subtle())
            },
            button(text("Copy").size(11).color(Palette::text_muted()))
                .on_press(Message::Response(ResponseMsg::CopyBody))
                .style(iced::widget::button::text)
                .padding([2, 8]),
        ]
        .align_y(iced::Alignment::Center)
        .padding([3, 6]),
    )
    .width(Length::Fill);

    let body: Element<Message> = if tab.viewer_processing {
        container(text("Parsing…").size(12).color(Palette::text_subtle()))
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

fn loading_view<'a>() -> Element<'a, Message> {
    container(
        column![
            text("⠋").size(32).color(Palette::accent()),
            Space::new().height(12),
            text("Sending request…").size(13).color(Palette::text_muted()),
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
