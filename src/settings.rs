/// Which rendering mode to use.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Normal,
    Decorated,
    Source,
}

/// Explicit color scheme override, or follow the OS setting.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
pub enum ColorScheme {
    #[default]
    Auto,
    Light,
    Dark,
}

/// Persisted application settings.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Settings {
    #[serde(default)]
    pub view_mode: ViewMode,

    #[serde(default = "default_true")]
    pub word_wrap: bool,

    #[serde(default = "default_font_size")]
    pub font_size: f32,

    #[serde(default)]
    pub color_scheme: ColorScheme,

    #[serde(default)]
    pub last_file: Option<String>,

    /// User-selected font family name. None = use system default.
    #[serde(default)]
    pub font_family: Option<String>,
}

fn default_true() -> bool { true }
fn default_font_size() -> f32 { 14.0 }

impl Default for Settings {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::default(),
            word_wrap: true,
            font_size: 14.0,
            color_scheme: ColorScheme::default(),
            last_file: None,
            font_family: None,
        }
    }
}
