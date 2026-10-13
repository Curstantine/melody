use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use core_engine::{AlbumSummary, SortDirection, TrackFilter};
use iced::border::Radius;
use iced::widget::{button, column, container, image, row, scrollable, Space};
use iced::{Alignment, Element, Length, Padding};

use crate::components::{self, text};
use crate::icons;
use crate::message::{AlbumSortField, Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

#[allow(dead_code)]
pub const DEFAULT_ALBUM_TILE_WIDTH: u32 = 140;
/// Minimum number of decoded album cover textures retained in RAM/VRAM.
/// The effective cap is `max(MAX_ALBUM_COVERS, window_len)` so covers currently
/// rendered by the virtualized window are never evicted.
pub const MAX_ALBUM_COVERS: usize = 120;

/// Fixed pixel height of a single list row.
pub const LIST_ROW_HEIGHT: f32 = 52.0;
/// Fixed height of the metadata block under a grid tile.
pub const TILE_META_HEIGHT: f32 = 64.0;
/// Vertical gap between catalog rows (baked into the virtualized row height).
pub const ROW_SPACING: f32 = 14.0;

const DEFAULT_COLUMNS: usize = 6;
const DEFAULT_VIEWPORT_HEIGHT: f32 = 720.0;
const HORIZONTAL_PADDING: f32 = 32.0;

/// Cache key used for album covers. Uses a unit-separator that cannot appear in
/// metadata so that `("A:B", "C")` and `("A", "B:C")` do not collide.
fn cover_key(album: &AlbumSummary) -> String {
    format!("{}\u{1f}{}", album.artist, album.album)
}

/// Computes how many tiles fit in a viewport of `width` pixels.
fn columns_for_width(width: f32, tile_width: u32) -> usize {
    let usable = (width - HORIZONTAL_PADDING).max(tile_width as f32);
    let tile = tile_width as f32;
    (((usable + ROW_SPACING) / (tile + ROW_SPACING)).floor() as usize).max(1)
}

/// Self-contained state and view component for the Albums catalog screen.
///
/// The full catalog is held in memory as lightweight summaries, and only the
/// visible window of tiles/rows is materialized. Spacer widgets above and below
/// the window keep the scrollbar proportional to the whole catalog, so there is
/// no incremental "load more" pagination.
///
/// TODO(virtualization): the virtualized album list is still finicky about
/// album covers. Fast scrolling can leave a subset of tiles on the placeholder
/// until the periodic self-heal pass catches up, and the cover pipeline relies
/// on request-id ownership plus in-flight expiry rather than real cancellation.
/// This needs a proper per-request cancellation/priority scheme to be solid.
pub struct AlbumsView {
    pub view_mode: ViewMode,
    pub tile_width: u32,
    pub albums: Vec<AlbumSummary>,
    pub album_covers: HashMap<String, image::Handle>,
    /// Recency order of cached covers: front = least recently used, back = most recently used.
    pub cover_order: VecDeque<String>,
    pub sort_field: AlbumSortField,
    pub sort_dir: SortDirection,
    /// Inclusive start index of the materialized window.
    pub window_start: usize,
    /// Exclusive end index of the materialized window.
    pub window_end: usize,
    /// Tiles per row in tile mode.
    pub columns: usize,
    pub viewport_height: f32,
    last_viewport_width: f32,
    last_scroll_y: f32,
    /// Cover keys with an in-flight request, mapped to the id of the request
    /// that owns them and the instant it started. Ownership prevents results
    /// from an abandoned request from corrupting a newer request for the same
    /// album; the timestamp lets error/abandoned requests expire.
    pub in_flight: HashMap<String, (u64, Instant)>,
    pub failed: HashSet<String>,
    next_request_id: u64,
    /// Precomputed catalog totals so the status bar never folds over the whole
    /// catalog on every frame.
    pub total_tracks: usize,
    pub total_duration_ms: u64,
}

impl AlbumsView {
    #[allow(dead_code)]
    pub fn new(view_mode: ViewMode, sort_field: AlbumSortField) -> Self {
        Self::with_tile_width(view_mode, sort_field, DEFAULT_ALBUM_TILE_WIDTH)
    }

    pub fn with_tile_width(
        view_mode: ViewMode,
        sort_field: AlbumSortField,
        tile_width: u32,
    ) -> Self {
        Self {
            view_mode,
            tile_width,
            albums: Vec::new(),
            album_covers: HashMap::new(),
            cover_order: VecDeque::new(),
            sort_field,
            sort_dir: SortDirection::Ascending,
            window_start: 0,
            window_end: 0,
            columns: DEFAULT_COLUMNS,
            viewport_height: DEFAULT_VIEWPORT_HEIGHT,
            last_viewport_width: 0.0,
            last_scroll_y: 0.0,
            in_flight: HashMap::new(),
            failed: HashSet::new(),
            next_request_id: 0,
            total_tracks: 0,
            total_duration_ms: 0,
        }
    }

    pub fn set_tile_width(&mut self, width: u32) -> bool {
        if self.tile_width == width {
            return false;
        }
        self.tile_width = width;
        self.refresh_window()
    }

    pub fn sort_albums(albums: &mut [AlbumSummary], field: AlbumSortField, dir: SortDirection) {
        fn cmp_case_insensitive(a: &str, b: &str) -> std::cmp::Ordering {
            let a_chars = a.chars().flat_map(|c| c.to_lowercase());
            let b_chars = b.chars().flat_map(|c| c.to_lowercase());
            a_chars.cmp(b_chars)
        }

        albums.sort_by(|a, b| {
            let ord = match field {
                AlbumSortField::Title => cmp_case_insensitive(&a.album, &b.album),
                AlbumSortField::Artist => cmp_case_insensitive(&a.artist, &b.artist)
                    .then_with(|| cmp_case_insensitive(&a.album, &b.album)),
                AlbumSortField::Year => a
                    .year
                    .cmp(&b.year)
                    .then_with(|| cmp_case_insensitive(&a.album, &b.album)),
                AlbumSortField::TrackCount => a
                    .track_count
                    .cmp(&b.track_count)
                    .then_with(|| cmp_case_insensitive(&a.album, &b.album)),
                AlbumSortField::Duration => a
                    .total_duration_ms
                    .cmp(&b.total_duration_ms)
                    .then_with(|| cmp_case_insensitive(&a.album, &b.album)),
                AlbumSortField::DateAdded => a
                    .added_timestamp
                    .cmp(&b.added_timestamp)
                    .then_with(|| cmp_case_insensitive(&a.album, &b.album)),
            };
            match dir {
                SortDirection::Ascending => ord,
                SortDirection::Descending => ord.reverse(),
            }
        });
    }

    pub fn sort(&mut self) {
        Self::sort_albums(&mut self.albums, self.sort_field, self.sort_dir);
    }

    pub fn set_albums(&mut self, mut albums: Vec<AlbumSummary>) {
        Self::sort_albums(&mut albums, self.sort_field, self.sort_dir);
        self.albums = albums;
        self.recompute_totals();
        self.failed.clear();
        self.in_flight.clear();
        self.last_scroll_y = 0.0;
        self.window_start = 0;
        self.window_end = self.default_window_end();
    }

    fn recompute_totals(&mut self) {
        self.total_tracks = self.albums.iter().map(|a| a.track_count).sum();
        self.total_duration_ms = self.albums.iter().map(|a| a.total_duration_ms).sum();
    }

    pub fn toggle_sort(&mut self, field: AlbumSortField) {
        if self.sort_field == field {
            self.sort_dir = match self.sort_dir {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_field = field;
            self.sort_dir = match field {
                AlbumSortField::Year
                | AlbumSortField::TrackCount
                | AlbumSortField::Duration
                | AlbumSortField::DateAdded => SortDirection::Descending,
                _ => SortDirection::Ascending,
            };
        }
        self.sort();
        // Keep the current scroll position: recompute the window for the new
        // ordering instead of snapping to the top, which would leave the screen
        // showing the bottom spacer with no materialized tiles.
        self.window_start = self.window_start.min(self.albums.len());
        self.window_end = self.window_start;
        self.in_flight.clear();
        self.failed.clear();
        self.refresh_window();
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) -> bool {
        if self.view_mode == mode {
            return false;
        }
        self.view_mode = mode;
        self.refresh_window()
    }

    /// Number of albums per row for the active view mode.
    fn items_per_row(&self) -> usize {
        match self.view_mode {
            ViewMode::Tile => self.columns.max(1),
            ViewMode::List => 1,
        }
    }

    pub fn row_height(&self) -> f32 {
        match self.view_mode {
            ViewMode::Tile => self.tile_width as f32 + TILE_META_HEIGHT + ROW_SPACING,
            ViewMode::List => LIST_ROW_HEIGHT,
        }
    }

    fn total_rows(&self) -> usize {
        let per_row = self.items_per_row();
        self.albums.len().div_ceil(per_row)
    }

    fn default_window_end(&self) -> usize {
        let per_row = self.items_per_row();
        let row_h = self.row_height().max(1.0);
        let rows = (DEFAULT_VIEWPORT_HEIGHT / row_h).ceil() as usize + 2;
        (rows * per_row).min(self.albums.len())
    }

    /// Recomputes the materialized window from an absolute scroll offset and
    /// viewport dimensions. Returns `true` when the window changed.
    fn recompute_window(
        &mut self,
        scroll_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.last_scroll_y = scroll_y;
        if viewport_width > 0.0 {
            self.last_viewport_width = viewport_width;
        }
        if viewport_height > 0.0 {
            self.viewport_height = viewport_height;
        }

        let total = self.albums.len();
        if total == 0 {
            let changed = self.window_start != 0 || self.window_end != 0;
            self.window_start = 0;
            self.window_end = 0;
            return changed;
        }

        if self.view_mode == ViewMode::Tile && self.last_viewport_width > 0.0 {
            self.columns = columns_for_width(self.last_viewport_width, self.tile_width);
        }

        let per_row = self.items_per_row();
        let row_h = self.row_height().max(1.0);
        let total_rows = self.total_rows();

        let first_row = (scroll_y.max(0.0) / row_h).floor() as usize;
        let visible_rows = ((self.viewport_height / row_h).ceil() as usize + 1).max(1);
        let buffer_rows = visible_rows;

        let start_row = first_row.saturating_sub(buffer_rows);
        let end_row = (first_row + visible_rows + buffer_rows).min(total_rows);

        let new_start = (start_row * per_row).min(total);
        let new_end = (end_row * per_row).min(total);

        if new_start != self.window_start || new_end != self.window_end {
            self.window_start = new_start;
            self.window_end = new_end;

            // Abandon in-flight cover loads for albums that scrolled out of the
            // window. Fast scrolling otherwise launches pipelines for every
            // intermediate window; their late-arriving chunks churn the LRU and
            // evict the covers of the window we actually landed on. Forgetting
            // out-of-window failures lets transient errors retry on return.
            let window_keys: HashSet<String> =
                self.visible_albums().iter().map(cover_key).collect();
            self.in_flight.retain(|k, _| window_keys.contains(k));
            self.failed.retain(|k| window_keys.contains(k));

            // Keep the recency of the newly visible covers current.
            self.touch_visible();

            true
        } else {
            false
        }
    }

    fn refresh_window(&mut self) -> bool {
        let y = self.last_scroll_y;
        let width = self.last_viewport_width;
        let height = self.viewport_height;
        self.recompute_window(y, width, height)
    }

    pub fn on_scroll(&mut self, viewport: &iced::widget::scrollable::Viewport) -> bool {
        let y = viewport.absolute_offset().y;
        let width = viewport.bounds().width;
        let height = viewport.bounds().height;
        self.recompute_window(y, width, height)
    }

    /// Testable entry point mirroring [`Self::on_scroll`] without an iced viewport.
    #[allow(dead_code)]
    pub fn on_scroll_metrics(
        &mut self,
        scroll_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.recompute_window(scroll_y, viewport_width, viewport_height)
    }

    pub fn window_len(&self) -> usize {
        self.window_end.saturating_sub(self.window_start)
    }

    pub fn get_missing_covers_to_load(&self) -> Vec<(String, i64)> {
        self.visible_albums()
            .iter()
            .filter_map(|alb| {
                let key = cover_key(alb);
                if self.album_covers.contains_key(&key)
                    || self.in_flight.contains_key(&key)
                    || self.failed.contains(&key)
                {
                    return None;
                }
                alb.cover_track_id.map(|id| (key, id))
            })
            .collect()
    }

    /// Marks `keys` as in-flight under a fresh request id, which is returned so
    /// the caller can tag the eventual results.
    pub fn mark_loading(&mut self, keys: impl IntoIterator<Item = String>) -> u64 {
        self.next_request_id = self.next_request_id.wrapping_add(1);
        let id = self.next_request_id;
        let now = Instant::now();
        for key in keys {
            self.in_flight.insert(key, (id, now));
        }
        id
    }

    pub fn is_in_flight(&self, key: &str) -> bool {
        self.in_flight.contains_key(key)
    }

    /// Releases in-flight keys whose owning request never delivered a result
    /// (e.g. a failed task), so they can be retried. Returns `true` if anything
    /// was released.
    pub fn expire_stale_in_flight(&mut self, max_age: Duration) -> bool {
        let now = Instant::now();
        let before = self.in_flight.len();
        self.in_flight
            .retain(|_, (_, started)| now.saturating_duration_since(*started) < max_age);
        self.in_flight.len() != before
    }

    /// Marks every currently visible, cached cover as most recently used.
    ///
    /// Rendering only has `&self`, so recency can't be bumped during `view`.
    /// Call this whenever the visible set changes (e.g. scroll, sort change, view-mode change).
    pub fn touch_visible(&mut self) {
        let mut seen: HashSet<String> = HashSet::new();
        let visible: Vec<String> = self
            .visible_albums()
            .iter()
            .map(cover_key)
            .filter(|k| self.album_covers.contains_key(k) && seen.insert(k.clone()))
            .collect();

        if visible.is_empty() {
            return;
        }

        let visible_set: HashSet<&str> = visible.iter().map(String::as_str).collect();
        self.cover_order
            .retain(|k| !visible_set.contains(k.as_str()));

        self.cover_order.extend(visible);
    }

    pub fn insert_covers(&mut self, request_id: u64, loaded: Vec<(String, Vec<u8>)>) {
        for (key, bytes) in loaded {
            // Ignore results for keys that are no longer requested.
            let Some(&(owner, _)) = self.in_flight.get(&key) else {
                continue;
            };

            if bytes.is_empty() {
                // A failure only taints the key if it belongs to the current
                // request; a stale abandoned request must not poison a fresh one.
                if owner == request_id {
                    self.in_flight.remove(&key);
                    self.failed.insert(key);
                }
                continue;
            }

            // Successful bytes are always worth keeping, even from a stale task.
            self.in_flight.remove(&key);

            let handle = image::Handle::from_bytes(bytes);
            if let Some(pos) = self.cover_order.iter().position(|k| k == &key) {
                self.cover_order.remove(pos);
            }

            self.cover_order.push_back(key.clone());
            self.album_covers.insert(key, handle);
        }

        self.touch_visible();

        // Windowed virtualization bounds `window_len`, so this cap only evicts
        // covers that have scrolled out of the window.
        let cap = MAX_ALBUM_COVERS.max(self.window_len());
        if self.cover_order.len() > cap {
            let excess = self.cover_order.len() - cap;
            for key in self.cover_order.drain(0..excess) {
                self.album_covers.remove(&key);
            }
        }
    }

    pub fn clear(&mut self) {
        // Replace with empty collections to drop the backing allocations entirely.
        // `.clear()` would retain the heap capacity; we want the memory gone.
        self.albums = Vec::new();
        self.failed = HashSet::new();
        self.total_tracks = 0;
        self.total_duration_ms = 0;
        self.clear_covers();
    }

    pub fn clear_covers(&mut self) {
        // Same reasoning: replace rather than clear so capacity is freed.
        self.album_covers = HashMap::new();
        self.cover_order = VecDeque::new();
        self.in_flight = HashMap::new();
        self.last_scroll_y = 0.0;
        self.window_start = 0;
        self.window_end = self.default_window_end();
    }

    pub fn visible_albums(&self) -> &[AlbumSummary] {
        let start = self.window_start.min(self.albums.len());
        let end = self.window_end.min(self.albums.len()).max(start);
        &self.albums[start..end]
    }

    pub fn len(&self) -> usize {
        self.albums.len()
    }

    pub fn is_empty(&self) -> bool {
        self.albums.is_empty()
    }

    pub fn view<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        if self.albums.is_empty() {
            return components::empty_state(
                theme_cfg,
                "No albums indexed",
                "Scan a folder to populate the album catalog",
            );
        }

        let header_bar = row![
            components::sort_chip_list(
                theme_cfg,
                &[
                    ("Title", AlbumSortField::Title),
                    ("Artist", AlbumSortField::Artist),
                    ("Year", AlbumSortField::Year),
                    ("Tracks", AlbumSortField::TrackCount),
                    ("Duration", AlbumSortField::Duration),
                    ("Date Added", AlbumSortField::DateAdded),
                ],
                self.sort_field,
                self.sort_dir,
                Message::AlbumSortChanged,
            ),
            Space::with_width(Length::Fill),
            components::view_mode_toggle(theme_cfg, self.view_mode, Message::SetAlbumsViewMode),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 6.0,
            left: 16.0,
        });

        let content_area: Element<'a, Message> = match self.view_mode {
            ViewMode::Tile => self.view_tiles(theme_cfg),
            ViewMode::List => self.view_list(theme_cfg),
        };

        column![header_bar, content_area]
            .spacing(4)
            .padding(iced::Padding {
                top: 8.0,
                right: 1.0,
                bottom: 0.0,
                left: 0.0,
            })
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn view_tiles<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let per_row = self.items_per_row();
        let total_rows = self.total_rows();
        let start_row = self.window_start / per_row;
        let end_row = self.window_end.div_ceil(per_row);
        let row_h = self.row_height();
        let tile_w = self.tile_width as f32;
        let disc_icon_size = (tile_w * 0.32).clamp(32.0, 72.0);

        let mut content = column![].spacing(0).width(Length::Fill);

        let top = start_row as f32 * row_h;
        if top > 0.0 {
            content = content.push(Space::with_height(top));
        }

        for chunk in self.visible_albums().chunks(per_row) {
            let mut tile_row = row![].spacing(ROW_SPACING).align_y(Alignment::Start);

            for album in chunk {
                let key = cover_key(album);
                let cover_art = components::cover_art(
                    theme_cfg,
                    self.album_covers.get(&key),
                    tile_w,
                    Radius::default(),
                    icons::DISC_SVG,
                    disc_icon_size,
                    theme_cfg.primary_color(),
                );

                let meta = container(
                    column![
                        text(theme_cfg, album.album.as_ref())
                            .size(13)
                            .color(theme_cfg.text_color()),
                        row![
                            text(theme_cfg, album.artist.as_ref())
                                .size(11)
                                .color(theme_cfg.text_muted_color())
                                .width(Length::Fill),
                            text(theme_cfg, album.year_label.as_str())
                                .size(11)
                                .color(theme_cfg.text_muted_color()),
                        ]
                        .align_y(Alignment::Center),
                        row![
                            text(theme_cfg, album.tracks_label.as_str())
                                .size(10)
                                .color(theme_cfg.text_muted_color()),
                            Space::with_width(Length::Fill),
                            text(theme_cfg, album.duration_label.as_str())
                                .size(10)
                                .color(theme_cfg.text_muted_color()),
                        ]
                        .align_y(Alignment::Center),
                    ]
                    .spacing(3)
                    .padding(8),
                )
                .width(tile_w)
                .height(TILE_META_HEIGHT);

                let card_content = container(column![cover_art, meta])
                    .width(tile_w)
                    .height(tile_w + TILE_META_HEIGHT)
                    .clip(true);

                let select_btn = button(card_content)
                    .style(styles::tile(theme_cfg))
                    .padding(0)
                    .on_press(Message::SelectAlbum(album.album.to_string()));

                let play_btn = button(icons::icon(icons::PLAY_SVG, 14.0, theme_cfg.text_color()))
                    .style(styles::icon_control_button(theme_cfg))
                    .padding(4)
                    .on_press(Message::PlayScope {
                        filter: TrackFilter::album(album.album.as_ref()),
                        name: album.album.as_ref().to_string(),
                        shuffle: false,
                    });

                let shuffle_btn = button(icons::icon(
                    icons::SHUFFLE_SVG,
                    14.0,
                    theme_cfg.text_color(),
                ))
                .style(styles::icon_control_button(theme_cfg))
                .padding(4)
                .on_press(Message::PlayScope {
                    filter: TrackFilter::album(album.album.as_ref()),
                    name: album.album.as_ref().to_string(),
                    shuffle: true,
                });

                let actions = container(row![play_btn, shuffle_btn].spacing(4))
                    .width(tile_w)
                    .height(tile_w)
                    .align_x(Alignment::End)
                    .align_y(Alignment::End)
                    .padding(6);

                let card = iced::widget::stack([select_btn.into(), actions.into()]);

                tile_row = tile_row.push(card);
            }

            content = content.push(
                container(tile_row)
                    .height(row_h)
                    .width(Length::Fill)
                    .align_y(Alignment::Start),
            );
        }

        let bottom = total_rows.saturating_sub(end_row) as f32 * row_h;
        if bottom > 0.0 {
            content = content.push(Space::with_height(bottom));
        }

        scrollable(container(content).width(Length::Fill).padding(Padding {
            left: 16.0,
            right: 16.0,
            ..Default::default()
        }))
        .id(scrollable::Id::new("albums_scroll"))
        .on_scroll(Message::AlbumsScrolled)
        .height(Length::Fill)
        .style(styles::scrollable_style(theme_cfg))
        .into()
    }

    fn view_list<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let total_rows = self.albums.len();
        let row_h = LIST_ROW_HEIGHT;

        let mut content = column![].spacing(0).width(Length::Fill);

        let top = self.window_start as f32 * row_h;
        if top > 0.0 {
            content = content.push(Space::with_height(top));
        }

        for album in self.visible_albums() {
            let key = cover_key(album);
            let disc_thumbnail = components::cover_art(
                theme_cfg,
                self.album_covers.get(&key),
                42.0,
                Radius::from(4.0),
                icons::DISC_SVG,
                20.0,
                theme_cfg.primary_color(),
            );

            let mut subtitle = row![text(theme_cfg, album.artist.as_ref())
                .size(11)
                .color(theme_cfg.text_muted_color())];
            if !album.year_label.is_empty() {
                subtitle = subtitle
                    .push(
                        text(theme_cfg, " • ")
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    )
                    .push(
                        text(theme_cfg, album.year_label.as_str())
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    );
            }

            let title_block = column![
                text(theme_cfg, album.album.as_ref())
                    .size(13)
                    .color(theme_cfg.text_color()),
                subtitle,
            ]
            .spacing(2)
            .width(Length::Fill);

            let details = column![
                text(theme_cfg, album.tracks_label.as_str())
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text(theme_cfg, album.duration_label.as_str())
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(2)
            .align_x(Alignment::End)
            .width(100);

            let album_row = row![disc_thumbnail, Space::with_width(12), title_block, details,]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding([0, 8]);

            let select_btn = button(album_row)
                .style(styles::tile(theme_cfg))
                .width(Length::Fill)
                .on_press(Message::SelectAlbum(album.album.to_string()));

            let play_btn = button(icons::icon(
                icons::PLAY_SVG,
                14.0,
                theme_cfg.text_muted_color(),
            ))
            .style(styles::icon_control_button(theme_cfg))
            .padding(4)
            .on_press(Message::PlayScope {
                filter: TrackFilter::album(album.album.as_ref()),
                name: album.album.as_ref().to_string(),
                shuffle: false,
            });

            let shuffle_btn = button(icons::icon(
                icons::SHUFFLE_SVG,
                14.0,
                theme_cfg.text_muted_color(),
            ))
            .style(styles::icon_control_button(theme_cfg))
            .padding(4)
            .on_press(Message::PlayScope {
                filter: TrackFilter::album(album.album.as_ref()),
                name: album.album.as_ref().to_string(),
                shuffle: true,
            });

            let actions = row![play_btn, shuffle_btn]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding([0, 8]);

            let full_row = row![select_btn, actions]
                .align_y(Alignment::Center)
                .width(Length::Fill);

            content = content.push(container(full_row).height(row_h).width(Length::Fill));
        }

        let bottom = total_rows.saturating_sub(self.window_end) as f32 * row_h;
        if bottom > 0.0 {
            content = content.push(Space::with_height(bottom));
        }

        scrollable(content)
            .id(scrollable::Id::new("albums_scroll"))
            .on_scroll(Message::AlbumsScrolled)
            .height(Length::Fill)
            .style(styles::scrollable_style(theme_cfg))
            .into()
    }
}
