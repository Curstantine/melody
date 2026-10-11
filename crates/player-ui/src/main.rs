mod app;
mod components;
pub mod config;
mod font;
mod icons;
mod message;
mod theme;
mod views;

use app::App;
use iced::window;

fn main() -> iced::Result {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let mut builder = iced::application(App::title, App::update, App::view)
        .theme(App::theme)
        .subscription(App::subscription)
        .window(window::Settings {
            size: iced::Size::new(1130.0, 740.0),
            min_size: Some(iced::Size::new(580.0, 520.0)),
            position: window::Position::Centered,
            ..Default::default()
        });

    let config = config::AppConfig::load();

    if let Some(ref font_name) = config.primary_font {
        tracing::info!("Setting default font from saved config to: {}", font_name);
        builder = builder.default_font(theme::font_from_family_name(Some(font_name)));
    }

    builder.run_with(App::new)
}
