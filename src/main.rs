
mod app;
mod domain;
mod jobs;
mod message;
mod services;
mod state;
mod ui;

fn main() -> iced::Result {
    app::run()
}
