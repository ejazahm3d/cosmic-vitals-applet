use app::YourApp;
mod app;
mod config;
mod core;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<YourApp>(true, ())
}
