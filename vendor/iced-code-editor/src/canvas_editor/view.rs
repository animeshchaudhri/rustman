//! Iced UI view and rendering logic.

use iced::Size;
use iced::advanced::input_method;
use iced::widget::canvas::Canvas;
use iced::widget::{Column, Row, Scrollable, Space, container, scrollable};
use iced::{Background, Border, Color, Element, Length, Rectangle, Shadow};

use super::ime_requester::ImeRequester;
use super::search_dialog;
use super::wrapping::{self, WrappingCalculator};
use super::{CodeEditor, GUTTER_WIDTH, Message};
use std::rc::Rc;

impl CodeEditor {
    /// Calculates visual lines and canvas height for the editor.
    ///
    /// Returns a tuple of (visual_lines, canvas_height) where:
    /// - visual_lines: The visual line mapping with wrapping applied
    /// - canvas_height: The total height needed for the canvas
    fn calculate_canvas_height(&self) -> (Rc<Vec<wrapping::VisualLine>>, f32) {
        // Reuse memoized visual lines so view layout (canvas height + IME cursor rect)
        // does not trigger repeated wrapping computation.
        let visual_lines = self.visual_lines_cached(self.viewport_width);
        let total_visual_lines = visual_lines.len();
        let content_height = total_visual_lines as f32 * self.line_height;

        // Use max of content height and viewport height to ensure the canvas
        // always covers the visible area (prevents visual artifacts when
        // content is shorter than viewport after reset/file change)
        let canvas_height = content_height.max(self.viewport_height);

        (visual_lines, canvas_height)
    }

    /// Creates the scrollable style function with custom colors.
    ///
    /// Returns a style function that configures the scrollbar appearance.
    fn create_scrollable_style(
        &self,
    ) -> impl Fn(&iced::Theme, scrollable::Status) -> scrollable::Style {
        let scrollbar_bg = self.style.scrollbar_background;
        let scroller_color = self.style.scroller_color;

        move |_theme, _status| scrollable::Style {
            container: container::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                ..container::Style::default()
            },
            vertical_rail: scrollable::Rail {
                background: Some(scrollbar_bg.into()),
                border: Border {
                    radius: 4.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                scroller: scrollable::Scroller {
                    background: scroller_color.into(),
                    border: Border {
                        radius: 4.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
            },
            horizontal_rail: scrollable::Rail {
                background: Some(scrollbar_bg.into()),
                border: Border {
                    radius: 4.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                scroller: scrollable::Scroller {
                    background: scroller_color.into(),
                    border: Border {
                        radius: 4.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
            },
            gap: None,
            auto_scroll: scrollable::AutoScroll {
                background: Color::TRANSPARENT.into(),
                border: Border::default(),
                shadow: Shadow::default(),
                icon: Color::TRANSPARENT,
            },
        }
    }

    /// Creates the canvas widget wrapped in a scrollable container.
    ///
    /// # Arguments
    ///
    /// * `canvas_height` - The total height of the canvas
    ///
    /// # Returns
    ///
    /// A configured scrollable widget containing the canvas
    fn create_canvas_with_scrollable(
        &self,
        canvas_height: f32,
    ) -> Scrollable<'_, Message> {
        let canvas = Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fixed(canvas_height));

        Scrollable::new(canvas)
            .id(self.scrollable_id.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .on_scroll(Message::Scrolled)
            .style(self.create_scrollable_style())
    }

    /// Creates the horizontal scrollbar element when wrap is disabled and content overflows.
    ///
    /// # Arguments
    ///
    /// * `max_content_width` - The total pixel width of the widest line
    ///
    /// # Returns
    ///
    /// `Some(element)` if a horizontal scrollbar is needed, `None` otherwise
    fn create_horizontal_scrollbar(
        &self,
        max_content_width: f32,
    ) -> Option<Element<'_, Message>> {
        if self.wrap_enabled || max_content_width <= self.viewport_width {
            return None;
        }

        let scrollbar_bg = self.style.scrollbar_background;
        let scroller_color = self.style.scroller_color;

        let h_scrollable = Scrollable::new(
            Space::new().width(Length::Fixed(max_content_width)).height(0.0),
        )
        .id(self.horizontal_scrollable_id.clone())
        .width(Length::Fill)
        .height(Length::Fixed(12.0))
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new(),
        ))
        .on_scroll(Message::HorizontalScrolled)
        .style(move |_theme, _status| scrollable::Style {
            container: container::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                ..container::Style::default()
            },
            vertical_rail: scrollable::Rail {
                background: Some(scrollbar_bg.into()),
                border: Border {
                    radius: 4.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                scroller: scrollable::Scroller {
                    background: scroller_color.into(),
                    border: Border {
                        radius: 4.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
            },
            horizontal_rail: scrollable::Rail {
                background: Some(scrollbar_bg.into()),
                border: Border {
                    radius: 4.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                scroller: scrollable::Scroller {
                    background: scroller_color.into(),
                    border: Border {
                        radius: 4.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
            },
            gap: None,
            auto_scroll: scrollable::AutoScroll {
                background: Color::TRANSPARENT.into(),
                border: Border::default(),
                shadow: Shadow::default(),
                icon: Color::TRANSPARENT,
            },
        });

        Some(h_scrollable.into())
    }

    /// Creates the gutter background container if line numbers are enabled.
    ///
    /// # Returns
    ///
    /// Some(container) if line numbers are enabled, None otherwise
    fn create_gutter_container(
        &self,
    ) -> Option<container::Container<'_, Message>> {
        if self.line_numbers_enabled {
            let gutter_background = self.style.gutter_background;
            Some(
                container(
                    Space::new().width(Length::Fill).height(Length::Fill),
                )
                .width(Length::Fixed(GUTTER_WIDTH))
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Background::Color(gutter_background)),
                    ..container::Style::default()
                }),
            )
        } else {
            None
        }
    }

    /// Creates the code area background container.
    ///
    /// # Returns
    ///
    /// The code background container widget
    fn create_code_background_container(
        &self,
    ) -> container::Container<'_, Message> {
        let background_color = self.style.background;
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(background_color)),
                ..container::Style::default()
            })
    }

    /// Creates the background layer combining gutter and code backgrounds.
    ///
    /// # Returns
    ///
    /// A row containing the background elements
    fn create_background_layer(&self) -> Row<'_, Message> {
        let gutter_container = self.create_gutter_container();
        let code_background_container = self.create_code_background_container();

        if let Some(gutter) = gutter_container {
            Row::new().push(gutter).push(code_background_container)
        } else {
            Row::new().push(code_background_container)
        }
    }

    /// Calculates the IME cursor rectangle for the current cursor position.
    ///
    /// # Arguments
    ///
    /// * `visual_lines` - The visual line mapping
    ///
    /// # Returns
    ///
    /// A rectangle representing the cursor position for IME
    fn calculate_ime_cursor_rect(
        &self,
        visual_lines: &[wrapping::VisualLine],
    ) -> Rectangle {
        let ime_enabled = self.is_focused() && self.has_canvas_focus;

        if !ime_enabled {
            return Rectangle::new(
                iced::Point::new(0.0, 0.0),
                Size::new(0.0, 0.0),
            );
        }

        if let Some(cursor_visual) = WrappingCalculator::logical_to_visual(
            visual_lines,
            self.cursors.primary_position().0,
            self.cursors.primary_position().1,
        ) {
            let vl = &visual_lines[cursor_visual];
            let line_content = self.buffer.line(vl.logical_line);
            let prefix_len =
                self.cursors.primary_position().1.saturating_sub(vl.start_col);
            let prefix_text: String = line_content
                .chars()
                .skip(vl.start_col)
                .take(prefix_len)
                .collect();
            let cursor_x = self.gutter_width()
                + 5.0
                + super::measure_text_width(
                    &prefix_text,
                    self.full_char_width,
                    self.char_width,
                )
                - self.horizontal_scroll_offset;

            // Calculate visual Y position relative to the viewport
            // We subtract viewport_scroll because the content is scrolled up/down
            // but the cursor position sent to IME must be relative to the visible area
            let cursor_y = (cursor_visual as f32 * self.line_height)
                - self.viewport_scroll;

            Rectangle::new(
                iced::Point::new(cursor_x, cursor_y + 2.0),
                Size::new(2.0, self.line_height - 4.0),
            )
        } else {
            Rectangle::new(iced::Point::new(0.0, 0.0), Size::new(0.0, 0.0))
        }
    }

    /// Creates the IME (Input Method Editor) layer widget.
    ///
    /// # Arguments
    ///
    /// * `cursor_rect` - The rectangle representing the cursor position
    ///
    /// # Returns
    ///
    /// An element containing the IME requester widget
    fn create_ime_layer(&self, cursor_rect: Rectangle) -> Element<'_, Message> {
        let ime_enabled = self.is_focused() && self.has_canvas_focus;

        let preedit =
            self.ime_preedit.as_ref().map(|p| input_method::Preedit {
                content: p.content.clone(),
                selection: p.selection.clone(),
                text_size: None,
            });

        let ime_layer = ImeRequester::new(ime_enabled, cursor_rect, preedit);
        iced::Element::new(ime_layer)
    }

    /// Creates the view element with scrollable wrapper.
    ///
    /// The backgrounds (editor and gutter) are handled by container styles
    /// to ensure proper clipping when the pane is resized.
    pub fn view(&self) -> Element<'_, Message> {
        // Calculate canvas height and visual lines
        let (visual_lines, canvas_height) = self.calculate_canvas_height();

        // Create scrollable containing the canvas
        let scrollable = self.create_canvas_with_scrollable(canvas_height);

        // Create background layer with gutter and code backgrounds
        let background_row = self.create_background_layer();

        // Build editor stack: backgrounds + scrollable
        let mut editor_stack =
            iced::widget::Stack::new().push(background_row).push(scrollable);

        // Add IME layer for input method support.
        // The IME requester needs the cursor rect in viewport coordinates, which
        // depends on the current logical↔visual mapping.
        let cursor_rect = self.calculate_ime_cursor_rect(visual_lines.as_ref());
        let ime_layer = self.create_ime_layer(cursor_rect);
        editor_stack = editor_stack.push(ime_layer);

        // Add search dialog overlay if open
        if self.search_state.is_open {
            let search_dialog =
                search_dialog::view(&self.search_state, &self.translations);

            // Position the dialog in top-right corner with 20px margin
            let positioned_dialog = container(
                Row::new()
                    .push(Space::new().width(Length::Fill))
                    .push(search_dialog),
            )
            .padding(20)
            .width(Length::Fill)
            .height(Length::Shrink);

            editor_stack = editor_stack.push(positioned_dialog);
        }

        // Wrap the editor stack in a container with clip
        let editor_container = container(editor_stack)
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true);

        // When wrap is disabled, add a horizontal scrollbar below the editor
        let max_content_width = self.max_content_width();
        if let Some(h_scrollbar) =
            self.create_horizontal_scrollbar(max_content_width)
        {
            Column::new().push(editor_container).push(h_scrollbar).into()
        } else {
            editor_container.into()
        }
    }
}
