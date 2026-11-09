use iced::border::Radius;
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderRadiusConfig {
    pub xs: Radius,
    pub sm: Radius,
    pub md: Radius,
    pub lg: Radius,
    pub xl: Radius,
}

const DEFAULT_RADIUS_XS: f32 = 2.0;
const DEFAULT_RADIUS_SM: f32 = 4.0;
const DEFAULT_RADIUS_MD: f32 = 6.0;
const DEFAULT_RADIUS_LG: f32 = 8.0;
const DEFAULT_RADIUS_XL: f32 = 16.0;

impl Default for BorderRadiusConfig {
    fn default() -> Self {
        Self {
            xs: Radius::from(DEFAULT_RADIUS_XS),
            sm: Radius::from(DEFAULT_RADIUS_SM),
            md: Radius::from(DEFAULT_RADIUS_MD),
            lg: Radius::from(DEFAULT_RADIUS_LG),
            xl: Radius::from(DEFAULT_RADIUS_XL),
        }
    }
}

impl Serialize for BorderRadiusConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("BorderRadiusConfig", 5)?;
        s.serialize_field("xs", &self.xs.top_left)?;
        s.serialize_field("sm", &self.sm.top_left)?;
        s.serialize_field("md", &self.md.top_left)?;
        s.serialize_field("lg", &self.lg.top_left)?;
        s.serialize_field("xl", &self.xl.top_left)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for BorderRadiusConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BorderRadiusVisitor;

        impl<'de> Visitor<'de> for BorderRadiusVisitor {
            type Value = BorderRadiusConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number or a border radius map with xs, sm, md, lg, xl")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_f64(v as f64)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_f64(v as f64)
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let r = v as f32;
                if r <= 0.0 {
                    Ok(BorderRadiusConfig {
                        xs: Radius::from(0.0),
                        sm: Radius::from(0.0),
                        md: Radius::from(0.0),
                        lg: Radius::from(0.0),
                        xl: Radius::from(0.0),
                    })
                } else {
                    Ok(BorderRadiusConfig {
                        xs: Radius::from((r * 0.5).round()),
                        sm: Radius::from(r),
                        md: Radius::from((r * 1.5).round()),
                        lg: Radius::from((r * 2.0).round()),
                        xl: Radius::from((r * 4.0).round()),
                    })
                }
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut xs = None;
                let mut sm = None;
                let mut md = None;
                let mut lg = None;
                let mut xl = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "xs" => xs = Some(map.next_value::<f32>()?),
                        "sm" => sm = Some(map.next_value::<f32>()?),
                        "md" => md = Some(map.next_value::<f32>()?),
                        "lg" => lg = Some(map.next_value::<f32>()?),
                        "xl" => xl = Some(map.next_value::<f32>()?),
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(BorderRadiusConfig {
                    xs: Radius::from(xs.unwrap_or(DEFAULT_RADIUS_XS)),
                    sm: Radius::from(sm.unwrap_or(DEFAULT_RADIUS_SM)),
                    md: Radius::from(md.unwrap_or(DEFAULT_RADIUS_MD)),
                    lg: Radius::from(lg.unwrap_or(DEFAULT_RADIUS_LG)),
                    xl: Radius::from(xl.unwrap_or(DEFAULT_RADIUS_XL)),
                })
            }
        }

        deserializer.deserialize_any(BorderRadiusVisitor)
    }
}
