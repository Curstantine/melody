use iced::widget::{
    button, column, container, image, mouse_area, row, scrollable, text_input, Space, Widget,
};
use iced::{Alignment, Background, Border, Color, ContentFit, Element, Length};

use component::{self, icons, styles, text};
use model::{format_template, PlaylistSortField, ViewMode};
use theme::ThemeConfig;

use crate::message::Message;

use super::*;

impl PlaylistsView {
    // ── Playlist catalog ────────────────────────────────────────────────────

    pub(super) fn view_catalog<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        playlists_fmt: &'a layout::PlaylistsFormatConfig,
    ) -> Element<'a, Message> {
        if self.creating {
            return self.view_new_playlist_form(theme_cfg, font);
        }

        if self.playlists.is_empty() {
            return component::empty_state(
                theme_cfg,
                font,
                "No playlists yet",
                "Import an existing .m3u / .m3u8 file or create a new playlist",
            );
        }

        let header = row![
            component::sort_chip_list(
                theme_cfg,
                font,
                &[
                    ("Name", PlaylistSortField::Name),
                    ("Tracks", PlaylistSortField::TrackCount),
                    ("Date added", PlaylistSortField::DateAdded),
                ],
                self.sort_field,
                self.sort_dir,
                Message::PlaylistSortChanged,
            ),
            Space::new().width(Length::Fill),
            component::view_mode_toggle(
                theme_cfg,
                font,
                self.view_mode,
                Message::SetPlaylistsViewMode,
            ),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 6.0,
            left: 16.0,
        });

        let content = match self.view_mode {
            ViewMode::List => self.view_catalog_list(theme_cfg, font, playlists_fmt),
            ViewMode::Tile => self.view_catalog_tiles(theme_cfg, font, playlists_fmt),
        };

        column![header, content]
            .spacing(2)
            .height(Length::Fill)
            .width(Length::Fill)
            .boxed()
    }

    pub(super) fn view_catalog_list<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        playlists_fmt: &'a layout::PlaylistsFormatConfig,
    ) -> Element<'a, Message> {
        let mut list = column![].spacing(2).width(Length::Fill);

        for idx in self.sorted_indices() {
            let playlist = &self.playlists[idx];
            let is_selected = self.selected_id == Some(playlist.id);

            let (line1, line2, details_label) = if let Some(cached) =
                self.rendered_playlists.get(idx)
            {
                (
                    std::borrow::Cow::Borrowed(cached.list_line1.as_str()),
                    std::borrow::Cow::Borrowed(cached.list_line2.as_str()),
                    std::borrow::Cow::Borrowed(cached.list_details.as_str()),
                )
            } else {
                (
                    std::borrow::Cow::Owned(format_template(&playlists_fmt.list_line1, playlist)),
                    std::borrow::Cow::Owned(format_template(&playlists_fmt.list_line2, playlist)),
                    std::borrow::Cow::Owned(format_template(&playlists_fmt.list_details, playlist)),
                )
            };

            let subtitle = text(font, line2).size(11).color(theme_cfg.text_muted);

            let title_block = column![text(font, line1).size(13).color(theme_cfg.text), subtitle,]
                .spacing(2)
                .width(Length::Fill);

            let details = column![
                text(font, details_label)
                    .size(12)
                    .color(theme_cfg.text_muted),
                text(
                    font,
                    if playlist.managed {
                        "Managed"
                    } else {
                        "External"
                    },
                )
                .size(10)
                .color(theme_cfg.text_muted),
            ]
            .spacing(2)
            .align_x(Alignment::End)
            .width(90);

            let row_content = row![title_block, details]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding(iced::Padding {
                    top: 4.0,
                    right: 8.0,
                    bottom: 4.0,
                    left: 16.0,
                });

            let open_row = mouse_area(
                button(row_content)
                    .style(styles::tile(theme_cfg))
                    .width(Length::Fill)
                    .padding(0)
                    .on_press(Message::SelectPlaylist(playlist.id)),
            )
            .on_right_press(Message::OpenPlaylistContextMenu(playlist.clone()));

            let play_btn = button(icons::icon(icons::PLAY_SVG, 12.0, theme_cfg.primary))
                .style(styles::icon_control_button(theme_cfg))
                .padding([4, 6])
                .on_press(Message::PlayPlaylist(playlist.id));

            let delete_btn = button(icons::icon(icons::TRASH_SVG, 12.0, theme_cfg.text_muted))
                .style(styles::icon_control_button(theme_cfg))
                .padding([4, 6])
                .on_press(Message::DeletePlaylist(playlist.id));

            // The highlight wraps the whole row so the background spans the
            // content and the action buttons.
            list = list.push(
                container(
                    row![open_row, play_btn, delete_btn]
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .width(Length::Fill)
                        .padding(iced::Padding {
                            top: 0.0,
                            right: 12.0,
                            bottom: 0.0,
                            left: 0.0,
                        }),
                )
                .style(styles::table_row_container(theme_cfg, is_selected))
                .width(Length::Fill)
                .boxed(),
            );
        }

        let scroll_offset = &self.last_playlists_scroll;
        scrollable(list)
            .id(iced::widget::Id::new("playlists_scroll"))
            .on_scroll(move |scroll| {
                scroll_offset.set(scroll.viewport.absolute_offset().y);
                None::<Message>
            })
            .height(Length::Fill)
            .style(styles::scrollable_style(theme_cfg))
            .boxed()
    }

    pub(super) fn view_catalog_tiles<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        playlists_fmt: &'a layout::PlaylistsFormatConfig,
    ) -> Element<'a, Message> {
        let mut grid = column![].spacing(12).width(Length::Fill);
        let mut tile_row = row![].spacing(12).align_y(Alignment::Start);
        let mut columns = 0;

        for idx in self.sorted_indices() {
            let playlist = &self.playlists[idx];
            let covers = self.playlist_covers.get(&playlist.id);

            let icon = render_playlist_tile_cover(
                theme_cfg,
                covers,
                PLAYLIST_TILE_SIZE,
                Message::SelectPlaylist(playlist.id),
            );

            let (line1, line2) = if let Some(cached) = self.rendered_playlists.get(idx) {
                (
                    std::borrow::Cow::Borrowed(cached.tile_line1.as_str()),
                    std::borrow::Cow::Borrowed(cached.tile_line2.as_str()),
                )
            } else {
                (
                    std::borrow::Cow::Owned(format_template(&playlists_fmt.tile_line1, playlist)),
                    std::borrow::Cow::Owned(format_template(&playlists_fmt.tile_line2, playlist)),
                )
            };

            let meta = container(
                column![
                    text(font, line1).size(12),
                    text(font, line2).size(10).color(theme_cfg.text_muted),
                ]
                .spacing(3)
                .padding(6),
            )
            .width(PLAYLIST_TILE_SIZE)
            .height(48);

            let meta_btn = button(meta)
                .style(styles::tile(theme_cfg))
                .padding(0)
                .on_press(Message::SelectPlaylist(playlist.id));

            let play_btn = button(icons::icon(icons::PLAY_SVG, 14.0, theme_cfg.text))
                .style(styles::icon_control_button(theme_cfg))
                .padding(4)
                .on_press(Message::PlayPlaylist(playlist.id));

            let delete_btn = button(icons::icon(icons::TRASH_SVG, 14.0, theme_cfg.text))
                .style(styles::icon_control_button(theme_cfg))
                .padding(4)
                .on_press(Message::DeletePlaylist(playlist.id));

            // Action buttons overlay the cover's bottom-right corner, just
            // like the album tile: the actions container is sized to the
            // cover and stacked above the select button.
            let actions = container(row![play_btn, delete_btn].spacing(4))
                .width(PLAYLIST_TILE_SIZE)
                .height(PLAYLIST_TILE_SIZE)
                .align_x(Alignment::End)
                .align_y(Alignment::End)
                .padding(6);

            let cover_block = iced::widget::stack([icon, actions.boxed()]);

            let card_content = column![cover_block, meta_btn].width(PLAYLIST_TILE_SIZE);

            let tile = mouse_area(card_content)
                .on_right_press(Message::OpenPlaylistContextMenu(playlist.clone()));

            tile_row = tile_row.push(tile);
            columns += 1;
            if columns == 5 {
                grid = grid.push(tile_row);
                tile_row = row![].spacing(12);
                columns = 0;
            }
        }
        if columns > 0 {
            grid = grid.push(tile_row);
        }

        let scroll_offset = &self.last_playlists_scroll;
        scrollable(container(grid).width(Length::Fill).padding(iced::Padding {
            left: 16.0,
            right: 16.0,
            ..Default::default()
        }))
        .id(iced::widget::Id::new("playlists_scroll"))
        .on_scroll(move |scroll| {
            scroll_offset.set(scroll.viewport.absolute_offset().y);
            None::<Message>
        })
        .height(Length::Fill)
        .style(styles::scrollable_style(theme_cfg))
        .boxed()
    }

    pub(super) fn view_new_playlist_form<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        let input = text_input("New playlist name...", &self.new_name_input)
            .size(13)
            .padding([7, 10])
            .style(styles::search_input_style(theme_cfg))
            .on_input(Message::NewPlaylistNameChanged)
            .on_submit(Message::NewPlaylistNameSubmitted(
                self.new_name_input.clone(),
            ))
            .width(Length::Fill);

        let create = button(text(font, "Create").size(12).color(theme_cfg.text))
            .padding([7, 16])
            .style(styles::ghost_button(theme_cfg, false))
            .on_press(Message::NewPlaylistNameSubmitted(
                self.new_name_input.clone(),
            ));

        let cancel = button(text(font, "Cancel").size(12).color(theme_cfg.text_muted))
            .padding([7, 16])
            .style(styles::ghost_button(theme_cfg, false))
            .on_press(Message::NewPlaylistCancelled);

        container(
            column![
                text(font, "Create a new playlist")
                    .size(16)
                    .color(theme_cfg.text),
                text(
                    font,
                    "A new .m3u8 file will be created in the Melody playlists directory."
                )
                .size(11)
                .color(theme_cfg.text_muted),
                Space::new().height(8),
                row![
                    input,
                    Space::new().width(8),
                    create,
                    Space::new().width(4),
                    cancel
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(6),
        )
        .padding(24)
        .width(Length::Fill)
        .boxed()
    }

    // ── Playlist detail ─────────────────────────────────────────────────────
}

/// Renders either a 2x2 grid of track album covers, a single full-size cover,
/// or a fallback icon inside the playlist tile.
pub(super) fn render_playlist_tile_cover<'a>(
    theme_cfg: &'a ThemeConfig,
    covers: Option<&Vec<image::Handle>>,
    size: f32,
    on_press: Message,
) -> Element<'a, Message> {
    let radius = theme_cfg.border_radius.md;
    let border_color = theme_cfg.border;

    match covers {
        Some(list) if list.len() >= 4 => {
            let half = (size / 2.0).floor();
            let mut top_row = row![].spacing(0);
            top_row = top_row.push(
                image(list[0].clone())
                    .width(Length::Fixed(half))
                    .height(Length::Fixed(half))
                    .content_fit(ContentFit::Cover),
            );
            top_row = top_row.push(
                image(list[1].clone())
                    .width(Length::Fixed(half))
                    .height(Length::Fixed(half))
                    .content_fit(ContentFit::Cover),
            );

            let mut bottom_row = row![].spacing(0);
            bottom_row = bottom_row.push(
                image(list[2].clone())
                    .width(Length::Fixed(half))
                    .height(Length::Fixed(half))
                    .content_fit(ContentFit::Cover),
            );
            bottom_row = bottom_row.push(
                image(list[3].clone())
                    .width(Length::Fixed(half))
                    .height(Length::Fixed(half))
                    .content_fit(ContentFit::Cover),
            );

            let grid_content = column![top_row, bottom_row].spacing(0);
            let image_layer = container(grid_content)
                .width(size)
                .height(size)
                .clip(true)
                .style(move |_theme| container::Style {
                    border: Border {
                        radius,
                        color: border_color,
                        width: 1.0,
                    },
                    ..Default::default()
                });

            let overlay_btn = button(Space::new().width(Length::Fill).height(Length::Fill))
                .width(size)
                .height(size)
                .style(move |_theme, status| {
                    let bg = match status {
                        button::Status::Active => Color::TRANSPARENT,
                        button::Status::Hovered | button::Status::Pressed => {
                            Color::from_rgba(0.0, 0.0, 0.0, 0.5)
                        }
                        button::Status::Disabled => Color::TRANSPARENT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        border: Border {
                            radius,
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(on_press);

            iced::widget::stack([image_layer.boxed(), overlay_btn.boxed()]).boxed()
        }
        Some(list) if !list.is_empty() => {
            // 1 to 3 covers: render first cover full size (classic music player behavior)
            component::catalog_cover_art(
                theme_cfg,
                Some(&list[0]),
                size,
                radius,
                icons::PLAYLIST_SVG,
                size * 0.34,
                theme_cfg.primary,
                on_press,
            )
        }
        _ => component::catalog_cover_art(
            theme_cfg,
            None,
            size,
            radius,
            icons::PLAYLIST_SVG,
            size * 0.34,
            theme_cfg.primary,
            on_press,
        ),
    }
}
