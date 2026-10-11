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

    // Discover and load CJK fonts to ensure Chinese, Japanese, and Korean
    // characters are rendered properly without corrupted glyphs.
    let font_bundle = font::load_cjk_fonts();

    let mut builder = iced::application(App::title, App::update, App::view)
        .theme(App::theme)
        .subscription(App::subscription)
        .window(window::Settings {
            size: iced::Size::new(1100.0, 740.0),
            min_size: Some(iced::Size::new(820.0, 520.0)),
            position: window::Position::Centered,
            ..Default::default()
        });

    let config = config::AppConfig::load();

    if let Some(ref font_name) = config.primary_font {
        tracing::info!("Setting default font from saved config to: {}", font_name);
        builder = builder.default_font(theme::font_from_family_name(Some(font_name)));
    } else if let Some(family) = &font_bundle.primary_family {
        tracing::info!("Setting default font to: {}", family);
        builder = builder.default_font(theme::font_from_family_name(Some(family)));
    }

    for font_bytes in font_bundle.font_data {
        builder = builder.font(font_bytes);
    }

    builder.run_with(App::new)
}
