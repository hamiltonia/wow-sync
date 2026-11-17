use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

pub struct Assets;

const ICONS: &[(&str, &str)] = &[
    ("icons/check.svg", include_str!("../assets/icons/check.svg")),
    ("icons/sync.svg", include_str!("../assets/icons/sync.svg")),
    (
        "icons/hourglass.svg",
        include_str!("../assets/icons/hourglass.svg"),
    ),
    ("icons/alert.svg", include_str!("../assets/icons/alert.svg")),
    ("icons/error.svg", include_str!("../assets/icons/error.svg")),
    ("icons/clock.svg", include_str!("../assets/icons/clock.svg")),
    (
        "icons/dashed.svg",
        include_str!("../assets/icons/dashed.svg"),
    ),
    (
        "icons/slashed.svg",
        include_str!("../assets/icons/slashed.svg"),
    ),
    ("icons/pause.svg", include_str!("../assets/icons/pause.svg")),
    (
        "icons/chevron.svg",
        include_str!("../assets/icons/chevron.svg"),
    ),
];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, s)| Cow::Borrowed(s.as_bytes())))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| SharedString::from(*p))
            .collect())
    }
}
