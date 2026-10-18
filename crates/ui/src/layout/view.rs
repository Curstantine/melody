use iced::widget::{button, column, container, mouse_area, row, stack, Space, Widget};
use iced::{Element, Length};

use crate::app::App;
use crate::components::text;
use crate::icons;
use crate::layout::schema::{ContainerStyle, LayoutComponent, LayoutNode, Orientation};
use crate::message::{Message, ViewTab};
use crate::theme::{styles, ThemeConfig};
use crate::views::{self, StatusBar, StatusBarContext};

impl App {
    pub fn render_layout<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let root = &self.layout_manager.current_template.root;
        let layout_element = self.render_layout_node(root, theme_cfg);

        container(layout_element)
            .style(styles::main_background(theme_cfg))
            .width(Length::Fill)
            .height(Length::Fill)
            .boxed()
    }

    pub fn render_layout_node<'a>(
        &'a self,
        node: &'a LayoutNode,
        theme_cfg: &'a ThemeConfig,
    ) -> Element<'a, Message> {
        match node {
            LayoutNode::Column {
                children,
                width,
                height,
                spacing,
                padding,
                align,
            } => {
                let mut col = column![]
                    .spacing(*spacing)
                    .padding(to_iced_padding(*padding))
                    .align_x(align.to_iced_alignment())
                    .width(width.to_iced_length())
                    .height(height.to_iced_length());

                for child in children {
                    col = col.push(self.render_layout_node(child, theme_cfg));
                }
                col.boxed()
            }

            LayoutNode::Row {
                children,
                width,
                height,
                spacing,
                padding,
                align,
            } => {
                let mut r = row![]
                    .spacing(*spacing)
                    .padding(to_iced_padding(*padding))
                    .align_y(align.to_iced_alignment())
                    .width(width.to_iced_length())
                    .height(height.to_iced_length());

                for child in children {
                    r = r.push(self.render_layout_node(child, theme_cfg));
                }
                r.boxed()
            }

            LayoutNode::Stack {
                children,
                width,
                height,
            } => {
                let mut st = stack![]
                    .width(width.to_iced_length())
                    .height(height.to_iced_length());

                for child in children {
                    st = st.push(self.render_layout_node(child, theme_cfg));
                }
                st.boxed()
            }

            LayoutNode::Container {
                child,
                width,
                height,
                padding,
                style,
            } => {
                let content = self.render_layout_node(child, theme_cfg);
                let cont = container(content)
                    .padding(to_iced_padding(*padding))
                    .width(width.to_iced_length())
                    .height(height.to_iced_length());

                match style {
                    ContainerStyle::Surface => cont.style(styles::sidebar_container(theme_cfg)),
                    ContainerStyle::MainBackground => {
                        cont.style(styles::main_background(theme_cfg))
                    }
                    ContainerStyle::Panel => cont.style(styles::sidebar_container(theme_cfg)),
                    ContainerStyle::Transparent => cont,
                }
                .boxed()
            }

            LayoutNode::Divider { orientation, size } => match orientation {
                Orientation::Horizontal => container("")
                    .width(Length::Fill)
                    .height(Length::Fixed(*size))
                    .style(move |_| container::Style {
                        background: Some(iced::Background::Color(theme_cfg.border_color())),
                        ..Default::default()
                    })
                    .boxed(),
                Orientation::Vertical => container("")
                    .width(Length::Fixed(*size))
                    .height(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(iced::Background::Color(theme_cfg.border_color())),
                        ..Default::default()
                    })
                    .boxed(),
            },

            LayoutNode::Spacer { width, height } => Space::new()
                .width(width.to_iced_length())
                .height(height.to_iced_length())
                .boxed(),

            LayoutNode::Component(component) => self.render_component(component, theme_cfg),
        }
    }

    pub fn render_component<'a>(
        &'a self,
        component: &'a LayoutComponent,
        theme_cfg: &'a ThemeConfig,
    ) -> Element<'a, Message> {
        match component {
            LayoutComponent::Navbar {
                orientation,
                show_settings,
            } => self.render_navbar(*orientation, *show_settings, theme_cfg),

            LayoutComponent::MainView => container(self.render_main_view(theme_cfg))
                .width(Length::Fill)
                .height(Length::Fill)
                .clip(true)
                .boxed(),

            LayoutComponent::Tracks => container(self.library.view(
                theme_cfg,
                self.current_track_id,
                self.is_playing,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true)
            .boxed(),

            LayoutComponent::Albums => container(self.albums.view(theme_cfg))
                .width(Length::Fill)
                .height(Length::Fill)
                .clip(true)
                .boxed(),

            LayoutComponent::Artists => container(self.artists.view(theme_cfg))
                .width(Length::Fill)
                .height(Length::Fill)
                .clip(true)
                .boxed(),

            LayoutComponent::Playlists => container(self.playlists.view(
                theme_cfg,
                &self.queues,
                self.dragging_queue_track,
                self.dragging_detail,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true)
            .boxed(),

            LayoutComponent::Queue => container(self.playlists.view_queues(
                theme_cfg,
                &self.queues,
                self.dragging_queue_track,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true)
            .boxed(),

            LayoutComponent::Lyrics => {
                let next_track = self.get_next_track();
                container(self.now_playing.view(
                    theme_cfg,
                    self.current_track.as_ref(),
                    self.current_cover.as_ref(),
                    self.current_blurred_cover.as_ref(),
                    next_track,
                    self.current_next_cover.as_ref(),
                    self.is_repeat,
                    self.is_shuffle,
                    self.current_tech_info.as_ref(),
                ))
                .width(Length::Fill)
                .height(Length::Fill)
                .clip(true)
                .boxed()
            }

            LayoutComponent::Settings => container(self.render_settings_view(theme_cfg))
                .width(Length::Fill)
                .height(Length::Fill)
                .clip(true)
                .boxed(),

            LayoutComponent::PlayerBar => self.player_bar.view(
                theme_cfg,
                self.current_track.as_ref(),
                self.current_cover.as_ref(),
                self.is_playing,
                self.position_ms,
                self.duration_ms,
                self.volume,
                self.is_shuffle,
                self.is_repeat,
            ),

            LayoutComponent::TrackInfo => container(self.player_bar.view_track_info(
                theme_cfg,
                self.current_track.as_ref(),
                self.current_cover.as_ref(),
            ))
            .padding([4, 8])
            .boxed(),

            LayoutComponent::PlayerControls => {
                container(self.player_bar.view_controls(theme_cfg, self.is_playing))
                    .padding([4, 8])
                    .boxed()
            }

            LayoutComponent::SeekBar => container(self.player_bar.view_seek_bar(
                theme_cfg,
                self.position_ms,
                self.duration_ms,
            ))
            .padding([4, 8])
            .boxed(),

            LayoutComponent::VolumeControl => {
                container(self.player_bar.view_volume(theme_cfg, self.volume))
                    .padding([4, 8])
                    .boxed()
            }

            LayoutComponent::PlaybackOptions => container(self.player_bar.view_options(
                theme_cfg,
                self.is_shuffle,
                self.is_repeat,
            ))
            .padding([4, 8])
            .boxed(),

            LayoutComponent::StatusBar => {
                StatusBar::view(theme_cfg, self.create_status_bar_context())
            }
        }
    }

    pub fn render_navbar<'a>(
        &'a self,
        orientation: Orientation,
        show_settings: bool,
        theme_cfg: &'a ThemeConfig,
    ) -> Element<'a, Message> {
        let is_any_dragging = self.dragging_tab.is_some();

        let tab_buttons = self.tab_order.iter().map(|&tab| -> Element<'_, Message> {
            let is_selected = self.active_tab == tab;
            let is_dragging = self.dragging_tab == Some(tab);
            let is_hovered = self.hovered_tab == Some(tab);

            let tab_icon_color = if is_selected || is_dragging {
                theme_cfg.primary_color()
            } else if is_hovered {
                theme_cfg.text_color()
            } else {
                theme_cfg.text_muted_color()
            };

            let tab_inner = row![
                icons::icon(tab.icon_svg(), 12.0, tab_icon_color),
                text(theme_cfg, tab.label())
                    .size(11)
                    .line_height(1.0)
                    .color(if is_selected || is_dragging {
                        theme_cfg.primary_color()
                    } else if is_hovered {
                        theme_cfg.text_color()
                    } else {
                        theme_cfg.text_muted_color()
                    }),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center);

            let mut tab_container =
                container(tab_inner)
                    .padding([5, 10])
                    .style(styles::header_tab_container(
                        theme_cfg,
                        is_selected,
                        is_hovered,
                        is_dragging,
                    ));

            if orientation == Orientation::Vertical {
                tab_container = tab_container.width(Length::Fill);
            }

            mouse_area(tab_container)
                .on_press(Message::TabDragStarted(tab))
                .on_release(Message::TabDragReleased)
                .on_enter(if is_any_dragging {
                    Message::TabDraggedOver(tab)
                } else {
                    Message::TabHovered(Some(tab))
                })
                .on_exit(Message::TabHovered(None))
                .interaction(iced::mouse::Interaction::Pointer)
                .boxed()
        });

        let settings_btn = button(icons::icon(
            icons::SETTINGS_SVG,
            14.0,
            if self.active_tab == ViewTab::Settings {
                theme_cfg.primary_color()
            } else {
                theme_cfg.text_muted_color()
            },
        ))
        .padding([4, 8])
        .style(styles::icon_control_button(theme_cfg))
        .on_press(Message::SelectTab(ViewTab::Settings));

        match orientation {
            Orientation::Horizontal => {
                let mut nav_row = row![].spacing(4).align_y(iced::Alignment::Center);
                for btn in tab_buttons {
                    nav_row = nav_row.push(btn);
                }
                if show_settings {
                    nav_row = nav_row.push(Space::new().width(Length::Fill).boxed());
                    nav_row = nav_row.push(settings_btn.boxed());
                }

                container(nav_row.padding([4, 12]).height(32))
                    .style(styles::top_bar_container(theme_cfg))
                    .width(Length::Fill)
                    .height(32)
                    .boxed()
            }

            Orientation::Vertical => {
                let mut nav_col = column![]
                    .spacing(4)
                    .align_x(iced::Alignment::Start)
                    .width(Length::Fill);

                for btn in tab_buttons {
                    nav_col = nav_col.push(btn);
                }

                if show_settings {
                    nav_col = nav_col.push(Space::new().height(Length::Fill).boxed());
                    let settings_is_selected = self.active_tab == ViewTab::Settings;
                    let settings_row = row![
                        icons::icon(
                            icons::SETTINGS_SVG,
                            12.0,
                            if settings_is_selected {
                                theme_cfg.primary_color()
                            } else {
                                theme_cfg.text_muted_color()
                            },
                        ),
                        text(theme_cfg, "Settings").size(11).line_height(1.0).color(
                            if settings_is_selected {
                                theme_cfg.primary_color()
                            } else {
                                theme_cfg.text_muted_color()
                            }
                        )
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center);

                    let settings_container = mouse_area(
                        container(settings_row)
                            .padding([5, 10])
                            .width(Length::Fill)
                            .style(styles::header_tab_container(
                                theme_cfg,
                                settings_is_selected,
                                false,
                                false,
                            )),
                    )
                    .on_press(Message::SelectTab(ViewTab::Settings))
                    .interaction(iced::mouse::Interaction::Pointer);

                    nav_col = nav_col.push(settings_container.boxed());
                }

                container(nav_col.padding([8, 8]))
                    .style(styles::top_bar_container(theme_cfg))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .boxed()
            }
        }
    }

    pub fn render_main_view<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        match self.active_tab {
            ViewTab::Tracks => self
                .library
                .view(theme_cfg, self.current_track_id, self.is_playing),
            ViewTab::Albums => self.albums.view(theme_cfg),
            ViewTab::Artists => self.artists.view(theme_cfg),
            ViewTab::Playlists => self.playlists.view(
                theme_cfg,
                &self.queues,
                self.dragging_queue_track,
                self.dragging_detail,
            ),
            ViewTab::Lyrics => {
                let next_track = self.get_next_track();

                self.now_playing.view(
                    theme_cfg,
                    self.current_track.as_ref(),
                    self.current_cover.as_ref(),
                    self.current_blurred_cover.as_ref(),
                    next_track,
                    self.current_next_cover.as_ref(),
                    self.is_repeat,
                    self.is_shuffle,
                    self.current_tech_info.as_ref(),
                )
            }
            ViewTab::Settings => self.render_settings_view(theme_cfg),
        }
    }

    pub fn render_settings_view<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let cache_stats = self.cache_stats;
        let db_size_bytes = self.db_size_bytes;

        self.settings.view(
            theme_cfg,
            views::settings::SettingsContext {
                theme_names: self.theme_manager.theme_names(),
                active_theme_name: &self.theme_manager.active_theme_name,
                layout_names: self.layout_manager.layout_names(),
                active_layout_name: &self.layout_manager.active_layout_name,
                albums_view_mode: self.albums.view_mode,
                artists_view_mode: self.artists.view_mode,
                page_size: self.library.page_size,
                album_tile_width: self.albums.tile_width,
                artist_tile_width: self.artists.tile_width,
                now_playing_cover_size: self.now_playing.cover_size,
                primary_font: self.theme_manager.current_config.font_family.as_deref(),
                total_tracks: self.library.total_tracks,
                total_albums: self.albums.len(),
                total_artists: self.artists.len(),
                db_size_bytes,
                cache_file_count: cache_stats.file_count,
                cache_size_bytes: cache_stats.total_bytes,
                is_scanning: self.is_scanning,

                listenbrainz_enabled: self.scrobble_config.listenbrainz_enabled,
                listenbrainz_token: &self.scrobble_config.listenbrainz_token,
                listenbrainz_api_url: &self.scrobble_config.listenbrainz_api_url,

                lastfm_enabled: self.scrobble_config.lastfm_enabled,
                lastfm_api_key: &self.scrobble_config.lastfm_api_key,
                lastfm_api_secret: &self.scrobble_config.lastfm_api_secret,
                lastfm_session_key: &self.scrobble_config.lastfm_session_key,
                lastfm_api_url: &self.scrobble_config.lastfm_api_url,

                submit_now_playing: self.scrobble_config.submit_now_playing,
                scrobble_percent: self.scrobble_config.scrobble_percent,
                gapless: self.gapless,
                watch_library: self.watch_library,
            },
        )
    }

    pub fn create_status_bar_context(&self) -> StatusBarContext<'_> {
        StatusBarContext {
            active_tab: self.active_tab,
            total_tracks: self.library.total_tracks,
            search_query: &self.library.search_query,
            album_count: self.albums.albums.len(),
            album_tracks: self.albums.total_tracks,
            album_duration_ms: self.albums.total_duration_ms,
            artist_count: self.artists.artists.len(),
            artist_albums: self.artists.total_albums,
            artist_tracks: self.artists.total_tracks,
            current_track: self.current_track.as_ref(),
            is_playing: self.is_playing,
            playlist_count: self.playlists.len(),
            queue_count: self.queues.summaries.len(),
            active_queue: self
                .queues
                .active_queue()
                .map(|q| (q.name.as_str(), q.len())),
            is_scanning: self.is_scanning,
            scan_progress: self.scan_progress,
            is_generating_thumbnails: self.is_generating_thumbnails,
        }
    }
}

fn to_iced_padding(p: [f32; 4]) -> iced::Padding {
    iced::Padding {
        top: p[0],
        right: p[1],
        bottom: p[2],
        left: p[3],
    }
}
