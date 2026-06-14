
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

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
