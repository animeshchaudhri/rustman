//! Compact go-to-line dialog UI.

use iced::mouse;
use iced::widget::canvas::{self, Canvas};
use iced::widget::{Space, Stack, button, container, row, text, text_input};
use iced::{Element, Event, Length, Rectangle, Renderer, Theme, keyboard};
use iced_font_awesome::fa_icon_solid;

use super::Message;
use super::goto_line::GotoLineState;

/// Transparent top layer that lets Escape close the dialog before the focused
/// text input consumes the key to merely unfocus itself.
struct EscapeListener;

impl canvas::Program<Message> for EscapeListener {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if matches!(
            event,
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            })
        ) {
            Some(canvas::Action::publish(Message::CloseGotoLine).and_capture())
        } else {
            None
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        _renderer: &Renderer,
        _theme: &Theme,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        Vec::new()
    }
}

/// Builds the go-to-line input shown over the editor.
pub(crate) fn view(
    state: &GotoLineState,
    line_count: usize,
) -> Element<'_, Message> {
    if !state.is_open {
        return Space::new().into();
    }

    let line_input = text_input("1..N", &state.query)
        .id(state.input_id.clone())
        .on_input(Message::GotoLineChanged)
        .on_submit(Message::SubmitGotoLine)
        .padding(6)
        .width(Length::Fixed(110.0));

    let close_button = button(fa_icon_solid("xmark").size(10.0))
        .on_press(Message::CloseGotoLine)
        .padding(4);

    let content = row![
        text(":").size(16),
        line_input,
        text(format!("/ {}", line_count.max(1))).size(12),
        close_button,
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let dialog = container(content).padding(8).style(|theme| {
        let base = container::rounded_box(theme);
        container::Style {
            background: base.background.map(|background| match background {
                iced::Background::Color(color) => {
                    iced::Background::Color(iced::Color { a: 0.9, ..color })
                }
                _ => background,
            }),
            ..base
        }
    });

    Stack::new()
        .push(dialog)
        .push(
            Canvas::new(EscapeListener)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .into()
}
