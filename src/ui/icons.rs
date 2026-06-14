
use iced::{
    widget::{container, text, Space, Text},
    Border, Color, Element,
};

/// The embedded icon font. The family name `lucide` is the one baked into the TTF.
pub const ICON_FONT: iced::Font = iced::Font::with_name("lucide");

/// Default on-screen size for an icon glyph.
const DEFAULT_SIZE: f32 = 16.0;

fn glyph(c: char) -> Text<'static> {
    text(c).font(ICON_FONT).size(DEFAULT_SIZE)
}

// ── Sidebar rail ────────────────────────────────────────────────────────────────
pub fn collections() -> Text<'static> { glyph('\u{e0d7}') } // folder
pub fn history() -> Text<'static> { glyph('\u{e1f5}') } // history
pub fn environments() -> Text<'static> { glyph('\u{e529}') } // layers
pub fn settings() -> Text<'static> { glyph('\u{e154}') } // settings

// ── Actions ─────────────────────────────────────────────────────────────────────
pub fn close() -> Text<'static> { glyph('\u{e1b2}') } // x
pub fn edit() -> Text<'static> { glyph('\u{e1f9}') } // pencil
pub fn copy() -> Text<'static> { glyph('\u{e09e}') } // copy
pub fn import() -> Text<'static> { glyph('\u{e0b2}') } // download
pub fn export() -> Text<'static> { glyph('\u{e19e}') } // upload
pub fn search() -> Text<'static> { glyph('\u{e151}') } // search
pub fn plus() -> Text<'static> { glyph('\u{e13d}') } // plus
// pub fn curl() -> Text<'static> { glyph('\u{e1e5}') } // terminal
pub fn curl() -> Text<'static> { glyph('\u{e245}') }

// ── Tree / disclosure ─────────────────────────────────────────────────────────────
pub fn chevron_down() -> Text<'static> { glyph('\u{e06d}') }
pub fn chevron_right() -> Text<'static> { glyph('\u{e06f}') }

// ── Direction / VCS / keys ────────────────────────────────────────────────────────
pub fn arrow_right() -> Text<'static> { glyph('\u{e049}') }
pub fn arrow_left() -> Text<'static> { glyph('\u{e048}') }
pub fn git_branch() -> Text<'static> { glyph('\u{e0e2}') } // git-branch
pub fn command() -> Text<'static> { glyph('\u{e09a}') } // command (⌘)


pub fn dot<'a, M: 'a>(color: Color) -> Element<'a, M> {
    container(Space::new())
        .width(8)
        .height(8)
        .style(move |_| container::Style {
            background: Some(color.into()),
            border: Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}
