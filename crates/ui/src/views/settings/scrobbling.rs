use iced::widget::{button, column, container, row, slider, text_input, toggler, Space, Widget};
use iced::{Alignment, Border, Color, Element, Length};

use crate::message::Message;
use component::{self, icons, styles, text};
use theme::ThemeConfig;

use super::*;

impl SettingsView {
    pub(super) fn view_scrobbling<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = component::settings_header(theme_cfg, font, "Scrobbling & social audio", Some("Track your listening habits and broadcast live playback status to ListenBrainz and Last.fm"));

        // 1. General Preferences Card
        let submit_now_playing_row = component::settings_row(
            theme_cfg,
            font,
            "Submit \"Now Playing\" status",
            "Broadcast current track to active scrobbler profiles in real-time",
            toggler(ctx.submit_now_playing)
                .on_toggle(Message::ToggleSubmitNowPlaying)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        );

        let threshold_row = component::settings_row(
            theme_cfg,
            font,
            "Scrobble threshold",
            "Percentage of track duration that must play before recording a scrobble",
            row![
                text(font, format!("{}%", ctx.scrobble_percent))
                    .size(12)
                    .color(theme_cfg.primary)
                    .width(Length::Fixed(45.0)),
                Space::new().width(SPACE_SM),
                slider(25.0..=100.0, ctx.scrobble_percent as f32, |val| {
                    Message::SetScrobblePercent(val.round() as u32)
                })
                .on_release(Message::SaveConfig)
                .style(styles::settings_slider_style(theme_cfg))
                .width(Length::Fixed(SETTING_SLIDER_WIDTH)),
            ]
            .align_y(Alignment::Center),
        );

        let prefs_card = component::settings_card(
            theme_cfg,
            font,
            "Scrobble preferences",
            column![submit_now_playing_row, threshold_row]
                .spacing(SECTION_SPACING)
                .boxed(),
        );

        // 2. ListenBrainz Card
        let lb_test_banner = if let Some((plugin::scrobbler::ScrobbleService::ListenBrainz, res)) =
            self.scrobble_test_status.as_ref()
        {
            let (color, msg) = match res {
                Ok(ref m) => (theme_cfg.success, m.as_str()),
                Err(ref e) => (theme_cfg.danger, e.as_str()),
            };
            Some(
                container(text(font, msg).size(12).color(color))
                    .padding(BANNER_PADDING)
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(
                            color.r, color.g, color.b, 0.1,
                        ))),
                        border: Border {
                            radius: theme_cfg.border_radius.sm,
                            color,
                            width: 1.0,
                        },
                        ..Default::default()
                    }),
            )
        } else {
            None
        };

        let lb_test_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 12.0, theme_cfg.primary),
                Space::new().width(SPACE_MD),
                text(font, "Test connection")
                    .size(11)
                    .color(theme_cfg.primary),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::TestScrobble(
            plugin::scrobbler::ScrobbleService::ListenBrainz,
        ));

        let mut lb_column = column![
            row![
                text(font, "Enable")
                    .size(12)
                    .color(theme_cfg.text_muted)
                    .width(Length::Fill),
                toggler(ctx.listenbrainz_enabled)
                    .on_toggle(Message::ToggleListenBrainz)
                    .size(TOGGLER_SIZE)
                    .style(styles::toggler_style(theme_cfg)),
            ]
            .align_y(Alignment::Center),
            column![
                text(font, "User token")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text_input("Enter ListenBrainz user token...", ctx.listenbrainz_token)
                    .size(12)
                    .padding(INPUT_PADDING)
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::ListenBrainzTokenChanged)
                    .width(Length::Fill),
                text(
                    font,
                    "Obtain your user token from your profile at https://listenbrainz.org/profile/"
                )
                .size(11)
                .color(theme_cfg.text_muted),
            ]
            .spacing(FORM_FIELD_SPACING),
            column![
                text(font, "API endpoint URL")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text_input(
                    "https://api.listenbrainz.org/1/submit-listens",
                    ctx.listenbrainz_api_url,
                )
                .size(12)
                .padding(INPUT_PADDING)
                .style(styles::search_input_style(theme_cfg))
                .on_input(Message::ListenBrainzUrlChanged)
                .width(Length::Fill),
            ]
            .spacing(FORM_FIELD_SPACING),
            lb_test_btn,
        ]
        .spacing(SECTION_SPACING);

        if let Some(banner) = lb_test_banner {
            lb_column = lb_column.push(banner.boxed());
        }

        let listenbrainz_card =
            component::settings_card(theme_cfg, font, "ListenBrainz plugin", lb_column.boxed());

        // 3. Last.fm Card
        let lastfm_test_banner = if let Some((plugin::scrobbler::ScrobbleService::LastFm, res)) =
            self.scrobble_test_status.as_ref()
        {
            let (color, msg) = match res {
                Ok(ref m) => (theme_cfg.success, m.as_str()),
                Err(ref e) => (theme_cfg.danger, e.as_str()),
            };
            Some(
                container(text(font, msg).size(12).color(color))
                    .padding(BANNER_PADDING)
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(
                            color.r, color.g, color.b, 0.1,
                        ))),
                        border: Border {
                            radius: theme_cfg.border_radius.sm,
                            color,
                            width: 1.0,
                        },
                        ..Default::default()
                    }),
            )
        } else {
            None
        };

        let lastfm_test_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 12.0, theme_cfg.primary),
                Space::new().width(SPACE_MD),
                text(font, "Test connection")
                    .size(11)
                    .color(theme_cfg.primary),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::TestScrobble(
            plugin::scrobbler::ScrobbleService::LastFm,
        ));

        let mut lastfm_column = column![
            row![
                text(font, "Enable")
                    .size(12)
                    .color(theme_cfg.text_muted)
                    .width(Length::Fill),
                toggler(ctx.lastfm_enabled)
                    .on_toggle(Message::ToggleLastFm)
                    .size(TOGGLER_SIZE)
                    .style(styles::toggler_style(theme_cfg)),
            ]
            .align_y(Alignment::Center),
            column![
                text(font, "API key").size(12).color(theme_cfg.text_muted),
                text_input("Enter Last.fm API key...", ctx.lastfm_api_key)
                    .size(12)
                    .padding(INPUT_PADDING)
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::LastFmApiKeyChanged)
                    .width(Length::Fill),
            ]
            .spacing(FORM_FIELD_SPACING),
            column![
                text(font, "Shared secret")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text_input("Enter Last.fm shared secret...", ctx.lastfm_api_secret)
                    .size(12)
                    .padding(INPUT_PADDING)
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::LastFmApiSecretChanged)
                    .width(Length::Fill),
            ]
            .spacing(FORM_FIELD_SPACING),
            column![
                text(font, "Session key (sk)")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text_input("Enter Last.fm session key (sk)...", ctx.lastfm_session_key)
                    .size(12)
                    .padding(INPUT_PADDING)
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::LastFmSessionKeyChanged)
                    .width(Length::Fill),
                text(
                    font,
                    "Obtain your API key and session key from your Last.fm API account settings"
                )
                .size(11)
                .color(theme_cfg.text_muted),
            ]
            .spacing(FORM_FIELD_SPACING),
            column![
                text(font, "API endpoint URL")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text_input("https://ws.audioscrobbler.com/2.0/", ctx.lastfm_api_url)
                    .size(12)
                    .padding(INPUT_PADDING)
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::LastFmUrlChanged)
                    .width(Length::Fill),
            ]
            .spacing(FORM_FIELD_SPACING),
            lastfm_test_btn,
        ]
        .spacing(SECTION_SPACING);

        if let Some(banner) = lastfm_test_banner {
            lastfm_column = lastfm_column.push(banner.boxed());
        }

        let lastfm_card =
            component::settings_card(theme_cfg, font, "Last.fm plugin", lastfm_column.boxed());

        column![
            header,
            prefs_card,
            listenbrainz_card,
            lastfm_card,
            Space::new().height(PAGE_BOTTOM_SPACER),
        ]
        .spacing(PAGE_SPACING)
        .padding(PAGE_PADDING)
        .width(Length::Fill)
        .boxed()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Debug Section
// ─────────────────────────────────────────────────────────────────────────────
