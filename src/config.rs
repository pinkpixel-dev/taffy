use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use dirs::{config_dir, picture_dir, video_dir};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CaptureKind {
    #[default]
    Screenshot,
    Gif,
    Video,
}

impl CaptureKind {
    pub const ALL: [CaptureKind; 3] = [
        CaptureKind::Screenshot,
        CaptureKind::Gif,
        CaptureKind::Video,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Screenshot => "Screenshot",
            Self::Gif => "GIF",
            Self::Video => "Video",
        }
    }
}

impl std::fmt::Display for CaptureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CaptureSource {
    #[default]
    #[serde(alias = "Interactive")]
    Interactive,
    WholeScreen,
}

impl CaptureSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Interactive => "Selection",
            Self::WholeScreen => "Whole screen",
        }
    }
}

impl std::fmt::Display for CaptureSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub capture_kind: CaptureKind,
    pub capture_source: CaptureSource,
    pub frame_rate: u32,
    pub start_delay_secs: u32,
    pub stop_delay_secs: u32,
    pub show_pointer: bool,
    pub start_shortcut: String,
    pub stop_shortcut: String,
    pub screenshot_shortcut: String,
    pub screenshot_directory: PathBuf,
    pub gif_directory: PathBuf,
    pub video_directory: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            capture_kind: CaptureKind::Screenshot,
            capture_source: CaptureSource::Interactive,
            frame_rate: 24,
            start_delay_secs: 0,
            stop_delay_secs: 0,
            show_pointer: true,
            start_shortcut: "Ctrl+Shift+R".into(),
            stop_shortcut: "Ctrl+Shift+S".into(),
            screenshot_shortcut: "Print".into(),
            screenshot_directory: default_picture_dir(),
            gif_directory: default_video_dir(),
            video_directory: default_video_dir(),
        }
    }
}

pub fn load() -> Result<AppConfig> {
    let path = config_file_path()?;
    if !path.exists() {
        let config = AppConfig::default();
        save(&config)?;
        return Ok(config);
    }

    let data = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;
    let config = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse config at {}", path.display()))?;
    Ok(config)
}

pub fn save(config: &AppConfig) -> Result<()> {
    let path = config_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory {}", parent.display()))?;
    }
    ensure_output_dir(&config.screenshot_directory)?;
    ensure_output_dir(&config.gif_directory)?;
    ensure_output_dir(&config.video_directory)?;
    let data = serde_json::to_string_pretty(config)?;
    fs::write(&path, data).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub fn ensure_output_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create output directory {}", path.display()))
}

fn config_file_path() -> Result<PathBuf> {
    let base = config_dir().context("Could not determine XDG config directory")?;
    Ok(base.join("taffy").join("config.json"))
}

fn default_picture_dir() -> PathBuf {
    picture_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Taffy")
}

fn default_video_dir() -> PathBuf {
    video_dir()
        .or_else(picture_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Taffy")
}
