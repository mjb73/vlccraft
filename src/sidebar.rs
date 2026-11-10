//! The four-tab playback sidebar and the queue drawer.

use crate::app::{App, Dialog, View};
use crate::model::{format_time, Bookmark};
use crate::skin::{self, Icon, Palette};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

impl App {
    pub fn sidebar_ui(&mut self, ui: &mut Ui, p: &Palette) {
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    skin::section(ui, "While you watch", p);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if skin::icon_button(ui, Icon::Close, 26.0, false, "Close sidebar · Esc", p).clicked() {
                            self.sidebar_open = false;
                            self.settings.sidebar_open = false;
                            self.touch();
                        }
                    });
                });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    let labels = ["Subtitles", "Audio", "Video", "Playback"];
                    for (i, label) in labels.iter().enumerate() {
                        if skin::text_tab(ui, label, self.sidebar_tab == i, p).clicked() {
                            self.sidebar_tab = i;
                            self.settings.sidebar_tab = i;
                            self.touch();
                        }
                    }
                });
                skin::divider(ui, p);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        match self.sidebar_tab {
                            0 => self.subtitle_tab(ui, p),
                            1 => self.audio_tab(ui, p),
                            2 => self.video_tab(ui, p),
                            _ => self.playback_tab(ui, p),
                        }
                        ui.add_space(24.0);
                    });
            });
    }

    // -----------------------------------------------------------------------
    // Subtitles
    // -----------------------------------------------------------------------

    fn subtitle_tab(&mut self, ui: &mut Ui, p: &Palette) {
        let (sub_name, cue_count, style, preset) = match &self.now {
            Some(now) => (
                now.subtitle_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|s| s.to_string_lossy().to_string()),
                now.cues.len(),
                now.style.clone(),
                now.preset_name.clone(),
            ),
            None => (None, 0, self.settings.subtitle_style.clone(), None),
        };

        egui::Frame::new()
            .fill(p.panel2)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(egui::Margin::same(11))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.painter().rect_filled(
                        Rect::from_min_size(ui.cursor().min, vec2(3.0, 32.0)),
                        CornerRadius::same(2),
                        if sub_name.is_some() { p.good } else { p.line },
                    );
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(sub_name.clone().unwrap_or_else(|| "No subtitle file".into()))
                                .size(12.5)
                                .color(if sub_name.is_some() { p.text } else { p.dim }),
                        );
                        ui.label(
                            egui::RichText::new(if sub_name.is_some() {
                                format!("{cue_count} cues · drawn by the skin")
                            } else {
                                "Matching is by filename; add one below".into()
                            })
                            .size(10.5)
                            .color(p.dim),
                        );
                    });
                });
            });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if skin::chip(ui, "Off", false, p).clicked() {
                self.select_subtitle(None);
            }
            let names = self
                .now
                .as_ref()
                .map(|n| n.subtitle_files.clone())
                .unwrap_or_default();
            for file in names.iter().take(3) {
                let label = file
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let active = self
                    .now
                    .as_ref()
                    .and_then(|n| n.subtitle_path.as_ref())
                    == Some(file);
                if skin::chip(ui, &label, active, p).clicked() {
                    self.select_subtitle(Some(file.clone()));
                }
            }
        });

        ui.add_space(10.0);
        skin::section(ui, "Add subtitles", p);
        ui.label(
            egui::RichText::new("One action, three scopes — no guessing which folder it means.")
                .size(10.5)
                .color(p.dim),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for (id, label) in [("file", "This file"), ("folder", "This folder"), ("playlist", "This playlist")] {
                if skin::chip(ui, label, self.subtitle_scope == id, p).clicked() {
                    self.subtitle_scope = id.into();
                    self.dialog = Some(Dialog::SubtitleBatch);
                    self.dlg_msg = None;
                }
            }
        });
        if preset.is_some() {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("Preset “{}” is applied", preset.unwrap_or_default()))
                    .size(10.5)
                    .color(p.accent),
            );
        }

        ui.add_space(12.0);
        skin::section(ui, "Timing and style", p);
        ui.add_space(4.0);
        let mut delay = style.delay_ms;
        if slider_row(ui, "Delay", &mut delay, -1500.0..=1500.0, "ms", p) {
            if let Some(now) = &mut self.now {
                now.style.delay_ms = delay;
            }
            self.touch();
        }
        ui.horizontal(|ui| {
            if skin::chip(ui, "−50 ms", false, p).clicked() {
                if let Some(now) = &mut self.now {
                    now.style.delay_ms -= 50;
                }
                self.touch();
            }
            if skin::chip(ui, "+50 ms", false, p).clicked() {
                if let Some(now) = &mut self.now {
                    now.style.delay_ms += 50;
                }
                self.touch();
            }
            if skin::chip(ui, "Reset", false, p).clicked() {
                if let Some(now) = &mut self.now {
                    now.style.delay_ms = self.settings.subtitle_style.delay_ms;
                }
                self.touch();
            }
        });

        let mut speed = style.scale;
        if slider_row_f32(ui, "Speed", &mut speed, 0.5..=2.0, "×", p) {
            if let Some(now) = &mut self.now {
                now.style.scale = speed;
            }
            self.touch();
        }
        let mut size = style.size_pct;
        if slider_row_f32(ui, "Size", &mut size, 2.0..=8.0, "%", p) {
            if let Some(now) = &mut self.now {
                now.style.size_pct = size;
            }
            self.touch();
        }
        let mut outline = style.outline;
        if slider_row_f32(ui, "Outline", &mut outline, 0.0..=3.0, "", p) {
            if let Some(now) = &mut self.now {
                now.style.outline = outline;
            }
            self.touch();
        }
        let mut pos = style.position_pct;
        if slider_row_f32(ui, "Position", &mut pos, 2.0..=26.0, "%", p) {
            if let Some(now) = &mut self.now {
                now.style.position_pct = pos;
            }
            self.touch();
        }
        let mut bg = style.background;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Solid backdrop").size(11.5).color(p.muted));
            if skin::toggle(ui, &mut bg, p).changed() {
                if let Some(now) = &mut self.now {
                    now.style.background = bg;
                }
                self.touch();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("A box behind the words").size(10.0).color(p.dim));
            });
        });

        ui.add_space(10.0);
        skin::section(ui, "Cue timeline", p);
        ui.label(
            egui::RichText::new("Click a cue to jump. Drag a cue left or right to move it alone.")
                .size(10.0)
                .color(p.dim),
        );
        ui.add_space(4.0);
        let cues = self.now.as_ref().map(|n| n.cues.clone()).unwrap_or_default();
        let duration = self.duration_ms().max(1);
        let current = self.time_ms();
        let mut seek = None;
        let mut shift: Option<(usize, i64)> = None;
        egui::Frame::new()
            .fill(p.video_bg)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(204.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if cues.is_empty() {
                            ui.label(
                                egui::RichText::new("No cues loaded for this item.")
                                    .size(11.0)
                                    .color(p.dim),
                            );
                        }
                        for (i, cue) in cues.iter().enumerate() {
                            let active = current >= cue.start_ms && current <= cue.end_ms;
                            let (rect, resp) = ui.allocate_exact_size(
                                vec2(ui.available_width(), 38.0),
                                Sense::click_and_drag(),
                            );
                            if active {
                                ui.painter().rect_filled(rect, CornerRadius::same(5), p.accent_soft);
                            } else if resp.hovered() {
                                ui.painter().rect_filled(rect, CornerRadius::same(5), p.hover);
                            }
                            if resp.clicked() {
                                seek = Some(cue.start_ms);
                            }
                            if resp.dragged() {
                                let delta = resp.drag_delta().x;
                                let ms = (delta / rect.width() * duration as f32) as i64;
                                if ms.abs() > 2 {
                                    shift = Some((i, ms));
                                }
                            }
                            ui.painter().text(
                                pos2(rect.left() + 8.0, rect.top() + 12.0),
                                Align2::LEFT_CENTER,
                                format_time(cue.start_ms),
                                skin::mono(8.5),
                                if active { p.accent } else { p.dim },
                            );
                            let max_chars = (((rect.width() - 56.0) / 5.4) as usize).max(12);
                            let preview = truncate(&cue.text.replace('\n', " "), max_chars);
                            ui.painter().text(
                                pos2(rect.left() + 46.0, rect.top() + 12.0),
                                Align2::LEFT_CENTER,
                                preview,
                                skin::font(9.5),
                                if active { p.text } else { p.muted },
                            );
                            let lane = Rect::from_min_max(
                                pos2(rect.left() + 46.0, rect.bottom() - 11.0),
                                pos2(rect.right() - 8.0, rect.bottom() - 8.0),
                            );
                            ui.painter().rect_filled(lane, CornerRadius::same(2), p.line_soft);
                            let a = (cue.start_ms as f32 / duration as f32).clamp(0.0, 1.0);
                            let b = (cue.end_ms as f32 / duration as f32).clamp(0.0, 1.0);
                            let seg = Rect::from_min_max(
                                pos2(lane.left() + lane.width() * a, lane.top()),
                                pos2(lane.left() + lane.width() * (b.max(a + 0.004)), lane.bottom()),
                            );
                            ui.painter().rect_filled(
                                seg,
                                CornerRadius::same(2),
                                if active { p.accent } else { p.dim },
                            );
                        }
                    });
            });
        if let Some(t) = seek {
            self.seek_to(t);
        }
        if let Some((i, delta)) = shift {
            if let Some(now) = &mut self.now {
                if let Some(cue) = now.cues.get_mut(i) {
                    cue.start_ms = (cue.start_ms + delta).max(0);
                    cue.end_ms = (cue.end_ms + delta).max(cue.start_ms + 200);
                }
            }
            self.touch();
        }

        ui.add_space(10.0);
        skin::divider(ui, p);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Keep this for every episode")
                    .size(11.0)
                    .color(p.muted),
            );
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.preset_save_name)
                    .hint_text("Preset name")
                    .desired_width(150.0),
            );
            if skin::primary(ui, "Save preset", p).clicked() {
                let name = if self.preset_save_name.trim().is_empty() {
                    self.now
                        .as_ref()
                        .map(|n| n.show.clone())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "Show preset".into())
                } else {
                    self.preset_save_name.trim().to_string()
                };
                self.save_preset_for_current(name);
                self.preset_save_name.clear();
            }
        });
        if skin::ghost(ui, "Manage presets", p).clicked() {
            self.dialog = Some(Dialog::Presets);
        }

        ui.add_space(10.0);
        skin::section(ui, "Online subtitles", p);
        ui.label(
            egui::RichText::new(
                "OpenSubtitles needs your own API key. Without one, playback is untouched and this says why.",
            )
            .size(10.0)
            .color(p.dim),
        );
        if skin::ghost(ui, "Find subtitles online…", p).clicked() {
            self.dlg_msg = None;
            self.dialog = Some(Dialog::SubtitleSearch);
        }
    }

    // -----------------------------------------------------------------------
    // Audio
    // -----------------------------------------------------------------------

    fn audio_tab(&mut self, ui: &mut Ui, p: &Palette) {
        skin::section(ui, "Level", p);
        ui.horizontal(|ui| {
            let mut v = self.settings.volume as i32;
            let resp = ui.add(
                egui::Slider::new(&mut v, 0..=125)
                    .show_value(false)
                    .trailing_fill(true),
            );
            if resp.changed() {
                self.set_volume(v as u8);
            }
            ui.label(
                egui::RichText::new(format!("{}%", self.settings.volume))
                    .size(11.0)
                    .color(p.muted),
            );
            let mut muted = self.settings.muted;
            if ui.checkbox(&mut muted, "Mute").changed() {
                self.toggle_mute();
            }
        });

        ui.add_space(10.0);
        skin::section(ui, "Tracks in this item", p);
        let tracks = self
            .now
            .as_ref()
            .map(|n| n.audio_tracks.clone())
            .unwrap_or_default();
        if tracks.is_empty() {
            ui.label(egui::RichText::new("No audio tracks reported yet.").size(11.0).color(p.dim));
        }
        for (i, (_, name)) in tracks.iter().enumerate() {
            let active = self.now.as_ref().map(|n| n.audio_track_index == i).unwrap_or(false);
            if ui.radio(active, name).clicked() {
                if let Some(now) = &mut self.now {
                    now.audio_track_index = i;
                }
                if let (Some(engine), Some(now)) = (&self.engine, &self.now) {
                    if let Some((id, _)) = now.audio_tracks.get(i) {
                        engine.set_audio_track(*id);
                    }
                }
                self.toast_now(format!("Audio track: {name}"));
            }
        }

        ui.add_space(12.0);
        skin::section(ui, "Output device", p);
        let options = self.audio_choices();
        for (label, value) in options {
            let active = self.settings.audio_output == value;
            if ui.radio(active, &label).clicked() {
                self.set_audio_choice(&value);
            }
        }
        ui.label(
            egui::RichText::new("Changing the device reopens the audio path; the picture does not restart.")
                .size(10.0)
                .color(p.dim),
        );

        ui.add_space(12.0);
        skin::section(ui, "Sound presets", p);
        ui.label(
            egui::RichText::new("VLC's own equalizer presets, applied live.")
                .size(10.0)
                .color(p.dim),
        );
        ui.add_space(4.0);
        let presets = self
            .engine
            .as_ref()
            .map(|e| e.equalizer_presets())
            .unwrap_or_default();
        let current = self.now.as_ref().and_then(|n| n.equalizer.clone());
        if presets.is_empty() {
            ui.label(
                egui::RichText::new("This libvlc build reports no equalizer presets.")
                    .size(10.5)
                    .color(p.dim),
            );
        } else {
            ui.horizontal_wrapped(|ui| {
                for (i, name) in presets.iter().enumerate().take(8) {
                    let active = current.as_deref() == Some(name.as_str());
                    let resp = skin::chip(ui, name, active, p);
                    if resp.clicked() {
                        if let Some(engine) = &self.engine {
                            engine.set_equalizer(Some(i));
                        }
                        if let Some(now) = &mut self.now {
                            now.equalizer = Some(name.clone());
                        }
                        self.toast_now(format!("Sound preset: {name}"));
                    }
                }
            });
            if skin::chip(ui, "Flat (off)", current.is_none(), p).clicked() {
                if let Some(engine) = &self.engine {
                    engine.set_equalizer(None);
                }
                if let Some(now) = &mut self.now {
                    now.equalizer = None;
                }
                self.toast_now("Equalizer off");
            }
        }
        ui.label(
            egui::RichText::new("Applied by libvlc to the live player; no restart needed.")
                .size(10.0)
                .color(p.dim),
        );
    }

    // -----------------------------------------------------------------------
    // Video
    // -----------------------------------------------------------------------

    fn video_tab(&mut self, ui: &mut Ui, p: &Palette) {
        let (mut rotate, mut flip_h, mut flip_v, mut zoom, aspect) = self
            .now
            .as_ref()
            .map(|n| (n.rotate, n.flip_h, n.flip_v, n.zoom, n.aspect.clone()))
            .unwrap_or((0, false, false, 1.0, "Auto".into()));

        skin::section(ui, "Picture", p);
        let mut zoom_changed = false;
        if slider_row_f32(ui, "Zoom", &mut zoom, 1.0..=2.0, "×", p) {
            zoom_changed = true;
        }
        ui.horizontal(|ui| {
            if skin::chip(ui, "Rotate 90°", false, p).clicked() {
                rotate = (rotate + 90) % 360;
            }
            if skin::chip(ui, if flip_h { "Unflip" } else { "Flip across" }, flip_h, p).clicked() {
                flip_h = !flip_h;
            }
            if skin::chip(ui, if flip_v { "Unflip ↓" } else { "Flip down" }, flip_v, p).clicked() {
                flip_v = !flip_v;
            }
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Aspect").size(11.5).color(p.muted));
            let mut selected = aspect.clone();
            egui::ComboBox::from_id_salt("aspect")
                .selected_text(&aspect)
                .show_ui(ui, |ui| {
                    for opt in ["Auto", "16:9", "4:3", "Fill"] {
                        if ui.selectable_value(&mut selected, opt.to_string(), opt).clicked() {
                            self.toast_now(format!("Aspect: {opt}"));
                        }
                    }
                });
            if selected != aspect {
                if let Some(now) = &mut self.now {
                    now.aspect = selected;
                }
            }
        });
        ui.label(
            egui::RichText::new("Everything here redraws the live picture — no pipeline rebuild.")
                .size(10.0)
                .color(p.dim),
        );

        if let Some(now) = &mut self.now {
            now.rotate = rotate;
            now.flip_h = flip_h;
            now.flip_v = flip_v;
            if zoom_changed {
                now.zoom = zoom;
            }
        }
        if zoom_changed {
            self.touch();
        }

        ui.add_space(12.0);
        skin::section(ui, "Reset", p);
        if skin::ghost(ui, "Restore the original picture", p).clicked() {
            if let Some(now) = &mut self.now {
                now.rotate = 0;
                now.flip_h = false;
                now.flip_v = false;
                now.zoom = 1.0;
                now.aspect = "Auto".into();
            }
            self.toast_now("Picture reset");
        }

        ui.add_space(12.0);
        skin::divider(ui, p);
        ui.label(
            egui::RichText::new("Hardware decoding, tone mapping and output colour live in Settings → Video.")
                .size(10.0)
                .color(p.dim),
        );
        if skin::ghost(ui, "Open video settings", p).clicked() {
            self.settings_cat = crate::app::SettingsCategory::Video;
            self.view = View::Settings;
        }
    }

    // -----------------------------------------------------------------------
    // Playback
    // -----------------------------------------------------------------------

    fn playback_tab(&mut self, ui: &mut Ui, p: &Palette) {
        skin::section(ui, "Speed", p);
        let rate = self.engine.as_ref().map(|e| e.rate()).unwrap_or(1.0);
        ui.horizontal(|ui| {
            for r in [0.5f32, 1.0, 1.25, 1.5, 2.0] {
                if skin::chip(ui, &format!("{r}×"), (rate - r).abs() < 0.02, p).clicked() {
                    if let Some(engine) = &self.engine {
                        engine.set_rate(r);
                    }
                    self.toast_now(format!("Speed {r}×"));
                }
            }
        });
        let mut speed = rate;
        if slider_row_f32(ui, "Fine", &mut speed, 0.25..=3.0, "×", p) {
            if let Some(engine) = &self.engine {
                engine.set_rate(speed);
            }
        }
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Reverse").size(11.5).color(p.muted));
            let mut reverse = rate < 0.0;
            if skin::toggle(ui, &mut reverse, p).changed() {
                if let Some(engine) = &self.engine {
                    let target = if reverse { -1.0 } else { 1.0 };
                    engine.set_rate(target);
                    let real = engine.rate();
                    if reverse && real > 0.0 {
                        self.toast_now("This file cannot play backwards");
                    } else {
                        self.toast_now(if reverse { "Reverse playback" } else { "Forward playback" });
                    }
                } else {
                    self.toast_now("Reverse needs live VLC playback");
                }
            }
        });

        ui.add_space(12.0);
        skin::section(ui, "Sleep timer", p);
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.sleep_minutes)
                    .range(1..=240)
                    .suffix(" min"),
            );
            if skin::chip(ui, "Start", self.sleep_until.is_some(), p).clicked() {
                if self.sleep_until.is_some() {
                    self.sleep_until = None;
                    self.toast_now("Sleep timer cancelled");
                } else {
                    self.sleep_until = Some(self.time + self.sleep_minutes as f64 * 60.0);
                    self.toast_now(format!("Sleeping in {} min", self.sleep_minutes));
                }
            }
            if let Some(at) = self.sleep_until {
                ui.label(
                    egui::RichText::new(format!("{} min left", ((at - self.time) / 60.0).max(0.0).ceil()))
                        .size(10.5)
                        .color(p.accent),
                );
            }
        });

        ui.add_space(12.0);
        skin::section(ui, "Timeline", p);
        let mut show_cues = self.settings.show_buffered;
        if ui.checkbox(&mut show_cues, "Show subtitle cues on the seek bar").changed() {
            self.settings.show_buffered = show_cues;
            self.touch();
        }
        ui.checkbox(&mut self.settings.show_thumbnails, "Frame preview when seeking");
        ui.checkbox(&mut self.settings.auto_advance, "Play the next queued item on its own (no message)");

        ui.add_space(12.0);
        skin::section(ui, "Bookmarks", p);
        let (path, bookmarks, chapters) = match &self.now {
            Some(now) => (
                now.path.clone(),
                self.library
                    .get(&now.path)
                    .map(|i| i.bookmarks.clone())
                    .unwrap_or_default(),
                now.chapters.clone(),
            ),
            None => (String::new(), vec![], vec![]),
        };

        let duration = self.duration_ms().max(1);
        let time = self.time_ms();
        let strip = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
        let strip_rect = strip.0;
        let painter = ui.painter_at(strip_rect);
        painter.rect_filled(strip_rect, CornerRadius::same(6), p.video_bg);
        let lane = Rect::from_min_max(
            pos2(strip_rect.left() + 10.0, strip_rect.center().y - 1.5),
            pos2(strip_rect.right() - 10.0, strip_rect.center().y + 1.5),
        );
        painter.rect_filled(lane, CornerRadius::same(2), p.line_soft);
        for ch in &chapters {
            let f = (ch.at_ms as f32 / duration as f32).clamp(0.0, 1.0);
            let x = lane.left() + lane.width() * f;
            painter.line_segment(
                [pos2(x, lane.top() - 5.0), pos2(x, lane.bottom() + 5.0)],
                Stroke::new(1.0, p.dim),
            );
        }
        for (i, bm) in bookmarks.iter().enumerate() {
            let f = (bm.at_ms as f32 / duration as f32).clamp(0.0, 1.0);
            let x = lane.left() + lane.width() * f;
            let mrect = Rect::from_center_size(pos2(x, lane.center().y), vec2(8.0, 16.0));
            painter.rect_filled(mrect, CornerRadius::same(2), p.accent);
            let resp = ui.interact(mrect.expand(4.0), Id::new(("sb-bm", i)), Sense::click_and_drag());
            if resp.clicked() {
                self.seek_to(bm.at_ms);
            }
            if resp.dragged() {
                if let Some(pos) = resp.interact_pointer_pos() {
                    let f2 = ((pos.x - lane.left()) / lane.width()).clamp(0.0, 1.0);
                    let new_at = (f2 * duration as f32) as i64;
                    if let Some(item) = self.library.get_mut(&path) {
                        if let Some(b) = item.bookmarks.get_mut(i) {
                            b.at_ms = new_at;
                        }
                    }
                }
            }
        }
        if let Some(pos) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            if strip_rect.contains(pos) {
                let f = ((pos.x - lane.left()) / lane.width()).clamp(0.0, 1.0);
                painter.text(
                    pos2(pos.x.clamp(strip_rect.left() + 20.0, strip_rect.right() - 20.0), strip_rect.top() + 7.0),
                    Align2::CENTER_CENTER,
                    format_time((f * duration as f32) as i64),
                    skin::mono(9.5),
                    p.text,
                );
            }
        }

        if skin::ghost(ui, "Add bookmark here", p).clicked() {
            self.add_bookmark();
        }
        ui.add_space(4.0);
        let mut delete = None;
        let mut nudge = None;
        let mut seek = None;
        for (i, bm) in bookmarks.iter().enumerate() {
            ui.horizontal(|ui| {
                if skin::chip(ui, &format_time(bm.at_ms), false, p).clicked() {
                    seek = Some(bm.at_ms);
                }
                let mut commit: Option<String> = None;
                if let Some((idx, text)) = self.rename_bookmark.as_mut() {
                    if *idx == i {
                        let resp = ui.add(
                            egui::TextEdit::singleline(text)
                                .desired_width(140.0)
                                .hint_text("Name this moment"),
                        );
                        if resp.lost_focus()
                            || ui.ctx().input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            commit = Some(text.clone());
                        }
                    }
                }
                if let Some(label) = commit {
                    self.rename_bookmark = None;
                    if let Some(item) = self.library.get_mut(&path) {
                        if let Some(b) = item.bookmarks.get_mut(i) {
                            b.label = label;
                        }
                    }
                    self.touch();
                    self.toast_now("Bookmark renamed");
                } else if self.rename_bookmark.as_ref().map(|(idx, _)| *idx) != Some(i) {
                    let clicked = ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new(&bm.label).size(11.5).color(p.text),
                            )
                            .sense(Sense::click()),
                        )
                        .clicked();
                    if clicked {
                        seek = Some(bm.at_ms);
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if skin::icon_button(ui, Icon::Trash, 22.0, false, "Delete bookmark", p).clicked() {
                        delete = Some(i);
                    }
                    if skin::chip(ui, "+1s", false, p).clicked() {
                        nudge = Some((i, 1000));
                    }
                    if skin::chip(ui, "−1s", false, p).clicked() {
                        nudge = Some((i, -1000));
                    }
                });
            });
        }
        if let Some(t) = seek {
            self.seek_to(t);
        }
        if let Some(i) = delete {
            self.delete_bookmark(i);
        }
        if let Some((i, d)) = nudge {
            self.nudge_bookmark(i, d);
        }
        if !bookmarks.is_empty() && skin::ghost(ui, "Export as chapters", p).clicked() {
            self.export_bookmarks();
        }

        ui.add_space(10.0);
        skin::section(ui, "Chapters", p);
        if chapters.is_empty() {
            ui.label(
                egui::RichText::new("No chapter file found next to this item.")
                    .size(10.5)
                    .color(p.dim),
            );
        } else {
            for ch in chapters.iter().take(8) {
                if skin::chip(ui, &format!("{}  {}", format_time(ch.at_ms), ch.label), false, p).clicked() {
                    self.seek_to(ch.at_ms);
                }
            }
        }

        ui.add_space(12.0);
        skin::section(ui, "About this item", p);
        if let Some(now) = &self.now {
            let item = self.library.get(&now.path);
            let rows = [
                ("Title", now.title.clone()),
                ("Show", now.show.clone()),
                ("File", now.path.clone()),
                (
                    "Length",
                    if duration > 0 { format_time(duration) } else { "Unknown".into() },
                ),
                (
                    "Size",
                    item.map(|i| human_size(i.size)).unwrap_or_else(|| "—".into()),
                ),
                ("Played", format!("{} of {}", format_time(time), if duration > 0 { format_time(duration) } else { "—".into() })),
            ];
            for (k, v) in rows {
                ui.horizontal(|ui| {
                    // A fixed label column keeps "File" visible; the value gets
                    // the rest of the row and truncates instead of running back
                    // over its own label.
                    let (lr, _) = ui.allocate_exact_size(vec2(46.0, 15.0), Sense::hover());
                    ui.painter().text(
                        lr.left_center(),
                        Align2::LEFT_CENTER,
                        k,
                        skin::font(10.0),
                        p.dim,
                    );
                    let shown = if k == "File" {
                        shorten_path(&v, 40)
                    } else {
                        v.clone()
                    };
                    let resp = ui.add(
                        egui::Label::new(
                            egui::RichText::new(&shown)
                                .size(10.0)
                                .color(p.muted)
                                .monospace(),
                        )
                        .truncate(),
                    );
                    if shown != v {
                        resp.on_hover_text(egui::RichText::new(&v).monospace());
                    }
                });
            }
        }
    }

    // -----------------------------------------------------------------------
    // Queue drawer
    // -----------------------------------------------------------------------

    pub fn queue_panel(&mut self, ctx: &egui::Context, p: &Palette) {
        egui::SidePanel::right("queue")
            .exact_width(302.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(p.panel2)
                    .inner_margin(egui::Margin::ZERO)
                    .stroke(Stroke::new(1.0, p.line_soft)),
            )
            .show(ctx, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            skin::section(ui, "Queue", p);
                            ui.label(
                                egui::RichText::new(format!("{}", self.queue.items.len().saturating_sub(self.queue.index)))
                                    .size(10.5)
                                    .color(p.accent),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if skin::icon_button(ui, Icon::Close, 24.0, false, "Close queue · Q", p).clicked() {
                                    self.queue_open = false;
                                }
                                let mut pinned = self.queue.pinned;
                                if skin::icon_button(ui, Icon::Pin, 24.0, pinned, "Keep the queue after a restart", p).clicked() {
                                    pinned = !pinned;
                                    self.queue.pinned = pinned;
                                    self.touch();
                                    self.toast_now(if pinned { "Queue pinned for next time" } else { "Queue is session-only" });
                                }
                            });
                        });
                        ui.label(
                            egui::RichText::new("Plays in order · drop files here")
                                .size(10.0)
                                .color(p.dim),
                        );
                    });
                skin::divider(ui, p);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut play_index = None;
                        let mut move_up = None;
                        let mut move_down = None;
                        let mut remove = None;
                        let mut dropped: Option<String> = None;

                        ui.add_space(6.0);
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(12, 0))
                            .show(ui, |ui| {
                                skin::section(ui, "Now playing", p);
                                if let Some(path) = self.queue.items.get(self.queue.index).cloned() {
                                    self.queue_row(ui, p, self.queue.index, &path, true, &mut play_index, &mut move_up, &mut move_down, &mut remove);
                                } else {
                                    ui.label(egui::RichText::new("Nothing playing").size(11.0).color(p.dim));
                                }
                                ui.add_space(10.0);
                                skin::section(ui, "Up next", p);
                                let items: Vec<(usize, String)> = self
                                    .queue
                                    .items
                                    .iter()
                                    .enumerate()
                                    .skip(self.queue.index + 1)
                                    .map(|(i, p)| (i, p.clone()))
                                    .collect();
                                if items.is_empty() {
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new("Empty — drop a file or add from the library.")
                                            .size(10.5)
                                            .color(p.dim),
                                    );
                                }
                                let drop_rect = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), 60.0));
                                let drop_resp = ui.interact(drop_rect, Id::new("queue-drop"), Sense::hover());
                                if let Some(payload) = drop_resp.dnd_release_payload::<String>() {
                                    dropped = Some((*payload).clone());
                                }
                                if drop_resp.dnd_hover_payload::<String>().is_some() {
                                    ui.painter().rect_stroke(
                                        drop_rect,
                                        CornerRadius::same(8),
                                        Stroke::new(1.0, p.accent),
                                        StrokeKind::Inside,
                                    );
                                }
                                for (i, path) in items {
                                    self.queue_row(ui, p, i, &path, false, &mut play_index, &mut move_up, &mut move_down, &mut remove);
                                }
                            });

                        ui.add_space(10.0);
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(12, 4))
                            .show(ui, |ui| {
                                if skin::ghost(ui, "Clear queue", p).clicked() {
                                    self.queue.items.clear();
                                    self.queue.index = 0;
                                    self.toast_now("Queue cleared");
                                    self.touch();
                                }
                                if skin::ghost(ui, "Save as playlist…", p).clicked() {
                                    self.dlg_text.clear();
                                    self.dlg_sel = self.queue.items.clone();
                                    self.dialog = Some(Dialog::NewPlaylist);
                                }
                            });

                        if let Some(path) = dropped {
                            self.enqueue(vec![path]);
                        }
                        if let Some(i) = play_index {
                            self.play_queue_index(i);
                        }
                        if let Some((from, to)) = move_up {
                            self.queue.items.swap(from, to);
                            self.touch();
                        }
                        if let Some((from, to)) = move_down {
                            self.queue.items.swap(from, to);
                            self.touch();
                        }
                        if let Some(i) = remove {
                            if i < self.queue.items.len() {
                                self.queue.items.remove(i);
                                if self.queue.index >= self.queue.items.len() && !self.queue.items.is_empty() {
                                    self.queue.index = self.queue.items.len() - 1;
                                }
                                self.touch();
                                self.toast_now("Removed from the queue");
                            }
                        }
                    });
            });
    }

    #[allow(clippy::too_many_arguments)]
    fn queue_row(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        index: usize,
        path: &str,
        current: bool,
        play_index: &mut Option<usize>,
        move_up: &mut Option<(usize, usize)>,
        move_down: &mut Option<(usize, usize)>,
        remove: &mut Option<usize>,
    ) {
        let title = self.library.title_for(path);
        let item = self.library.get(path).cloned();
        let active = current;
        let frame = egui::Frame::new()
            .fill(if active { p.hover } else { Color32::TRANSPARENT })
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(8, 6));
        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
                if resp.clicked() {
                    *play_index = Some(index);
                }
                let thumb = Rect::from_min_size(rect.min + vec2(0.0, 3.0), vec2(48.0, 27.0));
                skin::poster(ui.painter(), thumb, crate::skin::seed_for(path), p);
                if active {
                    skin::play_badge(ui.painter(), thumb.center(), 8.0, p);
                }
                ui.painter().text(
                    pos2(thumb.right() + 9.0, rect.top() + 11.0),
                    Align2::LEFT_CENTER,
                    truncate(&title, 22),
                    skin::font(11.5),
                    if active { p.accent } else { p.text },
                );
                let sub = item
                    .as_ref()
                    .map(|i| {
                        format!(
                            "{} · {}",
                            i.show,
                            if i.duration_ms > 0 { format_time(i.duration_ms) } else { "—".into() }
                        )
                    })
                    .unwrap_or_default();
                ui.painter().text(
                    pos2(thumb.right() + 9.0, rect.top() + 25.0),
                    Align2::LEFT_CENTER,
                    truncate(&sub, 26),
                    skin::font(9.5),
                    p.dim,
                );
                if resp.hovered() && !active {
                    let mut x = rect.right() - 6.0;
                    for (icon, action) in [(Icon::Trash, 2u8), (Icon::ChevronDown, 1), (Icon::ChevronUp, 0)] {
                        x -= 20.0;
                        let ar = Rect::from_center_size(pos2(x, rect.center().y), Vec2::splat(18.0));
                        let r = ui.interact(ar, Id::new(("qbtn", index, action)), Sense::click());
                        if r.clicked() {
                            match action {
                                0 => *move_up = Some((index, index.saturating_sub(1))),
                                1 => *move_down = Some((index, (index + 1).min(self.queue.items.len().saturating_sub(1)))),
                                _ => *remove = Some(index),
                            }
                        }
                        skin::icon(ui.painter(), icon, ar.shrink(3.0), p.muted, 1.3);
                    }
                }
            });
        });
    }
}

fn slider_row(
    ui: &mut Ui,
    label: &str,
    value: &mut i64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
    p: &Palette,
) -> bool {
    let mut f = *value as f64;
    let changed = slider_row_f(ui, label, &mut f, range, suffix, p);
    if changed {
        *value = f.round() as i64;
    }
    changed
}

fn slider_row_f(
    ui: &mut Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
    p: &Palette,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(11.5).color(p.muted));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let shown = match suffix {
                "×" => format!("{:.1}×", value),
                "ms" => format!("{:.0} ms", value),
                _ => format!("{:.1}{}", value, suffix),
            };
            ui.label(egui::RichText::new(shown).size(10.0).color(p.dim).monospace());
            let width = (ui.available_width() - 8.0).max(80.0);
            let mut v = *value as f32;
            let resp = skin::slider(
                ui,
                &mut v,
                *range.start() as f32..=*range.end() as f32,
                width,
                p,
            );
            if resp.changed() {
                *value = v as f64;
                changed = true;
            }
        });
    });
    changed
}

fn slider_row_f32(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
    p: &Palette,
) -> bool {
    let mut f = *value as f64;
    let changed = slider_row_f(ui, label, &mut f, range, suffix, p);
    if changed {
        *value = f as f32;
    }
    changed
}

pub fn pretty_aout(name: &str) -> String {
    match name {
        "pulse" => "PulseAudio".into(),
        "alsa" => "ALSA".into(),
        "jack" => "JACK".into(),
        "adummy" => "Silent (no output)".into(),
        "afile" => "Write to a file".into(),
        "amem" => "In memory".into(),
        "mmdevice" => "Windows audio".into(),
        "directsound" => "DirectSound".into(),
        "wasapi" => "WASAPI".into(),
        "coreaudio" => "CoreAudio".into(),
        other => other.to_string(),
    }
}

pub fn human_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.0} MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    } else {
        s.to_string()
    }
}

/// Shorten a path from the left, so the file name (the part a person reads)
/// survives: `…/Season 1/Aurora Drift S01E01 - First Light.mp4`.
fn shorten_path(path: &str, max: usize) -> String {
    if path.chars().count() <= max {
        return path.to_string();
    }
    let parts: Vec<&str> = path.split('/').collect();
    let mut tail = String::new();
    // Grow the tail one path segment at a time while it still fits.
    for seg in parts.iter().rev() {
        let candidate = if tail.is_empty() {
            (*seg).to_string()
        } else {
            format!("{seg}/{tail}")
        };
        if candidate.chars().count() + 1 > max {
            break;
        }
        tail = candidate;
    }
    if tail.is_empty() {
        // A single very long file name: keep its beginning and its extension.
        let name = parts.last().copied().unwrap_or(path);
        return truncate(name, max);
    }
    format!("…/{tail}")
}

#[allow(dead_code)]
fn bookmark_time(b: &Bookmark) -> String {
    format_time(b.at_ms)
}
