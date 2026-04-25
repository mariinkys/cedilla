// SPDX-License-Identifier: GPL-3.0

use std::path::PathBuf;

use crate::config::CedillaConfig;
use cosmic::cosmic_config;

/// Flags given to our COSMIC application to use in it's "init" function.
#[derive(Clone, Debug)]
pub struct Flags {
    pub config_handler: Option<cosmic_config::Config>,
    pub config: CedillaConfig,
    pub system_fonts: Vec<String>,
    pub open_with_file: Option<PathBuf>,
}

pub fn flags() -> Flags {
    let (config_handler, config) = (CedillaConfig::config_handler(), CedillaConfig::config());
    let system_fonts = load_system_fonts();
    let open_with_file = check_open_with_file();

    Flags {
        config_handler,
        config,
        system_fonts,
        open_with_file,
    }
}

fn check_open_with_file() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        let decoded = percent_encoding::percent_decode_str(&args[1])
            .decode_utf8_lossy()
            .into_owned();

        if is_markdown_file(&decoded) {
            Some(PathBuf::from(decoded))
        } else {
            None
        }
    } else {
        None
    }
}

fn load_system_fonts() -> Vec<String> {
    let source = font_kit::source::SystemSource::new();
    let mut names = source.all_families().unwrap_or_default();
    names.sort();
    names.dedup();
    names
}

fn is_markdown_file(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|ext| {
            ["md", "markdown"]
                .iter()
                .any(|e| ext.eq_ignore_ascii_case(e))
        })
        .unwrap_or(false)
}
