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
    is_shuffle: bool,
    is_repeat: bool,
    is_loved: bool,
) -> Element<'a, Message> {
    let bar_fg = theme_cfg.player_bar_text_color();

    // 1. Left: Track Info & Cover Thumbnail
    let cover_art: Element<'a, Message> = match current_cover {
        Some(handle) => container(
            image(handle.clone())
                .width(Length::Fixed(46.0))
                .height(Length::Fixed(46.0))
                .content_fit(ContentFit::Contain),
        )
        .width(46)
        .height(46)
        .clip(true)
        .style(|_theme| container::Style {
            border: Border {
                radius: Radius::from(3.0),
                color: theme_cfg.border_color(),
                width: 1.0,
            },
            ..Default::default()
        })
        .into(),
        None => container(icons::icon(icons::DISC_SVG, 22.0, bar_fg))
            .width(46)
            .height(46)
            .center_x(46)
            .center_y(46)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
                border: Border {
                    radius: Radius::from(3.0),
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
                .color(bar_fg),
            text(format!("{} • {}", track.artist, track.album))
                .size(11)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(2)
        .width(Length::Fill),
        None => column![
            text("No track playing")
                .size(13)
                .color(bar_fg),
            text("Vibecore")
                .size(11)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(2)
        .width(Length::Fill),
    };

    let track_section = row![cover_art, Space::with_width(10), track_details]
        .spacing(2)
        .align_y(Alignment::Center)
        .width(260);

    // 2. Center: Controls and Seek Bar
    let play_pause_icon_svg = if is_playing {
        icons::PAUSE_SVG
    } else {
        icons::PLAY_SVG
    };

    let prev_btn = button(icons::icon(icons::PREV_SVG, 15.0, bar_fg))
        .padding([6, 8])
        .style(styles::player_bar_button(theme_cfg, false))
        .on_press(Message::PreviousTrack);

    let play_pause_btn = button(icons::icon(play_pause_icon_svg, 17.0, theme_cfg.player_bar_bg_color()))
        .padding([8, 12])
        .style(styles::player_bar_play_button(theme_cfg))
        .on_press(Message::TogglePlayPause);

    let stop_btn = button(icons::icon(icons::STOP_SVG, 13.0, bar_fg))
        .padding([6, 8])
        .style(styles::player_bar_button(theme_cfg, false))
        .on_press(Message::Stop);

    let next_btn = button(icons::icon(icons::NEXT_SVG, 15.0, bar_fg))
        .padding([6, 8])
        .style(styles::player_bar_button(theme_cfg, false))
        .on_press(Message::NextTrack);

    let spectrum_indicator = container(icons::icon(
        icons::SPECTRUM_SVG,
        14.0,
        if is_playing {
            theme_cfg.primary_color()
        } else {
            theme_cfg.text_muted_color()
        },
    ))
    .padding([0, 4]);

    let buttons = row![
        prev_btn,
        play_pause_btn,
        stop_btn,
        next_btn,
        spectrum_indicator,
    ]
    .spacing(6)
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
            .color(bar_fg)
            .width(42),
        seek_slider,
        text(total_time_str)
            .size(11)
            .color(bar_fg)
            .width(42),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let center_panel = column![buttons, progress_bar]
        .spacing(4)
        .align_x(Alignment::Center)
        .width(Length::FillPortion(2));

    // 3. Right: Secondary Controls (Love, Shuffle, Repeat) & Volume Control
    let love_icon = if is_loved {
        icons::icon(icons::HEART_FILLED_SVG, 16.0, theme_cfg.danger_color())
    } else {
        icons::icon(icons::HEART_SVG, 16.0, bar_fg)
    };
    let love_btn = button(love_icon)
        .padding([6, 8])
        .style(styles::player_bar_button(theme_cfg, is_loved))
        .on_press(Message::ToggleLoveCurrentTrack);

    let shuffle_btn = button(icons::icon(
        icons::SHUFFLE_SVG,
        15.0,
        if is_shuffle {
            theme_cfg.primary_color()
        } else {
            bar_fg
        },
    ))
    .padding([6, 8])
    .style(styles::player_bar_button(theme_cfg, is_shuffle))
    .on_press(Message::ToggleShuffle);

    let repeat_btn = button(icons::icon(
        icons::REPEAT_SVG,
        15.0,
        if is_repeat {
            theme_cfg.primary_color()
        } else {
            bar_fg
        },
    ))
    .padding([6, 8])
    .style(styles::player_bar_button(theme_cfg, is_repeat))
    .on_press(Message::ToggleRepeat);

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
        .width(90);

    let vol_control = row![
        icons::icon(vol_icon_svg, 15.0, bar_fg),
        vol_slider,
        text(format!("{}%", vol_pct))
            .size(11)
            .color(theme_cfg.text_muted_color())
            .width(32),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let right_controls = row![
        love_btn,
        shuffle_btn,
        repeat_btn,
        Space::with_width(6),
        vol_control,
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let content = row![
        track_section,
        Space::with_width(Length::FillPortion(1)),
        center_panel,
        Space::with_width(Length::FillPortion(1)),
        right_controls,
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([6, 16]);

    container(content)
        .style(styles::player_bar_container(theme_cfg))
        .width(Length::Fill)
        .height(72)
        .into()
}
