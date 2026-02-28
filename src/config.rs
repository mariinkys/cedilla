// SPDX-License-Identifier: GPL-3.0

use std::{fmt::Display, path::PathBuf, sync::LazyLock};

use cosmic::{
    cosmic_config::{self, Config, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry},
    theme,
};
use serde::{Deserialize, Serialize};

use crate::fl;

const APP_ID: &str = "dev.mariinkys.Cedilla";
const CONFIG_VERSION: u64 = 1;

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
pub struct CedillaConfig {
    pub app_theme: AppTheme,
    pub vault_path: String,
    pub show_helper_header_bar: ShowState,
    pub show_status_bar: ShowState,
    pub last_navbar_showstate: ShowState,
    pub last_preview_showstate: ShowState,
    pub open_last_file: BoolState,
    pub last_open_file: Option<PathBuf>,
    pub scrollbar_sync: BoolState,
}

impl Default for CedillaConfig {
    fn default() -> Self {
        let vault_path = dirs::data_dir().unwrap().join(APP_ID).join("vault");

        if !vault_path.exists() {
            std::fs::create_dir_all(&vault_path).expect("Failed to create vault directory");
        }

        Self {
            app_theme: AppTheme::default(),
            vault_path: vault_path.to_string_lossy().to_string(),
            show_helper_header_bar: ShowState::default(),
            show_status_bar: ShowState::default(),
            last_navbar_showstate: ShowState::default(),
            last_preview_showstate: ShowState::default(),
            open_last_file: BoolState::default(),
            last_open_file: None,
            scrollbar_sync: BoolState::default(),
        }
    }
}

impl CedillaConfig {
    pub fn config_handler() -> Option<Config> {
        Config::new(APP_ID, CONFIG_VERSION).ok()
    }

    pub fn config() -> Self {
        match Self::config_handler() {
            Some(config_handler) => {
                CedillaConfig::get_entry(&config_handler).unwrap_or_else(|(error, config)| {
                    eprintln!("Error whilst loading config: {error:#?}");
                    config
                })
            }
            None => CedillaConfig::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppTheme {
    Dark,
    Light,
    #[default]
    System,
}

impl AppTheme {
    pub fn theme(&self) -> theme::Theme {
        match self {
            Self::Dark => theme::Theme::dark(),
            Self::Light => theme::Theme::light(),
            Self::System => theme::system_preference(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ShowState {
    #[default]
    Show,
    Hide,
}

impl Display for ShowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShowState::Show => write!(f, "{}", fl!("show")),
            ShowState::Hide => write!(f, "{}", fl!("hide")),
        }
    }
}

impl ShowState {
    pub fn all_labels() -> &'static [String] {
        static LABELS: LazyLock<Vec<String>> = LazyLock::new(|| vec![fl!("show"), fl!("hide")]);
        &LABELS
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => ShowState::Show,
            1 => ShowState::Hide,
            _ => ShowState::default(),
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            ShowState::Show => 0,
            ShowState::Hide => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum BoolState {
    #[default]
    Yes,
    No,
}

impl Display for BoolState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoolState::Yes => write!(f, "{}", fl!("yes")),
            BoolState::No => write!(f, "{}", fl!("no")),
        }
    }
}

impl BoolState {
    pub fn all_labels() -> &'static [String] {
        static LABELS: LazyLock<Vec<String>> = LazyLock::new(|| vec![fl!("yes"), fl!("no")]);
        &LABELS
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => BoolState::Yes,
            1 => BoolState::No,
            _ => BoolState::default(),
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            BoolState::Yes => 0,
            BoolState::No => 1,
        }
    }
}

/// Represents the different inputs that can happen in the config [`ContextPage`]
#[derive(Debug, Clone)]
pub enum ConfigInput {
    /// Update the application theme
    UpdateTheme(usize),
    /// Update the help bar show state
    HelperHeaderBarShowState(ShowState),
    /// Update the status bar show state
    StatusBarShowState(ShowState),
    /// Update if the user wants to open last opened file or not
    OpenLastFile(BoolState),
    /// Update if the user wants the editor and preview scrollbars to be in sync
    ScrollbarSync(BoolState),
}
