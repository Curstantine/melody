use iced::border::Radius;
use iced::widget::{button, container, slider, text_input};
use iced::{Background, Border, Color, Shadow, Theme};

use super::ThemeConfig;

pub fn main_background(cfg: &ThemeConfig) -> impl Fn(&Theme) -> container::Style + '_ {
    move |_theme| container::Style {
        background: Some(Background::Color(cfg.bg_color())),
        text_color: Some(cfg.text_color()),
        border: Border::default(),
        shadow: Shadow::default(),
    }
}

#[allow(dead_code)]
pub fn surface_container(cfg: &ThemeConfig) -> impl Fn(&Theme) -> container::Style + '_ {
    move |_theme| container::Style {
        background: Some(Background::Color(cfg.surface_color())),
        text_color: Some(cfg.text_color()),
        border: Border {
            color: cfg.border_color(),
            width: 1.0,
            radius: Radius::from(8.0),
        },
        shadow: Shadow::default(),
    }
}

pub fn sidebar_container(cfg: &ThemeConfig) -> impl Fn(&Theme) -> container::Style + '_ {
    move |_theme| container::Style {
        background: Some(Background::Color(cfg.surface_color())),
        text_color: Some(cfg.text_color()),
        border: Border {
            color: cfg.border_color(),
            width: 1.0,
            radius: Radius::default(),
        },
        shadow: Shadow::default(),
    }
}

pub fn player_bar_container(cfg: &ThemeConfig) -> impl Fn(&Theme) -> container::Style + '_ {
    move |_theme| container::Style {
        background: Some(Background::Color(cfg.surface_color())),
        text_color: Some(cfg.text_color()),
        border: Border {
            color: cfg.border_color(),
            width: 1.0,
            radius: Radius::default(),
        },
        shadow: Shadow::default(),
    }
}

pub fn table_row_container(
    cfg: &ThemeConfig,
    is_selected: bool,
) -> impl Fn(&Theme) -> container::Style + '_ {
    move |_theme| {
        let bg = if is_selected {
            cfg.surface_hover_color()
        } else {
            Color::TRANSPARENT
        };
        container::Style {
            background: Some(Background::Color(bg)),
            text_color: Some(cfg.text_color()),
            border: Border {
                color: if is_selected {
                    cfg.primary_color()
                } else {
                    Color::TRANSPARENT
                },
                width: if is_selected { 1.0 } else { 0.0 },
                radius: Radius::from(6.0),
            },
            shadow: Shadow::default(),
        }
    }
}

pub fn primary_button(cfg: &ThemeConfig) -> impl Fn(&Theme, button::Status) -> button::Style + '_ {
    move |_theme, status| {
        let (bg, text) = match status {
            button::Status::Active => (cfg.primary_color(), cfg.surface_color()),
            button::Status::Hovered => (
                blend_color(cfg.primary_color(), Color::WHITE, 0.15),
                cfg.surface_color(),
            ),
            button::Status::Pressed => (
                blend_color(cfg.primary_color(), Color::BLACK, 0.15),
                cfg.surface_color(),
            ),
            button::Status::Disabled => (cfg.surface_hover_color(), cfg.text_muted_color()),
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: Border {
                radius: Radius::from(6.0),
                ..Border::default()
            },
            shadow: Shadow::default(),
        }
    }
}

pub fn ghost_button(
    cfg: &ThemeConfig,
    is_active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style + '_ {
    move |_theme, status| {
        let (bg, text) = match status {
            button::Status::Active => {
                if is_active {
                    (cfg.surface_hover_color(), cfg.primary_color())
                } else {
                    (Color::TRANSPARENT, cfg.text_color())
                }
            }
            button::Status::Hovered => (cfg.surface_hover_color(), cfg.primary_color()),
            button::Status::Pressed => (cfg.surface_hover_color(), cfg.text_color()),
            button::Status::Disabled => (Color::TRANSPARENT, cfg.text_muted_color()),
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: Border {
                radius: Radius::from(6.0),
                ..Border::default()
            },
            shadow: Shadow::default(),
        }
    }
}

pub fn icon_control_button(
    cfg: &ThemeConfig,
) -> impl Fn(&Theme, button::Status) -> button::Style + '_ {
    move |_theme, status| {
        let (bg, text) = match status {
            button::Status::Active => (Color::TRANSPARENT, cfg.text_color()),
            button::Status::Hovered => (cfg.surface_hover_color(), cfg.primary_color()),
            button::Status::Pressed => (cfg.surface_hover_color(), cfg.text_muted_color()),
            button::Status::Disabled => (Color::TRANSPARENT, cfg.text_muted_color()),
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: Border {
                radius: Radius::from(20.0),
                ..Border::default()
            },
            shadow: Shadow::default(),
        }
    }
}

pub fn search_input_style(
    cfg: &ThemeConfig,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style + '_ {
    move |_theme, status| {
        let border_color = match status {
            text_input::Status::Focused { .. } => cfg.primary_color(),
            text_input::Status::Hovered => cfg.accent_color(),
            _ => cfg.border_color(),
        };

        text_input::Style {
            background: Background::Color(cfg.surface_color()),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: Radius::from(6.0),
            },
            icon: cfg.text_muted_color(),
            placeholder: cfg.text_muted_color(),
            value: cfg.text_color(),
            selection: cfg.accent_color(),
        }
    }
}

pub fn seek_slider_style(
    cfg: &ThemeConfig,
) -> impl Fn(&Theme, slider::Status) -> slider::Style + '_ {
    move |_theme, _status| slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(cfg.primary_color()),
                Background::Color(cfg.surface_hover_color()),
            ),
            width: 4.0,
            border: Border {
                radius: Radius::from(2.0),
                ..Border::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 7.0 },
            background: Background::Color(cfg.primary_color()),
            border_width: 1.5,
            border_color: cfg.surface_color(),
        },
    }
}

pub fn volume_slider_style(
    cfg: &ThemeConfig,
) -> impl Fn(&Theme, slider::Status) -> slider::Style + '_ {
    move |_theme, _status| slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(cfg.accent_color()),
                Background::Color(cfg.surface_hover_color()),
            ),
            width: 4.0,
            border: Border {
                radius: Radius::from(2.0),
                ..Border::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 6.0 },
            background: Background::Color(cfg.accent_color()),
            border_width: 1.0,
            border_color: cfg.surface_color(),
        },
    }
}

fn blend_color(c1: Color, c2: Color, factor: f32) -> Color {
    let factor = factor.clamp(0.0, 1.0);
    Color {
        r: c1.r * (1.0 - factor) + c2.r * factor,
        g: c1.g * (1.0 - factor) + c2.g * factor,
        b: c1.b * (1.0 - factor) + c2.b * factor,
        a: c1.a,
    }
}
