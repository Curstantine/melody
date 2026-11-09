use std::sync::Arc;

use iced::widget::{button, column, container, mouse_area, row, Space, Widget};
use iced::{Alignment, Background, Color, Element, Length, Point, Size};
use model::{AlbumSummary, ArtistSummary, PlaylistSummary, TrackRow};

use crate::icons;
use crate::styles;
use crate::text;
use theme::ThemeConfig;

pub const CONTEXT_MENU_WIDTH: f32 = 220.0;
pub const SUBMENU_WIDTH: f32 = 200.0;
pub const ITEM_HEIGHT: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackMenuContext {
    Library,
    Playlist { playlist_id: i64, index: usize },
    Queue { queue_id: u64, index: usize },
    PlayerBar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextMenuSubmenu {
    Playlists,
}

#[derive(Clone, Debug)]
pub struct ContextMenuState {
    pub position: Point,
    pub target: ContextMenuTarget,
    pub submenu: Option<ContextMenuSubmenu>,
}

#[derive(Clone, Debug)]
pub enum ContextMenuTarget {
    Track {
        track: TrackRow,
        context: TrackMenuContext,
    },
    Album(AlbumSummary),
    Artist(ArtistSummary),
    Playlist(PlaylistSummary),
    Queue {
        id: u64,
        name: String,
    },
}

#[derive(Clone, Debug)]
pub enum ContextMenuAction {
    Close,
    ToggleSubmenu(Option<ContextMenuSubmenu>),
    // Track actions
    PlayTrack(i64),
    PlayPlaylistItem {
        playlist_id: i64,
        index: usize,
    },
    PlayNextTrack(TrackRow),
    AddTrackToQueue(TrackRow),
    GoToAlbum(Arc<str>),
    GoToArtist(Arc<str>),
    RemoveFromPlaylist {
        playlist_id: i64,
        index: usize,
    },
    RemoveFromQueue {
        queue_id: u64,
        index: usize,
    },
    // Album actions
    PlayAlbum {
        album: Arc<str>,
        shuffle: bool,
    },
    PlayNextAlbum(AlbumSummary),
    AddAlbumToQueue(AlbumSummary),
    // Artist actions
    PlayArtist {
        artist: Arc<str>,
        shuffle: bool,
    },
    PlayNextArtist(ArtistSummary),
    AddArtistToQueue(ArtistSummary),
    // Playlist actions
    PlayPlaylist(i64),
    PlayNextPlaylist(i64),
    AddPlaylistToQueue(i64),
    RenamePlaylist(i64),
    DeletePlaylist(i64),
    // Queue actions
    PlayQueue(u64),
    DeleteQueue(u64),
    // Submenu actions
    NewPlaylistWithTrack(TrackRow),
    NewPlaylistWithAlbum(AlbumSummary),
    NewPlaylistWithArtist(ArtistSummary),
    AddTrackToPlaylist {
        track: TrackRow,
        playlist_id: i64,
    },
    AddAlbumToPlaylist {
        album: AlbumSummary,
        playlist_id: i64,
    },
    AddArtistToPlaylist {
        artist: ArtistSummary,
        playlist_id: i64,
    },
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{truncated}…")
    } else {
        s.to_string()
    }
}

fn menu_header<'a, M: 'a>(theme_cfg: &'a ThemeConfig, label: &str) -> Element<'a, M> {
    container(
        text(theme_cfg, truncate_str(label, 26))
            .size(11)
            .wrapping(iced::widget::text::Wrapping::None)
            .color(theme_cfg.text_muted),
    )
    .padding([3, 8])
    .width(Length::Fill)
    .boxed()
}

fn menu_separator<'a, M: 'a>(theme_cfg: &'a ThemeConfig) -> Element<'a, M> {
    container("")
        .width(Length::Fill)
        .height(1)
        .style(move |_| container::Style {
            background: Some(Background::Color(theme_cfg.border)),
            ..Default::default()
        })
        .boxed()
}

fn menu_item<'a, M: Clone + 'static>(
    theme_cfg: &'a ThemeConfig,
    icon_svg: &'static str,
    label: impl Into<String>,
    on_press: M,
    is_danger: bool,
    has_submenu: bool,
) -> Element<'a, M> {
    let icon_color = if is_danger {
        Color::from_rgb(0.95, 0.35, 0.35)
    } else {
        theme_cfg.text_muted
    };

    let mut content = row![
        icons::icon(icon_svg, 12.0, icon_color),
        Space::new().width(6),
        text(theme_cfg, label.into())
            .size(12)
            .wrapping(iced::widget::text::Wrapping::None),
    ]
    .align_y(Alignment::Center)
    .width(Length::Fill);

    if has_submenu {
        content = content
            .push(Space::new().width(Length::Fill).boxed())
            .push(icons::icon(icons::CHEVRON_RIGHT_SVG, 11.0, theme_cfg.text_muted).boxed());
    }

    button(content)
        .padding([3, 8])
        .height(Length::Fixed(ITEM_HEIGHT))
        .width(Length::Fill)
        .style(styles::context_menu_item(theme_cfg, is_danger))
        .on_press(on_press)
        .boxed()
}

pub fn render_context_menu<'a, M: Clone + 'static>(
    menu: &'a ContextMenuState,
    playlists: &'a [PlaylistSummary],
    theme_cfg: &'a ThemeConfig,
    window_size: Size,
    on_action: impl Fn(ContextMenuAction) -> M + Copy + 'a,
) -> Element<'a, M> {
    // 1. Full-screen transparent backdrop to catch clicks outside the menu
    let backdrop = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(on_action(ContextMenuAction::Close))
    .on_right_press(on_action(ContextMenuAction::Close));

    // 2. Build the main menu items
    let mut main_items = column![]
        .spacing(1)
        .width(Length::Fixed(CONTEXT_MENU_WIDTH));

    match &menu.target {
        ContextMenuTarget::Track { track, context } => {
            main_items = main_items.push(menu_header(theme_cfg, &track.title));
            main_items = main_items.push(menu_separator(theme_cfg));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::PLAY_SVG,
                "Play",
                on_action(match context {
                    TrackMenuContext::Playlist { playlist_id, index } => {
                        ContextMenuAction::PlayPlaylistItem {
                            playlist_id: *playlist_id,
                            index: *index,
                        }
                    }
                    _ => ContextMenuAction::PlayTrack(track.id),
                }),
                false,
                false,
            ));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::NEXT_SVG,
                "Play next",
                on_action(ContextMenuAction::PlayNextTrack(track.clone())),
                false,
                false,
            ));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::QUEUE_SVG,
                "Add to queue",
                on_action(ContextMenuAction::AddTrackToQueue(track.clone())),
                false,
                false,
            ));

            let submenu_toggle = if menu.submenu == Some(ContextMenuSubmenu::Playlists) {
                ContextMenuAction::ToggleSubmenu(None)
            } else {
                ContextMenuAction::ToggleSubmenu(Some(ContextMenuSubmenu::Playlists))
            };
            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::PLAYLIST_SVG,
                "Add to playlist",
                on_action(submenu_toggle),
                false,
                true,
            ));

            if !track.album.is_empty() {
                main_items = main_items.push(menu_separator(theme_cfg));
                main_items = main_items.push(menu_item(
                    theme_cfg,
                    icons::ALBUM_SVG,
                    format!("Go to album \"{}\"", truncate_str(&track.album, 14)),
                    on_action(ContextMenuAction::GoToAlbum(track.album.clone())),
                    false,
                    false,
                ));
            }

            if !track.artist.is_empty() {
                main_items = main_items.push(menu_item(
                    theme_cfg,
                    icons::USER_AVATAR_SVG,
                    format!("Go to artist \"{}\"", truncate_str(&track.artist, 14)),
                    on_action(ContextMenuAction::GoToArtist(track.artist.clone())),
                    false,
                    false,
                ));
            }

            match context {
                TrackMenuContext::Playlist { playlist_id, index } => {
                    main_items = main_items.push(menu_separator(theme_cfg));
                    main_items = main_items.push(menu_item(
                        theme_cfg,
                        icons::TRASH_SVG,
                        "Remove from playlist",
                        on_action(ContextMenuAction::RemoveFromPlaylist {
                            playlist_id: *playlist_id,
                            index: *index,
                        }),
                        true,
                        false,
                    ));
                }
                TrackMenuContext::Queue { queue_id, index } => {
                    main_items = main_items.push(menu_separator(theme_cfg));
                    main_items = main_items.push(menu_item(
                        theme_cfg,
                        icons::TRASH_SVG,
                        "Remove from queue",
                        on_action(ContextMenuAction::RemoveFromQueue {
                            queue_id: *queue_id,
                            index: *index,
                        }),
                        true,
                        false,
                    ));
                }
                _ => {}
            }
        }
        ContextMenuTarget::Album(album) => {
            main_items = main_items.push(menu_header(theme_cfg, &album.album));
            main_items = main_items.push(menu_separator(theme_cfg));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::PLAY_SVG,
                "Play album",
                on_action(ContextMenuAction::PlayAlbum {
                    album: album.album.clone(),
                    shuffle: false,
                }),
                false,
                false,
            ));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::SHUFFLE_SVG,
                "Shuffle album",
                on_action(ContextMenuAction::PlayAlbum {
                    album: album.album.clone(),
                    shuffle: true,
                }),
                false,
                false,
            ));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::NEXT_SVG,
                "Play next",
                on_action(ContextMenuAction::PlayNextAlbum(album.clone())),
                false,
                false,
            ));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::QUEUE_SVG,
                "Add album to queue",
                on_action(ContextMenuAction::AddAlbumToQueue(album.clone())),
                false,
                false,
            ));

            let submenu_toggle = if menu.submenu == Some(ContextMenuSubmenu::Playlists) {
                ContextMenuAction::ToggleSubmenu(None)
            } else {
                ContextMenuAction::ToggleSubmenu(Some(ContextMenuSubmenu::Playlists))
            };
            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::PLAYLIST_SVG,
                "Add to playlist",
                on_action(submenu_toggle),
                false,
                true,
            ));

            if !album.artist.is_empty() {
                main_items = main_items.push(menu_separator(theme_cfg));
                main_items = main_items.push(menu_item(
                    theme_cfg,
                    icons::USER_AVATAR_SVG,
                    format!("Go to artist \"{}\"", truncate_str(&album.artist, 14)),
                    on_action(ContextMenuAction::GoToArtist(album.artist.clone())),
                    false,
                    false,
                ));
            }
        }
        ContextMenuTarget::Artist(artist) => {
            main_items = main_items.push(menu_header(theme_cfg, &artist.artist));
            main_items = main_items.push(menu_separator(theme_cfg));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::PLAY_SVG,
                "Play artist",
                on_action(ContextMenuAction::PlayArtist {
                    artist: artist.artist.clone(),
                    shuffle: false,
                }),
                false,
                false,
            ));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::SHUFFLE_SVG,
                "Shuffle artist",
                on_action(ContextMenuAction::PlayArtist {
                    artist: artist.artist.clone(),
                    shuffle: true,
                }),
                false,
                false,
            ));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::NEXT_SVG,
                "Play next",
                on_action(ContextMenuAction::PlayNextArtist(artist.clone())),
                false,
                false,
            ));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::QUEUE_SVG,
                "Add artist to queue",
                on_action(ContextMenuAction::AddArtistToQueue(artist.clone())),
                false,
                false,
            ));

            let submenu_toggle = if menu.submenu == Some(ContextMenuSubmenu::Playlists) {
                ContextMenuAction::ToggleSubmenu(None)
            } else {
                ContextMenuAction::ToggleSubmenu(Some(ContextMenuSubmenu::Playlists))
            };
            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::PLAYLIST_SVG,
                "Add to playlist",
                on_action(submenu_toggle),
                false,
                true,
            ));
        }
        ContextMenuTarget::Playlist(playlist) => {
            main_items = main_items.push(menu_header(theme_cfg, &playlist.name));
            main_items = main_items.push(menu_separator(theme_cfg));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::PLAY_SVG,
                "Play playlist",
                on_action(ContextMenuAction::PlayPlaylist(playlist.id)),
                false,
                false,
            ));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::NEXT_SVG,
                "Play next",
                on_action(ContextMenuAction::PlayNextPlaylist(playlist.id)),
                false,
                false,
            ));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::QUEUE_SVG,
                "Add to queue",
                on_action(ContextMenuAction::AddPlaylistToQueue(playlist.id)),
                false,
                false,
            ));

            main_items = main_items.push(menu_separator(theme_cfg));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::SETTINGS_SVG,
                "Rename playlist",
                on_action(ContextMenuAction::RenamePlaylist(playlist.id)),
                false,
                false,
            ));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::TRASH_SVG,
                "Delete playlist",
                on_action(ContextMenuAction::DeletePlaylist(playlist.id)),
                true,
                false,
            ));
        }
        ContextMenuTarget::Queue { id, name } => {
            main_items = main_items.push(menu_header(theme_cfg, name));
            main_items = main_items.push(menu_separator(theme_cfg));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::PLAY_SVG,
                "Play queue",
                on_action(ContextMenuAction::PlayQueue(*id)),
                false,
                false,
            ));

            main_items = main_items.push(menu_separator(theme_cfg));

            main_items = main_items.push(menu_item(
                theme_cfg,
                icons::TRASH_SVG,
                "Delete queue",
                on_action(ContextMenuAction::DeleteQueue(*id)),
                true,
                false,
            ));
        }
    }

    let main_box = container(main_items)
        .padding([4, 4])
        .style(styles::context_menu_container(theme_cfg));

    // 3. Build submenu (if active)
    let submenu_box: Option<Element<'a, M>> = match menu.submenu {
        Some(ContextMenuSubmenu::Playlists) => {
            let mut sub_items = column![].spacing(1).width(Length::Fixed(SUBMENU_WIDTH));
            sub_items = sub_items.push(menu_header(theme_cfg, "Playlists"));
            sub_items = sub_items.push(menu_separator(theme_cfg));

            let new_playlist_action = match &menu.target {
                ContextMenuTarget::Track { track, .. } => {
                    ContextMenuAction::NewPlaylistWithTrack(track.clone())
                }
                ContextMenuTarget::Album(album) => {
                    ContextMenuAction::NewPlaylistWithAlbum(album.clone())
                }
                ContextMenuTarget::Artist(artist) => {
                    ContextMenuAction::NewPlaylistWithArtist(artist.clone())
                }
                _ => ContextMenuAction::Close,
            };

            sub_items = sub_items.push(menu_item(
                theme_cfg,
                icons::PLUS_SVG,
                "New playlist...",
                on_action(new_playlist_action),
                false,
                false,
            ));

            if !playlists.is_empty() {
                sub_items = sub_items.push(menu_separator(theme_cfg));
                for p in playlists {
                    let action = match &menu.target {
                        ContextMenuTarget::Track { track, .. } => {
                            ContextMenuAction::AddTrackToPlaylist {
                                track: track.clone(),
                                playlist_id: p.id,
                            }
                        }
                        ContextMenuTarget::Album(album) => ContextMenuAction::AddAlbumToPlaylist {
                            album: album.clone(),
                            playlist_id: p.id,
                        },
                        ContextMenuTarget::Artist(artist) => {
                            ContextMenuAction::AddArtistToPlaylist {
                                artist: artist.clone(),
                                playlist_id: p.id,
                            }
                        }
                        _ => ContextMenuAction::Close,
                    };

                    sub_items = sub_items.push(menu_item(
                        theme_cfg,
                        icons::PLAYLIST_SVG,
                        format!("{} ({})", truncate_str(&p.name, 14), p.track_count),
                        on_action(action),
                        false,
                        false,
                    ));
                }
            } else {
                sub_items = sub_items.push(menu_separator(theme_cfg));
                sub_items = sub_items.push(
                    container(
                        text(theme_cfg, "No playlists yet")
                            .size(11)
                            .wrapping(iced::widget::text::Wrapping::None)
                            .color(theme_cfg.text_muted),
                    )
                    .padding([5, 8])
                    .boxed(),
                );
            }

            Some(
                container(sub_items)
                    .padding([4, 4])
                    .style(styles::context_menu_container(theme_cfg))
                    .boxed(),
            )
        }
        None => None,
    };

    // 4. Compute clamped coordinates so context menu is never off-screen
    let has_sub = submenu_box.is_some();
    let total_width = if has_sub {
        CONTEXT_MENU_WIDTH + SUBMENU_WIDTH + 6.0
    } else {
        CONTEXT_MENU_WIDTH
    };

    let px = menu.position.x;
    let py = menu.position.y;

    let (render_sub_left, clamped_x) = if has_sub {
        if px + total_width > window_size.width - 16.0 {
            if px - SUBMENU_WIDTH - 6.0 >= 8.0 {
                (true, (px - CONTEXT_MENU_WIDTH).max(8.0))
            } else {
                (false, (window_size.width - total_width - 16.0).max(8.0))
            }
        } else {
            (false, px.max(8.0))
        }
    } else {
        (
            false,
            px.min(window_size.width - CONTEXT_MENU_WIDTH - 16.0)
                .max(8.0),
        )
    };

    let estimated_height = 280.0;
    let clamped_y = py.min(window_size.height - estimated_height).max(8.0);

    // 5. Build combined menu row (or with submenu)
    let menu_cluster = match submenu_box {
        Some(sub) => if render_sub_left {
            row![sub, Space::new().width(4), main_box.boxed()]
        } else {
            row![main_box.boxed(), Space::new().width(4), sub]
        }
        .align_y(Alignment::Start),
        None => row![main_box.boxed()],
    };

    let positioned_menu = column![
        Space::new().height(clamped_y),
        row![Space::new().width(clamped_x), menu_cluster],
    ];

    iced::widget::stack([backdrop.boxed(), positioned_menu.boxed()]).boxed()
}
