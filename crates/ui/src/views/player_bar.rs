use core_engine::{format_duration, Track};
use iced::border::Radius;
use iced::widget::{
    button, column, container, image, mouse_area, row, slider, stack, Space, Widget,
};
use iced::{Alignment, Element, Length};

use crate::components::{self, text};
use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

/// Self-contained state and view component for the bottom persistent player bar widget.
#[derive(Debug, Default, Clone)]
pub struct PlayerBar {
    pub is_seeking: bool,
    pub seek_pos_ms: u64,
}

impl PlayerBar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_seek(&mut self, ms: f32) {
        self.is_seeking = true;
        self.seek_pos_ms = ms as u64;
    }

    pub fn on_seek_finished(&mut self) -> u64 {
        self.is_seeking = false;
        self.seek_pos_ms
    }

    pub fn view_track_info<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        current_track: Option<&'a Track>,
        current_cover: Option<&'a image::Handle>,
    ) -> Element<'a, Message> {
        let bar_fg = theme_cfg.player_bar_text_color();
        let bar_muted = iced::Color { a: 0.6, ..bar_fg };

        let cover_art = components::cover_art(
            theme_cfg,
            current_cover,
            46.0,
            Radius::from(3.0),
            icons::DISC_SVG,
            22.0,
            bar_fg,
        );

        let track_details = match current_track {
            Some(track) => column![
                components::Marquee::new(&track.title)
                    .size(13.0)
                    .color(bar_fg)
                    .font(theme_cfg.font()),
                components::Marquee::new(format!("{} • {}", track.artist, track.album))
                    .size(11.0)
                    .color(bar_muted)
                    .font(theme_cfg.font()),
            ],
            None => column![
                text(theme_cfg, "No track playing").size(13).color(bar_fg),
                text(theme_cfg, "Melody").size(11).color(bar_muted),
            ],
        }
        .spacing(2)
        .width(Length::Fill);

        let content = row![cover_art, Space::new().width(10), track_details]
            .spacing(2)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        if current_track.is_some() {
            mouse_area(content)
                .on_right_press(Message::OpenPlayerBarContextMenu)
                .boxed()
        } else {
            content.boxed()
        }
    }

    pub fn view_controls<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        is_playing: bool,
    ) -> Element<'a, Message> {
        let bar_fg = theme_cfg.player_bar_text_color();

        let prev_btn = button(icons::icon(icons::PREV_SVG, 15.0, bar_fg))
            .padding([6, 8])
            .style(styles::player_bar_button(theme_cfg, false))
            .on_press(Message::PreviousTrack);

        let play_pause_btn = button(icons::icon(
            if is_playing {
                icons::PAUSE_SVG
            } else {
                icons::PLAY_SVG
            },
            17.0,
            theme_cfg.player_bar_bg_color(),
        ))
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

        row![prev_btn, play_pause_btn, stop_btn, next_btn]
            .spacing(6)
            .align_y(Alignment::Center)
            .boxed()
    }

    pub fn view_seek_bar<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        position_ms: u64,
        duration_ms: u64,
    ) -> Element<'a, Message> {
        let bar_fg = theme_cfg.player_bar_text_color();
        let bar_muted = iced::Color { a: 0.6, ..bar_fg };

        let active_pos = if self.is_seeking {
            self.seek_pos_ms
        } else {
            position_ms
        };
        let clamped_active_pos = if duration_ms > 0 {
            active_pos.min(duration_ms)
        } else {
            active_pos
        };
        let cur_time_str = format_duration(clamped_active_pos);
        let total_time_str = format_duration(duration_ms);

        let time_info = text(theme_cfg, format!("{cur_time_str} / {total_time_str}"))
            .size(10)
            .color(bar_muted);

        let seek_range_max = if duration_ms > 0 {
            duration_ms as f32
        } else {
            100.0
        };

        let seek_bar = slider(
            0.0..=seek_range_max,
            clamped_active_pos as f32,
            Message::Seek,
        )
        .on_release(Message::SeekFinished)
        .style(styles::seek_slider_style(theme_cfg))
        .width(Length::Fill)
        .height(8);

        row![seek_bar, Space::new().width(8), time_info]
            .spacing(4)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .boxed()
    }

    pub fn view_volume<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        volume: f32,
    ) -> Element<'a, Message> {
        let bar_fg = theme_cfg.player_bar_text_color();
        let bar_muted = iced::Color { a: 0.6, ..bar_fg };

        let vol_slider = slider(0.0..=1.0, volume, Message::VolumeChanged)
            .on_release(Message::SaveState)
            .style(styles::volume_slider_style(theme_cfg))
            .width(90);

        row![
            icons::icon(
                if volume <= 0.01 {
                    icons::VOL_MUTE_SVG
                } else if volume < 0.5 {
                    icons::VOL_LOW_SVG
                } else {
                    icons::VOL_HIGH_SVG
                },
                15.0,
                bar_fg
            ),
            vol_slider,
            text(theme_cfg, format!("{}%", (volume * 100.0).round() as u32))
                .size(11)
                .color(bar_muted)
                .width(32),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .boxed()
    }

    pub fn view_options<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        is_shuffle: bool,
        is_repeat: bool,
    ) -> Element<'a, Message> {
        let bar_fg = theme_cfg.player_bar_text_color();

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

        row![shuffle_btn, repeat_btn]
            .spacing(4)
            .align_y(Alignment::Center)
            .boxed()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn view<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        current_track: Option<&'a Track>,
        current_cover: Option<&'a image::Handle>,
        is_playing: bool,
        position_ms: u64,
        duration_ms: u64,
        volume: f32,
        is_shuffle: bool,
        is_repeat: bool,
    ) -> Element<'a, Message> {
        let bar_fg = theme_cfg.player_bar_text_color();
        let bar_muted = iced::Color { a: 0.6, ..bar_fg };

        // 1. Left: Track Info & Cover Thumbnail
        let track_section = self.view_track_info(theme_cfg, current_track, current_cover);

        // 2. Center: Controls
        let buttons = self.view_controls(theme_cfg, is_playing);

        let active_pos = if self.is_seeking {
            self.seek_pos_ms
        } else {
            position_ms
        };
        let clamped_active_pos = if duration_ms > 0 {
            active_pos.min(duration_ms)
        } else {
            active_pos
        };
        let cur_time_str = format_duration(clamped_active_pos);
        let total_time_str = format_duration(duration_ms);

        let time_info = text(theme_cfg, format!("{cur_time_str} / {total_time_str}"))
            .size(10)
            .align_x(Alignment::End)
            .color(bar_muted);

        let buttons_width = 176.0;

        // Perfectly centered controls across the entire screen
        let center_controls = container(buttons)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

        let options = self.view_options(theme_cfg, is_shuffle, is_repeat);
        let vol = self.view_volume(theme_cfg, volume);
        let right_controls = row![options, Space::new().width(6), vol]
            .spacing(4)
            .align_y(Alignment::Center);

        // Sides layer: Left track info and right controls bounded around center clearance
        let sides_row = row![
            container(track_section)
                .width(Length::Fill)
                .align_y(Alignment::Center),
            Space::new().width(buttons_width + 48.0),
            container(right_controls)
                .width(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::Center),
        ]
        .spacing(0)
        .align_y(Alignment::Center)
        .padding([0, 16])
        .width(Length::Fill)
        .height(Length::Fill);

        let seek_range_max = if duration_ms > 0 {
            duration_ms as f32
        } else {
            100.0
        };

        let seek_bar = slider(
            0.0..=seek_range_max,
            clamped_active_pos as f32,
            Message::Seek,
        )
        .on_release(Message::SeekFinished)
        .style(styles::seek_slider_style(theme_cfg))
        .width(Length::Fill)
        .height(8);

        let top_right_timing = row![Space::new().width(Length::Fill), time_info]
            .padding(iced::Padding {
                top: 8.0,
                right: 20.0,
                bottom: 0.0,
                left: 0.0,
            })
            .width(Length::Fill);

        let bar_stack = stack![sides_row, center_controls, top_right_timing, seek_bar]
            .width(Length::Fill)
            .height(72);

        container(bar_stack)
            .style(styles::player_bar_container(theme_cfg))
            .width(Length::Fill)
            .height(72)
            .boxed()
    }
}
