#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app::App;
use config::{AppConfig, AppState};
use iced::{
    window::{self, Position},
    Size,
};
use tracing::info;

mod app;
mod layout_view;
mod message;
mod tray;
mod views;
mod window_decorations;

#[cfg(feature = "tracy")]
#[global_allocator]
static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
    tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

fn main() -> iced::Result {
    updater::init();

    paths::init_app_start_time();

    // Initialize logging / tracing
    #[cfg(feature = "tracy")]
    {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        let _ = tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_tracy::TracyLayer::default())
            .try_init();

        info!(
            "Tracy profiler integration initialized (with ProfiledAllocator for memory tracking)"
        );
    }

    #[cfg(not(feature = "tracy"))]
    {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .try_init();
    }

    let state = AppState::load();
    let config = AppConfig::load();
    let app_config = config.clone();
    let app_state = state.clone();

    let mut builder = iced::daemon(
        move || App::new_with(app_config.clone(), app_state.clone()),
        App::update,
        App::view,
    )
    .title(App::title)
    .theme(App::theme)
    .subscription(App::subscription);

    if let Some(ref font_name) = config.fonts.primary {
        info!("Setting default font from saved config to: {font_name}");
        builder = builder.default_font(fonts::font_from_family_name(Some(font_name)));
    }

    builder.run()
}

pub fn build_window_settings(config: &AppConfig, width: f32, height: f32) -> window::Settings {
    let app_icon =
        iced::window::icon::from_file_data(include_bytes!("../../../assets/melody.png"), None).ok();

    let mut window_settings = window::Settings {
        icon: app_icon,
        size: Size::new(width, height),
        min_size: Some(Size::new(
            config::WINDOW_MIN_WIDTH,
            config::WINDOW_MIN_HEIGHT,
        )),
        position: Position::Centered,
        decorations: !config.custom_titlebar,
        exit_on_close_request: false,
        ..Default::default()
    };

    #[cfg(target_os = "windows")]
    {
        use iced::window::settings::platform::CornerPreference;
        window_settings.platform_specific.undecorated_shadow = true;
        window_settings.platform_specific.corner_preference = CornerPreference::Round;
    }

    #[cfg(target_os = "linux")]
    {
        window_settings.platform_specific.application_id = paths::app_name().to_string();
    }

    #[cfg(target_os = "macos")]
    {
        if !config.custom_titlebar {
            window_settings.platform_specific.titlebar_transparent = true;
            window_settings.platform_specific.fullsize_content_view = true;
        }
    }

    window_settings
}
