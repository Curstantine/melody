use core_engine::{ArtistSummary, SortDirection, TrackFilter};
use iced::border::Radius;
use iced::widget::{button, column, container, row, scrollable, Space};
use iced::{Alignment, Border, Element, Length, Padding};

use crate::components::{self, text};
use crate::icons;
use crate::message::{ArtistSortField, Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

pub const DEFAULT_ARTIST_TILE_WIDTH: u32 = 140;
/// Fixed pixel height of a single list row.
pub const LIST_ROW_HEIGHT: f32 = 52.0;
/// Height of the metadata block under a tile.
pub const TILE_META_HEIGHT: f32 = 64.0;
/// Vertical gap between catalog rows (baked into the virtualized row height).
pub const ROW_SPACING: f32 = 14.0;

const DEFAULT_COLUMNS: usize = 6;
const DEFAULT_VIEWPORT_HEIGHT: f32 = 720.0;
const HORIZONTAL_PADDING: f32 = 32.0;

/// Computes how many tiles fit in a viewport of `width` pixels.
fn columns_for_width(width: f32, tile_width: u32) -> usize {
    let usable = (width - HORIZONTAL_PADDING).max(tile_width as f32);
    let tile = tile_width as f32;
    (((usable + ROW_SPACING) / (tile + ROW_SPACING)).floor() as usize).max(1)
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

    fn avatar_size(&self) -> f32 {
        (self.tile_width as f32 * 0.5).clamp(50.0, 100.0)
    }

    pub fn row_height(&self) -> f32 {
        match self.view_mode {
            ViewMode::Tile => self.avatar_size() + TILE_META_HEIGHT + ROW_SPACING,
            ViewMode::List => LIST_ROW_HEIGHT,
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
            Space::with_width(Length::Fill),
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

        let mut content = column![].spacing(0).width(Length::Fill);

        let top = start_row as f32 * row_h;
        if top > 0.0 {
            content = content.push(Space::with_height(top));
        }

        for chunk in self.visible_artists().chunks(per_row) {
            let mut tile_row = row![].spacing(ROW_SPACING).align_y(Alignment::Start);

            for artist in chunk {
                tile_row = tile_row.push(self.tile(artist, theme_cfg));
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
        .id(scrollable::Id::new("artists_scroll"))
        .on_scroll(Message::ArtistsScrolled)
        .height(Length::Fill)
        .style(styles::scrollable_style(theme_cfg))
        .into()
    }

    fn tile<'a>(
        &self,
        artist: &'a ArtistSummary,
        theme_cfg: &'a ThemeConfig,
    ) -> Element<'a, Message> {
        let tile_w = self.tile_width as f32;
        let avatar_size = self.avatar_size();
        let icon_size = (avatar_size * 0.52).clamp(24.0, 52.0);

        let avatar = container(icons::icon(
            icons::USER_AVATAR_SVG,
            icon_size,
            theme_cfg.primary_color(),
        ))
        .center(avatar_size) // sets width + height and centers the icon
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
            border: Border {
                radius: Radius::from(avatar_size / 2.0),
                color: theme_cfg.border_color(),
                width: 1.0,
            },
            ..Default::default()
        });

        let meta = container(
            column![
                text(theme_cfg, artist.artist.as_ref())
                    .size(13)
                    .color(theme_cfg.text_color()),
                text(theme_cfg, artist.stats_label.as_str())
                    .size(10)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(3)
            .align_x(Alignment::Center),
        )
        .padding(8)
        .center_x(Length::Fill) // fills the tile width and centers the column
        .height(TILE_META_HEIGHT);

        let card = container(
            column![avatar, meta]
                .align_x(Alignment::Center)
                .width(Length::Fill),
        )
        .width(tile_w)
        .height(avatar_size + TILE_META_HEIGHT);

        let select_btn = button(card)
            .style(styles::tile(theme_cfg))
            .padding(0)
            .on_press(Message::SelectArtist(artist.artist.to_string()));

        select_btn.into()
    }

    fn view_list<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let total_rows = self.artists.len();
        let row_h = LIST_ROW_HEIGHT;

        let mut content = column![].spacing(0).width(Length::Fill);

        let top = self.window_start as f32 * row_h;
        if top > 0.0 {
            content = content.push(Space::with_height(top));
        }

        for artist in self.visible_artists() {
            let avatar = container(icons::icon(
                icons::USER_AVATAR_SVG,
                18.0,
                theme_cfg.primary_color(),
            ))
            .width(36)
            .height(36)
            .center_x(36)
            .center_y(36)
            .style(move |_theme| container::Style {
                background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
                border: Border {
                    color: theme_cfg.border_color(),
                    width: 1.0,
                    radius: Radius::from(18.0),
                },
                ..Default::default()
            });

            let title_block = column![
                text(theme_cfg, artist.artist.as_ref())
                    .size(13)
                    .color(theme_cfg.text_color()),
                text(theme_cfg, artist.stats_label.as_str())
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(2)
            .width(Length::Fill);

            let artist_row = row![avatar, Space::with_width(12), title_block,]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding([6, 12]);

            let select_btn = button(artist_row)
                .style(styles::tile(theme_cfg))
                .width(Length::Fill)
                .on_press(Message::SelectArtist(artist.artist.to_string()));

            let play_btn = button(icons::icon(
                icons::PLAY_SVG,
                14.0,
                theme_cfg.text_muted_color(),
            ))
            .style(styles::icon_control_button(theme_cfg))
            .padding(4)
            .on_press(Message::PlayScope {
                filter: TrackFilter::artist(artist.artist.as_ref()),
                name: artist.artist.as_ref().to_string(),
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
                filter: TrackFilter::artist(artist.artist.as_ref()),
                name: artist.artist.as_ref().to_string(),
                shuffle: true,
            });

            let actions = row![play_btn, shuffle_btn]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding([0, 12]);

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
            .id(scrollable::Id::new("artists_scroll"))
            .on_scroll(Message::ArtistsScrolled)
            .height(Length::Fill)
            .style(styles::scrollable_style(theme_cfg))
            .into()
    }
}
