//! 組み込みの配色。設定の `color.theme` で選ぶ。

use serde::Deserialize;

/// 組み込みの配色の名前。設定では `tokyo-night` のように、小文字とハイフンで書く。
///
/// @doc docs/specs/configuration.md#組み込みの配色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorTheme {
    #[default]
    Default,
    Dracula,
    Alucard,
    Monokai,
    Kanagawa,
    KanagawaLotus,
    TokyoNight,
    TokyoNightStorm,
    CatppuccinMocha,
    CatppuccinLatte,
    Nord,
    GruvboxDark,
    GruvboxLight,
    OneDark,
    SolarizedDark,
    SolarizedLight,
    RosePine,
    RosePineDawn,
    Everforest,
    GithubDark,
    GithubLight,
    XcodeDark,
    XcodeLight,
    XcodeDusk,
    XcodeMidnight,
    XcodeCivic,
}

/// 配色1つ分の色。値はすべて `#rrggbb`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub background: &'static str,
    pub surface: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub border: &'static str,
    pub accent: &'static str,
    pub accent_soft: &'static str,
    pub link: &'static str,
    pub code: &'static str,
    pub note: &'static str,
    pub tip: &'static str,
    pub warning: &'static str,
    pub caution: &'static str,
}

impl ColorTheme {
    /// すべての配色。配色を足したら、ここにも足す。
    pub const ALL: [Self; 26] = [
        Self::Default,
        Self::Dracula,
        Self::Alucard,
        Self::Monokai,
        Self::Kanagawa,
        Self::KanagawaLotus,
        Self::TokyoNight,
        Self::TokyoNightStorm,
        Self::CatppuccinMocha,
        Self::CatppuccinLatte,
        Self::Nord,
        Self::GruvboxDark,
        Self::GruvboxLight,
        Self::OneDark,
        Self::SolarizedDark,
        Self::SolarizedLight,
        Self::RosePine,
        Self::RosePineDawn,
        Self::Everforest,
        Self::GithubDark,
        Self::GithubLight,
        Self::XcodeDark,
        Self::XcodeLight,
        Self::XcodeDusk,
        Self::XcodeMidnight,
        Self::XcodeCivic,
    ];

    /// 設定に書く名前。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Dracula => "dracula",
            Self::Alucard => "alucard",
            Self::Monokai => "monokai",
            Self::Kanagawa => "kanagawa",
            Self::KanagawaLotus => "kanagawa-lotus",
            Self::TokyoNight => "tokyo-night",
            Self::TokyoNightStorm => "tokyo-night-storm",
            Self::CatppuccinMocha => "catppuccin-mocha",
            Self::CatppuccinLatte => "catppuccin-latte",
            Self::Nord => "nord",
            Self::GruvboxDark => "gruvbox-dark",
            Self::GruvboxLight => "gruvbox-light",
            Self::OneDark => "one-dark",
            Self::SolarizedDark => "solarized-dark",
            Self::SolarizedLight => "solarized-light",
            Self::RosePine => "rose-pine",
            Self::RosePineDawn => "rose-pine-dawn",
            Self::Everforest => "everforest",
            Self::GithubDark => "github-dark",
            Self::GithubLight => "github-light",
            Self::XcodeDark => "xcode-dark",
            Self::XcodeLight => "xcode-light",
            Self::XcodeDusk => "xcode-dusk",
            Self::XcodeMidnight => "xcode-midnight",
            Self::XcodeCivic => "xcode-civic",
        }
    }

    /// この配色の色。
    ///
    /// 色は、それぞれの配色が公開している値から役割に合うものを選んでいる。
    /// 補足の文字に使える明るさの色がない配色では、文字の色をコメントの色へ寄せた色
    /// （Dracula、Monokai、Nord）か、背景へ寄せた色（Xcode の Dusk、Midnight、Civic）を使う。
    /// Xcode の配色は、Xcode に同梱されている定義から取っている。
    /// 差し色の面は、差し色を背景へ混ぜた色（明るい配色は12%、暗い配色は25%）。
    #[rustfmt::skip]
    pub fn palette(self) -> Palette {
        // 背景、面、文字、補足、罫、差し色、差し色の面、リンク、コード、NOTE、TIP、WARNING、CAUTION の順。
        let [background, surface, text, muted, border, accent, accent_soft, link, code, note, tip, warning, caution] = match self {
            Self::Default         => ["#ffffff", "#f6f6f7", "#1f1f1f", "#4d4d4d", "#e5e5e8", "#4078e0", "#e8effc", "#2f63c8", "#c4314b", "#2f63c8", "#1a7f4b", "#9a6200", "#c8372d"],
            Self::Dracula         => ["#282a36", "#21222c", "#f8f8f2", "#adb5cb", "#44475a", "#bd93f9", "#4d4467", "#8be9fd", "#ff79c6", "#8be9fd", "#50fa7b", "#ffb86c", "#ff5555"],
            Self::Alucard         => ["#fffbeb", "#efeddc", "#1f1f1f", "#6c664b", "#dedccf", "#644ac9", "#ece6e7", "#036a96", "#a3144d", "#036a96", "#14710a", "#a34d14", "#cb3a2a"],
            Self::Monokai         => ["#272822", "#1e1f1c", "#f8f8f2", "#b6b4a8", "#414339", "#a6e22e", "#475625", "#ae81ff", "#fd971f", "#66d9ef", "#a6e22e", "#e6db74", "#f92672"],
            Self::Kanagawa        => ["#1f1f28", "#16161d", "#dcd7ba", "#c8c093", "#363646", "#7e9cd8", "#373e54", "#7fb4ca", "#d27e99", "#7fb4ca", "#98bb6c", "#ff9e3b", "#ff5d62"],
            Self::KanagawaLotus   => ["#f2ecbc", "#e5ddb0", "#545464", "#716e61", "#d5cea3", "#4d699b", "#dedcb8", "#5d57a3", "#b35b79", "#4e8ca2", "#6f894e", "#cc6d00", "#c84053"],
            Self::TokyoNight      => ["#1a1b26", "#16161e", "#c0caf5", "#a9b1d6", "#292e42", "#7aa2f7", "#323d5a", "#7dcfff", "#bb9af7", "#7dcfff", "#9ece6a", "#e0af68", "#f7768e"],
            Self::TokyoNightStorm => ["#24283b", "#1f2335", "#c0caf5", "#a9b1d6", "#3b4261", "#7aa2f7", "#3a466a", "#7dcfff", "#bb9af7", "#7dcfff", "#9ece6a", "#e0af68", "#f7768e"],
            Self::CatppuccinMocha => ["#1e1e2e", "#181825", "#cdd6f4", "#a6adc8", "#313244", "#cba6f7", "#494060", "#89b4fa", "#fab387", "#89b4fa", "#a6e3a1", "#f9e2af", "#f38ba8"],
            Self::CatppuccinLatte => ["#eff1f5", "#e6e9ef", "#4c4f69", "#5c5f77", "#ccd0da", "#8839ef", "#e3dbf4", "#1e66f5", "#e64553", "#1e66f5", "#40a02b", "#df8e1d", "#d20f39"],
            Self::Nord            => ["#2e3440", "#3b4252", "#d8dee9", "#a0a8b6", "#434c5e", "#88c0d0", "#445764", "#81a1c1", "#b48ead", "#81a1c1", "#a3be8c", "#ebcb8b", "#bf616a"],
            Self::GruvboxDark     => ["#282828", "#1d2021", "#ebdbb2", "#a89984", "#3c3836", "#fe8019", "#5e3e24", "#83a598", "#d3869b", "#83a598", "#b8bb26", "#fabd2f", "#fb4934"],
            Self::GruvboxLight    => ["#fbf1c7", "#f2e5bc", "#3c3836", "#665c54", "#d5c4a1", "#af3a03", "#f2dbaf", "#076678", "#8f3f71", "#076678", "#79740e", "#b57614", "#9d0006"],
            Self::OneDark         => ["#282c34", "#21252b", "#abb2bf", "#9da5b4", "#3e4451", "#61afef", "#364d63", "#56b6c2", "#c678dd", "#61afef", "#98c379", "#e5c07b", "#e06c75"],
            Self::SolarizedDark   => ["#002b36", "#073642", "#93a1a1", "#839496", "#004052", "#2aa198", "#0a484e", "#268bd2", "#d33682", "#268bd2", "#859900", "#b58900", "#dc322f"],
            Self::SolarizedLight  => ["#fdf6e3", "#eee8d5", "#586e75", "#657b83", "#ddd6c1", "#cb4b16", "#f7e1ca", "#268bd2", "#d33682", "#268bd2", "#859900", "#b58900", "#dc322f"],
            Self::RosePine        => ["#191724", "#1f1d2e", "#e0def4", "#908caa", "#403d52", "#ebbcba", "#4e404a", "#c4a7e7", "#9ccfd8", "#9ccfd8", "#95b1ac", "#f6c177", "#eb6f92"],
            Self::RosePineDawn    => ["#faf4ed", "#fffaf3", "#464261", "#575279", "#dfdad9", "#907aa9", "#ede5e5", "#286983", "#56949f", "#286983", "#6d8f89", "#ea9d34", "#b4637a"],
            Self::Everforest      => ["#2d353b", "#232a2e", "#d3c6aa", "#9da9a0", "#3d484d", "#a7c080", "#4c584c", "#7fbbb3", "#d699b6", "#7fbbb3", "#83c092", "#dbbc7f", "#e67e80"],
            Self::GithubDark      => ["#0d1117", "#161b22", "#e6edf3", "#8b949e", "#30363d", "#58a6ff", "#203651", "#58a6ff", "#db61a2", "#58a6ff", "#3fb950", "#d29922", "#f85149"],
            Self::GithubLight     => ["#ffffff", "#f6f8fa", "#1f2328", "#59636e", "#d0d7de", "#0969da", "#e1edfb", "#0969da", "#bf3989", "#0969da", "#1a7f37", "#9a6700", "#cf222e"],
            Self::XcodeDark       => ["#1f1f24", "#23252b", "#ddddde", "#92a1b1", "#414249", "#fc5fa3", "#562f44", "#5482ff", "#d0a8ff", "#5dd8ff", "#67b7a4", "#efb759", "#f74a4a"],
            Self::XcodeLight      => ["#ffffff", "#f5f5f5", "#262626", "#5d6c79", "#e1e1e1", "#9b2393", "#f3e5f2", "#0e0eff", "#6c36a9", "#0f68a0", "#326d74", "#815f03", "#c41a16"],
            Self::XcodeDusk       => ["#1e2028", "#2d2d2d", "#ffffff", "#b0b1b4", "#414249", "#83c057", "#374834", "#00a0be", "#c67c48", "#5dd8ff", "#41b645", "#efb759", "#f74a4a"],
            Self::XcodeMidnight   => ["#000000", "#1a1919", "#ffffff", "#a6a6a6", "#272727", "#d31895", "#350625", "#00a0ff", "#e47c48", "#5dd8ff", "#41cc45", "#efb759", "#f74a4a"],
            Self::XcodeCivic      => ["#1f2029", "#282938", "#e1e2e7", "#9d9ea4", "#414249", "#1da9a2", "#1e4247", "#41a1c0", "#c77a4b", "#5dd8ff", "#45bb3e", "#efb759", "#f74a4a"],
        };
        Palette {
            background,
            surface,
            text,
            muted,
            border,
            accent,
            accent_soft,
            link,
            code,
            note,
            tip,
            warning,
            caution,
        }
    }
}

/// `#rgb`、`#rrggbb`、`#rrggbbaa` の相対輝度（0〜1）。透明度は見ない。色として読めなければ `None`。
pub(crate) fn relative_luminance(color: &str) -> Option<f64> {
    let digits = color.strip_prefix('#')?;
    if !matches!(digits.len(), 3 | 6 | 8) || !digits.is_ascii() {
        return None;
    }
    let channel = |index: usize| -> Option<f64> {
        let value = if digits.len() == 3 {
            u8::from_str_radix(&digits[index..index + 1], 16).ok()? * 17
        } else {
            u8::from_str_radix(&digits[index * 2..index * 2 + 2], 16).ok()?
        };
        let value = f64::from(value) / 255.0;
        Some(if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        })
    };
    Some(0.2126 * channel(0)? + 0.7152 * channel(1)? + 0.0722 * channel(2)?)
}

/// 背景がこの輝度より暗いと、黒より白の文字のほうがコントラストを取れる。
const DARK_BACKGROUND_LUMINANCE: f64 = 0.179;

/// 背景として暗い色か。色として読めなければ、明るい色として扱う。
pub(crate) fn is_dark_background(color: &str) -> bool {
    relative_luminance(color).is_some_and(|luminance| luminance < DARK_BACKGROUND_LUMINANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrast(a: &str, b: &str) -> f64 {
        let a = relative_luminance(a).unwrap();
        let b = relative_luminance(b).unwrap();
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn all_lists_each_theme_once_in_declaration_order() {
        for (index, theme) in ColorTheme::ALL.into_iter().enumerate() {
            assert_eq!(theme as usize, index, "{}", theme.as_str());
        }
    }

    #[test]
    fn names_match_the_names_accepted_in_toml() {
        #[derive(Deserialize)]
        struct Table {
            theme: ColorTheme,
        }
        for theme in ColorTheme::ALL {
            let text = format!("theme = \"{}\"\n", theme.as_str());
            let table: Table = toml::from_str(&text).unwrap();
            assert_eq!(table.theme, theme, "{text}");
        }
    }

    #[test]
    fn palette_values_are_six_digit_hex() {
        for theme in ColorTheme::ALL {
            let p = theme.palette();
            for value in [
                p.background,
                p.surface,
                p.text,
                p.muted,
                p.border,
                p.accent,
                p.accent_soft,
                p.link,
                p.code,
                p.note,
                p.tip,
                p.warning,
                p.caution,
            ] {
                let ok = value.len() == 7
                    && value.starts_with('#')
                    && value[1..]
                        .chars()
                        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
                assert!(ok, "{}: {value}", theme.as_str());
            }
        }
    }

    /// 写し間違いで読めない配色にならないよう、背景とのコントラスト比に下限を置く。
    /// 元の配色がもともと淡い色を使うので、記号と罫だけに使う色の下限は低くしている。
    /// 差し色は見出しの文字にも使うので、リンクやコードと同じ下限にする。
    #[test]
    fn palette_colors_stay_readable_on_the_background() {
        for theme in ColorTheme::ALL {
            let p = theme.palette();
            let cases = [
                ("text", p.text, 4.5),
                ("muted", p.muted, 4.0),
                ("link", p.link, 3.0),
                ("code", p.code, 3.0),
                ("accent", p.accent, 3.0),
                ("note", p.note, 2.0),
                ("tip", p.tip, 2.0),
                ("warning", p.warning, 2.0),
                ("caution", p.caution, 2.0),
            ];
            for (role, color, minimum) in cases {
                let ratio = contrast(color, p.background);
                assert!(
                    ratio >= minimum,
                    "{} の {role}（{color}）は {ratio:.2}",
                    theme.as_str()
                );
            }
            // 表の見出し行は、面の上に補足の文字を置く。
            let ratio = contrast(p.muted, p.surface);
            assert!(
                ratio >= 3.5,
                "{} の muted は面の上で {ratio:.2}",
                theme.as_str()
            );
        }
    }

    #[test]
    fn surfaces_and_rules_are_distinct_from_the_background() {
        for theme in ColorTheme::ALL {
            let p = theme.palette();
            for (role, color) in [
                ("surface", p.surface),
                ("border", p.border),
                ("accent_soft", p.accent_soft),
            ] {
                assert_ne!(color, p.background, "{} の {role}", theme.as_str());
            }
            // コードブロックの中で選択した範囲が見えるようにする。
            assert_ne!(p.accent_soft, p.surface, "{}", theme.as_str());
        }
    }

    #[test]
    fn dark_and_light_themes_are_told_apart_by_the_background() {
        let dark = [
            ColorTheme::Dracula,
            ColorTheme::Monokai,
            ColorTheme::Kanagawa,
            ColorTheme::TokyoNight,
            ColorTheme::TokyoNightStorm,
            ColorTheme::CatppuccinMocha,
            ColorTheme::Nord,
            ColorTheme::GruvboxDark,
            ColorTheme::OneDark,
            ColorTheme::SolarizedDark,
            ColorTheme::RosePine,
            ColorTheme::Everforest,
            ColorTheme::GithubDark,
            ColorTheme::XcodeDark,
            ColorTheme::XcodeDusk,
            ColorTheme::XcodeMidnight,
            ColorTheme::XcodeCivic,
        ];
        for theme in ColorTheme::ALL {
            assert_eq!(
                is_dark_background(theme.palette().background),
                dark.contains(&theme),
                "{}",
                theme.as_str()
            );
        }
    }

    #[test]
    fn luminance_reads_each_hex_form_and_rejects_others() {
        assert_eq!(relative_luminance("#000"), Some(0.0));
        assert!((relative_luminance("#fff").unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(relative_luminance("#ffffff"), relative_luminance("#fff"));
        assert_eq!(relative_luminance("#ffffff80"), relative_luminance("#fff"));
        for value in ["", "fff", "#ff", "#gggggg", "#ああ"] {
            assert_eq!(relative_luminance(value), None, "{value}");
        }
        assert!(is_dark_background("#000000"));
        assert!(!is_dark_background("#ffffff"));
        assert!(!is_dark_background("red"));
    }
}
