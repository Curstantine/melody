use iced::widget::{column, row, text_input, Space, Widget};
use iced::{Alignment, Element, Length};

use ::queue::{queue_window_for_start, queue_window_start, QueueStore, QUEUE_ROW_HEIGHT};
use component::{self, button, icons, styles, text};
use model::format_template;
use theme::ThemeConfig;

use crate::message::Message;

use super::*;

impl PlaylistsView {
    pub fn reset_queue_window(&mut self) {
        self.queue_window_start = 0;
        self.last_queue_scroll.set(0.0);
        self.last_queue_click = None;
        self.selected_queue_track = None;
        self.hovered_queue_row = None;
    }

    /// Moves the queue detail window from an absolute scroll offset in pixels.
    /// Returns `true` when the window moved. Mirrors
    /// [`crate::views::library::LibraryView::on_scroll_offset`], but the data
    /// is already in memory so no follow-up load is needed.
    pub fn on_queue_scroll_offset(&mut self, scroll_y: f32, total: usize) -> bool {
        self.last_queue_scroll.set(scroll_y);
        if total == 0 || QUEUE_ROW_HEIGHT <= 0.0 {
            return false;
        }
        let first_visible = (scroll_y.max(0.0) / QUEUE_ROW_HEIGHT).floor() as usize;
        let desired = queue_window_start(total, first_visible);
        if desired == self.queue_window_start {
            return false;
        }
        self.queue_window_start = desired;
        true
    }

    /// Handles a queue track click. Returns `true` on double-click (playback
    /// should start), mirroring
    /// [`crate::views::library::LibraryView::on_track_click`].
    pub fn on_queue_click(&mut self, queue_id: u64, index: usize) -> bool {
        let now = Instant::now();
        let is_double_click = if let Some((last_id, last_index, last_time)) = self.last_queue_click
        {
            last_id == queue_id
                && last_index == index
                && now.duration_since(last_time).as_millis() <= 400
        } else {
            false
        };

        self.selected_queue_track = Some((queue_id, index));
        if is_double_click {
            self.last_queue_click = None;
            true
        } else {
            self.last_queue_click = Some((queue_id, index, now));
            false
        }
    }

    pub fn recompute_rendered_queue(
        &mut self,
        queue_fmt: &layout::QueueFormatConfig,
        store: &QueueStore,
    ) {
        self.rendered_queue_chips = store
            .summaries
            .iter()
            .map(|q| (q.id, format_template(&queue_fmt.chip_format, q)))
            .collect();

        if let Some(active) = store.active_queue() {
            self.queue_header_tracks_str = format!("{} tracks in queue", active.len());
            let (start, end) = queue_window_for_start(active.len(), self.queue_window_start);
            self.rendered_queue_window = active.tracks[start..end]
                .iter()
                .map(|track| RenderedQueueTrack {
                    primary: format_template(&queue_fmt.track_primary, track),
                    secondary: format_template(&queue_fmt.track_secondary, track),
                    details: format_template(&queue_fmt.track_details, track),
                })
                .collect();
        } else if let Some(id) = store.active {
            let count = store.summary(id).map(|s| s.track_count).unwrap_or(0);
            self.queue_header_tracks_str = format!("{} tracks in queue", count);
            self.rendered_queue_window.clear();
        } else {
            self.queue_header_tracks_str.clear();
            self.rendered_queue_window.clear();
        }
    }

    /// Handles an open-playlist row click. Returns `true` on double-click
    /// (playback should start).
    pub(super) fn queue_header<'a>(
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        name: &'a str,
        track_count_str: &'a str,
        active_id: u64,
    ) -> Element<'a, Message> {
        row![
            column![
                text(font, name).size(15).color(theme_cfg.text),
                text(font, track_count_str)
                    .size(11)
                    .color(theme_cfg.text_muted),
            ]
            .spacing(2)
            .width(Length::Fill),
            component::chip_with_icon(
                theme_cfg,
                font,
                icons::PLAYLIST_SVG,
                "Save as playlist",
                false,
                Message::QueueSaveClicked,
            ),
            component::chip_with_icon(
                theme_cfg,
                font,
                icons::TRASH_SVG,
                "Delete queue",
                false,
                Message::QueueDelete(active_id),
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .padding(iced::Padding {
            top: 6.0,
            right: 16.0,
            bottom: 8.0,
            left: 16.0,
        })
        .boxed()
    }

    /// "Save queue as playlist" inline form, shown when `saving_queue` is set.
    pub(super) fn queue_save_form<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        if self.saving_queue {
            row![
                text_input("Save queue as playlist...", &self.queue_save_input)
                    .size(12)
                    .padding([6, 10])
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::QueueSaveNameChanged)
                    .on_submit(Message::QueueSaveSubmitted(self.queue_save_input.clone()))
                    .width(Length::Fixed(280.0)),
                Space::new().width(8),
                button(text(font, "Save").size(12).color(theme_cfg.text))
                    .padding([6, 12])
                    .style(styles::ghost_button(theme_cfg, false))
                    .on_press(Message::QueueSaveSubmitted(self.queue_save_input.clone())),
                Space::new().width(4),
                button(text(font, "Cancel").size(12).color(theme_cfg.text_muted))
                    .padding([6, 12])
                    .style(styles::ghost_button(theme_cfg, false))
                    .on_press(Message::QueueSaveCancelled),
            ]
            .align_y(Alignment::Center)
            .padding(iced::Padding {
                top: 0.0,
                right: 16.0,
                bottom: 8.0,
                left: 16.0,
            })
            .boxed()
        } else {
            Space::new().height(0).boxed()
        }
    }
}
