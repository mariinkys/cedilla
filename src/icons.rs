// SPDX-License-Identifier: GPL-3.0-only
#![allow(dead_code)]

use cosmic::widget::icon;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub(crate) static ICON_CACHE: OnceLock<Mutex<IconCache>> = OnceLock::new();

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct IconCacheKey {
    name: Cow<'static, str>,
    size: u16,
}

pub struct IconCache {
    cache: HashMap<IconCacheKey, icon::Handle>,
}

impl IconCache {
    pub fn new() -> Self {
        let mut cache = HashMap::new();

        macro_rules! bundle {
            ($name:expr, $size:expr) => {
                let data: &'static [u8] =
                    include_bytes!(concat!("../resources/icons/bundled/", $name, ".svg"));
                cache.insert(
                    IconCacheKey {
                        name: Cow::Borrowed($name),
                        size: $size,
                    },
                    icon::from_svg_bytes(data).symbolic(true),
                );
            };
        }

        bundle!("show-symbolic", 18);
        bundle!("hide-symbolic", 18);

        bundle!("helperbar/bold-symbolic", 18);
        bundle!("helperbar/bulleted-list-symbolic", 18);
        bundle!("helperbar/checked-list-symbolic", 18);
        bundle!("helperbar/code-symbolic", 18);
        bundle!("helperbar/heading-symbolic", 18);
        bundle!("helperbar/image-symbolic", 18);
        bundle!("helperbar/italic-symbolic", 18);
        bundle!("helperbar/link-symbolic", 18);
        bundle!("helperbar/numbered-list-symbolic", 18);
        bundle!("helperbar/rule-symbolic", 18);

        Self { cache }
    }
}

pub fn get_handle(name: &'static str, size: u16) -> icon::Handle {
    let mut icon_cache = ICON_CACHE.get().unwrap().lock().unwrap();
    icon_cache
        .cache
        .entry(IconCacheKey {
            name: Cow::Borrowed(name),
            size,
        })
        .or_insert_with(|| icon::from_name(name).size(size).handle())
        .clone()
}

pub fn get_handle_owned(name: String, size: u16) -> icon::Handle {
    let mut icon_cache = ICON_CACHE.get().unwrap().lock().unwrap();
    icon_cache
        .cache
        .entry(IconCacheKey {
            name: Cow::Owned(name.clone()),
            size,
        })
        .or_insert_with(|| icon::from_name(name).size(size).handle())
        .clone()
}
