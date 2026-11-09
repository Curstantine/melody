use iced::Color;

pub fn parse_hex(hex: &str) -> Color {
    hex.trim().parse().unwrap_or(Color::BLACK)
}

pub mod hex_color {
    use super::*;
    use serde::{de::Error as _, Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Color>()
            .map_err(|_| D::Error::custom(format!("invalid hex color: {s}")))
    }

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let [r, g, b, a] = color.into_rgba8();
        let hex = if a == 255 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
        };
        serializer.serialize_str(&hex)
    }
}

pub mod opt_hex_color {
    use super::*;
    use serde::{de::Error as _, Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            Some(s) => s
                .parse::<Color>()
                .map(Some)
                .map_err(|_| D::Error::custom(format!("invalid hex color: {s}"))),
            None => Ok(None),
        }
    }

    pub fn serialize<S>(color: &Option<Color>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match color {
            Some(c) => super::hex_color::serialize(c, serializer),
            None => serializer.serialize_none(),
        }
    }
}
