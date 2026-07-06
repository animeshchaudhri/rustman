use iced::{
    widget::{column, container, row, scrollable, text},
    Background, Border, Element, Length,
};

use crate::{message::Message, state::tabs::RequestTabState, ui::{theme::Palette, widgets::kv_table}};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let Some(resp) = &tab.response else {
        return kv_table::empty_state("No response yet.");
    };

    let mut entries: Vec<_> = resp.headers.iter().collect();
    entries.sort_by_key(|(k, _)| k.as_str());

    if entries.is_empty() {
        return kv_table::empty_state("No response headers.");
    }

    let mut col = column![].spacing(0);
    for (i, (k, v)) in entries.into_iter().enumerate() {
        let bg = if i % 2 == 0 {
            Some(Background::Color(Palette::row_odd()))
        } else {
            None
        };
        let key = k.clone();
        let val = v.clone();
        col = col.push(
            container(
                row![
                    container(
                        text(format!("{}", i + 1))
                            .size(10)
                            .color(Palette::text_subtle())
                            .font(crate::ui::theme::MONO),
                    )
                    .width(32)
                    .padding([2, 4]),
                    container(
                        text(key)
                            .size(11)
                            .color(Palette::accent())
                            .font(crate::ui::theme::MONO),
                    )
                    .width(200)
                    .padding([2, 4]),
                    container(
                        text(val)
                            .size(11)
                            .color(Palette::text())
                            .font(crate::ui::theme::MONO),
                    )
                    .width(Length::Fill)
                    .padding([2, 4]),
                ]
                .align_y(iced::Alignment::Center)
                .width(Length::Fill),
            )
            .style(move |_| iced::widget::container::Style {
                background: bg,
                border: Border {
                    color: Palette::border_subtle(),
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .padding([1, 4])
            .width(Length::Fill),
        );
    }

    scrollable(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(crate::ui::theme::thin_scrollbar)
        .into()
}
