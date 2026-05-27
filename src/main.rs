#![allow(dead_code)]

mod app;
mod domain;
mod message;
mod services;
mod state;
mod ui;

fn main() -> iced::Result {
    app::run()
}
