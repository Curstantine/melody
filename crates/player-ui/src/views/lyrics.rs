use iced::border::Radius;
use iced::widget::{button, canvas, column, container, image, row, scrollable, stack, Space};
use iced::{Alignment, Element, Length};
use player_core::{ExtractedColors, Track};

use crate::components::{self, text};
use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

/// Dynamic canvas program rendering a luminous, blurred, flowing color background.
/// Smoothly drifts 5 fluid chromatic orbs extracted from the album cover art.
struct AmbientBackground {
    time: f32,
    palette: ExtractedColors,
}

impl<Message> canvas::Program<Message> for AmbientBackground {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let width = bounds.width;
        let height = bounds.height;
        let t = self.time;

        // 1. Deep ambient dark base
        let bg = self.palette.background;
        let base_bg = iced::Color::from_rgba(bg[0] * 0.70, bg[1] * 0.70, bg[2] * 0.70, 1.0);
        frame.fill_rectangle(iced::Point::ORIGIN, bounds.size(), base_bg);

        // 2. Multi-layered luminous chromatic orbs drifting in gentle organic Lissajous curves
        let p = self.palette.primary;
        let s = self.palette.secondary;
        let a = self.palette.accent;

        let min_dim = width.min(height);
        let blobs: [(f32, f32, f32, [f32; 4], f32); 5] = [
            // Blob 1: Primary color (drifting around top-left to center)
            (
                width * (0.32 + 0.22 * (t * 0.40).sin()),
                height * (0.36 + 0.20 * (t * 0.32).cos()),
                (min_dim * 0.45).clamp(240.0, 480.0) + 30.0 * (t * 0.25).sin(),
                p,
                0.36,
            ),
            // Blob 2: Secondary color (drifting lower-right to center)
            (
                width * (0.70 + 0.18 * (t * 0.35 + 1.2).cos()),
                height * (0.64 + 0.20 * (t * 0.38 + 0.9).sin()),
                (min_dim * 0.48).clamp(250.0, 500.0) + 35.0 * (t * 0.20).cos(),
                s,
                0.34,
            ),
            // Blob 3: Accent color (drifting upper-right)
            (
                width * (0.56 + 0.24 * (t * 0.28 + 2.5).sin()),
                height * (0.24 + 0.18 * (t * 0.45 + 1.5).cos()),
                (min_dim * 0.38).clamp(200.0, 420.0) + 25.0 * (t * 0.30).sin(),
                a,
                0.30,
            ),
            // Blob 4: Warm Primary-Secondary blend (floating lower-left)
            (
                width * (0.22 + 0.18 * (t * 0.36 + 3.1).cos()),
                height * (0.74 + 0.16 * (t * 0.25 + 2.2).sin()),
                (min_dim * 0.46).clamp(240.0, 460.0) + 30.0 * (t * 0.28).cos(),
                [
                    (p[0] + s[0]) * 0.5,
                    (p[1] + s[1]) * 0.5,
                    (p[2] + s[2]) * 0.5,
                    1.0,
                ],
                0.28,
            ),
            // Blob 5: Secondary-Accent blend (ambient glow upper-right)
            (
                width * (0.80 + 0.14 * (t * 0.48 + 4.0).sin()),
                height * (0.36 + 0.18 * (t * 0.30 + 3.2).cos()),
                (min_dim * 0.40).clamp(210.0, 440.0) + 25.0 * (t * 0.33).sin(),
                [
                    (s[0] + a[0]) * 0.5,
                    (s[1] + a[1]) * 0.5,
                    (s[2] + a[2]) * 0.5,
                    1.0,
                ],
                0.26,
            ),
        ];

        // Soft radial Gaussian blur approximation using 16 concentric rings per blob
        const RINGS: usize = 16;
        for (bx, by, radius, col, peak_alpha) in blobs {
            let center = iced::Point::new(bx, by);
            for step in 0..RINGS {
                let frac = step as f32 / (RINGS - 1) as f32;
                let r = radius * (1.0 - frac * 0.94);
                // Quadratic falloff from center outward creates velvety antialiased blur
                let alpha = peak_alpha * (1.0 - frac) * (1.0 - frac) * 0.10;
                let c = iced::Color::from_rgba(col[0], col[1], col[2], alpha);
                let path = canvas::Path::circle(center, r);
                frame.fill(&path, c);
            }
        }

        // 3. Frosted ambient dark glass overlay to guarantee high contrast & readability
        let dark_wash = iced::Color::from_rgba(0.04, 0.04, 0.07, 0.52);
        frame.fill_rectangle(iced::Point::ORIGIN, bounds.size(), dark_wash);

        vec![frame.into_geometry()]
    }
}

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    current_track: Option<&'a Track>,
    current_cover: Option<&'a image::Handle>,
    lyrics: Option<&'a str>,
    next_track: Option<Track>,
    palette: &'a ExtractedColors,
    animation_time: f32,
    is_repeat: bool,
    is_shuffle: bool,
    is_loved: bool,
) -> Element<'a, Message> {
    let primary_col = iced::Color::from_rgba(palette.primary[0], palette.primary[1], palette.primary[2], 1.0);
    let accent_col = iced::Color::from_rgba(palette.accent[0], palette.accent[1], palette.accent[2], 1.0);
    let secondary_col = iced::Color::from_rgba(palette.secondary[0], palette.secondary[1], palette.secondary[2], 1.0);

    let content: Element<'a, Message> = match current_track {
        Some(track) => {
            // Left area: Large album artwork & scrollable lyrics
            let cover_art = container(components::cover_art(
                theme_cfg,
                current_cover,
                240.0,
                Radius::from(14.0),
                icons::ALBUM_SVG,
                90.0,
                theme_cfg.primary_color(),
            ))
            .style(move |_| container::Style {
                shadow: iced::Shadow {
                    color: iced::Color::from_rgba(
                        palette.primary[0] * 0.4,
                        palette.primary[1] * 0.4,
                        palette.primary[2] * 0.4,
                        0.40,
                    ),
                    offset: iced::Vector::new(0.0, 8.0),
                    blur_radius: 24.0,
                },
                border: iced::Border {
                    radius: Radius::from(16.0),
                    color: iced::Color::from_rgba(
                        palette.primary[0],
                        palette.primary[1],
                        palette.primary[2],
                        0.35,
                    ),
                    width: 1.5,
                },
                ..Default::default()
            });

            let lyrics_body: Element<'a, Message> = if let Some(lyr) = lyrics {
                text(theme_cfg, lyr)
                    .size(15)
                    .color(theme_cfg.text_color())
                    .into()
            } else {
                column![
                    text(theme_cfg, "No synchronized lyrics found")
                        .size(15)
                        .color(theme_cfg.text_color()),
                    Space::with_height(4),
                    text(
                        theme_cfg,
                        "Place a .lrc file in the same directory or configure a lyrics plugin."
                    )
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                ]
                .into()
            };

            let lyrics_panel = container(
                column![
                    row![
                        icons::icon(icons::MUSIC_NOTE_SVG, 12.0, primary_col),
                        text(theme_cfg, "LYRICS")
                            .size(10)
                            .color(primary_col),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    Space::with_height(10),
                    scrollable(lyrics_body)
                        .height(Length::Fill)
                        .style(styles::scrollable_style(theme_cfg)),
                ]
                .height(Length::Fill),
            )
            .padding(18)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(
                    0.05, 0.05, 0.08, 0.55,
                ))),
                border: iced::Border {
                    radius: Radius::from(16.0),
                    color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                    width: 1.0,
                },
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill);

            let left_section = row![
                cover_art,
                Space::with_width(24),
                lyrics_panel,
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::Center);

            // Bottom-right docked card: Information & Up Next
            let live_badge = row![
                icons::icon(icons::SPEAKER_PLAYING_SVG, 12.0, primary_col),
                text(theme_cfg, "NOW PLAYING")
                    .size(10)
                    .color(primary_col),
            ]
            .spacing(6)
            .align_y(Alignment::Center);

            let loved_icon = if is_loved {
                icons::icon(icons::HEART_FILLED_SVG, 14.0, primary_col)
            } else {
                icons::icon(icons::HEART_SVG, 14.0, theme_cfg.text_muted_color())
            };
            let love_btn = button(loved_icon)
                .padding([2, 4])
                .style(styles::icon_control_button(theme_cfg))
                .on_press(Message::ToggleLoveCurrentTrack);

            let top_card_header = row![
                live_badge,
                Space::with_width(Length::Fill),
                love_btn,
            ]
            .align_y(Alignment::Center);

            let title_text = text(theme_cfg, &track.title)
                .size(17)
                .color(theme_cfg.text_color());

            let artist_album_text = text(
                theme_cfg,
                format!("{} • {}", track.artist, track.album),
            )
            .size(13)
            .color(theme_cfg.text_muted_color());

            // Badges row
            let mut badges = row![].spacing(6).align_y(Alignment::Center);
            if let Some(yr) = track.year {
                badges = badges.push(meta_badge(theme_cfg, format!("{}", yr), primary_col));
            }
            if let Some(ref g) = track.genre {
                if !g.trim().is_empty() {
                    badges = badges.push(meta_badge(theme_cfg, g.clone(), accent_col));
                }
            }
            if let Some(t_num) = track.track_num {
                if let Some(d_num) = track.disc_num {
                    badges = badges.push(meta_badge(
                        theme_cfg,
                        format!("D{} · T{}", d_num, t_num),
                        secondary_col,
                    ));
                } else {
                    badges = badges.push(meta_badge(
                        theme_cfg,
                        format!("Track {}", t_num),
                        secondary_col,
                    ));
                }
            }
            badges = badges.push(meta_badge(
                theme_cfg,
                track.formatted_duration(),
                theme_cfg.text_muted_color(),
            ));

            // Optional tags
            let mut extra_tags = column![].spacing(2);
            if let Some(ref alb_artist) = track.album_artist {
                if !alb_artist.trim().is_empty() && alb_artist != &track.artist {
                    extra_tags = extra_tags.push(
                        text(theme_cfg, format!("Album Artist: {}", alb_artist))
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    );
                }
            }
            if let Some(ref comp) = track.composer {
                if !comp.trim().is_empty() {
                    extra_tags = extra_tags.push(
                        text(theme_cfg, format!("Composer: {}", comp))
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    );
                }
            }

            // Up Next Section Header
            let mut up_next_tags = row![].spacing(6).align_y(Alignment::Center);
            if is_repeat {
                up_next_tags = up_next_tags.push(
                    row![
                        icons::icon(icons::REPEAT_SVG, 10.0, primary_col),
                        text(theme_cfg, "Repeat").size(9).color(primary_col),
                    ]
                    .spacing(3)
                    .align_y(Alignment::Center),
                );
            }
            if is_shuffle {
                up_next_tags = up_next_tags.push(
                    row![
                        icons::icon(icons::SHUFFLE_SVG, 10.0, accent_col),
                        text(theme_cfg, "Shuffle").size(9).color(accent_col),
                    ]
                    .spacing(3)
                    .align_y(Alignment::Center),
                );
            }

            let up_next_header = row![
                row![
                    icons::icon(icons::NEXT_SVG, 11.0, accent_col),
                    text(theme_cfg, "UP NEXT").size(10).color(accent_col),
                ]
                .spacing(5)
                .align_y(Alignment::Center),
                Space::with_width(Length::Fill),
                up_next_tags,
            ]
            .align_y(Alignment::Center);

            // Up Next Content Item
            let next_track_item: Element<'a, Message> = match next_track {
                Some(next) => {
                    let next_id = next.id;
                    let next_duration = next.formatted_duration();
                    let next_title = next.title;
                    let next_subtitle = format!("{} • {}", next.artist, next.album);

                    let disc_icon = container(icons::icon(icons::DISC_SVG, 16.0, primary_col))
                        .width(32)
                        .height(32)
                        .center_x(32)
                        .center_y(32)
                        .style(move |_| container::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgba(
                                primary_col.r,
                                primary_col.g,
                                primary_col.b,
                                0.15,
                            ))),
                            border: iced::Border {
                                radius: Radius::from(8.0),
                                color: iced::Color::from_rgba(
                                    primary_col.r,
                                    primary_col.g,
                                    primary_col.b,
                                    0.25,
                                ),
                                width: 1.0,
                            },
                            ..Default::default()
                        });

                    let next_info = column![
                        text(theme_cfg, next_title)
                            .size(12)
                            .color(theme_cfg.text_color()),
                        text(theme_cfg, next_subtitle)
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    ]
                    .spacing(1)
                    .width(Length::Fill);

                    let duration_tag = row![
                        text(theme_cfg, next_duration)
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                        icons::icon(icons::PLAY_SVG, 10.0, primary_col),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center);

                    button(
                        row![
                            disc_icon,
                            Space::with_width(8),
                            next_info,
                            duration_tag,
                        ]
                        .align_y(Alignment::Center)
                        .padding([6, 8]),
                    )
                    .style(next_track_btn_style(theme_cfg, primary_col))
                    .on_press(Message::PlayTrack(next_id))
                    .width(Length::Fill)
                    .into()
                }
                None => container(
                    text(theme_cfg, "End of queue · No upcoming tracks")
                        .size(11)
                        .color(theme_cfg.text_muted_color()),
                )
                .padding([6, 8])
                .into(),
            };

            // Bottom-right card container
            let bottom_right_card = container(
                column![
                    top_card_header,
                    title_text,
                    artist_album_text,
                    badges,
                    extra_tags,
                    components::border_vert(
                        1.0,
                        iced::Color::from_rgba(1.0, 1.0, 1.0, 0.10),
                    ),
                    up_next_header,
                    next_track_item,
                ]
                .spacing(9)
                .width(Length::Fill),
            )
            .width(360)
            .padding(16)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(
                    0.06, 0.06, 0.10, 0.84,
                ))),
                border: iced::Border {
                    radius: Radius::from(16.0),
                    color: iced::Color::from_rgba(
                        palette.primary[0],
                        palette.primary[1],
                        palette.primary[2],
                        0.35,
                    ),
                    width: 1.0,
                },
                shadow: iced::Shadow {
                    color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                    offset: iced::Vector::new(0.0, 8.0),
                    blur_radius: 20.0,
                },
                text_color: Some(theme_cfg.text_color()),
            });

            let right_section = column![
                Space::with_height(Length::Fill),
                bottom_right_card,
            ]
            .width(360)
            .height(Length::Fill)
            .align_x(Alignment::End);

            row![
                left_section,
                Space::with_width(20),
                right_section,
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        }
        None => {
            // Empty state when no track is selected
            let empty_card = container(
                column![
                    icons::icon(icons::DISC_SVG, 52.0, primary_col),
                    Space::with_height(6),
                    text(theme_cfg, "No Track Playing")
                        .size(19)
                        .color(theme_cfg.text_color()),
                    text(
                        theme_cfg,
                        "Select a track from the library to view animated art and lyrics"
                    )
                    .size(13)
                    .color(theme_cfg.text_muted_color()),
                ]
                .spacing(8)
                .align_x(Alignment::Center),
            )
            .padding(32)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(
                    0.06, 0.06, 0.10, 0.80,
                ))),
                border: iced::Border {
                    radius: Radius::from(16.0),
                    color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.10),
                    width: 1.0,
                },
                shadow: iced::Shadow {
                    color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.35),
                    offset: iced::Vector::new(0.0, 6.0),
                    blur_radius: 16.0,
                },
                text_color: Some(theme_cfg.text_color()),
            });

            container(empty_card)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        }
    };

    // Layer 0: Ambient moving blurred colors canvas
    // Layer 1: Glassmorphism interactive user interface
    stack![
        canvas(AmbientBackground {
            time: animation_time,
            palette: *palette,
        })
        .width(Length::Fill)
        .height(Length::Fill),
        container(content)
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Helper creating a rounded pill badge with semi-transparent background and border
fn meta_badge<'a>(
    theme_cfg: &'a ThemeConfig,
    label: String,
    accent_col: iced::Color,
) -> Element<'a, Message> {
    container(text(theme_cfg, label).size(10).color(accent_col))
        .padding([2, 7])
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                accent_col.r,
                accent_col.g,
                accent_col.b,
                0.12,
            ))),
            border: iced::Border {
                color: iced::Color::from_rgba(accent_col.r, accent_col.g, accent_col.b, 0.25),
                width: 1.0,
                radius: Radius::from(6.0),
            },
            ..Default::default()
        })
        .into()
}

/// Helper button style for the Up Next track card with hover illumination
fn next_track_btn_style(
    theme_cfg: &ThemeConfig,
    primary_col: iced::Color,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style + '_ {
    move |_, status| {
        let bg = match status {
            button::Status::Hovered => {
                iced::Color::from_rgba(primary_col.r, primary_col.g, primary_col.b, 0.18)
            }
            button::Status::Pressed => {
                iced::Color::from_rgba(primary_col.r, primary_col.g, primary_col.b, 0.28)
            }
            _ => iced::Color::from_rgba(1.0, 1.0, 1.0, 0.05),
        };
        let border_col = match status {
            button::Status::Hovered => {
                iced::Color::from_rgba(primary_col.r, primary_col.g, primary_col.b, 0.45)
            }
            button::Status::Pressed => primary_col,
            _ => iced::Color::from_rgba(1.0, 1.0, 1.0, 0.10),
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border {
                radius: Radius::from(10.0),
                color: border_col,
                width: 1.0,
            },
            text_color: theme_cfg.text_color(),
            shadow: iced::Shadow::default(),
        }
    }
}
