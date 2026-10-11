use iced::border::Radius;
use iced::widget::{button, column, container, image, row, slider, text, Space};
use iced::{Alignment, Border, ContentFit, Element, Length};
use player_core::{format_duration, Track};

use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    current_track: Option<&'a Track>,
    current_cover: Option<&'a image::Handle>,
    is_playing: bool,
    position_ms: u64,
    duration_ms: u64,
    volume: f32,
    is_seeking: bool,
    seek_pos_ms: u64,
) -> Element<'a, Message> {
    // 1. Left: Track Info & Cover Thumbnail
    let cover_art: Element<'a, Message> = match current_cover {
        Some(handle) => container(
            image(handle.clone())
                .width(Length::Fixed(48.0))
                .height(Length::Fixed(48.0))
                .content_fit(ContentFit::Cover),
        )
        .width(48)
        .height(48)
        .style(|_theme| container::Style {
            border: Border {
                radius: Radius::from(6.0),
                color: theme_cfg.border_color(),
                width: 1.0,
            },
            ..Default::default()
        })
        .into(),
        None => container(
            icons::icon(icons::MUSIC_NOTE_SVG, 22.0, theme_cfg.primary_color())
        )
        .width(48)
        .height(48)
        .center_x(48)
        .center_y(48)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
            border: Border {
                radius: Radius::from(6.0),
                color: theme_cfg.border_color(),
                width: 1.0,
            },
            ..Default::default()
        })
        .into(),
    };

    let track_details = match current_track {
        Some(track) => column![
            text(&track.title)
                .size(13)
                .color(theme_cfg.text_color()),
            text(format!("{} • {}", track.artist, track.album))
                .size(11)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(2)
        .width(Length::Fill),
        None => column![
            text("No track playing")
                .size(13)
                .color(theme_cfg.text_muted_color()),
            text("Select a song to start listening")
                .size(11)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(2)
        .width(Length::Fill),
    };

    let track_section = row![cover_art, Space::with_width(10), track_details]
        .spacing(2)
        .align_y(Alignment::Center)
        .width(280);

    // 2. Center: Controls and Seek Bar
    let play_pause_icon_svg = if is_playing {
        icons::PAUSE_SVG
    } else {
        icons::PLAY_SVG
    };

    let prev_btn = button(icons::icon(icons::PREV_SVG, 16.0, theme_cfg.text_color()))
        .padding([6, 10])
        .style(styles::icon_control_button(theme_cfg))
        .on_press(Message::PreviousTrack);

    let play_pause_btn = button(icons::icon(play_pause_icon_svg, 18.0, theme_cfg.surface_color()))
        .padding([8, 14])
        .style(styles::primary_button(theme_cfg))
        .on_press(Message::TogglePlayPause);

    let stop_btn = button(icons::icon(icons::STOP_SVG, 14.0, theme_cfg.text_color()))
        .padding([6, 10])
        .style(styles::icon_control_button(theme_cfg))
        .on_press(Message::Stop);

    let next_btn = button(icons::icon(icons::NEXT_SVG, 16.0, theme_cfg.text_color()))
        .padding([6, 10])
        .style(styles::icon_control_button(theme_cfg))
        .on_press(Message::NextTrack);

    let buttons = row![prev_btn, play_pause_btn, stop_btn, next_btn]
        .spacing(8)
        .align_y(Alignment::Center);

    let active_pos = if is_seeking { seek_pos_ms } else { position_ms };
    let cur_time_str = format_duration(active_pos);
    let total_time_str = format_duration(duration_ms);

    let seek_range_max = if duration_ms > 0 {
        duration_ms as f32
    } else {
        100.0
    };

    let seek_slider = slider(
        0.0..=seek_range_max,
        active_pos as f32,
        Message::Seek,
    )
    .on_release(Message::SeekFinished)
    .style(styles::seek_slider_style(theme_cfg))
    .width(Length::Fill);

    let progress_bar = row![
        text(cur_time_str)
            .size(11)
            .color(theme_cfg.text_muted_color())
            .width(45),
        seek_slider,
        text(total_time_str)
            .size(11)
            .color(theme_cfg.text_muted_color())
            .width(45),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let center_panel = column![buttons, progress_bar]
        .spacing(4)
        .align_x(Alignment::Center)
        .width(Length::FillPortion(2));

    // 3. Right: Volume Control
    let vol_pct = (volume * 100.0).round() as u32;
    let vol_icon_svg = if volume <= 0.01 {
        icons::VOL_MUTE_SVG
    } else if volume < 0.5 {
        icons::VOL_LOW_SVG
    } else {
        icons::VOL_HIGH_SVG
    };

    let vol_slider = slider(0.0..=1.0, volume, Message::VolumeChanged)
        .style(styles::volume_slider_style(theme_cfg))
        .width(100);

    let vol_control = row![
        icons::icon(vol_icon_svg, 16.0, theme_cfg.text_color()),
        vol_slider,
        text(format!("{}%", vol_pct))
            .size(11)
            .color(theme_cfg.text_muted_color())
            .width(36),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(180);

    let content = row![
        track_section,
        Space::with_width(Length::FillPortion(1)),
        center_panel,
        Space::with_width(Length::FillPortion(1)),
        vol_control,
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .padding([8, 18]);

    container(content)
        .style(styles::player_bar_container(theme_cfg))
        .width(Length::Fill)
        .height(80)
        .into()
}
