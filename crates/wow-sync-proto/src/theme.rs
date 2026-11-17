//! Colour tokens from docs/design/ui/style.md. No screen hard-codes a colour.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Light,
    Dark,
}

impl ThemeChoice {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "system" => Some(Self::System),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "Follow system",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    pub fn resolve(self, system: Mode) -> Mode {
        match self {
            Self::System => system,
            Self::Light => Mode::Light,
            Self::Dark => Mode::Dark,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Ok,
    Working,
    Waiting,
    Help,
    Error,
    Unknown,
    Paused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub bg: u32,
    pub surface: u32,
    pub sidebar: u32,
    pub selected: u32,
    pub hover: u32,
    pub divider: u32,
    pub control_border: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub on_accent: u32,
    pub focus: u32,
    pub disabled: u32,
    fg: [u32; 7],
    fill: [u32; 7],
}

fn idx(t: Tone) -> usize {
    match t {
        Tone::Ok => 0,
        Tone::Working => 1,
        Tone::Waiting => 2,
        Tone::Help => 3,
        Tone::Error => 4,
        Tone::Unknown => 5,
        Tone::Paused => 6,
    }
}

impl Palette {
    pub fn fg(&self, t: Tone) -> u32 {
        self.fg[idx(t)]
    }

    pub fn fill(&self, t: Tone) -> u32 {
        self.fill[idx(t)]
    }

    pub fn of(mode: Mode) -> Self {
        match mode {
            Mode::Light => Self {
                bg: 0xF5F6F8,
                surface: 0xFFFFFF,
                sidebar: 0xEBEEF2,
                selected: 0xDCE6F7,
                hover: 0xF0F3F7,
                divider: 0xD5DAE1,
                control_border: 0x7D8799,
                text: 0x1A1F29,
                muted: 0x4A5363,
                accent: 0x1D5BBF,
                on_accent: 0xFFFFFF,
                focus: 0x1D5BBF,
                disabled: 0x8A94A6,
                fg: [
                    0x17703A, 0x1D5BBF, 0x8A5300, 0xA64205, 0xB42318, 0x4A5363, 0x6941C6,
                ],
                fill: [
                    0xE3F3E8, 0xE3ECFA, 0xFBF0D9, 0xFDEBDD, 0xFCE4E2, 0xECEEF1, 0xEEE8FB,
                ],
            },
            Mode::Dark => Self {
                bg: 0x14171C,
                surface: 0x1D2128,
                sidebar: 0x191C22,
                selected: 0x26344D,
                hover: 0x242932,
                divider: 0x2C313A,
                control_border: 0x6B7585,
                text: 0xE8EAEE,
                muted: 0xA7AFBB,
                accent: 0x8AB8FF,
                on_accent: 0x0E1626,
                focus: 0xFFD166,
                disabled: 0x6B7585,
                fg: [
                    0x5CCB7F, 0x8AB8FF, 0xF2C14E, 0xFFA463, 0xFF8A80, 0xA7AFBB, 0xC4A8FF,
                ],
                fill: [
                    0x1C3326, 0x1E2D47, 0x3A2F14, 0x3F2716, 0x432020, 0x2A2F37, 0x2E2645,
                ],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_choice_overrides_system() {
        assert_eq!(ThemeChoice::System.resolve(Mode::Dark), Mode::Dark);
        assert_eq!(ThemeChoice::Light.resolve(Mode::Dark), Mode::Light);
        assert_eq!(ThemeChoice::Dark.resolve(Mode::Light), Mode::Dark);
        assert_eq!(ThemeChoice::parse("System"), Some(ThemeChoice::System));
        assert_eq!(ThemeChoice::parse("x"), None);
    }

    #[test]
    fn palettes_differ_and_tones_are_distinct() {
        let (l, d) = (Palette::of(Mode::Light), Palette::of(Mode::Dark));
        assert_ne!(l.bg, d.bg);
        assert_ne!(l.focus, d.focus);
        let tones = [
            Tone::Ok,
            Tone::Working,
            Tone::Waiting,
            Tone::Help,
            Tone::Error,
            Tone::Paused,
        ];
        for (i, a) in tones.iter().enumerate() {
            for b in &tones[i + 1..] {
                assert_ne!(l.fg(*a), l.fg(*b), "{a:?} vs {b:?}");
            }
        }
    }
}
