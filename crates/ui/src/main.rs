#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod components;
pub mod config;
mod font;
mod icons;
pub mod layout;
#[path = "layout/view.rs"]
mod layout_view;
mod message;
mod queue;
pub mod state;
mod theme;
mod views;

#[cfg(feature = "tracy")]
#[global_allocator]
static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
    tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

use app::App;
use iced::window;
use tracing::info;

fn main() -> iced::Result {
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

    let state = state::AppState::load();
    let (width, height) = state.window_size().unwrap_or((1130.0, 740.0));
    let width = width.max(1000.0);
    let height = height.max(520.0);

    let app_icon =
        iced::window::icon::from_file_data(include_bytes!("../../../assets/melody.png"), None).ok();

    let mut builder = iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .window(window::Settings {
            icon: app_icon,
            size: iced::Size::new(width, height),
            min_size: Some(iced::Size::new(1000.0, 520.0)),
            position: window::Position::Centered,
            ..Default::default()
        });

    let config = config::AppConfig::load();

    if let Some(ref font_name) = config.primary_font {
        info!("Setting default font from saved config to: {font_name}");
        builder = builder.font(theme::font_from_family_name(Some(font_name)));
    }

    builder.run()
}
