//! A tiny `Operation` that finds the on-screen bounds of a widget by `Id`.
//!
//! Used to keep an embedded native webview (see `services::webview`)
//! positioned exactly over the response body panel, whatever the current
//! window size or panel split happens to be — instead of recomputing the
//! layout math by hand, which would drift out of sync the moment the layout
//! changes.

use iced::advanced::widget::{operation::Outcome, Id, Operation};
use iced::Rectangle;

pub struct FindBounds {
    target: Id,
    found: Option<Rectangle>,
}

impl FindBounds {
    pub fn new(target: Id) -> Self {
        Self { target, found: None }
    }
}

impl Operation<Rectangle> for FindBounds {
    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.target) {
            self.found = Some(bounds);
        }
    }

    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Rectangle>)) {
        operate(self);
    }

    fn finish(&self) -> Outcome<Rectangle> {
        match self.found {
            Some(bounds) => Outcome::Some(bounds),
            None => Outcome::None,
        }
    }
}

/// Returns a `Task` that resolves to the screen-space bounds of the
/// container with the given `Id`. Never resolves if no such container is
/// currently in the widget tree (e.g. the response panel isn't visible).
pub fn find(target: Id) -> iced::Task<Rectangle> {
    iced::advanced::widget::operate(FindBounds::new(target))
}
