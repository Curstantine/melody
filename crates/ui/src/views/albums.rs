use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use core_engine::{format_template, AlbumSummary, SortDirection, TrackFilter};
use iced::widget::{column, container, image, row, scrollable, Space, Widget};
use iced::{Alignment, Element, Length};

use crate::components;
use crate::icons;
use crate::message::{AlbumSortField, Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

pub const DEFAULT_ALBUM_TILE_WIDTH: u32 = 140;
/// Minimum number of decoded album cover textures retained in RAM/VRAM.
/// The effective cap is `max(MAX_ALBUM_COVERS, window_len)` so covers currently
/// rendered by the virtualized window are never evicted.
pub const MAX_ALBUM_COVERS: usize = 120;

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
    (((usable + components::CATALOG_ROW_SPACING) / (tile + components::CATALOG_ROW_SPACING)).floor()
        as usize)
        .max(1)
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
            ViewMode::Tile => {
                self.tile_width as f32
                    + components::CATALOG_TILE_META_HEIGHT
                    + components::CATALOG_ROW_SPACING
            }
            ViewMode::List => components::CATALOG_LIST_ROW_HEIGHT,
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
        let width = viewport.bounds.width;
        let height = viewport.bounds.height;
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

    pub fn get_album_cover(&self, album: &AlbumSummary) -> Option<&image::Handle> {
        let artist = album.artist.as_bytes();
        let album_name = album.album.as_bytes();
        let total_len = artist.len() + 1 + album_name.len();
        if total_len <= 1024 {
            let mut buf = [0u8; 1024];
            buf[..artist.len()].copy_from_slice(artist);
            buf[artist.len()] = 0x1f;
            buf[artist.len() + 1..total_len].copy_from_slice(album_name);
            if let Ok(key) = std::str::from_utf8(&buf[..total_len]) {
                return self.album_covers.get(key);
            }
        }
        let key = cover_key(album);
        self.album_covers.get(&key)
    }

    pub fn view<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        format_cfg: &'a crate::layout::AlbumsFormatConfig,
    ) -> Element<'a, Message> {
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
                    ("Date added", AlbumSortField::DateAdded),
                ],
                self.sort_field,
                self.sort_dir,
                Message::AlbumSortChanged,
            ),
            Space::new().width(Length::Fill),
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
            ViewMode::Tile => self.view_tiles(theme_cfg, format_cfg),
            ViewMode::List => self.view_list(theme_cfg, format_cfg),
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
            .boxed()
    }

    fn view_tiles<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        format_cfg: &'a crate::layout::AlbumsFormatConfig,
    ) -> Element<'a, Message> {
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
            content = content.push(Space::new().height(top).boxed());
        }

        for chunk in self.visible_albums().chunks(per_row) {
            let mut tile_row = row![]
                .spacing(components::CATALOG_ROW_SPACING)
                .align_y(Alignment::Start);

            for album in chunk {
                let leading = components::catalog_cover_art(
                    theme_cfg,
                    self.get_album_cover(album),
                    tile_w,
                    theme_cfg.radius_md(),
                    icons::DISC_SVG,
                    disc_icon_size,
                    theme_cfg.primary_color(),
                    Message::SelectAlbum(album.album.to_string()),
                );

                let line1 = format_template(&format_cfg.tile_line1, album);
                let line2 = format_template(&format_cfg.tile_line2, album);
                let line3 = format_template(&format_cfg.tile_line3, album);

                tile_row = tile_row.push(
                    components::CatalogTile {
                        theme: theme_cfg,
                        tile_width: tile_w,
                        leading,
                        title: line1.into(),
                        mid_left: line2.into(),
                        mid_right: "".into(),
                        bottom_left: line3.into(),
                        bottom_right: "".into(),
                        on_select: Message::SelectAlbum(album.album.to_string()),
                        on_play: Message::PlayScope {
                            filter: TrackFilter::album(album.album.as_ref()),
                            name: album.album.as_ref().to_string(),
                            shuffle: false,
                        },
                        on_shuffle: Message::PlayScope {
                            filter: TrackFilter::album(album.album.as_ref()),
                            name: album.album.as_ref().to_string(),
                            shuffle: true,
                        },
                        on_right_press: Some(Message::OpenAlbumContextMenu(album.clone())),
                    }
                    .view(),
                );
            }

            content = content.push(
                container(tile_row)
                    .height(row_h)
                    .width(Length::Fill)
                    .align_y(Alignment::Start)
                    .boxed(),
            );
        }

        let bottom = total_rows.saturating_sub(end_row) as f32 * row_h;
        if bottom > 0.0 {
            content = content.push(Space::new().height(bottom).boxed());
        }

        scrollable(container(content).width(Length::Fill).padding([0, 16]))
            .id(iced::widget::Id::new("albums_scroll"))
            .on_scroll(|scroll| Message::AlbumsScrolled(scroll.viewport))
            .height(Length::Fill)
            .style(styles::scrollable_style(theme_cfg))
            .boxed()
    }

    fn view_list<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        format_cfg: &'a crate::layout::AlbumsFormatConfig,
    ) -> Element<'a, Message> {
        let total_rows = self.albums.len();
        let row_h = components::CATALOG_LIST_ROW_HEIGHT;

        let mut content = column![].spacing(0).width(Length::Fill);

        let top = self.window_start as f32 * row_h;
        if top > 0.0 {
            content = content.push(Space::new().height(top).boxed());
        }

        for album in self.visible_albums() {
            let line1 = format_template(&format_cfg.list_line1, album);
            let line2 = format_template(&format_cfg.list_line2, album);
            let details = format_template(&format_cfg.list_details, album);

            content = content.push(
                components::CatalogRow {
                    theme: theme_cfg,
                    leading: components::cover_art(
                        theme_cfg,
                        self.get_album_cover(album),
                        components::CATALOG_LIST_ART_SIZE,
                        theme_cfg.radius_sm(),
                        icons::DISC_SVG,
                        20.0,
                        theme_cfg.primary_color(),
                    ),
                    title: line1.into(),
                    subtitle_left: line2.into(),
                    subtitle_right: "".into(),
                    details: if details.is_empty() {
                        None
                    } else {
                        Some((details.into(), "".into()))
                    },
                    on_select: Message::SelectAlbum(album.album.to_string()),
                    on_play: Message::PlayScope {
                        filter: TrackFilter::album(album.album.as_ref()),
                        name: album.album.as_ref().to_string(),
                        shuffle: false,
                    },
                    on_shuffle: Message::PlayScope {
                        filter: TrackFilter::album(album.album.as_ref()),
                        name: album.album.as_ref().to_string(),
                        shuffle: true,
                    },
                    on_right_press: Some(Message::OpenAlbumContextMenu(album.clone())),
                    width: Length::Fill,
                    height: Length::Fixed(row_h),
                }
                .view(),
            );
        }

        let bottom = total_rows.saturating_sub(self.window_end) as f32 * row_h;
        if bottom > 0.0 {
            content = content.push(Space::new().height(bottom).boxed());
        }

        scrollable(content)
            .id(iced::widget::Id::new("albums_scroll"))
            .on_scroll(|scroll| Message::AlbumsScrolled(scroll.viewport))
            .height(Length::Fill)
            .style(styles::scrollable_style(theme_cfg))
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{AlbumsFormatConfig, ArtistsFormatConfig};
    use crate::message::ArtistSortField;
    use crate::theme::ThemeConfig;
    use crate::views::artists::ArtistsView;
    use core_engine::{AlbumSummary, ArtistSummary};

    fn sample_albums(count: usize) -> Vec<AlbumSummary> {
        (0..count)
            .map(|i| AlbumSummary {
                album: format!("Album {}", i).into(),
                artist: format!("Artist {}", i % 500).into(),
                track_count: 10,
                total_duration_ms: 180_000,
                year: Some(2020),
                cover_track_id: Some(i as i64 + 1),
                added_timestamp: 1_700_000_000 + i as i64,
                year_label: "2020".to_string(),
                tracks_label: "10 tracks".to_string(),
                duration_label: "3:00".to_string(),
            })
            .collect()
    }

    #[test]
    fn test_fast_scroll_abandons_stale_cover_loads() {
        let mut view = AlbumsView::new(ViewMode::List, AlbumSortField::Title);
        view.set_albums(sample_albums(5_000));

        // Begin loading covers for the initial window.
        let initial = view.get_missing_covers_to_load();
        assert!(!initial.is_empty());
        view.mark_loading(initial.iter().map(|(k, _)| k.clone()));
        assert!(!view.in_flight.is_empty());

        // Jump to the bottom: in-flight loads for the abandoned window must be dropped.
        let bottom = 5_000.0 * view.row_height();
        view.on_scroll_metrics(bottom, 1_100.0, 720.0);

        let visible_keys: std::collections::HashSet<String> = view
            .visible_albums()
            .iter()
            .map(|a| format!("{}\u{1f}{}", a.artist, a.album))
            .collect();
        assert!(
            view.in_flight.keys().all(|k| visible_keys.contains(k)),
            "stale in-flight cover loads were not abandoned"
        );

        // The landing window has covers to (re)load.
        assert!(!view.get_missing_covers_to_load().is_empty());

        // Scrolling back to the top must also report missing covers to reload.
        view.on_scroll_metrics(0.0, 1_100.0, 720.0);
        assert!(!view.get_missing_covers_to_load().is_empty());
    }

    #[test]
    fn in_flight_requests_expire_so_they_can_retry() {
        let mut view = AlbumsView::new(ViewMode::List, AlbumSortField::Title);
        view.set_albums(sample_albums(10));

        let to_load = view.get_missing_covers_to_load();
        view.mark_loading(to_load.iter().map(|(k, _)| k.clone()));
        assert!(!view.in_flight.is_empty());

        // A generous age leaves fresh requests alone.
        assert!(!view.expire_stale_in_flight(std::time::Duration::from_secs(60)));
        assert!(!view.in_flight.is_empty());

        // An expired age releases them so they can be requested again.
        assert!(view.expire_stale_in_flight(std::time::Duration::ZERO));
        assert!(view.in_flight.is_empty());
        assert!(!view.get_missing_covers_to_load().is_empty());
    }

    #[test]
    fn stale_failure_does_not_poison_a_new_request() {
        let mut view = AlbumsView::new(ViewMode::List, AlbumSortField::Title);
        view.set_albums(sample_albums(100));

        // A first request owns the visible covers.
        let to_load = view.get_missing_covers_to_load();
        let stale_id = view.mark_loading(to_load.iter().map(|(k, _)| k.clone()));
        let key = to_load[0].0.clone();

        // The album is re-requested under a newer id while still visible.
        let fresh_id = view.mark_loading(vec![key.clone()]);
        assert_ne!(stale_id, fresh_id);

        // The abandoned request fails for that key: it must not poison the fresh one.
        view.insert_covers(stale_id, vec![(key.clone(), Vec::new())]);
        assert!(
            !view.failed.contains(&key),
            "stale failure permanently poisoned the key"
        );
        assert!(view.is_in_flight(&key), "fresh request was wrongly cleared");

        // The fresh request succeeds and caches the cover.
        view.insert_covers(fresh_id, vec![(key.clone(), vec![1u8, 2, 3])]);
        assert!(view.album_covers.contains_key(&key));
        assert!(!view.failed.contains(&key));
    }

    #[test]
    fn sim_fast_scroll_roundtrip_converges() {
        // Mirrors the real-world pattern: user flings down, landing at multiple
        // intermediate offsets, then flings back up. Each step issues cover load
        // requests, some of which succeed, some fail, some arrive out of order.
        let mut view = AlbumsView::new(ViewMode::List, AlbumSortField::Title);
        view.set_albums(sample_albums(2_000));

        let row_h = view.row_height();
        let offsets = [
            0.0,
            200.0 * row_h,
            600.0 * row_h,
            1_200.0 * row_h,
            1_900.0 * row_h,
            1_100.0 * row_h,
            400.0 * row_h,
            0.0,
        ];

        // Store pending batch completions: (request_id, [(key, bytes)])
        struct PendingBatch {
            id: u64,
            covers: Vec<(String, Vec<u8>)>,
        }
        let mut pending: Vec<PendingBatch> = Vec::new();

        fn schedule(view: &mut AlbumsView, step: usize, pending: &mut Vec<PendingBatch>) {
            let missing = view.get_missing_covers_to_load();
            if missing.is_empty() {
                return;
            }
            let req_id = view.mark_loading(missing.iter().map(|(k, _)| k.clone()));
            let covers = missing
                .into_iter()
                .map(|(k, _)| {
                    // Fail 1 in 5 covers artificially to simulate corrupt/missing art
                    let bytes = if (k.len() + step).is_multiple_of(5) {
                        Vec::new()
                    } else {
                        vec![0xFF, 0xD8, 0xFF] // dummy jpeg header
                    };
                    (k, bytes)
                })
                .collect();
            pending.push(PendingBatch { id: req_id, covers });
        }

        // Fling down & up
        for (step, &y) in offsets.iter().enumerate() {
            view.on_scroll_metrics(y, 1_100.0, 720.0);
            schedule(&mut view, step, &mut pending);

            // Periodically deliver some out-of-order responses
            if step % 2 == 1 && !pending.is_empty() {
                let batch = pending.remove(0);
                view.insert_covers(batch.id, batch.covers);
            }
        }

        // Deliver all remaining pending responses
        while let Some(batch) = pending.pop() {
            view.insert_covers(batch.id, batch.covers);
        }

        // The user has rested at offset 0. Now run the idle retry loop: any missing
        // covers in the rest window must converge (either cached or marked failed).
        for _ in 0..10 {
            let missing = view.get_missing_covers_to_load();
            if missing.is_empty() {
                break;
            }
            let req_id = view.mark_loading(missing.iter().map(|(k, _)| k.clone()));
            let covers: Vec<(String, Vec<u8>)> = missing
                .into_iter()
                .map(|(k, _)| (k, vec![0xFF, 0xD8, 0xFF]))
                .collect();
            view.insert_covers(req_id, covers);
        }

        // Terminal invariant: at rest, every visible album's cover is resolved.
        let visible = view.visible_albums();
        assert!(!visible.is_empty());
        for album in visible {
            let key = format!("{}\u{1f}{}", album.artist, album.album);
            assert!(
                view.album_covers.contains_key(&key) || view.failed.contains(&key),
                "cover for visible album '{}' was neither cached nor marked failed",
                key
            );
        }
        assert!(
            view.get_missing_covers_to_load().is_empty(),
            "at rest there should be no missing covers left to load"
        );
    }

    #[test]
    fn test_albums_are_virtualized_not_paginated() {
        for view_mode in [ViewMode::Tile, ViewMode::List] {
            let mut view = AlbumsView::new(view_mode, AlbumSortField::Title);
            let albums = sample_albums(2_000);
            view.set_albums(albums);

            // Before any scroll, the initial slice is bounded.
            assert_eq!(view.window_start, 0);
            assert!(view.window_len() < 200, "window_len={}", view.window_len());

            // Recomputing with a viewport scrolled to the bottom keeps the window bounded.
            let bottom = 2_000.0 * view.row_height();
            view.on_scroll_metrics(bottom, 1_100.0, 720.0);
            assert!(view.window_len() < 200, "window_len={}", view.window_len());
            assert!(view.window_end <= 2_000);
            assert!(view.window_start > 0);

            // And visible_albums() matches the window.
            assert_eq!(view.visible_albums().len(), view.window_len());
        }
    }

    #[test]
    fn test_album_and_artist_views_render_sliced() {
        let theme_cfg = ThemeConfig::default();

        let albums_list: Vec<AlbumSummary> = (0..120)
            .map(|i| AlbumSummary {
                album: format!("Album {}", i).into(),
                artist: format!("Artist {}", i).into(),
                track_count: 10,
                total_duration_ms: 180_000,
                year: Some(2020),
                cover_track_id: None,
                added_timestamp: 1700000000 + i as i64,
                year_label: "2020".to_string(),
                tracks_label: "10 tracks".to_string(),
                duration_label: "3:00".to_string(),
            })
            .collect();

        let artists_list: Vec<ArtistSummary> = (0..120)
            .map(|i| ArtistSummary {
                artist: format!("Artist {}", i).into(),
                track_count: 5,
                album_count: 1,
                stats_label: "1 albums • 5 tracks".to_string(),
            })
            .collect();

        // Sliced view for albums (first 60 items)
        let albums_fmt = AlbumsFormatConfig::default();
        let artists_fmt = ArtistsFormatConfig::default();

        let mut albums_view_tile = AlbumsView::new(ViewMode::Tile, AlbumSortField::Title);
        albums_view_tile.set_albums(albums_list.clone());
        let _album_tile = albums_view_tile.view(&theme_cfg, &albums_fmt);

        let mut albums_view_list = AlbumsView::new(ViewMode::List, AlbumSortField::Title);
        albums_view_list.set_albums(albums_list);
        let _album_list = albums_view_list.view(&theme_cfg, &albums_fmt);

        // Sliced view for artists (first 60 items)
        let mut artists_view_tile = ArtistsView::new(ViewMode::Tile, ArtistSortField::Artist);
        artists_view_tile.set_artists(artists_list.clone());
        let _artist_tile = artists_view_tile.view(&theme_cfg, &artists_fmt);

        let mut artists_view_list = ArtistsView::new(ViewMode::List, ArtistSortField::Artist);
        artists_view_list.set_artists(artists_list);
        let _artist_list = artists_view_list.view(&theme_cfg, &artists_fmt);
    }

    #[test]
    fn test_scroll_messages_variants() {
        fn check_message(_m: Message) {}
        check_message(Message::NextTrack);
    }

    #[test]
    fn test_albums_view_component_state() {
        let mut albums_view = AlbumsView::new(ViewMode::Tile, AlbumSortField::Title);
        assert!(albums_view.is_empty());
        assert_eq!(albums_view.len(), 0);
        assert_eq!(albums_view.tile_width, 140);
        albums_view.set_tile_width(180);
        assert_eq!(albums_view.tile_width, 180);

        let albums_list: Vec<AlbumSummary> = (0..50)
            .map(|i| AlbumSummary {
                album: format!("Album {}", i).into(),
                artist: format!("Artist {}", i).into(),
                track_count: 10,
                total_duration_ms: 180_000,
                year: Some(2020),
                cover_track_id: None,
                added_timestamp: 1700000000 + i as i64,
                year_label: "2020".to_string(),
                tracks_label: "10 tracks".to_string(),
                duration_label: "3:00".to_string(),
            })
            .collect();
        albums_view.set_albums(albums_list);
        assert!(!albums_view.is_empty());
        assert_eq!(albums_view.len(), 50);

        albums_view.toggle_sort(AlbumSortField::Title);
        albums_view.set_view_mode(ViewMode::List);

        albums_view.clear();
        assert!(albums_view.is_empty());
        assert_eq!(albums_view.len(), 0);
        assert!(albums_view.album_covers.is_empty());
        assert!(albums_view.cover_order.is_empty());

        let theme_cfg = ThemeConfig::default();
        let albums_fmt = AlbumsFormatConfig::default();
        let _rendered = albums_view.view(&theme_cfg, &albums_fmt);
    }

    #[test]
    fn test_get_album_cover_lookup() {
        let mut view = AlbumsView::new(ViewMode::Tile, AlbumSortField::Title);

        let long_artist = "A".repeat(600);
        let long_album = "B".repeat(600);
        let test_cases = vec![
            ("Artist 1", "Album 1"),
            ("宇多田ヒカル", "First Love"),
            ("周杰伦", "范特西"),
            ("아이유 🎵", "Palette ✨"),
            (long_artist.as_str(), long_album.as_str()), // > 1024 bytes fallback
        ];

        let mut summaries = Vec::new();
        for (i, (artist, album)) in test_cases.into_iter().enumerate() {
            let key = format!("{}\u{1f}{}", artist, album);
            // Create 1x1 RGBA dummy image
            let handle = iced::widget::image::Handle::from_rgba(1, 1, vec![255, 0, 0, 255]);
            view.album_covers.insert(key, handle);

            summaries.push(AlbumSummary {
                album: album.into(),
                artist: artist.into(),
                track_count: 1,
                total_duration_ms: 60_000,
                year: Some(2024),
                cover_track_id: Some(i as i64 + 1),
                added_timestamp: 0,
                year_label: "2024".to_string(),
                tracks_label: "1 track".to_string(),
                duration_label: "1:00".to_string(),
            });
        }

        // Verify all covers resolve correctly via get_album_cover
        for summary in &summaries {
            assert!(
                view.get_album_cover(summary).is_some(),
                "Cover lookup failed for artist='{}', album='{}'",
                summary.artist,
                summary.album
            );
        }

        // Verify missing cover returns None
        let missing_summary = AlbumSummary {
            album: "Nonexistent".into(),
            artist: "Nobody".into(),
            track_count: 1,
            total_duration_ms: 60_000,
            year: None,
            cover_track_id: None,
            added_timestamp: 0,
            year_label: "".to_string(),
            tracks_label: "1 track".to_string(),
            duration_label: "1:00".to_string(),
        };
        assert!(view.get_album_cover(&missing_summary).is_none());
    }
}
