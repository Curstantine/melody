use std::time::Duration;

use iced::Task;
use model::TrackRow;

use crate::message::Message;
use config::{AppConfig, AppState};

use super::*;

impl App {
    pub(super) fn handle_persistence_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SaveConfig => {
                self.config_save_seq = self.config_save_seq.wrapping_add(1);
                self.save_config_task()
            }

            Message::DebouncedConfigSave(seq) => {
                if seq == self.config_save_seq {
                    self.save_config_task()
                } else {
                    Task::none()
                }
            }

            Message::ConfigSaved => Task::none(),

            Message::SaveState => {
                self.state_save_seq = self.state_save_seq.wrapping_add(1);
                self.save_state_task()
            }

            Message::DebouncedStateSave(seq) => {
                if seq == self.state_save_seq {
                    self.save_state_task()
                } else {
                    Task::none()
                }
            }

            Message::StateSaved => Task::none(),

            _ => unreachable!("routed in App::update"),
        }
    }

    pub fn to_config(&self) -> AppConfig {
        AppConfig {
            tab_order: self.tab_order.clone(),
            theme_name: self.theme_manager.active_theme_name.clone(),
            layout_name: self.layout_manager.active_layout_name.clone(),
            custom_titlebar: self.custom_titlebar,
            minimize_to_tray: self.minimize_to_tray,
            page_size: self.library.page_size,
            tile_width: self.albums.tile_width,
            album_tile_width: self.albums.tile_width,
            artist_tile_width: self.artists.tile_width,
            fonts: self.fonts.clone(),
            gapless: self.gapless,
            resampling: self.resampling_config.clone(),
            watch_library: self.watch_library,
            scrobble: self.scrobble_config.clone(),
        }
    }

    pub fn to_state(&self) -> AppState {
        let (queue_id, playing_seq) = if let Some(queue) = self.queues.active_queue() {
            (Some(queue.id), queue.current_index())
        } else if let Some(active_id) = self.queues.active {
            if let Some(summary) = self.queues.summary(active_id) {
                (
                    Some(summary.id),
                    if summary.track_count > 0 {
                        Some(summary.current)
                    } else {
                        None
                    },
                )
            } else {
                (Some(active_id), None)
            }
        } else {
            (None, None)
        };

        AppState {
            active_tab: self.active_tab,
            settings_tab: self.settings.active_tab,
            volume: self.volume,
            is_shuffle: self.is_shuffle,
            is_repeat: self.is_repeat,
            albums_view_mode: self.albums.view_mode,
            artists_view_mode: self.artists.view_mode,
            playlists_view_mode: self.playlists.view_mode,
            album_sort_field: self.albums.sort_field,
            artist_sort_field: self.artists.sort_field,
            playlist_sort_field: self.playlists.sort_field,
            window_width: self.window_size.width,
            window_height: self.window_size.height,
            queue_id,
            playing_seq,
        }
    }

    pub fn save_config_task(&self) -> Task<Message> {
        let config = self.to_config();
        let path = self.config_path;
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    config.save_to_path(path);
                })
                .await
                .ok();
            },
            |_| Message::ConfigSaved,
        )
    }

    pub fn save_config_debounced(&mut self) -> Task<Message> {
        self.config_save_seq = self.config_save_seq.wrapping_add(1);
        let seq = self.config_save_seq;
        Task::perform(
            async move {
                tokio::time::sleep(Duration::from_millis(400)).await;
                seq
            },
            Message::DebouncedConfigSave,
        )
    }

    pub fn save_state_task(&self) -> Task<Message> {
        let state = self.to_state();
        let path = self.state_path;
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    state.save_to_path(path);
                })
                .await
                .ok();
            },
            |_| Message::StateSaved,
        )
    }

    pub fn save_state_debounced(&mut self) -> Task<Message> {
        self.state_save_seq = self.state_save_seq.wrapping_add(1);
        let seq = self.state_save_seq;
        Task::perform(
            async move {
                tokio::time::sleep(Duration::from_millis(400)).await;
                seq
            },
            Message::DebouncedStateSave,
        )
    }

    /// Refreshes thumbnail-cache and database-size telemetry off the UI thread.
    /// Called only when the relevant screens/actions occur instead of every frame.
    pub(super) fn load_storage_stats_task(&self) -> Task<Message> {
        let db_path = self.db_path;
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let stats = thumbnail::get_thumbnail_cache_stats();
                    let db_size = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);
                    (stats, db_size)
                })
                .await
                .unwrap_or_default()
            },
            |(stats, db_size)| Message::StorageStatsLoaded(stats, db_size),
        )
    }

    pub(super) fn ensure_fonts_task(&mut self) -> Option<Task<Message>> {
        if !self.settings.fonts_loaded {
            self.settings.fonts_loaded = true;
            Some(Task::perform(
                async move {
                    tokio::task::spawn_blocking(fonts::list_font_families)
                        .await
                        .unwrap_or_default()
                },
                Message::FontsLoaded,
            ))
        } else {
            None
        }
    }

    pub(super) fn find_track_row(&self, track_id: i64) -> Option<TrackRow> {
        if let Some(row) = self.library.rows.iter().find(|t| t.id == track_id) {
            return Some(row.clone());
        }
        if let Some(row) = self
            .queues
            .active_queue()
            .and_then(|q| q.tracks.iter().find(|t| t.id == track_id))
        {
            return Some(row.clone());
        }
        if let Some(row) = self
            .playlists
            .detail_tracks
            .iter()
            .find(|t| t.id == track_id)
        {
            return Some(row.clone());
        }
        if let Some(track) = &self.current_track {
            if track.id == track_id {
                return Some(TrackRow::from(track));
            }
        }
        if let Some(item) = self
            .playlists
            .detail_items
            .iter()
            .find(|i| i.track_id == Some(track_id))
        {
            return Some(TrackRow {
                id: track_id,
                title: item.title.clone(),
                artist: item.artist.as_str().into(),
                album: "".into(),
                duration_ms: 0,
                file_path: String::new(),
                duration_label: item.duration_label.clone(),
                year: None,
                track_num: None,
                disc_num: None,
                album_artist: None,
                genre: None,
            });
        }
        None
    }
}
