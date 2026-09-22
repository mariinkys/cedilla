// SPDX-License-Identifier: GPL-3.0

use crate::app::{AppModel, Message};
use crate::config::{AppTheme, CedillaConfig, CedillaHighlighterTheme, ConfigInput};
use cosmic::iced::Font;
use cosmic::prelude::*;

impl AppModel {
    /// Applies a config change via the handler, falling back to an in-memory update if the handler fails or is missing.
    fn apply_config<F>(&mut self, updater: F) -> Task<cosmic::Action<Message>>
    where
        F: FnOnce(&mut CedillaConfig, Option<&cosmic::cosmic_config::Config>) -> Result<(), String>,
    {
        if let Some(handler) = &self.config_handler {
            if let Err(err) = updater(&mut self.config, Some(handler)) {
                eprintln!("{err}");
            }
        } else {
            let _ = updater(&mut self.config, None);
        }
        Task::none()
    }

    pub fn handle_config_input(&mut self, input: ConfigInput) -> Task<cosmic::Action<Message>> {
        match input {
            ConfigInput::SystemThemeModeChange => {
                cosmic::command::set_theme(self.config.app_theme.theme())
            }
            ConfigInput::UpdateTheme(index) => {
                let app_theme = match index {
                    1 => AppTheme::Dark,
                    2 => AppTheme::Light,
                    _ => AppTheme::System,
                };

                if let Some(handler) = &self.config_handler {
                    if let Err(err) = self.config.set_app_theme(handler, app_theme) {
                        eprintln!("{err}");
                        self.config.app_theme = app_theme;
                    }
                } else {
                    self.config.app_theme = app_theme;
                }

                cosmic::command::set_theme(self.config.app_theme.theme())
            }
            ConfigInput::HelperHeaderBarShowState(show_state) => {
                self.apply_config(|config, handler| {
                    if let Some(h) = handler {
                        config
                            .set_show_helper_header_bar(h, show_state)
                            .map_err(|e| e.to_string())?;
                    } else {
                        config.show_helper_header_bar = show_state;
                    }
                    Ok(())
                })
            }
            ConfigInput::StatusBarShowState(show_state) => self.apply_config(|config, handler| {
                if let Some(h) = handler {
                    config
                        .set_show_status_bar(h, show_state)
                        .map_err(|e| e.to_string())?;
                } else {
                    config.show_status_bar = show_state;
                }
                Ok(())
            }),
            ConfigInput::OpenLastFile(state) => self.apply_config(|config, handler| {
                if let Some(h) = handler {
                    config
                        .set_open_last_file(h, state)
                        .map_err(|e| e.to_string())?;
                } else {
                    config.open_last_file = state;
                }
                Ok(())
            }),
            ConfigInput::ScrollbarSync(state) => self.apply_config(|config, handler| {
                if let Some(h) = handler {
                    config
                        .set_scrollbar_sync(h, state)
                        .map_err(|e| e.to_string())?;
                } else {
                    config.scrollbar_sync = state;
                }
                Ok(())
            }),
            ConfigInput::VimMode(state) => self.apply_config(|config, handler| {
                if let Some(h) = handler {
                    config
                        .set_vim_mode(h, state)
                        .map_err(|e| e.to_string())?;
                } else {
                    config.vim_mode = state;
                }
                Ok(())
            }),
            ConfigInput::GotenbergUrlInput(state) => self.apply_config(|config, handler| {
                if let Some(h) = handler {
                    config
                        .set_gotenberg_url(h, state)
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            }),
            ConfigInput::GotenbergUrlSave => {
                self.gotenberg_client = gotenberg_pdf::Client::new(&self.config.gotenberg_url);
                Task::none()
            }
            ConfigInput::UpdateTextSize(new_size) => {
                let size = new_size as i32;
                self.apply_config(|config, handler| {
                    if let Some(h) = handler {
                        config.set_text_size(h, size).map_err(|e| e.to_string())?;
                    } else {
                        config.text_size = size;
                    }
                    Ok(())
                })
            }
            ConfigInput::UpdateLightHighlighterTheme(index) => {
                if let Some(&theme) = cosmic::iced::highlighter::Theme::ALL.get(index) {
                    let theme = CedillaHighlighterTheme::from(theme);
                    self.apply_config(move |config, handler| {
                        if let Some(h) = handler {
                            config
                                .set_light_highlighter_theme(h, theme)
                                .map_err(|e| e.to_string())?;
                        } else {
                            config.light_highlighter_theme = theme;
                        }
                        Ok(())
                    })
                } else {
                    Task::none()
                }
            }
            ConfigInput::UpdateDarkHighlighterTheme(index) => {
                if let Some(&theme) = cosmic::iced::highlighter::Theme::ALL.get(index) {
                    let theme = CedillaHighlighterTheme::from(theme);
                    self.apply_config(move |config, handler| {
                        if let Some(h) = handler {
                            config
                                .set_dark_highlighter_theme(h, theme)
                                .map_err(|e| e.to_string())?;
                        } else {
                            config.dark_highlighter_theme = theme;
                        }
                        Ok(())
                    })
                } else {
                    Task::none()
                }
            }
            ConfigInput::UpdateFont(index) => {
                if let Some(name) = self.system_fonts.get(index).cloned() {
                    // TODO: Is this the only way we can do this?
                    let static_name: &'static str = Box::leak(name.clone().into_boxed_str());

                    self.cedilla_font = Font {
                        family: cosmic::iced::font::Family::Name(static_name),
                        weight: cosmic::iced::font::Weight::Normal,
                        stretch: cosmic::iced::font::Stretch::Normal,
                        style: cosmic::iced::font::Style::Normal,
                    };

                    self.apply_config(move |config, handler| {
                        if let Some(h) = handler {
                            config
                                .set_selected_font_family(h, Some(name))
                                .map_err(|e| e.to_string())?;
                        } else {
                            config.selected_font_family = Some(name);
                        }
                        Ok(())
                    })
                } else {
                    Task::none()
                }
            }
            ConfigInput::ResetFont => {
                self.cedilla_font = Font::DEFAULT;

                self.apply_config(|config, handler| {
                    if let Some(h) = handler {
                        config
                            .set_selected_font_family(h, None)
                            .map_err(|e| e.to_string())?;
                    } else {
                        config.selected_font_family = None;
                    }
                    Ok(())
                })
            }
        }
    }
}
