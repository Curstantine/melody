use core_engine::{format_template, ArtistSummary, SortDirection, TrackFilter};
use iced::widget::{column, container, row, scrollable, Space, Widget};
use iced::{Alignment, Element, Length, Padding};

use crate::components;
use crate::icons;
use crate::message::{ArtistSortField, Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

pub const DEFAULT_ARTIST_TILE_WIDTH: u32 = 140;

const DEFAULT_COLUMNS: usize = 6;
const DEFAULT_VIEWPORT_HEIGHT: f32 = 720.0;
const HORIZONTAL_PADDING: f32 = 32.0;

/// Computes how many tiles fit in a viewport of `width` pixels.
fn columns_for_width(width: f32, tile_width: u32) -> usize {
    let usable = (width - HORIZONTAL_PADDING).max(tile_width as f32);
    let tile = tile_width as f32;
    (((usable + components::CATALOG_ROW_SPACING) / (tile + components::CATALOG_ROW_SPACING)).floor()
        as usize)
        .max(1)
}

#[derive(Clone, Debug, Default)]
pub struct RenderedArtistStrings {
    pub tile_line1: String,
    pub tile_line2: String,
    pub list_line1: String,
    pub list_line2: String,
}

/// Self-contained state and view component for the Artists catalog screen.
///
/// Like the Albums catalog, the *entire* artist list is held in memory as
/// lightweight, interned summaries, while only the visible window of
/// tiles/rows is materialized into widgets. Spacers above and below the window
/// keep the scrollbar proportional to the whole catalog, so memory and per-frame
/// widget construction stay bounded regardless of catalog size.
pub struct ArtistsView {
    pub view_mode: ViewMode,
    pub tile_width: u32,
    pub artists: Vec<ArtistSummary>,
    pub sort_field: ArtistSortField,
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
    /// Precomputed catalog totals so the status bar never folds over every
    /// artist on every frame.
    pub total_tracks: usize,
    pub total_albums: usize,
    /// Pre-rendered status strings so StatusBar rendering does not allocate per frame.
    pub status_count_str: String,
    pub status_albums_str: String,
    pub status_tracks_str: String,
    /// Pre-rendered strings for the visible window of artists to eliminate per-frame format allocations.
    pub rendered_window: Vec<RenderedArtistStrings>,
}

impl ArtistsView {
    #[allow(dead_code)]
    pub fn new(view_mode: ViewMode, sort_field: ArtistSortField) -> Self {
        Self::with_tile_width(view_mode, sort_field, DEFAULT_ARTIST_TILE_WIDTH)
    }

    pub fn with_tile_width(
        view_mode: ViewMode,
        sort_field: ArtistSortField,
        tile_width: u32,
    ) -> Self {
        let mut view = Self {
            view_mode,
            tile_width,
            artists: Vec::new(),
            sort_field,
            sort_dir: SortDirection::Ascending,
            window_start: 0,
            window_end: 0,
            columns: DEFAULT_COLUMNS,
            viewport_height: DEFAULT_VIEWPORT_HEIGHT,
            last_viewport_width: 0.0,
            last_scroll_y: 0.0,
            total_tracks: 0,
            total_albums: 0,
            status_count_str: String::new(),
            status_albums_str: String::new(),
            status_tracks_str: String::new(),
            rendered_window: Vec::new(),
        };
        view.recompute_totals();
        view
    }

    pub fn set_tile_width(&mut self, width: u32) -> bool {
        if self.tile_width == width {
            return false;
        }
        self.tile_width = width;
        self.refresh_window()
    }

    pub fn sort_artists(artists: &mut [ArtistSummary], field: ArtistSortField, dir: SortDirection) {
        fn cmp_case_insensitive(a: &str, b: &str) -> std::cmp::Ordering {
            let a_chars = a.chars().flat_map(|c| c.to_lowercase());
            let b_chars = b.chars().flat_map(|c| c.to_lowercase());
            a_chars.cmp(b_chars)
        }

        artists.sort_by(|a, b| {
            let ord = match field {
                ArtistSortField::Artist => cmp_case_insensitive(&a.artist, &b.artist),
                ArtistSortField::TrackCount => a
                    .track_count
                    .cmp(&b.track_count)
                    .then_with(|| cmp_case_insensitive(&a.artist, &b.artist)),
                ArtistSortField::AlbumCount => a
                    .album_count
                    .cmp(&b.album_count)
                    .then_with(|| cmp_case_insensitive(&a.artist, &b.artist)),
            };
            match dir {
                SortDirection::Ascending => ord,
                SortDirection::Descending => ord.reverse(),
            }
        });
    }

    pub fn sort(&mut self) {
        Self::sort_artists(&mut self.artists, self.sort_field, self.sort_dir);
    }

    pub fn set_artists(&mut self, mut artists: Vec<ArtistSummary>) {
        Self::sort_artists(&mut artists, self.sort_field, self.sort_dir);
        self.artists = artists;
        self.recompute_totals();
        self.last_scroll_y = 0.0;
        self.window_start = 0;
        self.window_end = self.default_window_end();
    }

    fn recompute_totals(&mut self) {
        self.total_tracks = self.artists.iter().map(|a| a.track_count).sum();
        self.total_albums = self.artists.iter().map(|a| a.album_count).sum();
        self.status_count_str = format!("{} artists", self.artists.len());
        self.status_albums_str = format!("{} albums", self.total_albums);
        self.status_tracks_str = format!("{} tracks", self.total_tracks);
    }

    pub fn recompute_rendered_window(&mut self, format_cfg: &crate::layout::ArtistsFormatConfig) {
        let visible = self.visible_artists();
        self.rendered_window = visible
            .iter()
            .map(|artist| RenderedArtistStrings {
                tile_line1: format_template(&format_cfg.tile_line1, artist),
                tile_line2: format_template(&format_cfg.tile_line2, artist),
                list_line1: format_template(&format_cfg.list_line1, artist),
                list_line2: format_template(&format_cfg.list_line2, artist),
            })
            .collect();
    }

    pub fn toggle_sort(&mut self, field: ArtistSortField) {
        if self.sort_field == field {
            self.sort_dir = match self.sort_dir {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_field = field;
            self.sort_dir = match field {
                ArtistSortField::TrackCount | ArtistSortField::AlbumCount => {
                    SortDirection::Descending
                }
                _ => SortDirection::Ascending,
            };
        }
        self.sort();
        // Keep the current scroll position: recompute the window for the new
        // ordering instead of snapping to the top.
        self.window_start = self.window_start.min(self.artists.len());
        self.window_end = self.window_start;
        self.refresh_window();
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) -> bool {
        if self.view_mode == mode {
            return false;
        }
        self.view_mode = mode;
        self.refresh_window()
    }

    pub fn on_scroll(&mut self, viewport: &iced::widget::scrollable::Viewport) -> bool {
        let y = viewport.absolute_offset().y;
        let width = viewport.bounds.width;
        let height = viewport.bounds.height;
        self.recompute_window(y, width, height)
    }

    /// Testable entry point mirroring [`Self::on_scroll`] without an iced viewport.
    #[cfg(test)]
    pub fn on_scroll_metrics(
        &mut self,
        scroll_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.recompute_window(scroll_y, viewport_width, viewport_height)
    }

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
        self.artists.len().div_ceil(per_row)
    }

    fn default_window_end(&self) -> usize {
        let per_row = self.items_per_row();
        let row_h = self.row_height().max(1.0);
        let rows = (DEFAULT_VIEWPORT_HEIGHT / row_h).ceil() as usize + 2;
        (rows * per_row).min(self.artists.len())
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

        let total = self.artists.len();
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

    #[allow(dead_code)]
    pub fn window_len(&self) -> usize {
        self.window_end.saturating_sub(self.window_start)
    }

    pub fn clear(&mut self) {
        // Replace rather than clear so the backing allocation is dropped.
        self.artists = Vec::new();
        self.rendered_window = Vec::new();
        self.window_start = 0;
        self.window_end = 0;
        self.total_tracks = 0;
        self.total_albums = 0;
        self.recompute_totals();
    }

    pub fn visible_artists(&self) -> &[ArtistSummary] {
        let start = self.window_start.min(self.artists.len());
        let end = self.window_end.min(self.artists.len()).max(start);
        &self.artists[start..end]
    }

    pub fn len(&self) -> usize {
        self.artists.len()
    }

    pub fn is_empty(&self) -> bool {
        self.artists.is_empty()
    }

    pub fn view<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        format_cfg: &'a crate::layout::ArtistsFormatConfig,
    ) -> Element<'a, Message> {
        if self.artists.is_empty() {
            return components::empty_state(
                theme_cfg,
                "No artists indexed",
                "Scan a folder in Preferences to populate the artist catalog",
            );
        }

        let header_bar = row![
            components::sort_chip_list(
                theme_cfg,
                &[
                    ("Artist", ArtistSortField::Artist),
                    ("Tracks", ArtistSortField::TrackCount),
                    ("Albums", ArtistSortField::AlbumCount),
                ],
                self.sort_field,
                self.sort_dir,
                Message::ArtistSortChanged,
            ),
            Space::new().width(Length::Fill),
            components::view_mode_toggle(theme_cfg, self.view_mode, Message::SetArtistsViewMode),
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
        format_cfg: &'a crate::layout::ArtistsFormatConfig,
    ) -> Element<'a, Message> {
        let per_row = self.items_per_row();
        let total_rows = self.total_rows();
        let start_row = self.window_start / per_row;
        let end_row = self.window_end.div_ceil(per_row);
        let row_h = self.row_height();
        let tile_w = self.tile_width as f32;
        let avatar_icon_size = (tile_w * 0.32).clamp(32.0, 72.0);

        let mut content = column![].spacing(0).width(Length::Fill);

        let top = start_row as f32 * row_h;
        if top > 0.0 {
            content = content.push(Space::new().height(top).boxed());
        }

        let visible = self.visible_artists();
        for (chunk_idx, chunk) in visible.chunks(per_row).enumerate() {
            let mut tile_row = row![]
                .spacing(components::CATALOG_ROW_SPACING)
                .align_y(Alignment::Start);

            for (item_in_chunk, artist) in chunk.iter().enumerate() {
                let win_idx = chunk_idx * per_row + item_in_chunk;
                let abs_idx = self.window_start + win_idx;
                let leading = components::catalog_cover_art(
                    theme_cfg,
                    None,
                    tile_w,
                    theme_cfg.radius_md(),
                    icons::USER_AVATAR_SVG,
                    avatar_icon_size,
                    theme_cfg.primary_color(),
                    Message::SelectArtist(artist.artist.clone()),
                );

                let (line1, line2) = if let Some(cached) = self.rendered_window.get(win_idx) {
                    (
                        std::borrow::Cow::Borrowed(cached.tile_line1.as_str()),
                        std::borrow::Cow::Borrowed(cached.tile_line2.as_str()),
                    )
                } else {
                    (
                        std::borrow::Cow::Owned(format_template(&format_cfg.tile_line1, artist)),
                        std::borrow::Cow::Owned(format_template(&format_cfg.tile_line2, artist)),
                    )
                };

                tile_row = tile_row.push(
                    components::CatalogTile {
                        theme: theme_cfg,
                        tile_width: tile_w,
                        leading,
                        title: line1,
                        mid_left: line2,
                        mid_right: "".into(),
                        bottom_left: "".into(),
                        bottom_right: "".into(),
                        on_select: Message::SelectArtist(artist.artist.clone()),
                        on_play: Message::PlayScope {
                            filter: TrackFilter::artist(artist.artist.as_ref()),
                            name: artist.artist.clone(),
                            shuffle: false,
                        },
                        on_shuffle: Message::PlayScope {
                            filter: TrackFilter::artist(artist.artist.as_ref()),
                            name: artist.artist.clone(),
                            shuffle: true,
                        },
                        on_right_press: Some(Message::OpenArtistContextMenu(abs_idx)),
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

        scrollable(container(content).width(Length::Fill).padding(Padding {
            left: 16.0,
            right: 16.0,
            ..Default::default()
        }))
        .id(iced::widget::Id::new("artists_scroll"))
        .on_scroll(|scroll| Message::ArtistsScrolled(scroll.viewport))
        .height(Length::Fill)
        .style(styles::scrollable_style(theme_cfg))
        .boxed()
    }

    fn view_list<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        format_cfg: &'a crate::layout::ArtistsFormatConfig,
    ) -> Element<'a, Message> {
        let total_rows = self.artists.len();
        let row_h = components::CATALOG_LIST_ROW_HEIGHT;

        let mut content = column![].spacing(0).width(Length::Fill);

        let top = self.window_start as f32 * row_h;
        if top > 0.0 {
            content = content.push(Space::new().height(top).boxed());
        }

        let visible = self.visible_artists();
        for (win_idx, artist) in visible.iter().enumerate() {
            let abs_idx = self.window_start + win_idx;
            let (line1, line2) = if let Some(cached) = self.rendered_window.get(win_idx) {
                (
                    std::borrow::Cow::Borrowed(cached.list_line1.as_str()),
                    std::borrow::Cow::Borrowed(cached.list_line2.as_str()),
                )
            } else {
                (
                    std::borrow::Cow::Owned(format_template(&format_cfg.list_line1, artist)),
                    std::borrow::Cow::Owned(format_template(&format_cfg.list_line2, artist)),
                )
            };

            content = content.push(
                components::CatalogRow {
                    theme: theme_cfg,
                    leading: components::cover_art(
                        theme_cfg,
                        None,
                        components::CATALOG_LIST_ART_SIZE,
                        theme_cfg.radius_sm(),
                        icons::USER_AVATAR_SVG,
                        20.0,
                        theme_cfg.primary_color(),
                    ),
                    title: line1,
                    subtitle_left: line2,
                    subtitle_right: "".into(),
                    details: None,
                    on_select: Message::SelectArtist(artist.artist.clone()),
                    on_play: Message::PlayScope {
                        filter: TrackFilter::artist(artist.artist.as_ref()),
                        name: artist.artist.clone(),
                        shuffle: false,
                    },
                    on_shuffle: Message::PlayScope {
                        filter: TrackFilter::artist(artist.artist.as_ref()),
                        name: artist.artist.clone(),
                        shuffle: true,
                    },
                    on_right_press: Some(Message::OpenArtistContextMenu(abs_idx)),
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
            .id(iced::widget::Id::new("artists_scroll"))
            .on_scroll(|scroll| Message::ArtistsScrolled(scroll.viewport))
            .height(Length::Fill)
            .style(styles::scrollable_style(theme_cfg))
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::ArtistsFormatConfig;
    use crate::theme::ThemeConfig;
    use core_engine::ArtistSummary;

    #[test]
    fn test_artists_view_component_state() {
        let mut artists_view = ArtistsView::new(ViewMode::List, ArtistSortField::Artist);
        assert!(artists_view.is_empty());
        assert_eq!(artists_view.len(), 0);
        assert_eq!(artists_view.tile_width, 140);
        artists_view.set_tile_width(200);
        assert_eq!(artists_view.tile_width, 200);

        let artists_list: Vec<ArtistSummary> = (0..30)
            .map(|i| ArtistSummary {
                artist: format!("Artist {}", i).into(),
                track_count: 3,
                album_count: 1,
                stats_label: "1 albums • 3 tracks".to_string(),
            })
            .collect();

        artists_view.set_artists(artists_list);
        assert!(!artists_view.is_empty());
        assert_eq!(artists_view.len(), 30);

        artists_view.toggle_sort(ArtistSortField::Artist);
        artists_view.set_view_mode(ViewMode::Tile);

        let theme_cfg = ThemeConfig::default();
        let artists_fmt = ArtistsFormatConfig::default();
        let _rendered = artists_view.view(&theme_cfg, &artists_fmt);
    }

    #[test]
    fn test_artists_view_is_virtualized() {
        let mut view = ArtistsView::new(ViewMode::List, ArtistSortField::Artist);
        let all: Vec<ArtistSummary> = (0..10_000)
            .map(|i| ArtistSummary {
                artist: format!("Artist {}", i).into(),
                track_count: 1,
                album_count: 1,
                stats_label: "1 albums • 1 tracks".to_string(),
            })
            .collect();
        view.set_artists(all);

        // Only a small window is materialized regardless of catalog size.
        assert!(view.window_len() < 200, "window_len={}", view.window_len());
        assert_eq!(view.total_tracks, 10_000);
        assert_eq!(view.total_albums, 10_000);

        // Scrolling to the middle relocates a bounded window without growing it.
        view.on_scroll_metrics(100_000.0, 1000.0, 700.0);
        assert!(view.window_len() < 200, "window_len={}", view.window_len());

        let theme_cfg = ThemeConfig::default();
        let artists_fmt = ArtistsFormatConfig::default();
        let _rendered = view.view(&theme_cfg, &artists_fmt);
    }
}
