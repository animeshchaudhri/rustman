use iced::{
    widget::{column, container, row, scrollable, text},
    Background, Border, Color, Element, Length,
};

use crate::{message::Message, state::tabs::RequestTabState, ui::theme::Palette};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let mut col = column![].spacing(0);

    if let Some(resp) = &tab.response {
        let cookies: Vec<(&str, &str)> = resp
            .headers
            .iter()
            .filter(|(k, _)| k.to_ascii_lowercase() == "set-cookie")
            .flat_map(|(_, v)| {
                // The first attribute is "name=value"; extras are ";Path=..." etc.
                let name_val = v.split(';').next().unwrap_or(v.trim());
                let (name, value) = if let Some(eq) = name_val.find('=') {
                    (&name_val[..eq], &name_val[eq + 1..])
                } else {
                    (name_val, "")
                };
                Some((name.trim(), value.trim()))
            })
            .collect();

        if cookies.is_empty() {
            col = col.push(
                container(
                    text("No Set-Cookie headers in this response.")
                        .size(12)
                        .color(Palette::text_muted()),
                )
                .padding([16, 12]),
            );
        } else {
            let header_bg = Palette::surface_high();
            col = col.push(
                container(
                    row![
                        container(
                            text("#").size(10).color(Palette::text_subtle()).font(crate::ui::theme::MONO),
                        )
                        .width(32)
                        .padding([3, 4]),
                        container(
                            text("Name").size(10).color(Palette::text_subtle()).font(crate::ui::theme::MONO),
                        )
                        .width(180)
                        .padding([3, 4]),
                        container(
                            text("Value").size(10).color(Palette::text_subtle()).font(crate::ui::theme::MONO),
                        )
                        .width(Length::Fill)
                        .padding([3, 4]),
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .style(move |_| iced::widget::container::Style {
                    background: Some(Background::Color(header_bg)),
                    border: Border {
                        color: Palette::border_subtle(),
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                })
                .padding([0, 4])
                .width(Length::Fill),
            );

            for (i, (name, value)) in cookies.iter().enumerate() {
                let bg = if i % 2 == 0 {
                    Some(Background::Color(Color { r: 0.075, g: 0.075, b: 0.085, a: 1.0 }))
                } else {
                    None
                };
                let name = name.to_string();
                let value = value.to_string();

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
                                text(name)
                                    .size(11)
                                    .color(Palette::accent())
                                    .font(crate::ui::theme::MONO),
                            )
                            .width(180)
                            .padding([2, 4]),
                            container(
                                text(value)
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
        }
    } else {
        col = col.push(
            container(text("No response yet.").size(13).color(Palette::text_muted()))
                .padding([12, 8]),
        );
    }

    scrollable(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(crate::ui::theme::thin_scrollbar)
        .into()
}
