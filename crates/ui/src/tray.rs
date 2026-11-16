use std::sync::Arc;

use tracing::{error, info};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ShowWindow,
    PlayPause,
    Next,
    Previous,
    Quit,
}

const MENU_ID_SHOW: &str = "melody.tray.show";
const MENU_ID_PLAY_PAUSE: &str = "melody.tray.play_pause";
const MENU_ID_NEXT: &str = "melody.tray.next";
const MENU_ID_PREV: &str = "melody.tray.prev";
const MENU_ID_QUIT: &str = "melody.tray.quit";

pub struct TrayController {
    _tray_icon: TrayIcon,
}

impl TrayController {
    pub fn new() -> Option<Self> {
        let icon = load_tray_icon()?;

        let menu = Menu::new();
        let show_item = MenuItem::with_id(MENU_ID_SHOW, "Show Melody", true, None);
        let play_pause_item = MenuItem::with_id(MENU_ID_PLAY_PAUSE, "Play / Pause", true, None);
        let next_item = MenuItem::with_id(MENU_ID_NEXT, "Next Track", true, None);
        let prev_item = MenuItem::with_id(MENU_ID_PREV, "Previous Track", true, None);
        let quit_item = MenuItem::with_id(MENU_ID_QUIT, "Quit", true, None);

        if let Err(e) = menu.append_items(&[
            &show_item,
            &PredefinedMenuItem::separator(),
            &play_pause_item,
            &next_item,
            &prev_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ]) {
            error!("Failed to build tray menu: {e}");
            return None;
        }

        match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Melody")
            .with_icon(icon)
            .build()
        {
            Ok(tray_icon) => {
                info!("System tray icon initialized");
                Some(Self {
                    _tray_icon: tray_icon,
                })
            }
            Err(e) => {
                error!("Failed to create system tray icon: {e}");
                None
            }
        }
    }

    /// Drains any pending tray icon or context-menu events.
    pub fn poll_actions() -> Vec<TrayAction> {
        let mut actions = Vec::new();

        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
                | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => {
                    actions.push(TrayAction::ShowWindow);
                }
                _ => {}
            }
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some(action) = map_menu_id(&event.id) {
                actions.push(action);
            }
        }

        actions
    }
}

fn map_menu_id(id: &MenuId) -> Option<TrayAction> {
    match id.0.as_str() {
        MENU_ID_SHOW => Some(TrayAction::ShowWindow),
        MENU_ID_PLAY_PAUSE => Some(TrayAction::PlayPause),
        MENU_ID_NEXT => Some(TrayAction::Next),
        MENU_ID_PREV => Some(TrayAction::Previous),
        MENU_ID_QUIT => Some(TrayAction::Quit),
        _ => None,
    }
}

fn load_tray_icon() -> Option<tray_icon::Icon> {
    let bytes: Arc<[u8]> = Arc::from(include_bytes!("../../../assets/melody.png").as_slice());
    let img = match image::load_from_memory(&bytes) {
        Ok(img) => img.into_rgba8(),
        Err(e) => {
            error!("Failed to decode tray icon PNG: {e}");
            return None;
        }
    };
    let (width, height) = img.dimensions();
    match tray_icon::Icon::from_rgba(img.into_raw(), width, height) {
        Ok(icon) => Some(icon),
        Err(e) => {
            error!("Failed to create tray_icon::Icon: {e}");
            None
        }
    }
}
