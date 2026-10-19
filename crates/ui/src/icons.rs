use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use iced::widget::svg::Handle;
use iced::widget::Svg;
use iced::{Color, Length, Theme};

// Icons sourced from Boxicons (https://boxicons.com / https://icon-sets.iconify.design/boxicons/),
// MIT licensed (https://github.com/box-icons/boxicons-core/blob/main/LICENSE).
// Each constant notes its Boxicons name below. All glyphs use `fill="currentColor"`
// so they are tinted at render time via [`icon`].

/// Boxicons `play-filled`.
pub const PLAY_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M6.51 18.87c.15.09.32.13.49.13s.36-.05.51-.14l10-6c.3-.18.49-.51.49-.86s-.18-.68-.49-.86l-10-6a.99.99 0 0 0-1.01-.01c-.31.18-.51.51-.51.87v12c0 .36.19.69.51.87Z"/></svg>"#;

/// Boxicons `play-circle`.
pub const PLAY_CIRCLE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12 2C6.486 2 2 6.486 2 12s4.486 10 10 10 10-4.486 10-10S17.514 2 12 2zm0 18c-4.411 0-8-3.589-8-8s3.589-8 8-8 8 3.589 8 8-3.589 8-8 8z"/><path fill="currentColor" d="M10 16.5l6-4.5-6-4.5z"/></svg>"#;

/// Boxicons `pause-filled`.
pub const PAUSE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect width="5" height="12" x="6" y="6" fill="currentColor" rx="1" ry="1"/><rect width="5" height="12" x="13" y="6" fill="currentColor" rx="1" ry="1"/></svg>"#;

/// Boxicons `skip-previous-filled`.
pub const PREV_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M17.46 6.11a1 1 0 0 0-1.04.08l-7 5c-.26.19-.42.49-.42.81s.16.63.42.81l7 5c.17.12.38.19.58.19c.16 0 .31-.04.46-.11c.33-.17.54-.51.54-.89V7c0-.37-.21-.72-.54-.89M6 6h2v12H6z"/></svg>"#;

/// Boxicons `skip-next-filled`.
pub const NEXT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="m14.58 11.19l-7-5c-.31-.22-.71-.25-1.04-.08S6 6.62 6 7v10c0 .37.21.72.54.89c.14.07.3.11.46.11c.21 0 .41-.06.58-.19l7-5c.26-.19.42-.49.42-.81s-.16-.63-.42-.81M16 6h2v12h-2z"/></svg>"#;

/// Boxicons `stop-filled`.
pub const STOP_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect width="12" height="12" x="6" y="6" fill="currentColor" rx="1" ry="1"/></svg>"#;

/// Boxicons `volume-mute`.
pub const VOL_MUTE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M22 12c0-4.09-2.47-7.61-6-9.16v2.24c2.39 1.39 4 3.96 4 6.92c0 1.85-.64 3.54-1.69 4.89l-1.42-1.42c.7-.98 1.12-2.17 1.12-3.47c0-1.77-.78-3.36-2-4.46v7.05l-2-2V4c0-.37-.2-.71-.53-.88c-.32-.17-.72-.15-1.03.05L7.73 6.32L2.71 1.29L1.3 2.7l20 20l1.41-1.41l-2.98-2.98A9.94 9.94 0 0 0 22 11.99Zm-10-1.41L9.17 7.76L12 5.87zm0 7.54l-4.45-2.96s-.06-.02-.08-.03c-.06-.03-.12-.06-.19-.08s-.13-.03-.2-.04c-.03 0-.06-.02-.09-.02h-3V9h.76L3.02 7.27C2.41 7.61 2 8.26 2 9v6c0 1.1.9 2 2 2h2.7l5.75 3.83c.17.11.36.17.55.17c.16 0 .32-.04.47-.12c.33-.17.53-.51.53-.88v-1.76l-2-2z"/></svg>"#;

/// Boxicons `volume-low`.
pub const VOL_LOW_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M4 17h2.7l5.75 3.83c.17.11.36.17.55.17c.16 0 .32-.04.47-.12c.33-.17.53-.51.53-.88V4c0-.37-.2-.71-.53-.88c-.32-.17-.72-.16-1.03.05L6.69 7h-2.7c-1.1 0-2 .9-2 2v6c0 1.1.9 2 2 2Zm0-8h3s.06-.02.09-.02c.07 0 .13-.02.19-.04a.6.6 0 0 0 .19-.08c.03-.01.06-.02.08-.03L12 5.87v12.26l-4.45-2.96s-.06-.02-.08-.03c-.06-.03-.12-.06-.19-.08s-.13-.03-.2-.04c-.03 0-.06-.02-.09-.02h-3V9Zm14 3c0-1.77-.78-3.36-2-4.46v8.92c1.22-1.1 2-2.69 2-4.46"/></svg>"#;

/// Boxicons `volume-full`.
pub const VOL_HIGH_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M4 17h2.7l5.75 3.83c.17.11.36.17.55.17c.16 0 .32-.04.47-.12c.33-.17.53-.51.53-.88V4c0-.37-.2-.71-.53-.88c-.32-.17-.72-.15-1.03.05L6.69 7h-2.7c-1.1 0-2 .9-2 2v6c0 1.1.9 2 2 2Zm0-8h3s.06-.02.09-.02c.06 0 .13-.02.19-.04a.6.6 0 0 0 .19-.08c.03-.01.06-.02.08-.03L12 5.87v12.26l-4.45-2.96s-.06-.02-.08-.03c-.06-.03-.12-.06-.19-.08c-.06-.02-.13-.03-.19-.04c-.03 0-.06-.02-.09-.02H4zm18 3c0-4.09-2.47-7.61-6-9.16v2.24c2.39 1.39 4 3.96 4 6.92s-1.61 5.53-4 6.92v2.24c3.53-1.55 6-5.07 6-9.16"/><path fill="currentColor" d="M18 12c0-1.77-.78-3.36-2-4.46v8.92c1.22-1.1 2-2.69 2-4.46"/></svg>"#;

/// Boxicons `album-covers`.
pub const ALBUM_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="m2.8 19.98l10 2c.07.01.13.02.2.02c.23 0 .45-.08.63-.23c.23-.19.37-.47.37-.77V3c0-.3-.13-.58-.37-.77s-.54-.26-.83-.21l-10 2c-.47.09-.8.5-.8.98v14c0 .48.34.89.8.98M4 5.82l8-1.6v15.56l-8-1.6zM16 2h2v20h-2zm4 0h2v20h-2z"/></svg>"#;

/// Boxicons `music`.
pub const MUSIC_NOTE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M21 3H9c-.55 0-1 .45-1 1v9.56c-.59-.34-1.27-.56-2-.56c-2.21 0-4 1.79-4 4s1.79 4 4 4s4-1.79 4-4V5h10v8.56c-.59-.34-1.27-.56-2-.56c-2.21 0-4 1.79-4 4s1.79 4 4 4s4-1.79 4-4V4c0-.55-.45-1-1-1M6 19c-1.1 0-2-.9-2-2s.9-2 2-2s2 .9 2 2s-.9 2-2 2m12 0c-1.1 0-2-.9-2-2s.9-2 2-2s2 .9 2 2s-.9 2-2 2"/></svg>"#;

/// Boxicons `list-play`.
pub const PLAYLIST_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M4 11h10v2H4zm0-5h16v2H4zm0 10h10v2H4zm12 4l6-4l-6-4z"/></svg>"#;

/// Boxicons `queue`.
pub const QUEUE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M3 14h18v2H3zm0 4h18v2H3zm0-8h18v2H3z"/><rect width="18" height="4" x="3" y="4" fill="currentColor" rx="1" ry="1"/></svg>"#;

/// Boxicons `shuffle`.
pub const SHUFFLE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M18 6h-2.59c-.53 0-1.04.21-1.41.59L4.59 16H2v2h2.59c.53 0 1.04-.21 1.41-.59L15.41 8H18v3l4-4l-4-4z"/><path fill="currentColor" d="M18 16h-1.59l-2.71-2.71l-1.41 1.41L15 17.41c.37.37.89.59 1.41.59H18v3l4-4l-4-4zM7.29 9.71L8.7 8.3L6.99 6.59C6.62 6.22 6.1 6 5.58 6H1.99v2h3.59z"/></svg>"#;

/// Boxicons `repeat`.
pub const REPEAT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M17 5H6c-1.1 0-2 .9-2 2v5h2V7h11v3l5-4l-5-4zm1 12H7v-3l-5 4l5 4v-3h11c1.1 0 2-.9 2-2v-5h-2z"/></svg>"#;

/// Boxicons `search`.
pub const SEARCH_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M18 10c0-4.41-3.59-8-8-8s-8 3.59-8 8s3.59 8 8 8c1.85 0 3.54-.63 4.9-1.69l5.1 5.1L21.41 20l-5.1-5.1A8 8 0 0 0 18 10M4 10c0-3.31 2.69-6 6-6s6 2.69 6 6s-2.69 6-6 6s-6-2.69-6-6"/></svg>"#;

/// Boxicons `volume-full` (now-playing / sound-on indicator).
pub const SPEAKER_PLAYING_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M4 17h2.7l5.75 3.83c.17.11.36.17.55.17c.16 0 .32-.04.47-.12c.33-.17.53-.51.53-.88V4c0-.37-.2-.71-.53-.88c-.32-.17-.72-.15-1.03.05L6.69 7h-2.7c-1.1 0-2 .9-2 2v6c0 1.1.9 2 2 2Zm0-8h3s.06-.02.09-.02c.06 0 .13-.02.19-.04a.6.6 0 0 0 .19-.08c.03-.01.06-.02.08-.03L12 5.87v12.26l-4.45-2.96s-.06-.02-.08-.03c-.06-.03-.12-.06-.19-.08c-.06-.02-.13-.03-.19-.04c-.03 0-.06-.02-.09-.02H4zm18 3c0-4.09-2.47-7.61-6-9.16v2.24c2.39 1.39 4 3.96 4 6.92s-1.61 5.53-4 6.92v2.24c3.53-1.55 6-5.07 6-9.16"/><path fill="currentColor" d="M18 12c0-1.77-.78-3.36-2-4.46v8.92c1.22-1.1 2-2.69 2-4.46"/></svg>"#;

/// Boxicons `chevron-up`.
pub const CHEVRON_UP_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="m7.71 15.71l4.29-4.3l4.29 4.3l1.42-1.42L12 8.59l-5.71 5.7z"/></svg>"#;

/// Boxicons `chevron-down`.
pub const CHEVRON_DOWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="m12 15.41l5.71-5.7l-1.42-1.42l-4.29 4.3l-4.29-4.3l-1.42 1.42z"/></svg>"#;

/// Boxicons `chevron-right`.
pub const CHEVRON_RIGHT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="m9.71 17.71l5.7-5.71l-5.7-5.71L8.29 7.71L12.59 12l-4.3 4.29z"/></svg>"#;

/// Boxicons `disc`.
pub const DISC_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12 10c-1.1 0-2 .9-2 2s.9 2 2 2s2-.9 2-2s-.9-2-2-2"/><path fill="currentColor" d="M12 2C6.49 2 2 6.49 2 12s4.49 10 10 10s10-4.49 10-10S17.51 2 12 2m0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8s8 3.59 8 8s-3.59 8-8 8"/><path fill="currentColor" d="M12 6c-3.31 0-6 2.69-6 6h2c0-2.21 1.79-4 4-4z"/></svg>"#;

/// Boxicons `user-circle`.
pub const USER_AVATAR_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12 6c-2.28 0-4 1.72-4 4s1.72 4 4 4s4-1.72 4-4s-1.72-4-4-4m0 6c-1.18 0-2-.82-2-2s.82-2 2-2s2 .82 2 2s-.82 2-2 2"/><path fill="currentColor" d="M12 2C6.49 2 2 6.49 2 12c0 3.26 1.58 6.16 4 7.98V20h.03c1.67 1.25 3.73 2 5.97 2s4.31-.75 5.97-2H18v-.02c2.42-1.83 4-4.72 4-7.98c0-5.51-4.49-10-10-10M8.18 19.02C8.59 17.85 9.69 17 11 17h2c1.31 0 2.42.85 2.82 2.02c-1.14.62-2.44.98-3.82.98s-2.69-.35-3.82-.98m9.3-1.21c-.81-1.66-2.51-2.82-4.48-2.82h-2c-1.97 0-3.66 1.16-4.48 2.82A7.96 7.96 0 0 1 4 11.99c0-4.41 3.59-8 8-8s8 3.59 8 8c0 2.29-.97 4.36-2.52 5.82"/></svg>"#;

/// Boxicons `cog`.
pub const SETTINGS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12 8c-2.21 0-4 1.79-4 4s1.79 4 4 4s4-1.79 4-4s-1.79-4-4-4m0 6c-1.08 0-2-.92-2-2s.92-2 2-2s2 .92 2 2s-.92 2-2 2"/><path fill="currentColor" d="m20.42 13.4l-.51-.29c.05-.37.08-.74.08-1.11s-.03-.74-.08-1.11l.51-.29c.96-.55 1.28-1.78.73-2.73l-1-1.73a2.006 2.006 0 0 0-2.73-.73l-.53.31c-.58-.46-1.22-.83-1.9-1.11v-.6c0-1.1-.9-2-2-2h-2c-1.1 0-2 .9-2 2v.6c-.67.28-1.31.66-1.9 1.11l-.53-.31c-.96-.55-2.18-.22-2.73.73l-1 1.73c-.55.96-.22 2.18.73 2.73l.51.29c-.05.37-.08.74-.08 1.11s.03.74.08 1.11l-.51.29c-.96.55-1.28 1.78-.73 2.73l1 1.73c.55.95 1.77 1.28 2.73.73l.53-.31c.58.46 1.22.83 1.9 1.11v.6c0 1.1.9 2 2 2h2c1.1 0 2-.9 2-2v-.6a8.7 8.7 0 0 0 1.9-1.11l.53.31c.95.55 2.18.22 2.73-.73l1-1.73c.55-.96.22-2.18-.73-2.73m-2.59-2.78c.11.45.17.92.17 1.38s-.06.92-.17 1.38a1 1 0 0 0 .47 1.11l1.12.65l-1 1.73l-1.14-.66c-.38-.22-.87-.16-1.19.14c-.68.65-1.51 1.13-2.38 1.4c-.42.13-.71.52-.71.96v1.3h-2v-1.3c0-.44-.29-.83-.71-.96c-.88-.27-1.7-.75-2.38-1.4a1.01 1.01 0 0 0-1.19-.15l-1.14.66l-1-1.73l1.12-.65c.39-.22.58-.68.47-1.11c-.11-.45-.17-.92-.17-1.38s.06-.93.17-1.38A1 1 0 0 0 5.7 9.5l-1.12-.65l1-1.73l1.14.66c.38.22.87.16 1.19-.14c.68-.65 1.51-1.13 2.38-1.4c.42-.13.71-.52.71-.96v-1.3h2v1.3c0 .44.29.83.71.96c.88.27 1.7.75 2.38 1.4c.32.31.81.36 1.19.14l1.14-.66l1 1.73l-1.12.65c-.39.22-.58.68-.47 1.11Z"/></svg>"#;

/// Boxicons `list-ul`.
pub const LIST_VIEW_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M8 11h13v2H8zm0-5h13v2H8zm0 10h13v2H8zM3 5.5h3v3H3zm0 5h3v3H3zm0 5h3v3H3z"/></svg>"#;

/// Boxicons `grid`.
pub const GRID_VIEW_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M4.5 11h5c.83 0 1.5-.67 1.5-1.5v-5c0-.83-.67-1.5-1.5-1.5h-5C3.67 3 3 3.67 3 4.5v5c0 .83.67 1.5 1.5 1.5M5 5h4v4H5zm14.5-2h-5c-.83 0-1.5.67-1.5 1.5v5c0 .83.67 1.5 1.5 1.5h5c.83 0 1.5-.67 1.5-1.5v-5c0-.83-.67-1.5-1.5-1.5M19 9h-4V5h4zM4.5 21h5c.83 0 1.5-.67 1.5-1.5v-5c0-.83-.67-1.5-1.5-1.5h-5c-.83 0-1.5.67-1.5 1.5v5c0 .83.67 1.5 1.5 1.5m.5-6h4v4H5zm14.5-2h-5c-.83 0-1.5.67-1.5 1.5v5c0 .83.67 1.5 1.5 1.5h5c.83 0 1.5-.67 1.5-1.5v-5c0-.83-.67-1.5-1.5-1.5m-.5 6h-4v-4h4z"/></svg>"#;

/// Boxicons `folder-filled`.
pub const FOLDER_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M20 4h-8.59L10 2.59C9.63 2.22 9.11 2 8.59 2H4c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2"/></svg>"#;

/// Boxicons `refresh-cw`.
pub const REFRESH_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M19.07 4.93a9.9 9.9 0 0 0-3.18-2.14a10.12 10.12 0 0 0-7.79 0c-1.19.5-2.26 1.23-3.18 2.14S3.28 6.92 2.78 8.11A9.95 9.95 0 0 0 1.99 12h2c0-1.08.21-2.13.63-3.11c.4-.95.98-1.81 1.72-2.54c.73-.74 1.59-1.31 2.54-1.71c1.97-.83 4.26-.83 6.23 0c.95.4 1.81.98 2.54 1.72c.17.17.33.34.48.52L16 9.01h6V3l-2.45 2.45c-.15-.18-.31-.36-.48-.52m.3 10.18c-.4.95-.98 1.81-1.72 2.54c-.73.74-1.59 1.31-2.54 1.71c-1.97.83-4.26.83-6.23 0c-.95-.4-1.81-.98-2.54-1.72c-.17-.17-.33-.34-.48-.52l2.13-2.13H2v6l2.45-2.45c.15.18.31.36.48.52c.92.92 1.99 1.64 3.18 2.14c1.23.52 2.54.79 3.89.79s2.66-.26 3.89-.79c1.19-.5 2.26-1.23 3.18-2.14s1.64-1.99 2.14-3.18c.52-1.23.79-2.54.79-3.89h-2c0 1.08-.21 2.13-.63 3.11Z"/></svg>"#;

/// Boxicons `check`.
pub const CHECK_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M9 15.59L4.71 11.3L3.3 12.71l5 5c.2.2.45.29.71.29s.51-.1.71-.29l11-11l-1.41-1.41L9.02 15.59Z"/></svg>"#;

/// Boxicons `palette`.
pub const PALETTE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M13.4 2.1c-3.16-.43-6.24.6-8.47 2.83S1.67 10.25 2.1 13.4c.53 3.89 3.46 7.21 7.29 8.25c.86.23 1.74.35 2.62.35h.14c1.03-.02 1.97-.55 2.52-1.43c.54-.88.6-1.95.15-2.88l-.2-.42c-.45-.94-.1-1.8.39-2.28s1.34-.84 2.28-.39l.41.2c.93.45 2 .39 2.88-.15a3 3 0 0 0 1.43-2.52c.01-.92-.1-1.85-.35-2.76c-1.04-3.83-4.35-6.75-8.25-7.29Zm6.12 10.86c-.3.18-.65.2-.96.05l-.41-.2a3.96 3.96 0 0 0-4.56.78a3.96 3.96 0 0 0-.78 4.56l.2.42c.15.31.13.66-.05.96c-.19.3-.49.47-.84.48c-.74.02-1.48-.08-2.21-.28c-3.06-.83-5.4-3.48-5.83-6.59c-.34-2.53.48-5 2.27-6.79a7.96 7.96 0 0 1 5.66-2.34c.37 0 .75.03 1.13.08c3.11.42 5.75 2.76 6.59 5.83c.2.73.29 1.47.28 2.21c0 .35-.18.66-.48.84Z"/><path fill="currentColor" d="M7.33 12.76a1 1 0 1 0 0 2a1 1 0 1 0 0-2m.07-3.83a1.12 1.12 0 1 0 0 2.24a1.12 1.12 0 1 0 0-2.24m2.81-2.87a1.25 1.25 0 1 0 0 2.5a1.25 1.25 0 1 0 0-2.5m4.06.11a1.38 1.38 0 1 0 0 2.76a1.38 1.38 0 1 0 0-2.76"/></svg>"#;

/// Boxicons `database`.
pub const DATABASE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M12 3C7.66 3 4 4.83 4 7v10c0 2.17 3.66 4 8 4s8-1.83 8-4V7c0-2.17-3.66-4-8-4m0 2c3.68 0 5.91 1.49 6 2c-.09.51-2.32 2-6 2S6.07 7.49 6 7.01C6.07 6.51 8.31 5 12 5M6 9.61c1.48.85 3.64 1.39 6 1.39s4.52-.55 6-1.39V12c-.07.5-2.31 2-6 2s-5.93-1.51-6-2zM12 19c-3.69 0-5.93-1.51-6-2v-2.39c1.48.85 3.64 1.39 6 1.39s4.52-.55 6-1.39V17c-.07.5-2.31 2-6 2"/></svg>"#;

/// Boxicons `trash`.
pub const TRASH_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M17 6V4c0-1.1-.9-2-2-2H9c-1.1 0-2 .9-2 2v2H2v2h2v12c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V8h2V6zM9 4h6v2H9zM6 20V8h12v12z"/><path fill="currentColor" d="M9 10h2v8H9zm4 0h2v8h-2z"/></svg>"#;

/// Boxicons `plus`.
pub const PLUS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M3 13h8v8h2v-8h8v-2h-8V3h-2v8H3z"/></svg>"#;

/// Boxicons `station` (broadcast waves indicator for scrobbling).
pub const RADIO_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M14 12c0-1.1-.9-2-2-2s-2 .9-2 2a2 2 0 1 0 4 0m-6 0c0-1.07.42-2.07 1.17-2.82L7.76 7.76A5.97 5.97 0 0 0 6 12c0 1.6.62 3.11 1.76 4.25l1.41-1.42A3.96 3.96 0 0 1 8 12m8.24-4.24l-1.41 1.41C15.59 9.93 16 10.93 16 12s-.42 2.07-1.17 2.83l1.41 1.41C17.37 15.11 18 13.6 18 12s-.62-3.11-1.76-4.24"/><path fill="currentColor" d="M6.34 17.66C4.83 16.15 3.99 14.14 3.99 12s.83-4.14 2.34-5.65L4.92 4.93C3.03 6.82 1.99 9.33 1.99 12s1.04 5.18 2.93 7.07l1.41-1.41ZM19.07 4.93l-1.41 1.41C19.17 7.85 20 9.86 20 12s-.83 4.15-2.34 5.66l1.41 1.41C20.96 17.18 22 14.67 22 12s-1.04-5.18-2.93-7.07"/></svg>"#;

/// Boxicons `chevron-left`.
pub const ARROW_LEFT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M14.29 6.29L8.59 12l5.7 5.71l1.42-1.42l-4.3-4.29l4.3-4.29z"/></svg>"#;

static SVG_CACHE: OnceLock<Mutex<HashMap<&'static str, Handle>>> = OnceLock::new();

pub fn handle_for_svg(svg_str: &'static str) -> Handle {
    let cache = SVG_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(handle) = guard.get(svg_str) {
            return handle.clone();
        }
    }
    let handle = Handle::from_memory(svg_str.as_bytes());
    if let Ok(mut guard) = cache.lock() {
        guard.insert(svg_str, handle.clone());
    }
    handle
}

pub fn icon<'a>(svg_str: &'static str, size: f32, color: Color) -> Svg<'a, Theme> {
    iced::widget::svg(handle_for_svg(svg_str))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_theme, _status| iced::widget::svg::Style { color: Some(color) })
}
