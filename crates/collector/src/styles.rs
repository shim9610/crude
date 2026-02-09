// crates/collector/src/styles.rs
//! Style definitions for Handler Cards and Lists

use iced::widget::container;
use iced::{Background, Border, Color, Shadow, Theme, Vector};
/// Generate Card Style
pub fn card_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    
    container::Style {
        background: Some(Background::Color(Color {
            r: palette.background.r * 1.1,
            g: palette.background.g * 1.1,
            b: palette.background.b * 1.1,
            a: 1.0,
        })),
        border: Border {
            color: Color::from_rgba(0.3, 0.3, 0.35, 0.5),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        text_color: Some(palette.text),
        snap: true,
    }
}
/// Card Style on Hover
pub fn card_hover_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    
    container::Style {
        background: Some(Background::Color(Color {
            r: palette.background.r * 1.2,
            g: palette.background.g * 1.2,
            b: palette.background.b * 1.2,
            a: 1.0,
        })),
        border: Border {
            color: palette.primary,
            width: 2.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        text_color: Some(palette.text),
        snap: true,
    }
}
/// Generic Badge Style with Custom Color
pub fn badge_style(r: f32, g: f32, b: f32) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(Color::from_rgba(r, g, b, 0.2))),
        border: Border {
            color: Color::from_rgb(r, g, b),
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(Color::from_rgb(r, g, b)),
        snap: true,
    }
}
/// List Container Style
pub fn list_container_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    
    container::Style {
        background: Some(Background::Color(Color {
            r: palette.background.r * 0.95,
            g: palette.background.g * 0.95,
            b: palette.background.b * 0.95,
            a: 1.0,
        })),
        border: Border {
            color: Color::from_rgba(0.2, 0.2, 0.25, 0.3),
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(palette.text),
        snap: true,
    }
}

/// Header Style
pub fn header_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    
    container::Style {
        background: Some(Background::Color(Color {
            r: palette.primary.r * 0.3,
            g: palette.primary.g * 0.3,
            b: palette.primary.b * 0.3,
            a: 1.0,
        })),
        border: Border {
            color: palette.primary,
            width: 0.0,
            radius: 12.0.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(palette.text),
        snap: true,
    }
}

/// Code Block Style
pub fn code_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.3))),
        border: Border {
            color: Color::from_rgba(0.4, 0.4, 0.4, 0.3),
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(Color {
            r: palette.text.r * 0.9,
            g: palette.text.g * 0.95,
            b: palette.text.b,
            a: 1.0,
        }),
        snap: true,
    }
}

/// Circular Index Badge Style
pub fn index_badge_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    
    container::Style {
        background: Some(Background::Color(palette.primary)),
        border: Border {
            color: palette.primary,
            width: 0.0,
            radius: 50.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 3.0,
        },
        text_color: Some(Color::WHITE),
        snap: true,
    }
}

/// Detail Row Style
pub fn detail_row_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.1))),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        text_color: None,
        snap: true,
    }
}