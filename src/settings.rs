//! One settings surface with categories, search, visible explanations and
//! named choices instead of numeric enums.

use crate::app::{App, Dialog, SettingsCategory, View};
use crate::model::SUBTITLE_PRECISIONS;
use crate::skin::{self, Icon, Palette};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, Vec2,
};

impl App {
    pub fn settings_view(&mut self, ctx: &egui::Context, p: &Palette) {
        egui::SidePanel::left("settings-cats")
            .exact_width(224.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(p.panel)
                    .inner_margin(egui::Margin::symmetric(12, 12))
                    .stroke(Stroke::new(1.0, p.line_soft)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if skin::icon_button(ui, Icon::ArrowLeft, 26.0, false, "Back to the player", p).clicked() {
                        self.view = View::Player;
                    }
                    ui.label(egui::RichText::new("Settings").size(15.0).strong().color(p.text));
                });
                ui.add_space(8.0);
                for cat in SettingsCategory::ALL {
                    let active = self.settings_cat == cat;
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                    if active {
                        ui.painter().rect_filled(rect, CornerRadius::same(7), p.panel2);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, CornerRadius::same(7), p.hover);
                    }
                    ui.painter().text(
                        pos2(rect.left() + 10.0, rect.center().y),
                        Align2::LEFT_CENTER,
                        cat.label(),
                        skin::font(12.2),
                        if active { p.accent } else { p.muted },
                    );
                    if resp.clicked() {
                        self.settings_cat = cat;
                        self.settings_highlight = None;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.label(
                        egui::RichText::new("Settings file keeps old keys; nothing is thrown away.")
                            .size(9.0)
                            .color(p.dim),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg).inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                // One column, centred in the window, so a wide screen does not
                // leave a page of dead space beside the settings.
                let avail = ui.available_width();
                let col = (avail - 64.0).clamp(420.0, 860.0);
                let pad = ((avail - col) * 0.5).max(0.0);

                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(0, 14))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(pad);
                            ui.vertical(|ui| {
                                ui.set_width(col);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(self.settings_cat.label())
                                            .size(21.0)
                                            .strong()
                                            .color(p.text),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.add_sized(
                                                vec2(260.0, 30.0),
                                                egui::TextEdit::singleline(&mut self.settings_search)
                                                    .hint_text("Search settings, words and values")
                                                    .margin(vec2(9.0, 6.0)),
                                            );
                                            ui.label(
                                                egui::RichText::new(
                                                    "One place; advanced options stay in their category.",
                                                )
                                                .size(10.0)
                                                .color(p.dim),
                                            );
                                        },
                                    );
                                });
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(
                                        "Every option explains itself. No numeric codes, no duplicate simple/advanced screens.",
                                    )
                                    .size(10.5)
                                    .color(p.dim),
                                );
                            });
                        });
                    });
                skin::divider(ui, p);
                // A search hit jumps to a category; say which option was
                // asked for instead of leaving the person to hunt for it.
                if let Some(hit) = self
                    .settings_highlight
                    .clone()
                    .filter(|h| h != "default.player")
                {
                    ui.horizontal(|ui| {
                        ui.add_space(pad);
                        ui.label(egui::RichText::new("Jumped to").size(10.0).color(p.dim));
                        ui.label(egui::RichText::new(&hit).size(10.6).color(p.accent));
                        if skin::ghost(ui, "Clear", p).clicked() {
                            self.settings_highlight = None;
                        }
                    });
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(pad);
                            ui.vertical(|ui| {
                                ui.set_width(col);
                                egui::Frame::new()
                                    .fill(p.panel)
                                    .stroke(Stroke::new(1.0, p.line_soft))
                                    .corner_radius(CornerRadius::same(12))
                                    .inner_margin(egui::Margin::same(22))
                                    .show(ui, |ui| {
                                        ui.set_width(col - 44.0);
                                        if self.settings_search.trim().is_empty() {
                                            self.settings_body(ui, p);
                                        } else {
                                            self.settings_hit_list(ui, p);
                                        }
                                        ui.add_space(20.0);
                                    });
                                ui.add_space(24.0);
                            });
                        });
                    });
            });
    }

    fn settings_results(&mut self, ui: &mut Ui, p: &Palette) {
        let q = self.settings_search.trim().to_lowercase();
        let mut hits: Vec<(SettingsCategory, String)> = Vec::new();
        for cat in SettingsCategory::ALL {
            for (label, syn) in setting_index(cat) {
                if label.to_lowercase().contains(&q) || syn.contains(&q) {
                    hits.push((cat, label.to_string()));
                }
            }
        }
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(26, 8))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{} matching settings", hits.len()))
                            .size(10.0)
                            .color(p.dim),
                    );
                    for (cat, label) in hits.iter().take(6) {
                        if skin::chip(ui, &format!("{} · {}", cat.label(), label), false, p).clicked() {
                            self.settings_cat = *cat;
                            self.settings_highlight = None;
                            self.settings_search.clear();
                        }
                    }
                });
            });
    }

    fn settings_hit_list(&mut self, ui: &mut Ui, p: &Palette) {
        let q = self.settings_search.trim().to_lowercase();
        let mut hits: Vec<(SettingsCategory, &'static str, &'static str)> = Vec::new();
        for cat in SettingsCategory::ALL {
            for (label, syn) in setting_index(cat) {
                if label.to_lowercase().contains(&q) || syn.contains(&q) {
                    hits.push((cat, label, syn));
                }
            }
        }
        if hits.is_empty() {
            ui.label(
                egui::RichText::new("No setting matches that. Try “delay”, “device”, “theme” or “resume”.")
                    .size(11.0)
                    .color(p.dim),
            );
            return;
        }
        ui.label(
            egui::RichText::new(format!(
                "{} matching settings — click one to jump to it",
                hits.len()
            ))
            .size(10.0)
            .color(p.dim),
        );
        ui.add_space(6.0);
        for (cat, label, _) in hits {
            let (rect, resp) =
                ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, CornerRadius::same(6), p.hover);
            }
            ui.painter().text(
                pos2(rect.left() + 4.0, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                skin::font(12.0),
                p.text,
            );
            ui.painter().text(
                pos2(rect.right() - 4.0, rect.center().y),
                Align2::RIGHT_CENTER,
                cat.label(),
                skin::font(10.0),
                p.dim,
            );
            if resp.clicked() {
                self.settings_cat = cat;
                self.settings_search.clear();
                self.settings_highlight = Some(label.to_string());
            }
        }
    }

    fn settings_body(&mut self, ui: &mut Ui, p: &Palette) {
        // Comfortable sliders: the settings column is wide enough for them and
        // a 150 px track is a precision instrument, not a control.
        ui.spacing_mut().slider_width = 300.0;
        // The row helper is a free function, so the one setting the search
        // found travels through egui's own store instead of every call site.
        ui.ctx().data_mut(|d| {
            d.insert_temp(
                egui::Id::new("settings_highlight"),
                self.settings_highlight.clone(),
            )
        });
        match self.settings_cat {
            SettingsCategory::General => self.cat_general(ui, p),
            SettingsCategory::Interface => self.cat_interface(ui, p),
            SettingsCategory::Privacy => self.cat_privacy(ui, p),
            SettingsCategory::Controls => self.cat_controls(ui, p),
            SettingsCategory::Playback => self.cat_playback(ui, p),
            SettingsCategory::Video => self.cat_video(ui, p),
            SettingsCategory::Audio => self.cat_audio(ui, p),
            SettingsCategory::Subtitles => self.cat_subtitles(ui, p),
            SettingsCategory::Library => self.cat_library(ui, p),
            SettingsCategory::Network => self.cat_network(ui, p),
            SettingsCategory::Performance => self.cat_performance(ui, p),
            SettingsCategory::Recording => self.cat_recording(ui, p),
            SettingsCategory::Extensions => self.cat_extensions(ui, p),
        }
    }

    // -----------------------------------------------------------------------
    // Category bodies
    // -----------------------------------------------------------------------


    fn cat_general(&mut self, ui: &mut Ui, p: &Palette) {
        self.default_player_card(ui, p);
        ui.add_space(22.0);

        let before = self.settings.start_page.clone();
        setting(
            ui,
            p,
            "Start page",
            "Where VLCCraft opens. The library is always one click away.",
            |ui| {
                egui::ComboBox::from_id_salt("startpage")
                    .selected_text(&self.settings.start_page)
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for page in ["Home", "Videos", "Last watched"] {
                            ui.selectable_value(&mut self.settings.start_page, page.to_string(), page);
                        }
                    });
            },
        );
        if before != self.settings.start_page {
            self.touch();
        }

        setting(
            ui,
            p,
            "Check for updates",
            "Asks the package manager. VLC's own files are never replaced.",
            |ui| {
                let mut check = self.settings.check_updates;
                if skin::toggle(ui, &mut check, p).changed() {
                    self.settings.check_updates = check;
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Show the welcome tour again",
            "Theme choice and the shortcuts worth knowing. Skippable.",
            |ui| {
                if skin::ghost(ui, "Open welcome tour", p).clicked() {
                    self.dialog = Some(Dialog::Onboarding);
                }
            },
        );

        self.advanced(ui, p, "Advanced");
        setting(
            ui,
            p,
            "Save settings on change",
            "Each store is written atomically, so one bad file never loses the library.",
            |ui| {
                ui.label(egui::RichText::new("Always on").size(11.0).color(p.muted));
            },
        );
    }

    fn default_player_card(&mut self, ui: &mut Ui, p: &Palette) {
        let s = self.defaults.clone();
        let ours = s.available && s.video_ours && s.audio_ours;
        let highlighted = matches!(
            self.settings_highlight.as_deref(),
            Some("default.player") | Some("Default player")
        );
        egui::Frame::new()
            .fill(p.panel2)
            .stroke(Stroke::new(
                if highlighted { 1.6 } else { 1.0 },
                if highlighted || ours { p.accent } else { p.line_soft },
            ))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Default player")
                            .size(13.5)
                            .strong()
                            .color(p.text),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (tag, colour) = if !s.available {
                            ("not available on this system", p.warn)
                        } else if ours {
                            ("VLCCraft opens both", p.accent)
                        } else if s.video_any || s.audio_any {
                            ("partly set", p.warn)
                        } else {
                            ("another player has them", p.muted)
                        };
                        ui.label(egui::RichText::new(tag).size(10.2).color(colour));
                    });
                });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(if !s.available {
                        "Linux hands files over through a small app entry; macOS and Windows decide this in the system's own settings, so there is nothing to switch here."
                    } else if ours {
                        "Double-clicked video and music files open here. VLC stays installed and updates exactly as before."
                    } else {
                        "Pick which kinds of file VLCCraft should open, then hand them over."
                    })
                    .size(10.8)
                    .color(p.muted),
                );

                ui.add_space(8.0);
                setting(
                    ui,
                    p,
                    "Open double-clicked video files",
                    "Hands the video types over once; VLC keeps its own package and updates.",
                    |ui| {
                        let mut v = self.settings.default_video;
                        if skin::toggle(ui, &mut v, p).changed() {
                            self.settings.default_video = v;
                            self.touch();
                        }
                    },
                );
                setting(
                    ui,
                    p,
                    "Open double-clicked music files",
                    "Audio can stay with the other player if you would rather split the two.",
                    |ui| {
                        let mut a = self.settings.default_audio;
                        if skin::toggle(ui, &mut a, p).changed() {
                            self.settings.default_audio = a;
                            self.touch();
                        }
                    },
                );
                setting(ui, p, "Video files open in", "", |ui| {
                    self.handler_dot(ui, p, &s.video_name, s.video_ours);
                });
                setting(ui, p, "Music files open in", "", |ui| {
                    self.handler_dot(ui, p, &s.audio_name, s.audio_ours);
                });

                let both = self.settings.default_video && self.settings.default_audio;
                let none = !self.settings.default_video && !self.settings.default_audio;
                if none && s.available {
                    ui.label(
                        egui::RichText::new("Pick at least one kind of file to hand over.")
                            .size(10.2)
                            .color(p.dim),
                    );
                }
                if let Some(msg) = &self.defaults_msg {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(msg).size(10.2).color(p.bad));
                }

                let mut apply_now = false;
                let mut hand_back = false;
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !none && s.available {
                            let label = if both {
                                if ours { "Keep VLCCraft as the default" } else { "Make VLCCraft the default" }
                            } else if self.settings.default_video {
                                "Make VLCCraft open video"
                            } else {
                                "Make VLCCraft open music"
                            };
                            apply_now = skin::primary(ui, label, p).clicked();
                        }
                        if self.defaults_rec.can_hand_back() {
                            hand_back = skin::ghost(ui, "Hand back to the other player", p).clicked();
                        }
                    });
                });
                if apply_now {
                    match crate::defaults::apply(
                        self.settings.default_video,
                        self.settings.default_audio,
                        self.settings.accent,
                    ) {
                        Ok(msg) => {
                            self.defaults_msg = None;
                            self.settings.default_hint = false;
                            self.toast_now(msg);
                        }
                        Err(e) => self.defaults_msg = Some(e),
                    }
                    self.refresh_defaults();
                    self.touch();
                }
                if hand_back {
                    match crate::defaults::restore() {
                        Ok(msg) => {
                            self.defaults_msg = None;
                            self.toast_now(msg);
                        }
                        Err(e) => self.defaults_msg = Some(e),
                    }
                    self.refresh_defaults();
                    self.touch();
                }
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(
                        if crate::platform::can_set_default_player() {
                            "VLCCraft keeps a small copy of itself in your own folder and adds an ordinary app entry. No system folder is changed, and the other player can have the files back at any time."
                        } else {
                            "The default player is chosen in the system's own settings on this machine; VLC stays installed either way and updates normally."
                        },
                    )
                    .size(9.6)
                    .color(p.dim),
                );
            });
    }

    fn handler_dot(&self, ui: &mut Ui, p: &Palette, name: &str, ours: bool) {
        // Right to left: the name first, then its dot to the left of it.
        ui.label(
            egui::RichText::new(name)
                .size(11.0)
                .color(if ours { p.accent } else { p.muted }),
        );
        let (rect, _) = ui.allocate_exact_size(vec2(9.0, 9.0), Sense::hover());
        ui.painter().circle_filled(
            rect.center(),
            3.4,
            if ours { p.accent } else { p.dim },
        );
    }


    fn cat_interface(&mut self, ui: &mut Ui, p: &Palette) {
        setting(
            ui,
            p,
            "Theme",
            "Four calm palettes. The video area stays neutral in all of them.",
            |ui| {
                // Right to left: the first chip takes the row's right edge.
                for theme in skin::THEMES.iter().rev() {
                    if skin::chip(ui, theme, &self.settings.theme == theme, p).clicked() {
                        self.settings.theme = theme.to_string();
                        self.touch();
                    }
                }
            },
        );

        setting(
            ui,
            p,
            "Accent",
            "Recolours controls and highlights; the shape of the mark never changes.",
            |ui| {
                if let Some(rgb) = swatches(ui, p, self.settings.accent) {
                    self.settings.accent = rgb;
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Interface scale",
            "Larger text and controls, without touching the picture.",
            |ui| {
                let mut scale = self.settings.interface_scale;
                if row_slider(ui, p, "scale", &mut scale, 0.85..=1.35, 0.05, " %", percent_scale) {
                    self.settings.interface_scale = scale;
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Reduce motion",
            "Stops the small slide-in flourishes; the player itself is untouched.",
            |ui| {
                let mut reduce = self.settings.reduce_motion;
                if skin::toggle(ui, &mut reduce, p).changed() {
                    self.settings.reduce_motion = reduce;
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Remember the window size",
            "Applies on the next start.",
            |ui| {
                let mut remember = self.settings.remember_window;
                if skin::toggle(ui, &mut remember, p).changed() {
                    self.settings.remember_window = remember;
                    self.touch();
                }
            },
        );

        self.advanced(ui, p, "Advanced");
        setting(
            ui,
            p,
            "Control bar",
            "The default bar is complete. Editing it is opt-in, with drop targets listed as rows.",
            |ui| {
                stack(ui, |ui| {
                    if skin::ghost(ui, "Open the control editor", p).clicked() {
                        self.dialog = Some(Dialog::Customise);
                    }
                    if skin::ghost(ui, "Restore the default layout", p).clicked() {
                        self.settings.layout = crate::model::Layout::default();
                        self.toast_now("Default controls restored");
                        self.touch();
                    }
                });
            },
        );
    }


    fn cat_privacy(&mut self, ui: &mut Ui, p: &Palette) {
        setting(
            ui,
            p,
            "Allow online subtitle lookup",
            "Off by default. When on, only a search you start sends text to the provider.",
            |ui| {
                let mut online = self.settings.subtitle_online_enabled;
                if skin::toggle(ui, &mut online, p).changed() {
                    self.settings.subtitle_online_enabled = online;
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Telemetry",
            "There is none. The skin talks to libvlc on this machine and to nothing else.",
            |ui| {
                ui.label(egui::RichText::new("Nothing collected").size(11.0).color(p.muted));
            },
        );

        setting(
            ui,
            p,
            "Local by default",
            "The library index, resume points and presets never leave this machine.",
            |ui| {
                ui.label(
                    egui::RichText::new("Kept in your own folder")
                        .size(11.0)
                        .color(p.muted),
                );
            },
        );
    }


    fn cat_controls(&mut self, ui: &mut Ui, p: &Palette) {
        setting(
            ui,
            p,
            "Keyboard shortcuts",
            "Search, rebind, spot conflicts and reset one action or all of them.",
            |ui| {
                if skin::primary(ui, "Open shortcut editor", p).clicked() {
                    self.dialog = Some(Dialog::Hotkeys);
                }
            },
        );

        setting(
            ui,
            p,
            "Escape",
            "Closes the innermost surface only: popover, then dialog, then full screen.",
            |ui| {
                ui.label(egui::RichText::new("Fixed · Esc").size(11.5).color(p.muted));
            },
        );

        setting(
            ui,
            p,
            "Control bar",
            "Add, remove and reorder controls; drop targets are explicit.",
            |ui| {
                if skin::ghost(ui, "Customise controls…", p).clicked() {
                    self.dialog = Some(Dialog::Customise);
                }
            },
        );

        self.advanced(ui, p, "Advanced");
        setting(
            ui,
            p,
            "Layout profiles",
            "A profile carries the bar order and the look; switching is one action.",
            |ui| {
                stack(ui, |ui| {
                    if skin::ghost(ui, "Save current as a profile…", p).clicked() {
                        self.profile_name = format!("Profile {}", self.profiles.len() + 1);
                        self.dialog = Some(Dialog::Customise);
                    }
                    if skin::ghost(ui, "Manage profiles…", p).clicked() {
                        self.dialog = Some(Dialog::Customise);
                    }
                });
            },
        );
    }

    fn cat_playback(&mut self, ui: &mut Ui, p: &Palette) {
        setting(
            ui,
            p,
            "Resume where you stopped",
            "Position is stored per item and offered near the controls, not in a corner.",
            |ui| {
                let mut resume = self.settings.resume_enabled;
                if skin::toggle(ui, &mut resume, p).changed() {
                    self.settings.resume_enabled = resume;
                    self.touch();
                }
            },
        );

        let auto = self.settings.auto_advance;
        let help = if auto {
            "On — the next item in the queue starts by itself, with nothing drawn over the picture."
        } else {
            "Off — playback stops at the end of each item. The play button then starts it again."
        };
        setting(
            ui,
            p,
            "Play the queue automatically",
            help,
            |ui| {
                let mut auto = self.settings.auto_advance;
                if skin::toggle(ui, &mut auto, p).changed() {
                    self.settings.auto_advance = auto;
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Default volume",
            "Applies to new sessions; the level you set while watching is kept for the session.",
            |ui| {
                let mut volume = self.settings.volume as f32;
                if row_slider(ui, p, "volume", &mut volume, 0.0..=125.0, 1.0, " %", whole) {
                    self.settings.volume = volume.round().clamp(0.0, 125.0) as u8;
                    self.touch();
                }
            },
        );

        self.advanced(ui, p, "Advanced");
        setting(
            ui,
            p,
            "End of an item",
            "Nothing is drawn over the last frame when the queue is empty; the play button simply returns.",
            |ui| {
                ui.label(egui::RichText::new("Quiet by design").size(11.0).color(p.muted));
            },
        );
    }

    fn cat_video(&mut self, ui: &mut Ui, p: &Palette) {
        setting(
            ui,
            p,
            "Hardware decoding",
            "Off is a useful fallback for a broken driver; it applies on the next item.",
            |ui| {
                let mut hw = self.settings.hardware_decode;
                if skin::toggle(ui, &mut hw, p).changed() {
                    self.settings.hardware_decode = hw;
                    self.touch();
                    self.toast_now("Decoding change applies to the next item");
                }
            },
        );

        setting(
            ui,
            p,
            "Frame previews when seeking",
            "Built from frames already decoded; bounded, and dropped rather than stalling a seek.",
            |ui| {
                let mut thumbs = self.settings.show_thumbnails;
                if skin::toggle(ui, &mut thumbs, p).changed() {
                    self.settings.show_thumbnails = thumbs;
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Reveal distance",
            "How near the edge the pointer has to be before the full-screen bars slide in.",
            |ui| {
                let mut d = self.settings.reveal_distance;
                if row_stepper(ui, p, "reveal", &mut d, 2.0..=40.0, 1.0, " px") {
                    self.settings.reveal_distance = d;
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Hide after",
            "How long the bars wait in full screen before sliding away again.",
            |ui| {
                let mut ms = self.settings.hide_delay_ms as f32;
                if row_stepper(ui, p, "hide", &mut ms, 400.0..=6000.0, 100.0, " ms") {
                    self.settings.hide_delay_ms = ms.round() as u64;
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Picture adjustments",
            "Rotate, flip, zoom and aspect live in the sidebar, over the moving picture.",
            |ui| {
                if skin::ghost(ui, "Open the video tab", p).clicked() {
                    self.view = View::Player;
                    self.sidebar_open = true;
                    self.sidebar_tab = 2;
                }
            },
        );
    }

    fn cat_audio(&mut self, ui: &mut Ui, p: &Palette) {
        let options = self.audio_choices();
        let current_label = options
            .iter()
            .find(|(_, value)| value == &self.settings.audio_output)
            .map(|(label, _)| label.clone())
            .unwrap_or_else(|| "Automatic".into());
        setting(
            ui,
            p,
            "Output device",
            "Switching reopens audio only; the picture keeps playing.",
            |ui| {
                egui::ComboBox::from_id_salt("audio-out")
                    .selected_text(current_label)
                    .width(240.0)
                    .show_ui(ui, |ui| {
                        let mut picked: Option<String> = None;
                        for (label, value) in &options {
                            if ui
                                .selectable_label(&self.settings.audio_output == value, label)
                                .clicked()
                            {
                                picked = Some(value.clone());
                            }
                        }
                        if let Some(value) = picked {
                            self.set_audio_choice(&value);
                        }
                    });
            },
        );

        let presets = self
            .engine
            .as_ref()
            .map(|e| e.equalizer_presets())
            .unwrap_or_default();
        if presets.is_empty() {
            setting(
                ui,
                p,
                "Sound presets",
                "VLC's equalizer presets, applied to the live player.",
                |ui| {
                    ui.label(
                        egui::RichText::new("This libvlc build reports no presets.")
                            .size(10.5)
                            .color(p.dim),
                    );
                },
            );
        } else {
            let current = self
                .now
                .as_ref()
                .and_then(|n| n.equalizer.clone())
                .unwrap_or_else(|| "Off".into());
            let mut chosen: Option<Option<usize>> = None;
            setting(
                ui,
                p,
                "Sound preset",
                "VLC's equalizer presets, applied to the live player. Flat is a real preset, not a reset.",
                |ui| {
                    egui::ComboBox::from_id_salt("audio-eq")
                        .selected_text(&current)
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(current == "Off", "Off").clicked() {
                                chosen = Some(None);
                            }
                            for (i, name) in presets.iter().enumerate() {
                                if ui.selectable_label(current == *name, name).clicked() {
                                    chosen = Some(Some(i));
                                }
                            }
                        });
                },
            );
            match chosen {
                Some(None) => {
                    if let Some(engine) = &self.engine {
                        engine.set_equalizer(None);
                    }
                    if let Some(now) = &mut self.now {
                        now.equalizer = None;
                    }
                }
                Some(Some(i)) => {
                    if let Some(engine) = &self.engine {
                        engine.set_equalizer(Some(i));
                    }
                    if let Some(now) = &mut self.now {
                        now.equalizer = presets.get(i).cloned();
                    }
                }
                None => {}
            }
        }

        self.advanced(ui, p, "Advanced");
        setting(
            ui,
            p,
            "Mute keeps decoding",
            "The stream keeps its position; unmuting does not rewind.",
            |ui| {
                ui.label(egui::RichText::new("Always on").size(11.0).color(p.muted));
            },
        );
    }

    fn cat_subtitles(&mut self, ui: &mut Ui, p: &Palette) {
        setting(
            ui,
            p,
            "Default subtitle language",
            "Used when a file offers more than one track and no preset overrides it.",
            |ui| {
                egui::ComboBox::from_id_salt("sub-lang")
                    .selected_text(&self.settings.subtitle_default_language)
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for lang in ["English", "Spanish", "French", "German", "Japanese", "None"] {
                            ui.selectable_value(
                                &mut self.settings.subtitle_default_language,
                                lang.to_string(),
                                lang,
                            );
                        }
                    });
            },
        );

        let current = SUBTITLE_PRECISIONS
            .iter()
            .find(|(k, _)| *k == self.settings.subtitle_precision)
            .map(|(_, v)| *v)
            .unwrap_or("Filename contains the movie name");
        setting(
            ui,
            p,
            "Subtitle filename matching",
            "How a video finds the subtitle beside it — named choices, never a number.",
            |ui| {
                egui::ComboBox::from_id_salt("sub-precision")
                    .selected_text(current)
                    .width(240.0)
                    .show_ui(ui, |ui| {
                        for (key, label) in SUBTITLE_PRECISIONS {
                            if ui
                                .selectable_label(self.settings.subtitle_precision == *key, *label)
                                .clicked()
                            {
                                self.settings.subtitle_precision = (*key).into();
                                self.touch();
                            }
                        }
                    });
            },
        );

        setting(
            ui,
            p,
            "Subtitle delay",
            "Applied live, with the picture still moving. Also a slider in the sidebar.",
            |ui| {
                let mut delay = self.settings.subtitle_style.delay_ms as f32;
                if row_slider(ui, p, "sub-delay", &mut delay, -1500.0..=1500.0, 10.0, " ms", whole) {
                    let ms = delay.round() as i64;
                    self.settings.subtitle_style.delay_ms = ms;
                    if let Some(now) = &mut self.now {
                        now.style.delay_ms = ms;
                    }
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Subtitle size",
            "Relative to the picture, so it keeps its proportion in full screen.",
            |ui| {
                let mut size = self.settings.subtitle_style.size_pct;
                if row_slider(ui, p, "sub-size", &mut size, 2.0..=8.0, 0.1, " %", one_decimal) {
                    self.settings.subtitle_style.size_pct = size;
                    if let Some(now) = &mut self.now {
                        now.style.size_pct = size;
                    }
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Outline",
            "A dark edge keeps the words readable over a bright scene.",
            |ui| {
                let mut outline = self.settings.subtitle_style.outline;
                if row_slider(ui, p, "sub-outline", &mut outline, 0.0..=3.0, 0.1, "", one_decimal) {
                    self.settings.subtitle_style.outline = outline;
                    if let Some(now) = &mut self.now {
                        now.style.outline = outline;
                    }
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Position",
            "Distance from the bottom of the picture.",
            |ui| {
                let mut pos = self.settings.subtitle_style.position_pct;
                if row_slider(ui, p, "sub-pos", &mut pos, 2.0..=26.0, 0.5, " %", one_decimal) {
                    self.settings.subtitle_style.position_pct = pos;
                    if let Some(now) = &mut self.now {
                        now.style.position_pct = pos;
                    }
                    self.touch();
                }
            },
        );

        let loaded = self
            .now
            .as_ref()
            .and_then(|n| n.subtitle_path.as_ref())
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "None".into());
        setting(
            ui,
            p,
            "Loaded subtitle",
            "Which file is loaded, per episode; reopening remembers the choice.",
            |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(loaded).size(10.6).color(p.muted))
                        .wrap(),
                );
            },
        );

        self.advanced(ui, p, "Advanced");
        setting(
            ui,
            p,
            "Timing scale",
            "Stretches cue timing for a badly mastered release.",
            |ui| {
                let mut scale = self.settings.subtitle_style.scale;
                if row_slider(ui, p, "sub-scale", &mut scale, 0.5..=2.0, 0.01, " ×", two_decimals) {
                    self.settings.subtitle_style.scale = scale;
                    if let Some(now) = &mut self.now {
                        now.style.scale = scale;
                    }
                    self.touch();
                }
            },
        );
        setting(
            ui,
            p,
            "Per-show presets",
            "Delay, style, track and filter choices saved for a whole folder.",
            |ui| {
                if skin::ghost(ui, "Manage presets…", p).clicked() {
                    self.dialog = Some(Dialog::Presets);
                }
            },
        );
    }

    fn cat_library(&mut self, ui: &mut Ui, p: &Palette) {
        let folders = self.library.folders.clone();
        let mut add_folder = false;
        let mut rescan = false;
        setting(
            ui,
            p,
            "Indexed folders",
            "Indexing is recursive and repeatable; removing a folder keeps the resume points.",
            |ui| {
                stack(ui, |ui| {
                    rescan = skin::ghost(ui, "Rescan all", p).clicked();
                    add_folder = skin::primary(ui, "Add folder", p).clicked();
                });
            },
        );
        if add_folder {
            self.dlg_text.clear();
            self.dlg_msg = None;
            self.dialog = Some(Dialog::OpenFolder);
        }
        if rescan {
            let folders = self.library.folders.clone();
            let mut added = 0;
            for f in folders {
                added += self.library.ensure_folder(std::path::Path::new(&f));
            }
            self.toast_now(format!("Rescan finished · {added} new"));
            self.touch();
            self.save_library();
        }

        let mut remove: Option<String> = None;
        if folders.is_empty() {
            ui.label(
                egui::RichText::new("No folder yet — add one and everything inside is indexed.")
                    .size(10.6)
                    .color(p.dim),
            );
            ui.add_space(6.0);
        }
        for folder in &folders {
            let path = std::path::Path::new(folder);
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| folder.clone());
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(vec2(15.0, 15.0), Sense::hover());
                skin::icon(ui.painter(), Icon::Folder, icon_rect, p.muted, 1.2);
                ui.add_space(4.0);
                ui.label(egui::RichText::new(name).size(11.6).color(p.text));
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(folder).size(9.8).color(p.dim).monospace(),
                    )
                    .truncate(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if skin::icon_button(ui, Icon::Trash, 22.0, false, "Remove from the library", p)
                        .clicked()
                    {
                        remove = Some(folder.clone());
                    }
                });
            });
            ui.add_space(6.0);
        }
        if let Some(folder) = remove {
            self.library.folders.retain(|f| f != &folder);
            self.library.items.retain(|it| !it.path.starts_with(&folder));
            self.touch();
            self.save_all();
            self.toast_now("Folder removed from the library");
        }
        ui.add_space(4.0);

        setting(
            ui,
            p,
            "Watched at",
            "The same value drives the badge on a card and the watched filters.",
            |ui| {
                let mut threshold = self.settings.watched_threshold as f32;
                if row_slider(ui, p, "watched", &mut threshold, 70.0..=100.0, 1.0, " %", whole) {
                    self.settings.watched_threshold = threshold.round() as u8;
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Offer to relink moved files",
            "A moved file keeps its position and subtitles once you point at it.",
            |ui| {
                let mut relink = self.settings.relink_prompt;
                if skin::toggle(ui, &mut relink, p).changed() {
                    self.settings.relink_prompt = relink;
                    self.touch();
                }
            },
        );

        self.advanced(ui, p, "Advanced");
        setting(
            ui,
            p,
            "Save bookmarks beside media",
            "Otherwise they live in the library database; both survive a rescan.",
            |ui| {
                let mut sidecar = self.settings.save_metadata_beside_media;
                if skin::toggle(ui, &mut sidecar, p).changed() {
                    self.settings.save_metadata_beside_media = sidecar;
                    self.touch();
                }
            },
        );
    }

    fn cat_network(&mut self, ui: &mut Ui, p: &Palette) {
        setting(
            ui,
            p,
            "Open a stream",
            "One dialog, one job. A link, a disc and a capture device never share a form.",
            |ui| {
                stack(ui, |ui| {
                    if skin::ghost(ui, "Network stream…", p).clicked() {
                        self.dlg_text.clear();
                        self.dlg_msg = None;
                        self.dialog = Some(Dialog::OpenUrl);
                    }
                    if skin::ghost(ui, "Disc…", p).clicked() {
                        self.dialog = Some(Dialog::OpenDisc);
                    }
                    if skin::ghost(ui, "Capture device…", p).clicked() {
                        self.dialog = Some(Dialog::Capture);
                    }
                });
            },
        );

        setting(
            ui,
            p,
            "Network timeout",
            "A dead link gives up instead of leaving the interface waiting.",
            |ui| {
                let mut t = self.settings.network_timeout_ms as f32;
                if row_stepper(ui, p, "timeout", &mut t, 1000.0..=60000.0, 1000.0, " ms") {
                    self.settings.network_timeout_ms = t.round() as u32;
                    self.touch();
                }
            },
        );

        setting(
            ui,
            p,
            "Channel lists",
            "Add a public list by URL and refresh it later. Lists open as a queue, not one item.",
            |ui| {
                // Backwards on purpose: this row reads right to left, so the
                // button has to be added before the field to end up at the edge.
                if skin::ghost(ui, "Add", p).clicked() {
                    let url = self.dlg_text.clone();
                    self.open_url(&url);
                    self.dlg_text.clear();
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.dlg_text)
                        .hint_text("https://…/channels.m3u")
                        .desired_width(206.0),
                );
            },
        );
    }

    fn cat_performance(&mut self, ui: &mut Ui, p: &Palette) {
        setting(
            ui,
            p,
            "Seek preview cache",
            "A bounded ring of small frames; a slow decoder skips previews rather than stalling.",
            |ui| {
                let mut cap = self.settings.thumbnail_cap as f32;
                if row_slider(ui, p, "thumb-cap", &mut cap, 10.0..=200.0, 1.0, " frames", whole) {
                    self.settings.thumbnail_cap = cap.round() as u32;
                    self.touch();
                }
            },
        );

        self.advanced(ui, p, "Advanced");
        let version = self
            .libvlc_version
            .clone()
            .unwrap_or_else(|| "libvlc not available".into());
        setting(
            ui,
            p,
            "Player",
            "The engine is the system libvlc. The skin adds no decoding thread of its own.",
            |ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(version).size(10.4).color(p.muted).monospace(),
                    )
                    .wrap(),
                );
            },
        );
    }

    fn cat_recording(&mut self, ui: &mut Ui, p: &Palette) {
        ui.label(
            egui::RichText::new("Recording is not part of this build")
                .size(13.0)
                .color(p.text),
        );
        ui.add_space(3.0);
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "The specification lists recording here but does not require it, so nothing pretends to record. The two choices below are what a later version would use.",
                )
                .size(10.4)
                .color(p.dim),
            )
            .wrap(),
        );
        ui.add_space(16.0);

        setting(
            ui,
            p,
            "Save recordings to",
            "A folder for a future recording command; nothing is written to it today.",
            |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings.recording_dir)
                        .hint_text("~/Videos/Recordings")
                        .desired_width(240.0),
                );
            },
        );

        setting(
            ui,
            p,
            "Container",
            "Matroska survives an interrupted recording; MP4 needs a clean stop.",
            |ui| {
                egui::ComboBox::from_id_salt("rec-format")
                    .selected_text(&self.settings.recording_format)
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for opt in ["Matroska (.mkv)", "MPEG-4 (.mp4)"] {
                            ui.selectable_value(
                                &mut self.settings.recording_format,
                                opt.to_string(),
                                opt,
                            );
                        }
                    });
            },
        );
    }

    fn cat_extensions(&mut self, ui: &mut Ui, p: &Palette) {
        let addons = self.settings.addons.clone();
        setting_block(
            ui,
            p,
            "Add-ons",
            "Installable additions you choose. Each one can be switched off without touching VLC.",
            |ui| {
                if addons.is_empty() {
                    ui.label(egui::RichText::new("None installed.").size(10.5).color(p.dim));
                }
                for addon in &addons {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(addon).size(11.4).color(p.text));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            skin::badge(ui, "ADD-ON", p);
                        });
                    });
                }
            },
        );

        // VLC keeps its plugins somewhere different on each system, so the
        // first folder that exists is the one counted.
        let found = crate::platform::vlc_plugins_dirs()
            .into_iter()
            .find(|p| p.is_dir());
        let plugin_dir = found.unwrap_or_else(|| std::path::PathBuf::from("/usr/lib/vlc/plugins"));
        let mut rows: Vec<(String, usize)> = Vec::new();
        if plugin_dir.is_dir() {
            if let Ok(read) = std::fs::read_dir(&plugin_dir) {
                for entry in read.flatten() {
                    if entry.path().is_dir() {
                        let count = std::fs::read_dir(entry.path())
                            .map(|r| r.flatten().count())
                            .unwrap_or(0);
                        rows.push((entry.file_name().to_string_lossy().to_string(), count));
                    }
                }
            }
        }
        rows.sort();
        let summary = if plugin_dir.is_dir() {
            format!(
                "{} categories · {} plugins · {}",
                rows.len(),
                rows.iter().map(|(_, c)| c).sum::<usize>(),
                plugin_dir.display()
            )
        } else {
            "No system plugin directory found on this machine.".to_string()
        };
        setting_block(
            ui,
            p,
            "Plugins",
            &summary,
            |ui| {
                for (name, count) in &rows {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(name)
                                .size(10.4)
                                .color(p.muted)
                                .monospace(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            skin::badge(ui, "READ-ONLY", p);
                            ui.label(
                                egui::RichText::new(format!("{count} plugins"))
                                    .size(9.6)
                                    .color(p.dim),
                            );
                        });
                    });
                    ui.add_space(4.0);
                }
            },
        );
    }

    fn advanced(&mut self, ui: &mut Ui, p: &Palette, title: &str) {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(title.to_uppercase())
                    .size(9.5)
                    .color(p.faint_or(p))
                    .strong(),
            );
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter().line_segment(
                [rect.left_center(), rect.right_center()],
                Stroke::new(1.0, p.line_soft),
            );
        });
        ui.add_space(2.0);
    }
}

// ---------------------------------------------------------------------------
// One setting, one row
// ---------------------------------------------------------------------------

/// The name and its one-sentence explanation on the left, the control in a
/// column on the right. The sentence is always on screen: the point of the
/// settings surface is that nothing has to be discovered.
fn setting(ui: &mut Ui, p: &Palette, label: &str, help: &str, control: impl FnOnce(&mut Ui)) {
    let full = ui.available_width().max(340.0);
    let right_w = (full * 0.40).clamp(210.0, 320.0);
    let left_w = (full - right_w - 28.0).max(150.0);
    let picked = ui
        .data(|d| d.get_temp::<Option<String>>(egui::Id::new("settings_highlight")))
        .flatten();
    let highlighted = picked.as_deref() == Some(label);
    let row = ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            vec2(left_w, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(label)
                            .size(12.4)
                            .color(if highlighted { p.accent } else { p.text }),
                    )
                    .selectable(false),
                );
                if !help.is_empty() {
                    ui.add_space(1.0);
                    ui.add(
                        egui::Label::new(egui::RichText::new(help).size(10.2).color(p.dim))
                            .wrap(),
                    );
                }
            },
        );
        ui.add_space(28.0);
        // The control column ends at one edge, whatever the control is: a
        // switch, a menu, a slider with its number, or a row of choices.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.set_min_width(right_w);
            ui.spacing_mut().interact_size.y = 24.0;
            control(ui);
        });
    });
    if highlighted {
        // A quiet outline instead of a block of colour: the row is easy to
        // find again without the page shouting.
        ui.painter().rect_stroke(
            row.response.rect.expand2(vec2(10.0, 6.0)),
            CornerRadius::same(8),
            Stroke::new(1.2, p.accent),
            StrokeKind::Outside,
        );
    }
    ui.add_space(9.0);
    row_rule(ui, p);
    ui.add_space(9.0);
}

/// Two or three controls stacked on the right of one row.
fn stack(ui: &mut Ui, control: impl FnOnce(&mut Ui)) {
    ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
        ui.spacing_mut().item_spacing.y = 5.0;
        control(ui);
    });
}

/// A slider that draws its own track and writes its value at the row's right
/// edge. Nothing here depends on which way the surrounding layout reads.
fn row_slider(
    ui: &mut Ui,
    p: &Palette,
    key: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    suffix: &str,
    show: fn(f32) -> String,
) -> bool {
    let (min, max) = (*range.start(), *range.end());
    let width = ui.available_width().clamp(120.0, 320.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 28.0), Sense::hover());
    let value_w = 62.0;
    let track = Rect::from_min_max(
        pos2(rect.left(), rect.center().y - 7.0),
        pos2(rect.right() - value_w - 10.0, rect.center().y + 7.0),
    );
    let resp = ui.interact(track, ui.id().with(("slider", key)), Sense::click_and_drag());
    let mut changed = false;
    if resp.clicked() || resp.dragged() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let f = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            let raw = min + (max - min) * f;
            let snapped = if step > 0.0 { (raw / step).round() * step } else { raw };
            let next = snapped.clamp(min, max);
            if (next - *value).abs() > 1e-6 {
                *value = next;
                changed = true;
            }
        }
    }

    let rail = Rect::from_center_size(track.center(), vec2(track.width(), 4.0));
    ui.painter().rect_filled(rail, CornerRadius::same(2), p.line);
    let frac = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    if frac > 0.0 {
        ui.painter().rect_filled(
            Rect::from_min_size(rail.min, vec2(rail.width() * frac, 4.0)),
            CornerRadius::same(2),
            p.accent,
        );
    }
    let live = resp.hovered() || resp.dragged();
    let knob = pos2(rail.left() + rail.width() * frac, rail.center().y);
    if live {
        ui.painter().circle_filled(knob, 9.0, p.accent_soft);
    }
    ui.painter().circle_filled(knob, 5.0, p.accent);
    ui.painter().text(
        pos2(rect.right(), rect.center().y),
        Align2::RIGHT_CENTER,
        format!("{}{}", show(*value), suffix),
        skin::font(11.3),
        if live { p.text } else { p.muted },
    );
    changed
}

/// A stepper for numbers that have to be hit exactly: value in a box, minus
/// and plus beside it, all flush to the row's right edge.
fn row_stepper(
    ui: &mut Ui,
    p: &Palette,
    key: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    suffix: &str,
) -> bool {
    let (min, max) = (*range.start(), *range.end());
    let width = ui.available_width().clamp(160.0, 320.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 28.0), Sense::hover());
    let btn = 26.0;
    let field_w = 100.0;
    let plus = Rect::from_center_size(
        pos2(rect.right() - btn * 0.5, rect.center().y),
        Vec2::splat(btn),
    );
    let minus = Rect::from_center_size(
        pos2(plus.left() - 6.0 - btn * 0.5, rect.center().y),
        Vec2::splat(btn),
    );
    let field = Rect::from_min_max(
        pos2(minus.left() - 8.0 - field_w, rect.center().y - 13.0),
        pos2(minus.left() - 8.0, rect.center().y + 13.0),
    );

    let plus_resp = ui.interact(plus, ui.id().with(("plus", key)), Sense::click());
    let minus_resp = ui.interact(minus, ui.id().with(("minus", key)), Sense::click());
    let mut changed = false;
    if plus_resp.clicked() {
        *value = (*value + step).clamp(min, max);
        changed = true;
    }
    if minus_resp.clicked() {
        *value = (*value - step).clamp(min, max);
        changed = true;
    }

    for (r, resp, glyph) in [
        (plus, plus_resp, "+"),
        (minus, minus_resp, "−"),
    ] {
        let fill = if resp.hovered() { p.hover } else { p.panel2 };
        ui.painter().rect_filled(r, CornerRadius::same(6), fill);
        ui.painter().rect_stroke(
            r,
            CornerRadius::same(6),
            Stroke::new(1.0, p.line),
            StrokeKind::Inside,
        );
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            glyph,
            skin::font(14.0),
            if resp.hovered() { p.text } else { p.muted },
        );
    }

    ui.painter().rect_filled(field, CornerRadius::same(6), p.panel2);
    ui.painter().rect_stroke(
        field,
        CornerRadius::same(6),
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    ui.painter().text(
        field.center(),
        Align2::CENTER_CENTER,
        format!("{}{}", whole(*value), suffix),
        skin::font(11.6),
        p.text,
    );
    changed
}

fn whole(v: f32) -> String {
    format!("{:.0}", v)
}

fn one_decimal(v: f32) -> String {
    format!("{:.1}", v)
}

fn two_decimals(v: f32) -> String {
    format!("{:.2}", v)
}

fn percent_scale(v: f32) -> String {
    format!("{:.0}", v * 100.0)
}

/// The colours as dots. Compact enough to sit inside a row without wrapping,
/// and every one is its own target with the name on hover.
fn swatches(ui: &mut Ui, p: &Palette, current: [u8; 3]) -> Option<[u8; 3]> {
    let mut picked = None;
    // Reversed, because this row reads right to left: the first added dot
    // takes the right edge.
    for (name, rgb) in skin::ACCENTS.iter().rev() {
        let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 28.0), Sense::click());
        let centre = rect.center();
        if *rgb == current {
            ui.painter().circle_stroke(centre, 10.5, Stroke::new(1.4, p.text));
        } else if resp.hovered() {
            ui.painter().circle_filled(centre, 12.0, p.hover);
        }
        ui.painter()
            .circle_filled(centre, 7.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
        let tip = (*name).to_string();
        let resp = resp.on_hover_ui(|ui| {
            ui.label(egui::RichText::new(tip).size(11.5));
        });
        if resp.clicked() {
            picked = Some(*rgb);
        }
    }
    picked
}

/// A row whose control needs the whole width: swatches, lists, several
/// buttons. The name and sentence still sit together at the top.
fn setting_block(ui: &mut Ui, p: &Palette, label: &str, help: &str, control: impl FnOnce(&mut Ui)) {
    ui.add(
        egui::Label::new(egui::RichText::new(label).size(12.4).color(p.text)).selectable(false),
    );
    if !help.is_empty() {
        ui.add_space(1.0);
        ui.add(
            egui::Label::new(egui::RichText::new(help).size(10.2).color(p.dim)).wrap(),
        );
    }
    ui.add_space(8.0);
    control(ui);
    ui.add_space(9.0);
    row_rule(ui, p);
    ui.add_space(9.0);
}

fn row_rule(ui: &mut Ui, p: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(p.line_soft.r(), p.line_soft.g(), p.line_soft.b(), 170),
        ),
    );
}


impl Palette {
    pub fn faint_or(&self, _p: &Palette) -> Color32 {
        self.dim
    }
}


/// Names and synonyms used by the settings search across categories.
fn setting_index(cat: SettingsCategory) -> Vec<(&'static str, &'static str)> {
    match cat {
        SettingsCategory::General => vec![
            ("Default player", "standard default media association xdg mime double click open files video music"),
            ("Start page", "startup home opening"),
            ("Check for updates", "version package"),
            ("Show the welcome tour again", "onboarding first run"),
            ("Save settings on change", "persistence atomic"),
        ],
        SettingsCategory::Interface => vec![
            ("Theme", "colour scheme dark light"),
            ("Accent", "colour highlight"),
            ("Interface scale", "size zoom dpi"),
            ("Reduce motion", "animation accessibility"),
            ("Remember the window size", "window geometry"),
            ("Control bar", "customise layout buttons"),
        ],
        SettingsCategory::Privacy => vec![
            ("Allow online subtitle lookup", "network opensubtitles"),
            ("Telemetry", "tracking"),
            ("Local by default", "storage offline"),
        ],
        SettingsCategory::Controls => vec![
            ("Keyboard shortcuts", "keys bindings"),
            ("Escape", "close cancel"),
            ("Control bar", "buttons layout"),
            ("Layout profiles", "profiles export"),
        ],
        SettingsCategory::Playback => vec![
            ("Resume where you stopped", "continue watching"),
            ("Play the queue automatically", "autoplay next queue countdown"),
            ("Default volume", "sound level"),
            ("End of an item", "prompt card countdown"),
        ],
        SettingsCategory::Video => vec![
            ("Hardware decoding", "gpu vaapi acceleration"),
            ("Frame previews when seeking", "thumbnail"),
            ("Full-screen bands", "fullscreen auto hide"),
            ("Picture adjustments", "zoom flip rotate"),
        ],
        SettingsCategory::Audio => vec![
            ("Output device", "speakers hdmi pulse"),
            ("Sound presets", "equalizer bass voice"),
            ("Mute keeps decoding", "silence"),
        ],
        SettingsCategory::Subtitles => vec![
            ("Default subtitle language", "language track"),
            ("Subtitle filename matching", "match precision"),
            ("Subtitle delay", "timing sync offset"),
            ("Subtitle size", "font text"),
            ("Outline", "border stroke"),
            ("Position", "placement bottom"),
            ("Loaded subtitle", "srt file"),
            ("Timing scale", "speed stretch"),
            ("Per-show presets", "preset folder"),
        ],
        SettingsCategory::Library => vec![
            ("Indexed folders", "watch sources"),
            ("Watched at", "percent"),
            ("Offer to relink moved files", "missing path"),
            ("Save bookmarks beside media", "chapter sidecar"),
        ],
        SettingsCategory::Network => vec![
            ("Open a stream", "url http"),
            ("Network timeout", "buffer wait"),
            ("Channel lists", "iptv m3u"),
        ],
        SettingsCategory::Performance => vec![
            ("Seek preview cache", "thumbnail memory"),
            ("Player", "libvlc version"),
        ],
        SettingsCategory::Recording => vec![
            ("Save recordings to", "output folder"),
            ("Container", "mkv mp4"),
        ],
        SettingsCategory::Extensions => vec![
            ("Add-ons", "extensions installable"),
            ("Plugins", "modules read only"),
        ],
    }
}
