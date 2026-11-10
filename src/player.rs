//! Chrome, the player surface, the thin control bar and its popovers.

use crate::app::{App, Dialog, Popup, View};
use crate::model::{format_time, Cue};
use crate::skin::{self, Icon, Palette};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};

impl App {
    pub fn top_chrome(&mut self, ctx: &egui::Context, p: &Palette) {
        if self.fullscreen {
            return;
        }
        egui::TopBottomPanel::top("chrome")
            .exact_height(58.0)
            .frame(
                egui::Frame::new()
                    .fill(p.panel)
                    .inner_margin(egui::Margin::symmetric(14, 0))
                    .stroke(Stroke::new(1.0, p.line_soft)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    self.brand(ui, p);
                    ui.add_space(10.0);
                    let bar = egui::MenuBar::new().ui(ui, |ui| {
                        self.menus(ui, p);
                    });
                    let _ = bar;
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let full = if self.fullscreen {
                            Icon::ExitFullscreen
                        } else {
                            Icon::Fullscreen
                        };
                        if skin::icon_button(ui, full, 30.0, false, "Full screen · F", p).clicked() {
                            self.set_fullscreen(ui.ctx(), !self.fullscreen);
                        }
                        if skin::icon_button(ui, Icon::Settings, 30.0, self.view == View::Settings, "Settings · Ctrl+,", p).clicked() {
                            self.view = if self.view == View::Settings { View::Player } else { View::Settings };
                        }
                        let q_active = self.queue_open;
                        let q = skin::icon_button(ui, Icon::Queue, 30.0, q_active, "Queue · Q", p);
                        if q.clicked() {
                            self.queue_open = !self.queue_open;
                        }
                        let count = self.queue.items.len().saturating_sub(self.queue.index + 1);
                        if count > 0 {
                            let badge = pos2(q.rect.right() - 4.0, q.rect.top() + 5.0);
                            ui.painter().circle_filled(badge, 7.0, p.accent);
                            ui.painter().text(
                                badge,
                                Align2::CENTER_CENTER,
                                format!("{count}"),
                                skin::font(8.5),
                                p.accent_text_or_dark(),
                            );
                        }
                        if skin::ghost(ui, "Library", p).clicked() {
                            self.view = View::Library;
                            self.section = crate::app::Section::Home;
                        }
                        ui.add_space(4.0);
                        let status = self
                            .libvlc_version
                            .clone()
                            .unwrap_or_else(|| "libvlc not found".into());
                        let label = format!("VLC {}", status.split(' ').next().unwrap_or(&status));
                        ui.label(
                            egui::RichText::new(label)
                                .size(10.5)
                                .color(if self.engine.is_some() { p.dim } else { p.bad }),
                        );
                    });
                });
            });
    }

    fn brand(&mut self, ui: &mut Ui, p: &Palette) {
        let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
        ui.painter().rect_filled(rect, CornerRadius::same(9), p.accent);
        let tri = Rect::from_center_size(rect.center(), Vec2::splat(20.0));
        skin::icon(ui.painter(), Icon::Play, tri, p.accent_text_or_dark(), 1.6);
        if resp.clicked() {
            self.view = View::Library;
            self.section = crate::app::Section::Home;
        }
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.add_space(1.0);
                ui.label(egui::RichText::new("VLCCraft").size(15.0).strong().color(p.text));
                ui.label(
                    egui::RichText::new("a calm skin over VLC")
                        .size(9.5)
                        .color(p.dim),
                );
            });
        });
    }

    fn menus(&mut self, ui: &mut Ui, p: &Palette) {
        let _ = p;
        ui.menu_button("Media", |ui| {
            if ui.button("Open file…").clicked() {
                self.open_file_dialog();
                ui.close();
            }
            if ui.button("Open folder…").clicked() {
                self.dlg_text = String::new();
                self.dlg_msg = None;
                self.dialog = Some(Dialog::OpenFolder);
                ui.close();
            }
            if ui.button("Open network stream…").clicked() {
                self.dlg_text.clear();
                self.dlg_msg = None;
                self.dialog = Some(Dialog::OpenUrl);
                ui.close();
            }
            if ui.button("Open disc…").clicked() {
                self.dlg_text.clear();
                self.dlg_msg = None;
                self.dialog = Some(Dialog::OpenDisc);
                ui.close();
            }
            if ui.button("Capture device…").clicked() {
                self.dlg_text.clear();
                self.dlg_msg = None;
                self.dialog = Some(Dialog::Capture);
                ui.close();
            }
            ui.separator();
            if ui.button("Media library").clicked() {
                self.view = View::Library;
                ui.close();
            }
            if ui.button("Quit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button("Playback", |ui| {
            if ui.button("Play / pause      Space").clicked() {
                self.toggle_play();
                ui.close();
            }
            if ui.button("Next item          N").clicked() {
                self.play_next();
                ui.close();
            }
            if ui.button("Previous item      P").clicked() {
                self.play_prev();
                ui.close();
            }
            ui.separator();
            ui.menu_button("Speed", |ui| {
                for rate in [0.5f32, 0.75, 1.0, 1.25, 1.5, 2.0] {
                    if ui
                        .selectable_label(
                            (self.engine.as_ref().map(|e| e.rate()).unwrap_or(1.0) - rate).abs() < 0.01,
                            format!("{rate}×"),
                        )
                        .clicked()
                    {
                        if let Some(engine) = &self.engine {
                            engine.set_rate(rate);
                        }
                        self.toast_now(format!("Speed {rate}×"));
                        ui.close();
                    }
                }
            });
            if ui.button("Add bookmark      Ctrl+B").clicked() {
                self.add_bookmark();
                ui.close();
            }
            ui.separator();
            if ui.button("Picture-in-picture").clicked() {
                self.toggle_pip(ui.ctx());
                ui.close();
            }
            let fs = if self.fullscreen { "Leave full screen    F" } else { "Full screen           F" };
            if ui.button(fs).clicked() {
                self.set_fullscreen(ui.ctx(), !self.fullscreen);
                ui.close();
            }
        });
        ui.menu_button("Audio", |ui| {
            let mute = if self.settings.muted { "Unmute      M" } else { "Mute          M" };
            if ui.button(mute).clicked() {
                self.toggle_mute();
                ui.close();
            }
            ui.menu_button("Volume", |ui| {
                let mut v = self.settings.volume as i32;
                if ui.add(egui::Slider::new(&mut v, 0..=100)).changed() {
                    self.set_volume(v as u8);
                }
            });
            ui.separator();
            if ui.button("Audio tracks…").clicked() {
                self.popup = Some(Popup::Audio);
                self.popup_anchor = ui.ctx().input(|i| i.pointer.hover_pos().unwrap_or(pos2(200.0, 300.0)));
                ui.close();
            }
            ui.menu_button("Output device", |ui| {
                let choices = self.audio_choices();
                for (label, value) in choices {
                    if ui
                        .selectable_label(self.settings.audio_output == value, &label)
                        .clicked()
                    {
                        self.set_audio_choice(&value);
                        ui.close();
                    }
                }
            });
        });
        ui.menu_button("Video", |ui| {
            if ui.button("Full screen      F").clicked() {
                self.set_fullscreen(ui.ctx(), !self.fullscreen);
                ui.close();
            }
            if ui.button("Picture-in-picture").clicked() {
                self.toggle_pip(ui.ctx());
                ui.close();
            }
            ui.separator();
            if ui.button("Video tracks…").clicked() {
                self.popup = Some(Popup::VideoTracks);
                self.popup_anchor = ui.ctx().input(|i| i.pointer.hover_pos().unwrap_or(pos2(200.0, 300.0)));
                ui.close();
            }
            if ui.button("Video settings…").clicked() {
                self.view = View::Player;
                self.sidebar_open = true;
                self.sidebar_tab = 2;
                ui.close();
            }
        });
        ui.menu_button("Subtitles", |ui| {
            if ui.button("Subtitle sidebar      S").clicked() {
                self.sidebar_open = true;
                self.sidebar_tab = 0;
                ui.close();
            }
            if ui.button("Add subtitles to this file…").clicked() {
                self.dialog = Some(Dialog::SubtitleBatch);
                self.subtitle_scope = "file".into();
                ui.close();
            }
            if ui.button("Match subtitles in a folder…").clicked() {
                self.dialog = Some(Dialog::SubtitleBatch);
                self.subtitle_scope = "folder".into();
                ui.close();
            }
            if ui.button("Find subtitles online…").clicked() {
                self.dialog = Some(Dialog::SubtitleSearch);
                self.dlg_msg = None;
                ui.close();
            }
            ui.separator();
            if ui.button("Subtitle presets…").clicked() {
                self.dialog = Some(Dialog::Presets);
                ui.close();
            }
            ui.menu_button("Delay", |ui| {
                if self.now.is_some() {
                    if ui.button("−50 ms      [").clicked() {
                        if let Some(now) = &mut self.now {
                            now.style.delay_ms -= 50;
                        }
                        self.touch();
                        self.toast_now("Subtitle delay −50 ms");
                        ui.close();
                    }
                    if ui.button("+50 ms      ]").clicked() {
                        if let Some(now) = &mut self.now {
                            now.style.delay_ms += 50;
                        }
                        self.touch();
                        self.toast_now("Subtitle delay +50 ms");
                        ui.close();
                    }
                } else {
                    ui.label("Open a video first");
                }
            });
        });
        ui.menu_button("View", |ui| {
            if ui.button("Media library      L").clicked() {
                self.view = View::Library;
                ui.close();
            }
            if ui.button("Queue      Q").clicked() {
                self.queue_open = !self.queue_open;
                ui.close();
            }
            if ui.button("Sidebar      S").clicked() {
                self.sidebar_open = !self.sidebar_open;
                ui.close();
            }
            ui.separator();
            if ui.button("Hide the interface      H").clicked() {
                self.interface_hidden = true;
                ui.close();
            }
            if ui.button("Customise controls…").clicked() {
                self.dialog = Some(Dialog::Customise);
                ui.close();
            }
            ui.separator();
            if ui.button("Settings      Ctrl+,").clicked() {
                self.view = View::Settings;
                ui.close();
            }
            if ui.button("Keyboard shortcuts…").clicked() {
                self.dialog = Some(Dialog::Hotkeys);
                ui.close();
            }
        });
        ui.menu_button("Tools", |ui| {
            if ui.button("Convert to a new file…").clicked() {
                self.open_convert_dialog();
                ui.close();
            }
            if ui.button("Stream out to the network…").clicked() {
                self.open_stream_dialog();
                ui.close();
            }
            if self.job.is_some() {
                if ui.button("Stop the running job").clicked() {
                    self.stop_job();
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Settings…").clicked() {
                self.view = View::Settings;
                ui.close();
            }
            if ui.button("Presets…").clicked() {
                self.dialog = Some(Dialog::Presets);
                ui.close();
            }
            if ui.button("Layout profiles…").clicked() {
                self.dialog = Some(Dialog::Customise);
                ui.close();
            }
            ui.separator();
            if ui.button("Make VLCCraft the default player…").clicked() {
                self.view = View::Settings;
                self.settings_cat = crate::app::SettingsCategory::General;
                self.settings_search.clear();
                self.settings_highlight = Some("default.player".into());
                self.refresh_defaults();
                ui.close();
            }
            if ui.button("About VLCCraft").clicked() {
                self.dialog = Some(Dialog::About);
                ui.close();
            }
        });
    }

    pub fn player_view(&mut self, ctx: &egui::Context, p: &Palette) {
        // The docks belong to the windowed player: in full screen the picture
        // owns the whole display and the bands carry the controls instead.
        if !self.fullscreen {
        if self.compact_library {
            self.compact_library_panel(ctx, p);
        }
        if self.queue_open {
            self.queue_panel(ctx, p);
        }
        if self.sidebar_open {
            egui::SidePanel::right("sidebar")
                .exact_width(352.0)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(p.panel)
                        .inner_margin(egui::Margin::ZERO)
                        .stroke(Stroke::new(1.0, p.line_soft)),
                )
                .show(ctx, |ui| {
                    self.sidebar_ui(ui, p);
                });
        }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg).inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                if self.fullscreen {
                    self.draw_video(ui, p, rect, true);
                    self.fullscreen_bands(ui, p, rect);
                } else {
                    let header_h = 46.0;
                    let bar_h = 56.0;
                    let header = Rect::from_min_size(rect.min, vec2(rect.width(), header_h));
                    let bar = Rect::from_min_size(
                        pos2(rect.left(), rect.bottom() - bar_h),
                        vec2(rect.width(), bar_h),
                    );
                    let video = Rect::from_min_max(
                        pos2(rect.left(), rect.top() + header_h),
                        pos2(rect.right(), rect.bottom() - bar_h),
                    );
                    self.player_header(ui, p, header);
                    self.draw_video(ui, p, video, false);
                    self.control_bar(ui, p, bar, false);
                }
                self.resume_chip(ui, p);
            });

        self.popovers(ctx, p);
    }

    pub fn player_video_only(&mut self, ctx: &egui::Context, p: &Palette) {
        let full = ctx.screen_rect();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::BLACK).inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                self.draw_video(ui, p, full, true);
                let moved = ctx.input(|i| i.pointer.delta() != Vec2::ZERO);
                if moved {
                    self.interface_hidden = false;
                }
                // Any click brings the bar back too, but it draws on top.
                if moved {
                    self.control_bar(ui, p, Rect::from_min_size(
                        pos2(full.left(), full.bottom() - 64.0),
                        vec2(full.width(), 64.0),
                    ), true);
                }
            });
    }

    fn player_header(&mut self, ui: &mut Ui, p: &Palette, rect: Rect) {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(vec2(18.0, 0.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.horizontal_centered(|ui| {
            let (show, title, sub) = self
                .now
                .as_ref()
                .map(|n| {
                    (
                        n.show.clone(),
                        n.title.clone(),
                        n.subtitle_path.as_ref().map(|_| {
                            format!(
                                "SRT · {} cue{}",
                                n.cues.len(),
                                if n.cues.len() == 1 { "" } else { "s" }
                            )
                        }),
                    )
                })
                .unwrap_or_else(|| (String::new(), "Nothing playing".into(), None));
            ui.vertical(|ui| {
                // Two lines inside a 46 px band: the title row and one line of
                // facts. Default item spacing made the second line spill out.
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.horizontal(|ui| {
                    if !show.is_empty() {
                        let show_resp = ui.add(
                            egui::Label::new(
                                egui::RichText::new(show.to_uppercase())
                                    .size(9.5)
                                    .color(p.dim)
                                    .strong(),
                            )
                            .sense(Sense::click()),
                        );
                        let show_hover = show_resp.hovered();
                        if show_hover {
                            ui.painter().line_segment(
                                [
                                    pos2(show_resp.rect.left(), show_resp.rect.bottom()),
                                    pos2(show_resp.rect.right(), show_resp.rect.bottom()),
                                ],
                                Stroke::new(1.0, p.accent),
                            );
                        }
                        ui.painter().circle_filled(
                            pos2(show_resp.rect.right() + 8.0, show_resp.rect.center().y),
                            1.5,
                            p.dim,
                        );
                        if show_resp.clicked() {
                            self.search = show.clone();
                            self.section = crate::app::Section::Videos;
                            self.view = View::Library;
                        }
                        ui.add_space(8.0);
                    }
                    let sub_w = sub
                        .as_ref()
                        .map(|s| {
                            ui.painter()
                                .layout_no_wrap(s.clone(), skin::font(10.5), p.dim)
                                .size()
                                .x
                                + 10.0
                        })
                        .unwrap_or(0.0);
                    let reserve = 190.0 + sub_w;
                    let title_max = (ui.available_width() - reserve).max(110.0);
                    ui.scope(|ui| {
                        ui.set_max_width(title_max);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(title).size(14.5).color(p.text),
                            )
                            .truncate(),
                        );
                    });
                    if let Some(sub) = sub {
                        if ui.available_width() > 70.0 {
                            ui.add_space(4.0);
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(sub).size(10.5).color(p.dim),
                                )
                                .truncate(),
                            );
                        }
                    }
                });
                let meta = match &self.now {
                    Some(n) => {
                        let dur = self.duration_ms();
                        let pos = self.time_ms();
                        let folder = std::path::Path::new(&n.path)
                            .parent()
                            .and_then(|d| d.file_name())
                            .map(|s| s.to_string_lossy().to_string())
                            .filter(|s| !s.is_empty() && s != "media")
                            .unwrap_or_else(|| n.show.clone());
                        if dur > 0 {
                            format!("{} · {} left", folder, format_time((dur - pos).max(0)))
                        } else {
                            folder
                        }
                    }
                    None => "Open a file, a folder or a link to begin".to_string(),
                };
                ui.label(egui::RichText::new(meta).size(9.5).color(p.dim));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(now) = &self.now {
                    if let Some(preset) = &now.preset_name {
                        ui.label(
                            egui::RichText::new(format!("PRESET · {preset}"))
                                .size(9.5)
                                .color(p.accent),
                        );
                    }
                }
                if self.now.is_some() {
                    if skin::icon_button(ui, Icon::Bookmark, 28.0, false, "Add bookmark · Ctrl+B", p).clicked() {
                        self.add_bookmark();
                    }
                }
                let lib_label = if self.compact_library { "Hide library" } else { "Library" };
                if skin::ghost(ui, lib_label, p).clicked() {
                    self.compact_library = !self.compact_library;
                    if self.compact_library {
                        self.section = crate::app::Section::Videos;
                    }
                }
            });
        });
    }

    pub fn draw_video(&mut self, ui: &mut Ui, p: &Palette, rect: Rect, fullscreen: bool) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::ZERO, if fullscreen { Color32::BLACK } else { p.video_bg });

        let (tex, size) = match &mut self.engine {
            Some(engine) => engine.texture(ui.ctx()).unwrap_or((egui::TextureId::default(), Vec2::ZERO)),
            None => (egui::TextureId::default(), Vec2::ZERO),
        };

        let has_frame = self.engine.as_ref().map(|e| e.has_frame()).unwrap_or(false) && size.x > 1.0;
        let target = if has_frame {
            let ar = size.x / size.y.max(1.0);
            let avail = rect.shrink(if fullscreen { 0.0 } else { 8.0 });
            let mut w = avail.width();
            let mut h = w / ar;
            if h > avail.height() {
                h = avail.height();
                w = h * ar;
            }
            Rect::from_center_size(avail.center(), vec2(w, h))
        } else {
            let ar = 16.0 / 9.0;
            let mut w = rect.width();
            let mut h = w / ar;
            if h > rect.height() {
                h = rect.height();
                w = h * ar;
            }
            Rect::from_center_size(rect.center(), vec2(w, h))
        };

        if has_frame {
            ui.painter().image(
                tex,
                target,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else if self.at_end() {
            // The item has finished. Nothing is drawn over the picture area:
            // no "up next" card, no countdown, no placeholder sentence. The
            // round button simply shows the play glyph again.
        } else {
            // A calm placeholder until the first decoded frame arrives.
            skin::poster(&painter, target, 42.0, p);
            painter.rect_filled(target, CornerRadius::ZERO, Color32::from_black_alpha(60));
            if let Some(now) = &self.now {
                painter.text(
                    target.center(),
                    Align2::CENTER_CENTER,
                    "Opening the stream…",
                    skin::font(14.0),
                    p.muted,
                );
                if let Some(err) = &now.error {
                    painter.text(
                        target.center() + vec2(0.0, 22.0),
                        Align2::CENTER_CENTER,
                        err,
                        skin::font(11.0),
                        p.bad,
                    );
                }
            }
        }

        // Subtitles are drawn by the skin, so delay, size, outline and
        // position all update while the picture keeps playing.
        if let Some(now) = &self.now {
            if let Some(cue) = now.active_cue(self.time_ms()) {
                self.draw_subtitle(&painter, target, cue, now.style.size_pct, now.style.outline, now.style.position_pct, now.style.background, p);
            }
        }

        if let Some(engine) = &self.engine_error {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                format!("Playback unavailable: {engine}"),
                skin::font(12.0),
                p.bad,
            );
        }
    }

    fn draw_subtitle(
        &self,
        painter: &egui::Painter,
        video: Rect,
        cue: &Cue,
        size_pct: f32,
        outline: f32,
        position_pct: f32,
        background: bool,
        p: &Palette,
    ) {
        let size = (video.height() * size_pct / 100.0).clamp(10.0, 72.0);
        let font = skin::font(size);
        let y = video.bottom() - video.height() * position_pct / 100.0;
        let lines: Vec<&str> = cue.text.lines().collect();
        let line_h = size * 1.28;
        let center = pos2(video.center().x, y - line_h * (lines.len().saturating_sub(1)) as f32 * 0.5);
        let galley = painter.layout(
            cue.text.clone(),
            font.clone(),
            Color32::WHITE,
            video.width() * 0.86,
        );
        let text_rect = Rect::from_center_size(center, galley.size());
        if background {
            painter.rect_filled(
                text_rect.expand2(vec2(10.0, 6.0)),
                CornerRadius::same(4),
                Color32::from_black_alpha(150),
            );
        }
        if outline > 0.0 {
            let o = outline * (size / 28.0).max(0.6);
            for (dx, dy) in [(-o, 0.0), (o, 0.0), (0.0, -o), (0.0, o), (-o, -o), (o, o), (-o, o), (o, -o)] {
                painter.galley(
                    text_rect.min + vec2(dx, dy),
                    galley.clone(),
                    Color32::from_rgba_unmultiplied(0, 0, 0, 220),
                );
            }
        }
        painter.galley(text_rect.min, galley, Color32::WHITE);
        let _ = p;
    }

    fn fullscreen_bands(&mut self, ui: &mut Ui, p: &Palette, rect: Rect) {
        let pointer = ui.ctx().input(|i| i.pointer.hover_pos());
        let (px, py) = pointer
            .map(|pos| (pos.x, pos.y))
            .unwrap_or((-1.0, -1.0));
        let recent = self.time - self.last_pointer_move < 1.5 && self.time - self.last_interaction < 1.5;

        let top_rect = Rect::from_min_size(rect.min, vec2(rect.width(), 62.0));
        let bottom_rect = Rect::from_min_size(
            pos2(rect.left(), rect.bottom() - 84.0),
            vec2(rect.width(), 84.0),
        );

        let show_top = top_rect.contains(pos2(px, py)) || (py >= 0.0 && py < 14.0) || recent;
        let show_bottom = bottom_rect.contains(pos2(px, py)) || (py >= rect.bottom() - 14.0) || recent;

        if show_top {
            let painter = ui.painter_at(top_rect);
            let mut mesh = egui::Mesh::default();
            let _ = &mut mesh;
            painter.rect_filled(top_rect, CornerRadius::ZERO, Color32::from_black_alpha(170));
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(top_rect.shrink2(vec2(20.0, 0.0)))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            child.horizontal_centered(|ui| {
                let (title, show) = self
                    .now
                    .as_ref()
                    .map(|n| (n.title.clone(), n.show.clone()))
                    .unwrap_or_default();
                ui.horizontal(|ui| {
                    if !show.is_empty() {
                        ui.label(
                            egui::RichText::new(show.to_uppercase())
                                .size(9.5)
                                .color(p.muted),
                        );
                        ui.label(egui::RichText::new("·").color(p.dim));
                    }
                    ui.label(egui::RichText::new(title).size(14.0).color(Color32::WHITE));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if skin::icon_button(ui, Icon::ExitFullscreen, 30.0, false, "Leave full screen · Esc", p).clicked() {
                        self.set_fullscreen(ui.ctx(), false);
                    }
                    if skin::icon_button(ui, Icon::Pip, 30.0, false, "Picture-in-picture", p).clicked() {
                        self.toggle_pip(ui.ctx());
                    }
                    if skin::icon_button(ui, Icon::Library, 30.0, false, "Media library · L", p).clicked() {
                        self.set_fullscreen(ui.ctx(), false);
                        self.view = View::Library;
                    }
                    let count = self.queue.items.len();
                    let label = if count > 0 { format!("Queue · {count}") } else { "Queue".to_string() };
                    if skin::ghost(ui, &label, p).clicked() {
                        self.queue_open = !self.queue_open;
                    }
                });
            });
        }

        if show_bottom {
            self.control_bar(ui, p, bottom_rect.shrink2(vec2(0.0, 4.0)), true);
        }
    }

    pub fn control_bar(&mut self, ui: &mut Ui, p: &Palette, rect: Rect, overlay: bool) {
        let bg = if overlay {
            Color32::from_black_alpha(186)
        } else {
            p.panel
        };
        ui.painter().rect_filled(rect, CornerRadius::ZERO, bg);
        if overlay {
            ui.painter().rect_stroke(
                rect,
                CornerRadius::ZERO,
                Stroke::new(1.0, Color32::from_white_alpha(18)),
                StrokeKind::Inside,
            );
        }

        let mut x = rect.left() + 12.0;
        let cy = rect.center().y - 4.0;
        let btn = 30.0;

        let prev = Rect::from_center_size(pos2(x + btn / 2.0, cy), Vec2::splat(btn));
        x += btn + 2.0;
        let play = Rect::from_center_size(pos2(x + 21.0, cy), Vec2::splat(42.0));
        x += 46.0;
        let next = Rect::from_center_size(pos2(x + btn / 2.0, cy), Vec2::splat(btn));
        x += btn + 10.0;

        let r_prev = ui.interact(prev, Id::new("bar-prev"), Sense::click());
        if r_prev.clicked() {
            self.play_prev();
        }
        let r_prev = r_prev.on_hover_text(format!("Previous · {}", self.settings.hotkey("Previous item")));
        let _ = r_prev;
        skin::icon(
            ui.painter(),
            Icon::Prev,
            prev.shrink(7.0),
            if r_prev.hovered() { p.text } else { p.muted },
            1.6,
        );

        let r_play = ui.interact(play, Id::new("bar-play"), Sense::click());
        ui.painter().circle_filled(play.center(), 19.0, if r_play.hovered() { skin::mix(p.accent, Color32::WHITE, 0.12) } else { p.accent });
        // Only two glyphs ever appear on this button: pause while playing,
        // play at every other moment. A finished item is simply not playing,
        // so it shows the play glyph again; pressing it starts the item over
        // rather than doing nothing, which is what a stuck "ended" player used
        // to do.
        let ended = self.at_end();
        // Two glyphs only: pause while the picture is actually moving, play
        // at every other moment, including the moment the item finishes.
        let playing = !ended && self.is_playing();
        skin::icon(
            ui.painter(),
            if playing { Icon::Pause } else { Icon::Play },
            Rect::from_center_size(play.center(), Vec2::splat(24.0)),
            p.accent_text_or_dark(),
            1.8,
        );
        if r_play.clicked() {
            self.toggle_play();
        }
        let r_play = r_play.on_hover_text(format!(
            "Play / pause · {}",
            self.settings.hotkey("Play / pause")
        ));
        let _ = r_play;

        let r_next = ui.interact(next, Id::new("bar-next"), Sense::click());
        if r_next.clicked() {
            self.play_next();
        }
        let r_next = r_next.on_hover_text(format!("Next · {}", self.settings.hotkey("Next item")));
        skin::icon(
            ui.painter(),
            Icon::Next,
            next.shrink(7.0),
            if r_next.hovered() { p.text } else { p.muted },
            1.6,
        );

        let time_text = format!(
            "{} / {}",
            format_time(self.time_ms()),
            if self.duration_ms() > 0 { format_time(self.duration_ms()) } else { "--:--".into() }
        );
        let time_galley = ui.painter().layout_no_wrap(time_text, skin::mono(11.5), if overlay { Color32::from_white_alpha(200) } else { p.muted });
        let time_w = time_galley.size().x + 12.0;
        ui.painter().galley(pos2(x, cy - time_galley.size().y / 2.0), time_galley, Color32::TRANSPARENT);
        x += time_w;

        // Right cluster, measured from the right edge.
        let mut rx = rect.right() - 12.0;
        let mut right_buttons: Vec<(Icon, &'static str, &'static str)> = Vec::new();
        let visible = self.settings.layout.controls.clone();
        let has = |id: &str| visible.iter().any(|c| c == id);
        if has("settings") {
            right_buttons.push((Icon::Settings, "Settings · Ctrl+,", "settings"));
        }
        if has("fullscreen") {
            right_buttons.push((if self.fullscreen { Icon::ExitFullscreen } else { Icon::Fullscreen }, "Full screen · F", "fullscreen"));
        }
        if has("pip") {
            right_buttons.push((Icon::Pip, "Picture-in-picture", "pip"));
        }
        if has("queue") {
            right_buttons.push((Icon::Queue, "Queue · Q", "queue"));
        }
        if has("bookmark") {
            right_buttons.push((Icon::Bookmark, "Add bookmark · Ctrl+B", "bookmark"));
        }
        if has("audio") {
            right_buttons.push((Icon::Speaker, "Audio tracks and output", "audio"));
        }
        if has("subtitles") {
            right_buttons.push((Icon::Subtitle, "Subtitles · S", "subtitles"));
        }

        if has("volume") {
            let vol_w = 108.0;
            rx -= vol_w;
            let vol_rect = Rect::from_min_size(pos2(rx, cy - 12.0), vec2(vol_w, 24.0));
            let mute = self.settings.muted || self.settings.volume == 0;
            let icon_rect = Rect::from_min_size(vol_rect.min, vec2(24.0, 24.0));
            let r_vol_icon = ui.interact(icon_rect, Id::new("bar-vol-icon"), Sense::click());
            if r_vol_icon.clicked() {
                self.toggle_mute();
            }
            let _ = r_vol_icon.on_hover_text(format!("Mute · {}", self.settings.hotkey("Mute")));
            skin::icon(
                ui.painter(),
                if mute { Icon::Mute } else { Icon::Volume },
                icon_rect.shrink(2.0),
                if mute { p.danger_or(p) } else { p.muted },
                1.5,
            );
            let slider_rect = Rect::from_min_size(pos2(icon_rect.right() + 4.0, cy - 1.5), vec2(vol_w - 34.0, 3.0));
            ui.painter().rect_filled(slider_rect, CornerRadius::same(2), if overlay { Color32::from_white_alpha(40) } else { p.line });
            let frac = (self.settings.volume as f32 / 125.0).clamp(0.0, 1.0);
            let filled = Rect::from_min_size(slider_rect.min, vec2(slider_rect.width() * frac, 3.0));
            ui.painter().rect_filled(filled, CornerRadius::same(2), p.accent);
            let knob = pos2(slider_rect.left() + slider_rect.width() * frac, cy);
            ui.painter().circle_filled(knob, 4.0, p.text);
            ui.painter().circle_stroke(knob, 4.0, Stroke::new(1.0, Color32::from_black_alpha(60)));
            let r_slider = ui.interact(
                slider_rect.expand2(vec2(0.0, 10.0)),
                Id::new("bar-vol"),
                Sense::click_and_drag(),
            );
            if r_slider.clicked() || r_slider.dragged() {
                if let Some(pos) = r_slider.interact_pointer_pos() {
                    let f = ((pos.x - slider_rect.left()) / slider_rect.width()).clamp(0.0, 1.0);
                    self.set_volume((f * 125.0) as u8);
                }
            }
            rx -= 10.0;
        }

        for (icon, tip, id) in right_buttons {
            rx -= 36.0;
            let r = Rect::from_center_size(pos2(rx + 16.0, cy), Vec2::splat(32.0));
            let resp = ui.interact(r, Id::new(("bar", id)), Sense::click());
            let hovered = resp.hovered();
            if hovered {
                ui.painter().rect_filled(r, CornerRadius::same(6), if overlay { Color32::from_white_alpha(22) } else { p.hover });
            }
            let active = match id {
                "subtitles" => self.sidebar_open && self.sidebar_tab == 0,
                "queue" => self.queue_open,
                "settings" => false,
                _ => false,
            };
            skin::icon(
                ui.painter(),
                icon,
                r.shrink(3.0),
                if active { p.accent } else if hovered { p.text } else { p.muted },
                1.5,
            );
            let resp = resp.on_hover_text(tip);
            if resp.clicked() {
                match id {
                    "settings" => self.view = View::Settings,
                    "fullscreen" => self.set_fullscreen(ui.ctx(), !self.fullscreen),
                    "pip" => self.toggle_pip(ui.ctx()),
                    "queue" => self.queue_open = !self.queue_open,
                    "bookmark" => self.add_bookmark(),
                    "audio" => {
                        self.popup = Some(Popup::Audio);
                        self.popup_anchor = pos2(r.left(), r.top());
                    }
                    "subtitles" => {
                        self.popup = Some(Popup::Subtitles);
                        self.popup_anchor = pos2(r.left() - 220.0, r.top());
                    }
                    _ => {}
                }
            }
        }

        // Seek bar fills the space between timecode and the right cluster.
        let seek_left = x + 6.0;
        let seek_right = rx - 8.0;
        if seek_right - seek_left > 90.0 {
            let track = Rect::from_min_max(
                pos2(seek_left, cy - 5.0),
                pos2(seek_right, cy + 5.0),
            );
            self.seek_bar(ui, p, track, overlay);
        }
    }

    fn seek_bar(&mut self, ui: &mut Ui, p: &Palette, track: Rect, overlay: bool) -> Response {
        let dur = self.duration_ms().max(1);
        let time = self.time_ms();
        let hit = Rect::from_min_max(
            pos2(track.left(), track.center().y - 12.0),
            pos2(track.right(), track.center().y + 12.0),
        );
        let resp = ui.interact(hit, Id::new("seek"), Sense::click_and_drag());
        let hovered = resp.hovered() || resp.dragged();

        let track_h = if hovered { 9.0 } else { 7.0 };
        let bar = Rect::from_center_size(track.center(), vec2(track.width(), track_h));
        ui.painter().rect_filled(bar, CornerRadius::same(4), if overlay { Color32::from_white_alpha(42) } else { p.line });

        if self.settings.show_buffered && !self.is_stream() {
            let buffered = Rect::from_min_size(bar.min, vec2(bar.width(), bar.height()));
            ui.painter().rect_filled(buffered, CornerRadius::same(4), if overlay { Color32::from_white_alpha(26) } else { p.line_soft });
        }

        let frac = (time as f32 / dur as f32).clamp(0.0, 1.0);
        let played = Rect::from_min_size(bar.min, vec2(bar.width() * frac, bar.height()));
        ui.painter().rect_filled(played, CornerRadius::same(4), p.accent);

        // Subtitle cue segments, chapters and bookmarks share the timeline.
        if let Some(now) = &self.now {
            let cue_color = if overlay { Color32::from_white_alpha(70) } else { p.line };
            for cue in &now.cues {
                let a = (cue.start_ms as f32 / dur as f32).clamp(0.0, 1.0);
                let b = (cue.end_ms as f32 / dur as f32).clamp(0.0, 1.0);
                if b - a > 0.002 {
                    let r = Rect::from_min_max(
                        pos2(bar.left() + bar.width() * a, bar.top() - 3.0),
                        pos2(bar.left() + bar.width() * b, bar.top() - 1.0),
                    );
                    ui.painter().rect_filled(r, CornerRadius::same(1), cue_color);
                }
            }
            for ch in &now.chapters {
                let f = (ch.at_ms as f32 / dur as f32).clamp(0.0, 1.0);
                let x = bar.left() + bar.width() * f;
                ui.painter().line_segment(
                    [pos2(x, bar.bottom() + 1.0), pos2(x, bar.bottom() + 5.0)],
                    Stroke::new(1.0, if overlay { Color32::from_white_alpha(90) } else { p.dim }),
                );
            }
        }
        let now_path = self.now.as_ref().map(|n| n.path.clone());
        let bookmark_list: Vec<i64> = now_path
            .as_ref()
            .and_then(|path| self.library.get(path))
            .map(|item| item.bookmarks.iter().map(|b| b.at_ms).collect())
            .unwrap_or_default();
        let mut moved_bookmark: Option<(usize, i64)> = None;
        for (i, at) in bookmark_list.iter().enumerate() {
            let f = (*at as f32 / dur as f32).clamp(0.0, 1.0);
            let x = bar.left() + bar.width() * f;
            let mark = Rect::from_center_size(pos2(x, bar.top() - 6.0), vec2(6.0, 9.0));
            ui.painter().rect_filled(mark, CornerRadius::same(2), p.accent);
            let mark_resp = ui.interact(mark.expand(3.0), Id::new(("bm-drag", i)), Sense::click_and_drag());
            if mark_resp.dragged() {
                if let Some(pos) = mark_resp.interact_pointer_pos() {
                    let f2 = ((pos.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
                    moved_bookmark = Some((i, (f2 * dur as f32) as i64));
                }
            }
        }
        if let (Some(path), Some((i, at))) = (now_path, moved_bookmark) {
            if let Some(item) = self.library.get_mut(&path) {
                if let Some(b) = item.bookmarks.get_mut(i) {
                    b.at_ms = at;
                }
            }
            self.touch();
        }

        let knob = pos2(bar.left() + bar.width() * frac, bar.center().y);
        if hovered {
            ui.painter().circle_filled(knob, 6.0, p.accent);
            ui.painter().circle_filled(knob, 2.2, if p.light { Color32::WHITE } else { Color32::BLACK });
        }

        if (resp.clicked() || resp.dragged()) && self.now.is_some() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let f = ((pos.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
                self.seek_to((f * dur as f32) as i64);
            }
        }

        if hovered && self.now.is_some() {
            if let Some(pos) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                let f = ((pos.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
                let t = (f * dur as f32) as i64;
                let bubble = Rect::from_center_size(
                    pos2(pos.x.clamp(hit.left() + 40.0, hit.right() - 40.0), hit.top() - 26.0),
                    vec2(76.0, 40.0),
                );
                let thumb = self.nearest_thumb(t);
                if let Some(tex) = thumb {
                    let tr = Rect::from_min_size(
                        pos2(bubble.center().x - 34.0, bubble.top() - 40.0),
                        vec2(68.0, 38.0),
                    );
                    ui.painter().rect_filled(tr.expand(2.0), CornerRadius::same(4), Color32::from_black_alpha(200));
                    ui.painter().image(
                        tex,
                        tr,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                ui.painter().rect_filled(bubble, CornerRadius::same(5), Color32::from_black_alpha(210));
                ui.painter().text(
                    bubble.center(),
                    Align2::CENTER_CENTER,
                    format_time(t),
                    skin::mono(11.0),
                    Color32::WHITE,
                );
            }
        }
        resp.on_hover_text("Seek — ←/→ 5 s, Shift for 30 s")
    }

    fn is_stream(&self) -> bool {
        self.now
            .as_ref()
            .map(|n| n.path.contains("://"))
            .unwrap_or(false)
    }

    fn nearest_thumb(&self, t: i64) -> Option<egui::TextureId> {
        if self.thumbs.is_empty() {
            return None;
        }
        let mut best = self.thumbs[0].0;
        for (at, _) in &self.thumbs {
            if (*at - t).abs() < (best - t).abs() {
                best = *at;
            }
        }
        self.thumbs
            .iter()
            .find(|(at, _)| *at == best)
            .map(|(_, tex)| tex.id())
    }

    fn resume_chip(&mut self, ui: &mut Ui, p: &Palette) {
        let Some(resume) = self.resume.clone() else { return };
        if resume.dismissed {
            return;
        }
        let bar = Rect::from_min_size(
            pos2(ui.max_rect().left() + 18.0, ui.max_rect().bottom() - 116.0),
            vec2(340.0, 48.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(bar));
        egui::Frame::new()
            .fill(p.panel)
            .stroke(Stroke::new(1.0, p.accent))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(12, 6))
            .show(&mut child, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Continue where you stopped")
                                .size(11.0)
                                .color(p.text),
                        );
                        ui.label(
                            egui::RichText::new(format!("Resume at {}", format_time(resume.position_ms)))
                                .size(10.0)
                                .color(p.dim),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if skin::primary(ui, "Resume", p).clicked() {
                            let t = resume.position_ms;
                            self.seek_to(t);
                            self.resume = None;
                            self.toast_now("Resumed");
                        }
                        if skin::ghost(ui, "Start over", p).clicked() {
                            self.seek_to(0);
                            self.resume = None;
                        }
                    });
                });
            });
    }

    fn popovers(&mut self, ctx: &egui::Context, p: &Palette) {
        let Some(popup) = self.popup else { return };
        let screen = ctx.screen_rect();
        let width = 292.0;
        let mut pos = self.popup_anchor;
        pos.x = pos.x.clamp(screen.left() + 12.0, screen.right() - width - 12.0);
        pos.y = pos.y.clamp(screen.top() + 160.0, screen.bottom() - 12.0);
        let max_h = (pos.y - screen.top() - 24.0).clamp(140.0, 480.0);
        let id = match popup {
            Popup::Subtitles => "pop-subtitles",
            Popup::Audio => "pop-audio",
            Popup::VideoTracks => "pop-video",
        };
        let mut close = false;
        egui::Area::new(Id::new(id))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .pivot(egui::Align2::LEFT_BOTTOM)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0, p.line))
                    .corner_radius(CornerRadius::same(11))
                    .inner_margin(egui::Margin::same(10))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_black_alpha(140),
                    })
                    .show(ui, |ui| {
                        ui.set_width(width);
                        egui::ScrollArea::vertical()
                            .max_height(max_h)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                match popup {
                                    Popup::Subtitles => close = self.subtitle_popover(ui, p),
                                    Popup::Audio => close = self.audio_popover(ui, p),
                                    Popup::VideoTracks => close = self.video_popover(ui, p),
                                }
                            });
                    });
            });
        if close {
            self.popup = None;
        }
    }

    fn subtitle_popover(&mut self, ui: &mut Ui, p: &Palette) -> bool {
        let mut close = false;
        skin::section(ui, "Subtitles", p);
        let active = self.now.as_ref().and_then(|n| n.subtitle_path.clone());
        if ui
            .selectable_label(active.is_none(), "Off")
            .clicked()
        {
            self.select_subtitle(None);
            close = true;
        }
        let files = self
            .now
            .as_ref()
            .map(|n| n.subtitle_files.clone())
            .unwrap_or_default();
        for file in files {
            let name = file
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let is_active = active.as_ref() == Some(&file);
            if ui.selectable_label(is_active, format!("{}  ·  SRT", name)).clicked() {
                self.select_subtitle(Some(file));
                close = true;
            }
        }
        if active.is_none() {
            ui.label(egui::RichText::new("No matching subtitle file").size(10.5).color(p.dim));
        }
        skin::divider(ui, p);
        if ui.button("Add subtitles to this file…").clicked() {
            self.dialog = Some(Dialog::SubtitleBatch);
            close = true;
        }
        if ui.button("Subtitle style and timing").clicked() {
            self.sidebar_open = true;
            self.sidebar_tab = 0;
            close = true;
        }
        close
    }

    fn audio_popover(&mut self, ui: &mut Ui, p: &Palette) -> bool {
        let mut close = false;
        ui.horizontal(|ui| {
            if skin::text_tab(ui, "Audio tracks", self.audio_popup_tab == 0, p).clicked() {
                self.audio_popup_tab = 0;
            }
            if skin::text_tab(ui, "Video", self.audio_popup_tab == 1, p).clicked() {
                self.audio_popup_tab = 1;
            }
        });
        if self.audio_popup_tab == 1 {
            skin::section(ui, "Video tracks", p);
            let tracks = self
                .now
                .as_ref()
                .map(|n| n.video_tracks.clone())
                .unwrap_or_default();
            for (i, (_, name)) in tracks.iter().enumerate() {
                let active = self.now.as_ref().map(|n| n.video_track_index == i).unwrap_or(false);
                if ui.selectable_label(active, name).clicked() {
                    if let Some(now) = &mut self.now {
                        now.video_track_index = i;
                    }
                    if let (Some(engine), Some(now)) = (&self.engine, &self.now) {
                        if let Some((id, _)) = now.video_tracks.get(i) {
                            engine.set_video_track(*id);
                        }
                    }
                    self.toast_now(format!("Video track: {name}"));
                    close = true;
                }
            }
            return close;
        }
        skin::section(ui, "Audio tracks", p);
        let tracks = self
            .now
            .as_ref()
            .map(|n| n.audio_tracks.clone())
            .unwrap_or_default();
        if tracks.is_empty() {
            ui.label(egui::RichText::new("No audio tracks reported yet").size(11.0).color(p.dim));
        }
        for (i, (_, name)) in tracks.iter().enumerate() {
            let active = self.now.as_ref().map(|n| n.audio_track_index == i).unwrap_or(false);
            if ui.selectable_label(active, name).clicked() {
                if let Some(now) = &mut self.now {
                    now.audio_track_index = i;
                }
                if let (Some(engine), Some(now)) = (&self.engine, &self.now) {
                    if let Some((id, _)) = now.audio_tracks.get(i) {
                        engine.set_audio_track(*id);
                    }
                }
                self.toast_now(format!("Audio track: {name}"));
                close = true;
            }
        }
        skin::divider(ui, p);
        skin::section(ui, "Output device", p);
        let device_count = self
            .engine
            .as_ref()
            .map(|e| e.audio_devices().len())
            .unwrap_or(0);
        let options = self.audio_choices();
        for (label, value) in options {
            let active = self.settings.audio_output == value;
            if ui.selectable_label(active, &label).clicked() {
                self.set_audio_choice(&value);
                close = true;
            }
        }
        if device_count == 0 {
            ui.label(
                egui::RichText::new(
                    "No device list from the current output yet — these are output modules. Devices appear once playback starts.",
                )
                .size(9.5)
                .color(p.dim),
            );
        }
        close
    }

    fn video_popover(&mut self, ui: &mut Ui, p: &Palette) -> bool {
        let mut close = false;
        skin::section(ui, "Video tracks", p);
        let tracks = self
            .now
            .as_ref()
            .map(|n| n.video_tracks.clone())
            .unwrap_or_default();
        if tracks.is_empty() {
            ui.label(egui::RichText::new("No video tracks reported yet").size(11.0).color(p.dim));
        }
        for (i, (_, name)) in tracks.iter().enumerate() {
            let active = self.now.as_ref().map(|n| n.video_track_index == i).unwrap_or(false);
            if ui.selectable_label(active, name).clicked() {
                if let Some(now) = &mut self.now {
                    now.video_track_index = i;
                }
                if let (Some(engine), Some(now)) = (&self.engine, &self.now) {
                    if let Some((id, _)) = now.video_tracks.get(i) {
                        engine.set_video_track(*id);
                    }
                }
                self.toast_now(format!("Video track: {name}"));
                close = true;
            }
        }
        skin::divider(ui, p);
        ui.label(
            egui::RichText::new("Video tracks live here and in the right-click menu — not on the bar.")
                .size(10.5)
                .color(p.dim),
        );
        close
    }

    pub fn toggle_pip(&mut self, ctx: &egui::Context) {
        // A compact always-on-top window; toggling restores the normal size.
        let size = ctx.screen_rect().size();
        let (w, h) = if size.x > 900.0 { (460.0, 300.0) } else { (1100.0, 720.0) };
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(w, h)));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            if w < 600.0 {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            },
        ));
        self.toast_now(if w < 600.0 { "Picture-in-picture" } else { "Back to the main window" });
    }
}

impl Palette {
    pub fn accent_text_or_dark(&self) -> Color32 {
        if self.light {
            Color32::WHITE
        } else {
            Color32::from_rgb(22, 14, 6)
        }
    }
    pub fn danger_or(&self, _p: &Palette) -> Color32 {
        self.bad
    }
}
