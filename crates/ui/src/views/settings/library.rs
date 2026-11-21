use iced::widget::{column, container, row, toggler, Space, Widget};
use iced::{Alignment, Border, Element, Length};

use crate::message::Message;
use component::{self, button, icons, styles, text};
use theme::ThemeConfig;

use super::*;

impl SettingsView {
    pub(super) fn view_library<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = component::settings_header(theme_cfg, font, "Library & storage", Some("Manage indexed audio directories, rescan metadata, monitor disk caches, or reset storage"));

        // Storage and Index Metrics
        let storage_stats_row1 = row![
            component::metric_box(
                theme_cfg,
                font,
                "Songs indexed",
                format!("{}", ctx.total_tracks)
            ),
            component::metric_box(
                theme_cfg,
                font,
                "Albums cataloged",
                format!("{}", ctx.total_albums)
            ),
            component::metric_box(
                theme_cfg,
                font,
                "Artists cataloged",
                format!("{}", ctx.total_artists)
            ),
        ]
        .spacing(CHIP_ROW_SPACING);

        let storage_stats_row2 = row![
            component::metric_box(
                theme_cfg,
                font,
                "Database file size",
                format_file_size(ctx.db_size_bytes),
            ),
            component::metric_box(
                theme_cfg,
                font,
                "Disk thumbnail cache",
                format!(
                    "{} files ({})",
                    ctx.cache_file_count,
                    format_file_size(ctx.cache_size_bytes)
                ),
            ),
        ]
        .spacing(CHIP_ROW_SPACING);

        let stats_card = component::settings_card(
            theme_cfg,
            font,
            "Storage & index metrics",
            column![storage_stats_row1, storage_stats_row2]
                .spacing(CARD_SPACING)
                .boxed(),
        );

        let add_folder_btn = button(
            row![
                icons::icon(icons::PLUS_SVG, 13.0, theme_cfg.primary),
                Space::new().width(SPACE_MD),
                text(
                    font,
                    if ctx.is_scanning {
                        "Scanning library..."
                    } else {
                        "Add music folder"
                    }
                )
                .size(12)
                .color(theme_cfg.text),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press_maybe(if ctx.is_scanning {
            None
        } else {
            Some(Message::AddLibraryFolderClicked)
        });

        let folders_content: Element<'a, Message> = if self.library_folders.is_empty() {
            container(
                column![
                    text(font, "No music directories configured yet.")
                        .size(13)
                        .color(theme_cfg.text_muted),
                    text(
                        font,
                        "Click 'Add music folder' to choose a directory containing audio files."
                    )
                    .size(11)
                    .color(theme_cfg.text_muted),
                ]
                .spacing(FORM_FIELD_SPACING)
                .align_x(Alignment::Center),
            )
            .padding(EMPTY_STATE_PADDING)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .style(move |_theme| container::Style {
                background: Some(iced::Background::Color(theme_cfg.surface)),
                border: Border {
                    radius: theme_cfg.border_radius.sm,
                    color: theme_cfg.border,
                    width: 1.0,
                },
                ..Default::default()
            })
            .boxed()
        } else {
            let mut col = column![].spacing(CHIP_ROW_SPACING);
            for folder_path in &self.library_folders {
                let path_clone = folder_path.clone();
                let path_buf = PathBuf::from(folder_path);

                let rescan_btn = button(icons::icon(icons::REFRESH_SVG, 12.0, theme_cfg.text))
                    .padding(BUTTON_PADDING_SM)
                    .style(styles::ghost_button(theme_cfg, false))
                    .on_press_maybe(if ctx.is_scanning {
                        None
                    } else {
                        Some(Message::RescanLibraryFolder(path_buf))
                    });

                let remove_btn = button(icons::icon(icons::TRASH_SVG, 12.0, theme_cfg.danger))
                    .padding(BUTTON_PADDING_SM)
                    .style(styles::ghost_button(theme_cfg, false))
                    .on_press_maybe(if ctx.is_scanning {
                        None
                    } else {
                        Some(Message::RemoveLibraryFolder(path_clone))
                    });

                let item_row = container(
                    row![
                        text(font, folder_path.as_str())
                            .size(12)
                            .color(theme_cfg.text)
                            .width(Length::Fill),
                        rescan_btn,
                        Space::new().width(SPACE_MD),
                        remove_btn,
                    ]
                    .align_y(Alignment::Center),
                )
                .padding(ROW_ITEM_PADDING)
                .width(Length::Fill)
                .style(move |_theme| container::Style {
                    background: Some(iced::Background::Color(theme_cfg.surface)),
                    border: Border {
                        radius: theme_cfg.border_radius.sm,
                        color: theme_cfg.border,
                        width: 1.0,
                    },
                    ..Default::default()
                });

                col = col.push(item_row.boxed());
            }
            col.boxed()
        };

        let watch_dirs_row = component::settings_row(
            theme_cfg,
            font,
            "Watch library folders",
            "Automatically monitor directories and scan for new audio files",
            toggler(ctx.watch_library)
                .on_toggle(Message::ToggleWatchLibrary)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        );

        let folders_card = component::settings_card(
            theme_cfg,
            font,
            "Scanned directories",
            column![
                watch_dirs_row,
                Space::new().height(SPACE_LG),
                folders_content,
                Space::new().height(CARD_SPACING),
                add_folder_btn
            ]
            .boxed(),
        );

        // Maintenance & Actions Card
        let scan_new_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.primary),
                Space::new().width(SPACE_MD),
                text(
                    font,
                    if ctx.is_scanning {
                        "Scanning new files..."
                    } else {
                        "Scan for new files"
                    }
                )
                .size(12)
                .color(theme_cfg.primary),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press_maybe(if ctx.is_scanning {
            None
        } else {
            Some(Message::ScanLibraryForNewFiles)
        });

        let reindex_all_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.primary),
                Space::new().width(SPACE_MD),
                text(
                    font,
                    if ctx.is_scanning {
                        "Reindexing library..."
                    } else {
                        "Reindex entire library"
                    }
                )
                .size(12)
                .color(theme_cfg.primary),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press_maybe(if ctx.is_scanning {
            None
        } else {
            Some(Message::ReindexLibrary)
        });

        let clear_cache_btn = button(
            row![
                icons::icon(icons::TRASH_SVG, 13.0, theme_cfg.text),
                Space::new().width(SPACE_MD),
                text(font, "Clear thumbnail cache")
                    .size(12)
                    .color(theme_cfg.text),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::ClearThumbnailCache);

        let clear_db_btn = button(
            row![
                icons::icon(icons::TRASH_SVG, 13.0, theme_cfg.danger),
                Space::new().width(SPACE_MD),
                text(font, "Delete library database")
                    .size(12)
                    .color(theme_cfg.danger),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::danger_button(theme_cfg))
        .on_press_maybe(if ctx.is_scanning {
            None
        } else {
            Some(Message::ClearLibrary)
        });

        let actions_card = component::settings_card(
            theme_cfg,
            font,
            "Library actions & maintenance",
            column![
                component::settings_row(
                    theme_cfg,
                    font,
                    "Scan for new files",
                    "Quickly checks library folders and indexes newly added files without rescanning unchanged tracks.",
                    scan_new_btn,
                ),
                Space::new().height(SPACE_MD),
                component::settings_row(
                    theme_cfg,
                    font,
                    "Full reindex",
                    "Scans all configured music folders, updates metadata tags, and repopulates the library.",
                    reindex_all_btn,
                ),
                Space::new().height(SPACE_MD),
                component::settings_row(
                    theme_cfg,
                    font,
                    "Purge thumbnail cache",
                    format!(
                        "Deletes all {} cached thumbnail images ({}) from disk and evicts active decoded cover textures from memory. Thumbnails will regenerate when viewed.",
                        ctx.cache_file_count,
                        format_file_size(ctx.cache_size_bytes),
                    ),
                    clear_cache_btn,
                ),
                Space::new().height(SPACE_MD),
                component::settings_danger_row(
                    theme_cfg,
                    font,
                    "Reset library database",
                    "Clears all indexed tracks, albums, and artist metadata from database. Your audio files on disk remain untouched.",
                    clear_db_btn,
                ),
            ]
            .spacing(CARD_SPACING).boxed(),
        );

        column![header, stats_card, folders_card, actions_card,]
            .spacing(PAGE_SPACING)
            .padding(PAGE_PADDING)
            .width(Length::Fill)
            .boxed()
    }
}
