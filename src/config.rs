// SPDX-License-Identifier: GPL-3.0

use std::{fmt::Display, sync::LazyLock};

use cosmic::{
    cosmic_config::{self, Config, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry},
    theme,
};
use serde::{Deserialize, Serialize};

use crate::fl;

const APP_ID: &str = "dev.mariinkys.Cedilla";
const CONFIG_VERSION: u64 = 1;

#[derive(Debug, Default, Clone, CosmicConfigEntry, Eq, PartialEq)]
pub struct CedillaConfig {
    pub app_theme: AppTheme,
    pub show_helper_header_bar: ShowState,
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

/// Represents the different inputs that can happen in the config [`ContextPage`]
#[derive(Debug, Clone)]
pub enum ConfigInput {
    /// Update the application theme
    UpdateTheme(usize),
    /// Update the help bar show state
    UpdateHelperHeaderBarShowState(ShowState),
}
