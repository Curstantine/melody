use core_engine::{ArtistSummary, SortDirection, TrackFilter};
use iced::border::Radius;
use iced::widget::{column, container, row, scrollable, Space};
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
        Self {
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
        }
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
        self.window_start = 0;
        self.window_end = 0;
        self.total_tracks = 0;
        self.total_albums = 0;
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

    pub fn view<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
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
        let avatar_icon_size = (tile_w * 0.32).clamp(32.0, 72.0);

        let mut content = column![].spacing(0).width(Length::Fill);

        let top = start_row as f32 * row_h;
        if top > 0.0 {
            content = content.push(Space::new().height(top));
        }

        for chunk in self.visible_artists().chunks(per_row) {
            let mut tile_row = row![]
                .spacing(components::CATALOG_ROW_SPACING)
                .align_y(Alignment::Start);

            for artist in chunk {
                let leading = components::cover_art(
                    theme_cfg,
                    None,
                    tile_w,
                    Radius::default(),
                    icons::USER_AVATAR_SVG,
                    avatar_icon_size,
                    theme_cfg.primary_color(),
                );

                tile_row = tile_row.push(
                    components::CatalogTile {
                        theme: theme_cfg,
                        tile_width: tile_w,
                        leading,
                        title: artist.artist.as_ref(),
                        mid_left: artist.stats_label.as_str(),
                        mid_right: "",
                        bottom_left: "",
                        bottom_right: "",
                        on_select: Message::SelectArtist(artist.artist.to_string()),
                        on_play: Message::PlayScope {
                            filter: TrackFilter::artist(artist.artist.as_ref()),
                            name: artist.artist.as_ref().to_string(),
                            shuffle: false,
                        },
                        on_shuffle: Message::PlayScope {
                            filter: TrackFilter::artist(artist.artist.as_ref()),
                            name: artist.artist.as_ref().to_string(),
                            shuffle: true,
                        },
                    }
                    .view(),
                );
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
            content = content.push(Space::new().height(bottom));
        }

        scrollable(container(content).width(Length::Fill).padding(Padding {
            left: 16.0,
            right: 16.0,
            ..Default::default()
        }))
        .id(iced::widget::Id::new("artists_scroll"))
        .on_scroll(Message::ArtistsScrolled)
        .height(Length::Fill)
        .style(styles::scrollable_style(theme_cfg))
        .into()
    }

    fn view_list<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let total_rows = self.artists.len();
        let row_h = components::CATALOG_LIST_ROW_HEIGHT;

        let mut content = column![].spacing(0).width(Length::Fill);

        let top = self.window_start as f32 * row_h;
        if top > 0.0 {
            content = content.push(Space::new().height(top));
        }

        for artist in self.visible_artists() {
            content = content.push(
                components::CatalogRow {
                    theme: theme_cfg,
                    leading: components::cover_art(
                        theme_cfg,
                        None,
                        components::CATALOG_LIST_ART_SIZE,
                        Radius::from(4.0),
                        icons::USER_AVATAR_SVG,
                        20.0,
                        theme_cfg.primary_color(),
                    ),
                    title: artist.artist.as_ref(),
                    subtitle_left: artist.stats_label.as_str(),
                    subtitle_right: "",
                    details: None,
                    on_select: Message::SelectArtist(artist.artist.to_string()),
                    on_play: Message::PlayScope {
                        filter: TrackFilter::artist(artist.artist.as_ref()),
                        name: artist.artist.as_ref().to_string(),
                        shuffle: false,
                    },
                    on_shuffle: Message::PlayScope {
                        filter: TrackFilter::artist(artist.artist.as_ref()),
                        name: artist.artist.as_ref().to_string(),
                        shuffle: true,
                    },
                    width: Length::Fill,
                    height: Length::Fixed(row_h),
                }
                .view(),
            );
        }

        let bottom = total_rows.saturating_sub(self.window_end) as f32 * row_h;
        if bottom > 0.0 {
            content = content.push(Space::new().height(bottom));
        }

        scrollable(content)
            .id(iced::widget::Id::new("artists_scroll"))
            .on_scroll(Message::ArtistsScrolled)
            .height(Length::Fill)
            .style(styles::scrollable_style(theme_cfg))
            .into()
    }
}
