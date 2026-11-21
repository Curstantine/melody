use iced::widget::{column, container, pick_list, row, slider, text_input, toggler, Space, Widget};
use iced::{Alignment, Border, Color, Element, Length};

use crate::message::Message;
use component::{self, button, icons, styles, text};
use plugin::{
    ExtensionOrigin, ExtensionPermission, ExtensionSnapshot, ExtensionStatus, SelectChoice,
    SettingKind, SettingOption, SettingValue,
};
use theme::ThemeConfig;

use super::*;

impl SettingsView {
    pub(super) fn view_extensions<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = component::settings_header(
            theme_cfg,
            font,
            "Extensions",
            Some("Install, manage, and configure sandboxed WebAssembly extensions"),
        );

        let install_archive_btn = action_button(
            theme_cfg,
            font,
            icons::DOWNLOAD_SVG,
            "Install archive (.zip / .mext)",
            theme_cfg.primary,
            Message::InstallExtensionArchiveClicked,
        );
        let install_folder_btn = action_button(
            theme_cfg,
            font,
            icons::FOLDER_SVG,
            "Install from folder",
            theme_cfg.text,
            Message::InstallExtensionFolderClicked,
        );
        let reload_btn = action_button(
            theme_cfg,
            font,
            icons::REFRESH_SVG,
            "Reload from disk",
            theme_cfg.text,
            Message::ReloadExtensions,
        );
        let open_folder_btn = action_button(
            theme_cfg,
            font,
            icons::OPEN_SVG,
            "Open extensions folder",
            theme_cfg.text,
            Message::OpenExtensionsFolder,
        );

        let mut mgmt_col = column![row![
            install_archive_btn,
            install_folder_btn,
            reload_btn,
            open_folder_btn,
        ]
        .spacing(SPACE_MD)
        .wrap()]
        .spacing(SECTION_SPACING);

        if let Some((ok, msg)) = self.extension_banner.as_ref() {
            let color = if *ok {
                theme_cfg.success
            } else {
                theme_cfg.danger
            };
            mgmt_col = mgmt_col.push(status_banner(theme_cfg, font, color, msg.as_str()));
        }

        let management_card =
            component::settings_card(theme_cfg, font, "Extension management", mgmt_col.boxed());

        let mut page = column![header, management_card]
            .spacing(PAGE_SPACING)
            .padding(PAGE_PADDING)
            .width(Length::Fill);

        if ctx.extensions.is_empty() {
            let empty_box = container(
                column![
                    text(font, "No extensions discovered.")
                        .size(13)
                        .color(theme_cfg.text_muted),
                    text(
                        font,
                        "Click 'Install archive (.zip / .mext)' or 'Install from folder' to add a WebAssembly extension.",
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
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(theme_cfg.surface)),
                border: Border {
                    radius: theme_cfg.border_radius.sm,
                    color: theme_cfg.border,
                    width: 1.0,
                },
                ..Default::default()
            });

            page = page.push(component::settings_card(
                theme_cfg,
                font,
                "Installed extensions",
                empty_box.boxed(),
            ));
        } else {
            for ext in ctx.extensions {
                page = page.push(self.view_extension_card(theme_cfg, font, ext, ctx));
            }
        }

        page = page.push(Space::new().height(PAGE_BOTTOM_SPACER).boxed());

        page.boxed()
    }

    fn view_extension_card<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        ext: &'a ExtensionSnapshot,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let ext_id = &ext.manifest.id;
        let origin_label = match ext.origin {
            ExtensionOrigin::Bundled => "Bundled",
            ExtensionOrigin::User => "Installed",
        };
        let (status_label, status_color) = match &ext.status {
            ExtensionStatus::Running => ("Active", theme_cfg.success),
            ExtensionStatus::Disabled => ("Disabled", theme_cfg.text_muted),
            ExtensionStatus::Error(_) => ("Error", theme_cfg.danger),
        };

        let mut meta_parts = format!("v{}", ext.manifest.version);
        if !ext.manifest.authors.is_empty() {
            meta_parts.push_str(" • ");
            meta_parts.push_str(&ext.manifest.authors.join(", "));
        }
        meta_parts.push_str(" • ");
        meta_parts.push_str(ext_id);

        let left_meta = row![
            text(font, meta_parts).size(11).color(theme_cfg.text_muted),
            pill_badge(theme_cfg, font, origin_label, theme_cfg.text_muted),
            pill_badge(theme_cfg, font, status_label, status_color),
        ]
        .spacing(SPACE_MD)
        .align_y(Alignment::Center)
        .wrap();

        let toggle_id = ext_id.clone();
        let mut right_controls = row![].spacing(SPACE_MD).align_y(Alignment::Center);
        if ext.origin == ExtensionOrigin::User {
            let uninstall_id = ext_id.clone();
            right_controls = right_controls.push(
                button(icons::icon(icons::TRASH_SVG, 12.0, theme_cfg.danger))
                    .padding(BUTTON_PADDING_SM)
                    .style(styles::ghost_button(theme_cfg, false))
                    .on_press(Message::UninstallExtension(uninstall_id))
                    .boxed(),
            );
        }
        right_controls = right_controls.push(
            toggler(ext.enabled)
                .on_toggle(move |v| Message::ToggleExtension(toggle_id.clone(), v))
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg))
                .boxed(),
        );

        let header_row = row![container(left_meta).width(Length::Fill), right_controls,]
            .spacing(SPACE_LG)
            .align_y(Alignment::Center)
            .boxed();

        let mut card_col = column![header_row].spacing(SECTION_SPACING);

        if !ext.manifest.description.trim().is_empty() {
            card_col = card_col.push(
                text(font, ext.manifest.description.as_str())
                    .size(12)
                    .color(theme_cfg.text_muted)
                    .boxed(),
            );
        }

        let permissions_row = view_permissions_row(theme_cfg, font, ext);
        card_col = card_col.push(permissions_row);

        if let ExtensionStatus::Error(ref err) = ext.status {
            card_col = card_col.push(status_banner(
                theme_cfg,
                font,
                theme_cfg.danger,
                err.as_str(),
            ));
        }

        if ext.manifest.has_permission(ExtensionPermission::Settings)
            && !ext.settings_schema.is_empty()
        {
            let saved_settings = ctx
                .extensions_config
                .get(ext_id)
                .map(|entry| &entry.settings);

            for option in &ext.settings_schema {
                let saved_value = saved_settings.and_then(|map| map.get(&option.key));
                card_col = card_col.push(self.view_extension_setting(
                    theme_cfg,
                    font,
                    ext,
                    option,
                    saved_value,
                ));
            }
        }

        if let Some(res) = self.extension_action_status.get(ext_id) {
            let (color, msg) = match res {
                Ok(m) => (theme_cfg.success, m.as_str()),
                Err(e) => (theme_cfg.danger, e.as_str()),
            };
            card_col = card_col.push(status_banner(theme_cfg, font, color, msg));
        }

        component::settings_card(
            theme_cfg,
            font,
            ext.manifest.name.as_str(),
            card_col.boxed(),
        )
    }

    fn view_extension_setting<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        ext: &'a ExtensionSnapshot,
        option: &'a SettingOption,
        saved_value: Option<&'a SettingValue>,
    ) -> Element<'a, Message> {
        let ext_id = &ext.manifest.id;
        let key = &option.key;
        let desc = option.description.as_deref().unwrap_or("");

        match &option.kind {
            SettingKind::Toggle { default } => {
                let current = saved_value
                    .and_then(SettingValue::as_bool)
                    .unwrap_or(*default);
                let ext_id_owned = ext_id.clone();
                let key_owned = key.clone();
                component::settings_row(
                    theme_cfg,
                    font,
                    option.label.as_str(),
                    desc,
                    toggler(current)
                        .on_toggle(move |val| Message::ExtensionSettingChanged {
                            extension_id: ext_id_owned.clone(),
                            key: key_owned.clone(),
                            value: SettingValue::Bool(val),
                            save_immediately: true,
                        })
                        .size(TOGGLER_SIZE)
                        .style(styles::toggler_style(theme_cfg)),
                )
            }

            SettingKind::Text {
                default,
                placeholder,
            } => {
                let current = saved_value
                    .and_then(SettingValue::as_str)
                    .unwrap_or(default.as_str());
                let ph = placeholder.as_deref().unwrap_or("");
                let ext_id_owned = ext_id.clone();
                let key_owned = key.clone();

                let mut field = column![
                    text(font, option.label.as_str())
                        .size(12)
                        .color(theme_cfg.text_muted),
                    text_input(ph, current)
                        .size(12)
                        .padding(INPUT_PADDING)
                        .style(styles::search_input_style(theme_cfg))
                        .on_input(move |val| Message::ExtensionSettingChanged {
                            extension_id: ext_id_owned.clone(),
                            key: key_owned.clone(),
                            value: SettingValue::String(val),
                            save_immediately: false,
                        })
                        .width(Length::Fill),
                ]
                .spacing(FORM_FIELD_SPACING);

                if !desc.is_empty() {
                    field = field.push(
                        text(font, desc)
                            .size(11)
                            .color(theme_cfg.text_muted)
                            .boxed(),
                    );
                }
                field.boxed()
            }

            SettingKind::Secret {
                default,
                placeholder,
            } => {
                let current = saved_value
                    .and_then(SettingValue::as_str)
                    .unwrap_or(default.as_str());
                let ph = placeholder.as_deref().unwrap_or("");
                let ext_id_owned = ext_id.clone();
                let key_owned = key.clone();

                let mut field = column![
                    text(font, option.label.as_str())
                        .size(12)
                        .color(theme_cfg.text_muted),
                    text_input(ph, current)
                        .secure(true)
                        .size(12)
                        .padding(INPUT_PADDING)
                        .style(styles::search_input_style(theme_cfg))
                        .on_input(move |val| Message::ExtensionSettingChanged {
                            extension_id: ext_id_owned.clone(),
                            key: key_owned.clone(),
                            value: SettingValue::String(val),
                            save_immediately: false,
                        })
                        .width(Length::Fill),
                ]
                .spacing(FORM_FIELD_SPACING);

                if !desc.is_empty() {
                    field = field.push(
                        text(font, desc)
                            .size(11)
                            .color(theme_cfg.text_muted)
                            .boxed(),
                    );
                }
                field.boxed()
            }

            SettingKind::Number {
                default,
                min,
                max,
                step,
                unit,
            } => {
                let current = saved_value
                    .and_then(SettingValue::as_f64)
                    .unwrap_or(*default)
                    .clamp(*min, *max);
                let step_val = *step;
                let min_val = *min;
                let max_val = *max;
                let unit_str = unit.as_deref().unwrap_or("");
                let display_val = if step_val.fract() == 0.0 {
                    format!("{:.0}{unit_str}", current.round())
                } else {
                    format!("{current:.2}{unit_str}")
                };

                let ext_id_owned = ext_id.clone();
                let key_owned = key.clone();

                component::settings_row(
                    theme_cfg,
                    font,
                    option.label.as_str(),
                    desc,
                    row![
                        text(font, display_val)
                            .size(12)
                            .color(theme_cfg.primary)
                            .width(Length::Fixed(52.0)),
                        Space::new().width(SPACE_SM),
                        slider(min_val..=max_val, current, move |val| {
                            let snapped = if step_val > 0.0 {
                                (min_val + ((val - min_val) / step_val).round() * step_val)
                                    .clamp(min_val, max_val)
                            } else {
                                val
                            };
                            let sv = if step_val.fract() == 0.0 {
                                SettingValue::Integer(snapped.round() as i64)
                            } else {
                                SettingValue::Float(snapped)
                            };
                            Message::ExtensionSettingChanged {
                                extension_id: ext_id_owned.clone(),
                                key: key_owned.clone(),
                                value: sv,
                                save_immediately: false,
                            }
                        })
                        .step(step_val)
                        .on_release(Message::SaveConfig)
                        .style(styles::settings_slider_style(theme_cfg))
                        .width(Length::Fixed(SETTING_SLIDER_WIDTH)),
                    ]
                    .align_y(Alignment::Center),
                )
            }

            SettingKind::Select { default, options } => {
                let current_val = saved_value
                    .and_then(SettingValue::as_str)
                    .unwrap_or(default.as_str());
                let selected = options
                    .iter()
                    .find(|c| c.value == current_val)
                    .cloned()
                    .or_else(|| options.first().cloned());

                let ext_id_owned = ext_id.clone();
                let key_owned = key.clone();

                let picker = pick_list(selected, options.clone(), |choice: &SelectChoice| {
                    choice.label.clone()
                })
                .on_select(move |choice| Message::ExtensionSettingChanged {
                    extension_id: ext_id_owned.clone(),
                    key: key_owned.clone(),
                    value: SettingValue::String(choice.value),
                    save_immediately: true,
                })
                .font(font)
                .padding(INPUT_PADDING)
                .text_size(12)
                .handle(pick_list::Handle::Arrow {
                    size: Some(11.0.into()),
                })
                .width(Length::Fixed(SETTING_DROPDOWN_WIDTH))
                .style(styles::pick_list_style(theme_cfg))
                .menu_style(styles::menu_style(theme_cfg));

                component::settings_row(theme_cfg, font, option.label.as_str(), desc, picker)
            }

            SettingKind::Action { button_label } => {
                let is_busy = self
                    .extension_action_busy
                    .contains(&(ext_id.clone(), key.clone()));
                let btn_text = if is_busy {
                    "Running..."
                } else {
                    button_label.as_str()
                };
                let ext_id_owned = ext_id.clone();
                let key_owned = key.clone();

                let action_btn = button(
                    row![
                        icons::icon(icons::REFRESH_SVG, 12.0, theme_cfg.primary),
                        Space::new().width(SPACE_MD),
                        text(font, btn_text).size(11).color(theme_cfg.primary),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding(BUTTON_PADDING_MD)
                .style(styles::ghost_button(theme_cfg, false))
                .on_press_maybe(if ext.enabled && !is_busy {
                    Some(Message::TriggerExtensionAction {
                        extension_id: ext_id_owned,
                        action_id: key_owned,
                    })
                } else {
                    None
                });

                component::settings_row(theme_cfg, font, option.label.as_str(), desc, action_btn)
            }
        }
    }
}

fn view_permissions_row<'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    ext: &'a ExtensionSnapshot,
) -> Element<'a, Message> {
    let mut badges = row![].spacing(CHIP_ROW_SPACING);
    if ext.manifest.permissions.is_empty() {
        badges = badges.push(pill_badge(
            theme_cfg,
            font,
            "No special permissions",
            theme_cfg.text_muted,
        ));
    } else {
        for perm in &ext.manifest.permissions {
            let label = if *perm == ExtensionPermission::Network
                && !ext.manifest.allowed_hosts.is_empty()
            {
                format!(
                    "{} ({})",
                    perm.label(),
                    ext.manifest.allowed_hosts.join(", ")
                )
            } else {
                perm.label().to_string()
            };
            badges = badges.push(pill_badge(theme_cfg, font, label, theme_cfg.text_muted));
        }
    }

    row![
        text(font, "Permissions")
            .size(11)
            .color(theme_cfg.text_muted),
        badges.wrap(),
    ]
    .spacing(SPACE_LG)
    .align_y(Alignment::Center)
    .boxed()
}

fn action_button<'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    icon_svg: &'static str,
    label: &'static str,
    color: Color,
    on_press: Message,
) -> Element<'a, Message> {
    button(
        row![
            icons::icon(icon_svg, 13.0, color),
            Space::new().width(SPACE_MD),
            text(font, label).size(12).color(color),
        ]
        .align_y(Alignment::Center),
    )
    .padding(BUTTON_PADDING_MD)
    .style(styles::ghost_button(theme_cfg, false))
    .on_press(on_press)
    .boxed()
}

fn pill_badge<'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    label: impl Into<String>,
    color: Color,
) -> Element<'a, Message> {
    container(text(font, label.into()).size(10).color(color))
        .padding([2, 6])
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(theme_cfg.surface)),
            border: Border {
                radius: theme_cfg.border_radius.sm,
                color: theme_cfg.border,
                width: 1.0,
            },
            ..Default::default()
        })
        .boxed()
}

fn status_banner<'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    color: Color,
    msg: &'a str,
) -> Element<'a, Message> {
    container(text(font, msg).size(12).color(color))
        .padding(BANNER_PADDING)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                color.r, color.g, color.b, 0.1,
            ))),
            border: Border {
                radius: theme_cfg.border_radius.sm,
                color,
                width: 1.0,
            },
            ..Default::default()
        })
        .boxed()
}
