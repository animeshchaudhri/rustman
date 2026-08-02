//! Tab-key focus navigation between focusable inputs — e.g. a header/param
//! row's Key field to its Value field. This iced version has no built-in
//! Tab-to-next-input handling for `text_input` (unlike the vendored code
//! editor, which handles its own Tab for indentation), so it's built here
//! from the low-level `Operation` primitives iced does expose.

pub fn focus_next<Message: Send + 'static>() -> iced::Task<Message> {
    iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::focus_next::<()>())
        .discard()
}

pub fn focus_previous<Message: Send + 'static>() -> iced::Task<Message> {
    iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::focus_previous::<()>())
        .discard()
}
