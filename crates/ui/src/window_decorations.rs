use iced::window::Id;
use iced::Task;

use crate::message::Message;
use theme::ThemeConfig;

/// Asynchronously applies the theme colors to the native titlebar for supported platforms.
pub fn sync_titlebar_theme(window_id: Option<Id>, theme: &ThemeConfig) -> Task<Message> {
    #[cfg(target_os = "windows")]
    {
        use iced::window::raw_window_handle::RawWindowHandle;

        // Match the topbar/navbar background color token (theme.surface) and border
        let bg = theme.surface;
        let text = theme.text;
        let border = theme.border;
        // Relative luminance to pick dark vs light caption button styling
        let luminance = 0.2126 * bg.r + 0.7152 * bg.g + 0.0722 * bg.b;
        let is_dark = luminance < 0.5;

        let apply_fn = move |id: Id| {
            iced::window::run(id, move |window| {
                if let Ok(handle) = window.window_handle() {
                    if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                        let hwnd = win32.hwnd.get() as windows_sys::Win32::Foundation::HWND;
                        apply_windows_titlebar_theme(hwnd, bg, text, border, is_dark);
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
fn color_to_colorref(c: iced::Color) -> u32 {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u32;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u32;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u32;
    (b << 16) | (g << 8) | r
}

#[cfg(target_os = "windows")]
fn apply_windows_titlebar_theme(
    hwnd: windows_sys::Win32::Foundation::HWND,
    bg: iced::Color,
    text: iced::Color,
    border: iced::Color,
    is_dark: bool,
) {
    use std::ffi::c_void;
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
    };

    let caption_color = color_to_colorref(bg);
    let text_color = color_to_colorref(text);
    let border_color = color_to_colorref(border);
    let dark_mode_val: i32 = if is_dark { 1 } else { 0 };

    unsafe {
        // Windows 10 (1809+) & Windows 11: Dark mode for window frame
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &dark_mode_val as *const _ as *const c_void,
            std::mem::size_of::<i32>() as u32,
        );

        // Windows 11 Build 22000+: Border color for undecorated DWM shadow window
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR as u32,
            &border_color as *const _ as *const c_void,
            std::mem::size_of::<u32>() as u32,
        );

        // Windows 11 Build 22000+: Caption & text color (for system menu/fallback)
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR as u32,
            &caption_color as *const _ as *const c_void,
            std::mem::size_of::<u32>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TEXT_COLOR as u32,
            &text_color as *const _ as *const c_void,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

/// Windows 11 Snap Layouts (`HTMAXBUTTON`) support for the custom borderless titlebar.
///
/// Why this exists:
/// - `winit` does not expose `WM_NCHITTEST` or custom-titlebar caption button hit-testing, so an
///   undecorated window defaults to `HTCLIENT` everywhere and Windows never knows where the
///   Maximize/Restore button is.
/// - Subclassing the `HWND` lets us return `HTMAXBUTTON` for the Maximize/Restore button rect,
///   handle its non-client clicks (`WM_NCLBUTTONDOWN`/`WM_NCLBUTTONUP` -> `SC_MAXIMIZE`/`SC_RESTORE`),
///   and track hover/pressed visual state via `WM_NCMOUSEMOVE`/`WM_NCMOUSELEAVE`.
/// - When unmaximized with `undecorated_shadow`, `winit` also shifts the client rect by `1px` in
///   `WM_NCCALCSIZE` (`top += 1; bottom += 1`) to preserve the DWM shadow, which causes Windows 11
///   to suppress the hover Snap Layouts flyout. Returning `0` on unmaximized `WM_NCCALCSIZE` and
///   using `DwmExtendFrameIntoClientArea` (`cyBottomHeight: 1`) keeps the shadow without shifting
///   the client geometry.
///
/// See: https://github.com/rust-windowing/winit/issues/3884
#[cfg(target_os = "windows")]
pub mod snap_layouts {
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

    use iced::widget::button;
    use iced::window::raw_window_handle::RawWindowHandle;
    use iced::window::Id;
    use iced::Task;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
    use windows_sys::Win32::Graphics::Gdi::{InvalidateRect, ScreenToClient};
    use windows_sys::Win32::UI::Controls::MARGINS;
    use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        TrackMouseEvent, TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT,
    };
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetWindowLongPtrW, IsZoomed, PostMessageW, SetWindowLongPtrW, SetWindowPos,
        GWL_STYLE, HTMAXBUTTON, SC_MAXIMIZE, SC_RESTORE, SWP_FRAMECHANGED, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WM_CAPTURECHANGED, WM_NCCALCSIZE, WM_NCDESTROY,
        WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE,
        WM_SYSCOMMAND, WS_MAXIMIZEBOX,
    };

    use crate::message::Message;

    const SNAP_SUBCLASS_ID: usize = 0x4D45_4C4F; // "MELO"
    const WIN_BTN_WIDTH_LOGICAL: f32 = 46.0;
    const RESIZE_EDGE_TOP_LOGICAL: f32 = 5.0;

    const STATE_NORMAL: u8 = 0;
    const STATE_HOVERED: u8 = 1;
    const STATE_PRESSED: u8 = 2;

    static SUBCLASS_INSTALLED: AtomicBool = AtomicBool::new(false);
    static CUSTOM_TITLEBAR_ENABLED: AtomicBool = AtomicBool::new(false);
    static TITLEBAR_HEIGHT_X10: AtomicU32 = AtomicU32::new(300);
    static MAX_BTN_STATE: AtomicU8 = AtomicU8::new(STATE_NORMAL);
    static PRESSED_MAX: AtomicBool = AtomicBool::new(false);

    pub fn sync_state(custom_titlebar: bool, titlebar_height: f32) {
        CUSTOM_TITLEBAR_ENABLED.store(custom_titlebar, Ordering::Relaxed);
        TITLEBAR_HEIGHT_X10.store(
            (titlebar_height.max(1.0) * 10.0).round() as u32,
            Ordering::Relaxed,
        );
        if !custom_titlebar {
            PRESSED_MAX.store(false, Ordering::Relaxed);
            MAX_BTN_STATE.store(STATE_NORMAL, Ordering::Relaxed);
        }
    }

    pub fn maximize_button_status() -> Option<button::Status> {
        if !CUSTOM_TITLEBAR_ENABLED.load(Ordering::Relaxed) {
            return None;
        }
        match MAX_BTN_STATE.load(Ordering::Relaxed) {
            STATE_HOVERED => Some(button::Status::Hovered),
            STATE_PRESSED => Some(button::Status::Pressed),
            _ => None,
        }
    }

    pub fn install_hook(
        window_id: Id,
        custom_titlebar: bool,
        titlebar_height: f32,
    ) -> Task<Message> {
        sync_state(custom_titlebar, titlebar_height);

        iced::window::run(window_id, move |window| {
            if let Ok(handle) = window.window_handle() {
                if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                    let hwnd = win32.hwnd.get() as HWND;
                    unsafe {
                        // DWM requires WS_MAXIMIZEBOX on the window to show Snap Layouts on HTMAXBUTTON.
                        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                        if (style & WS_MAXIMIZEBOX as isize) == 0 {
                            SetWindowLongPtrW(hwnd, GWL_STYLE, style | WS_MAXIMIZEBOX as isize);
                            SetWindowPos(
                                hwnd,
                                std::ptr::null_mut(),
                                0,
                                0,
                                0,
                                0,
                                SWP_FRAMECHANGED
                                    | SWP_NOMOVE
                                    | SWP_NOSIZE
                                    | SWP_NOZORDER
                                    | SWP_NOACTIVATE,
                            );
                        }

                        if !SUBCLASS_INSTALLED.swap(true, Ordering::AcqRel) {
                            if SetWindowSubclass(
                                hwnd,
                                Some(snap_layouts_subclass_proc),
                                SNAP_SUBCLASS_ID,
                                0,
                            ) == 0
                            {
                                SUBCLASS_INSTALLED.store(false, Ordering::Release);
                            } else if custom_titlebar {
                                // Keep DWM drop shadow without winit's 1px WM_NCCALCSIZE top/bottom shift.
                                let margins = MARGINS {
                                    cxLeftWidth: 0,
                                    cxRightWidth: 0,
                                    cyTopHeight: 0,
                                    cyBottomHeight: 1,
                                };
                                let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
                                SetWindowPos(
                                    hwnd,
                                    std::ptr::null_mut(),
                                    0,
                                    0,
                                    0,
                                    0,
                                    SWP_FRAMECHANGED
                                        | SWP_NOMOVE
                                        | SWP_NOSIZE
                                        | SWP_NOZORDER
                                        | SWP_NOACTIVATE,
                                );
                            }
                        }
                    }
                }
            }
        })
        .discard()
    }

    fn set_visual_state(hwnd: HWND, new_state: u8) {
        if MAX_BTN_STATE.swap(new_state, Ordering::Relaxed) != new_state {
            unsafe {
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
        }
    }

    fn is_over_maximize_button(hwnd: HWND, l_param: LPARAM) -> bool {
        let x = (l_param as i32 & 0xFFFF) as i16 as i32;
        let y = ((l_param as i32 >> 16) & 0xFFFF) as i16 as i32;
        let mut pt = POINT { x, y };
        if unsafe { ScreenToClient(hwnd, &mut pt) } == 0 {
            return false;
        }

        let mut rect = RECT::default();
        if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
            return false;
        }

        let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
        let scale = dpi as f32 / 96.0;
        let btn_width = WIN_BTN_WIDTH_LOGICAL * scale;
        let titlebar_h = (TITLEBAR_HEIGHT_X10.load(Ordering::Relaxed) as f32 / 10.0) * scale;
        let is_zoomed = unsafe { IsZoomed(hwnd) } != 0;
        let top_inset = if is_zoomed {
            0.0
        } else {
            RESIZE_EDGE_TOP_LOGICAL * scale
        };

        let max_right = rect.right as f32 - btn_width;
        let max_left = rect.right as f32 - 2.0 * btn_width;

        (pt.x as f32) >= max_left
            && (pt.x as f32) < max_right
            && (pt.y as f32) >= top_inset
            && (pt.y as f32) < titlebar_h
    }

    unsafe extern "system" fn snap_layouts_subclass_proc(
        hwnd: HWND,
        msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
        u_id_subclass: usize,
        _dw_ref_data: usize,
    ) -> LRESULT {
        if msg == WM_NCDESTROY {
            SUBCLASS_INSTALLED.store(false, Ordering::Release);
            PRESSED_MAX.store(false, Ordering::Relaxed);
            MAX_BTN_STATE.store(STATE_NORMAL, Ordering::Relaxed);
            RemoveWindowSubclass(hwnd, Some(snap_layouts_subclass_proc), u_id_subclass);
            return DefSubclassProc(hwnd, msg, w_param, l_param);
        }

        if !CUSTOM_TITLEBAR_ENABLED.load(Ordering::Relaxed) {
            return DefSubclassProc(hwnd, msg, w_param, l_param);
        }

        match msg {
            WM_NCCALCSIZE => {
                if w_param != 0 && IsZoomed(hwnd) == 0 {
                    // Bypass winit's `top += 1; bottom += 1` undecorated_shadow shift so
                    // the unmaximized client rect matches the full window rect cleanly.
                    return 0;
                }
            }
            WM_NCHITTEST => {
                if is_over_maximize_button(hwnd, l_param) {
                    return HTMAXBUTTON as LRESULT;
                }
            }
            WM_NCMOUSEMOVE => {
                if w_param as u32 == HTMAXBUTTON {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_NONCLIENT | TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    TrackMouseEvent(&mut tme);
                    let state = if PRESSED_MAX.load(Ordering::Relaxed) {
                        STATE_PRESSED
                    } else {
                        STATE_HOVERED
                    };
                    set_visual_state(hwnd, state);
                } else {
                    set_visual_state(hwnd, STATE_NORMAL);
                }
            }
            WM_NCMOUSELEAVE => {
                PRESSED_MAX.store(false, Ordering::Relaxed);
                set_visual_state(hwnd, STATE_NORMAL);
            }
            WM_NCLBUTTONDOWN => {
                if w_param as u32 == HTMAXBUTTON {
                    PRESSED_MAX.store(true, Ordering::Relaxed);
                    set_visual_state(hwnd, STATE_PRESSED);
                    return 0;
                }
            }
            WM_NCLBUTTONUP => {
                if w_param as u32 == HTMAXBUTTON {
                    let was_pressed = PRESSED_MAX.swap(false, Ordering::Relaxed);
                    set_visual_state(hwnd, STATE_HOVERED);
                    if was_pressed {
                        let cmd = if IsZoomed(hwnd) != 0 {
                            SC_RESTORE
                        } else {
                            SC_MAXIMIZE
                        };
                        PostMessageW(hwnd, WM_SYSCOMMAND, cmd as WPARAM, 0);
                    }
                    return 0;
                } else if PRESSED_MAX.swap(false, Ordering::Relaxed) {
                    set_visual_state(hwnd, STATE_NORMAL);
                }
            }
            WM_CAPTURECHANGED => {
                PRESSED_MAX.store(false, Ordering::Relaxed);
                set_visual_state(hwnd, STATE_NORMAL);
            }
            _ => {}
        }

        DefSubclassProc(hwnd, msg, w_param, l_param)
    }
}
