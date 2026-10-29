use iced::window::Id;
use iced::Task;

use crate::message::Message;
use crate::theme::ThemeConfig;

/// Asynchronously applies the theme colors to the native titlebar for supported platforms.
pub fn sync_titlebar_theme(window_id: Option<Id>, theme: &ThemeConfig) -> Task<Message> {
    #[cfg(target_os = "windows")]
    {
        use iced::window::raw_window_handle::RawWindowHandle;

        // Match the topbar/navbar background color token (theme.surface)
        let bg = theme.surface_color();
        let text = theme.text_color();
        // Relative luminance to pick dark vs light caption button styling
        let luminance = 0.2126 * bg.r + 0.7152 * bg.g + 0.0722 * bg.b;
        let is_dark = luminance < 0.5;

        let apply_fn = move |id: Id| {
            iced::window::run(id, move |window| {
                if let Ok(handle) = window.window_handle() {
                    if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                        let hwnd = win32.hwnd.get() as windows_sys::Win32::Foundation::HWND;
                        apply_windows_titlebar_theme(hwnd, bg, text, is_dark);
                    }
                }
            })
            .discard()
        };

        if let Some(id) = window_id {
            apply_fn(id)
        } else {
            iced::window::oldest().and_then(apply_fn)
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window_id, theme);
        Task::none()
    }
}

#[cfg(target_os = "windows")]
fn apply_windows_titlebar_theme(
    hwnd: windows_sys::Win32::Foundation::HWND,
    bg: iced::Color,
    text: iced::Color,
    is_dark: bool,
) {
    use std::ffi::c_void;
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE,
    };

    // Convert iced::Color (0.0..=1.0) to Win32 COLORREF (0x00BBGGRR)
    let r = (bg.r.clamp(0.0, 1.0) * 255.0).round() as u32;
    let g = (bg.g.clamp(0.0, 1.0) * 255.0).round() as u32;
    let b = (bg.b.clamp(0.0, 1.0) * 255.0).round() as u32;
    let caption_color: u32 = (b << 16) | (g << 8) | r;

    let tr = (text.r.clamp(0.0, 1.0) * 255.0).round() as u32;
    let tg = (text.g.clamp(0.0, 1.0) * 255.0).round() as u32;
    let tb = (text.b.clamp(0.0, 1.0) * 255.0).round() as u32;
    let text_color: u32 = (tb << 16) | (tg << 8) | tr;

    let dark_mode_val: i32 = if is_dark { 1 } else { 0 };

    unsafe {
        // Windows 10 (1809+) & Windows 11: Dark mode for caption buttons and borders
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &dark_mode_val as *const _ as *const c_void,
            std::mem::size_of::<i32>() as u32,
        );

        // Windows 11 Build 22000+: Titlebar background color
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR as u32,
            &caption_color as *const _ as *const c_void,
            std::mem::size_of::<u32>() as u32,
        );

        // Windows 11 Build 22000+: Title text color
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TEXT_COLOR as u32,
            &text_color as *const _ as *const c_void,
            std::mem::size_of::<u32>() as u32,
        );
    }
}
