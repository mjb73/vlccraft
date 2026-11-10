//! Dialogs, onboarding, profiles, the shortcut editor, and on-screen status.

use crate::app::{App, Dialog, SettingsCategory, View};
use crate::model::{
    data_dir, find_subtitle, format_time, match_folder_subtitles, new_id, now_ms, parse_m3u,
    playlist_to_m3u, plural, Bookmark, Preset, Profile, SettingsPatch,
};
use crate::skin::{self, Icon, Palette};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use std::path::{Path, PathBuf};

impl App {
    pub fn status_overlays(&mut self, ctx: &egui::Context, p: &Palette) {
        // Toast: names the action and the shortcut that ran it (DSC-3).
        if let Some(toast) = &self.toast {
            let screen = ctx.screen_rect();
            let y = screen.bottom() - 96.0;
            egui::Area::new(Id::new("toast"))
                .order(egui::Order::Foreground)
                .fixed_pos(pos2(screen.center().x - 170.0, y))
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(p.panel)
                        .stroke(Stroke::new(1.0, p.line))
                        .corner_radius(CornerRadius::same(9))
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 6],
                            blur: 18,
                            spread: 0,
                            color: Color32::from_black_alpha(120),
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.painter().circle_filled(
                                    pos2(ui.cursor().min.x + 3.0, ui.cursor().min.y + 9.0),
                                    3.0,
                                    p.accent,
                                );
                                ui.add_space(10.0);
                                ui.label(
                                    egui::RichText::new(&toast.text).size(11.5).color(p.text),
                                );
                            });
                        });
                });
        }

        // File drops on the player: left half plays now, right half queues.
        let hovered = ctx.input(|i| i.raw.hovered_files.clone());
        if !hovered.is_empty() && self.video_rect.width() > 1.0 {
            let r = self.video_rect;
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                Id::new("drop-layer"),
            ));
            painter.rect_filled(r, CornerRadius::ZERO, Color32::from_black_alpha(120));
            let half = Rect::from_min_max(r.min, pos2(r.center().x, r.max.y)).shrink(18.0);
            let other = Rect::from_min_max(pos2(r.center().x, r.min.y), r.max).shrink(18.0);
            for (rect, title, body) in [
                (half, "Drop to play now", "Replaces what is playing"),
                (other, "Drop to add to the queue", "Keeps this playing"),
            ] {
                painter.rect_filled(rect, CornerRadius::same(12), Color32::from_white_alpha(14));
                painter.rect_stroke(
                    rect,
                    CornerRadius::same(12),
                    Stroke::new(1.0, Color32::from_white_alpha(90)),
                    StrokeKind::Inside,
                );
                painter.text(
                    pos2(rect.center().x, rect.center().y - 8.0),
                    Align2::CENTER_CENTER,
                    title,
                    skin::font(15.0),
                    Color32::WHITE,
                );
                painter.text(
                    pos2(rect.center().x, rect.center().y + 14.0),
                    Align2::CENTER_CENTER,
                    body,
                    skin::font(10.5),
                    Color32::from_white_alpha(180),
                );
            }
        }
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            let mut paths: Vec<String> = dropped
                .iter()
                .filter_map(|f| f.path.as_ref().map(|p| p.to_string_lossy().to_string()))
                .collect();
            if !paths.is_empty() {
                let play_now = ctx
                    .input(|i| i.pointer.hover_pos())
                    .map(|pos| self.video_rect.width() > 1.0 && pos.x < self.video_rect.center().x)
                    .unwrap_or(false);
                if play_now {
                    let first = paths.remove(0);
                    self.queue.items.insert(self.queue.index + 1, first.clone());
                    self.open_path(&first, true);
                    if !paths.is_empty() {
                        self.enqueue(paths);
                    }
                } else {
                    self.enqueue(paths);
                }
            }
        }

        if let Some(err) = &self.engine_error {
            if self.view == View::Library {
                let screen = ctx.screen_rect();
                egui::Area::new(Id::new("engine-error"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(pos2(screen.center().x - 260.0, screen.bottom() - 52.0))
                    .show(ctx, |ui| {
                        egui::Frame::new()
                            .fill(p.panel)
                            .stroke(Stroke::new(1.0, p.bad))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(egui::Margin::symmetric(12, 7))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "VLC playback is unavailable — the library still works. {err}"
                                    ))
                                    .size(10.5)
                                    .color(p.muted),
                                );
                            });
                    });
            }
        }
    }

    pub fn dialogs(&mut self, ctx: &egui::Context, p: &Palette) {
        let Some(dialog) = self.dialog.clone() else { return };
        match dialog {
            Dialog::OpenFile => self.dlg_open_file(ctx, p),
            Dialog::OpenFolder => self.dlg_open_folder(ctx, p),
            Dialog::OpenUrl => self.dlg_open_url(ctx, p),
            Dialog::OpenDisc => self.dlg_open_disc(ctx, p),
            Dialog::Capture => self.dlg_capture(ctx, p),
            Dialog::NewPlaylist => self.dlg_new_playlist(ctx, p),
            Dialog::AddToPlaylist(path) => self.dlg_add_to_playlist(ctx, p, &path),
            Dialog::EditPlaylist(id) => self.dlg_edit_playlist(ctx, p, &id),
            Dialog::SubtitleBatch => self.dlg_subtitle_batch(ctx, p),
            Dialog::SubtitleSearch => self.dlg_subtitle_search(ctx, p),
            Dialog::Convert => self.dlg_convert(ctx, p),
            Dialog::StreamOut => self.dlg_stream(ctx, p),
            Dialog::Presets => self.dlg_presets(ctx, p),
            Dialog::Hotkeys => self.dlg_hotkeys(ctx, p),
            Dialog::Customise => self.dlg_customise(ctx, p),
            Dialog::Relink(path) => self.dlg_relink(ctx, p, &path),
            Dialog::About => self.dlg_about(ctx, p),
            Dialog::Onboarding => self.dlg_onboarding(ctx, p),
        }
    }

    // -----------------------------------------------------------------------
    // Open dialogs
    // -----------------------------------------------------------------------

    fn dlg_open_file(&mut self, ctx: &egui::Context, p: &Palette) {
        shell(ctx, p, "Open a file", 620.0, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("File").size(11.0).color(p.muted));
                ui.add(
                    egui::TextEdit::singleline(&mut self.dlg_text)
                        .hint_text("/path/to/video.mp4")
                        .desired_width(400.0),
                );
                if skin::primary(ui, "Open", p).clicked() {
                    let path = self.dlg_text.clone();
                    self.open_media_file(&path);
                }
            });
            if let Some(msg) = &self.dlg_msg {
                ui.label(egui::RichText::new(msg).size(10.5).color(p.bad));
            }
            ui.add_space(6.0);
            self.browser(ui, p, false);
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Single job: choose a file. Network, disc and devices are their own dialogs.")
                    .size(10.0)
                    .color(p.dim),
            );
        });
    }

    fn dlg_open_folder(&mut self, ctx: &egui::Context, p: &Palette) {
        shell(ctx, p, "Add a folder to the library", 620.0, |ui| {
            ui.label(
                egui::RichText::new("It is indexed recursively, in one action. Shows inside get their own group.")
                    .size(10.5)
                    .color(p.dim),
            );
            ui.add_space(4.0);
            self.browser(ui, p, true);
            if let Some(msg) = &self.dlg_msg {
                ui.label(egui::RichText::new(msg).size(10.5).color(p.bad));
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if skin::primary(ui, "Use this folder", p).clicked() {
                    let dir = self.browse_dir.clone();
                    self.add_folder(&dir);
                }
                if skin::ghost(ui, "Cancel", p).clicked() {
                    self.dialog = None;
                }
            });
        });
    }

    fn browser(&mut self, ui: &mut Ui, p: &Palette, folder_mode: bool) {
        let dir = PathBuf::from(self.browse_dir.clone());
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(truncate(&dir.to_string_lossy(), 62))
                    .size(10.0)
                    .color(p.muted)
                    .monospace(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if skin::chip(ui, "Home", false, p).clicked() {
                    self.browse_dir = crate::platform::browse_start().to_string_lossy().to_string();
                }
                if skin::chip(ui, "Up", false, p).clicked() {
                    if let Some(parent) = dir.parent() {
                        self.browse_dir = parent.to_string_lossy().to_string();
                    }
                }
            });
        });
        egui::Frame::new()
            .fill(p.video_bg)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(6))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(230.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut entries: Vec<(bool, String, PathBuf)> = Vec::new();
                        if let Ok(read) = std::fs::read_dir(&dir) {
                            for entry in read.flatten() {
                                let path = entry.path();
                                // A name that is not text cannot be shown in a
                                // list without turning into boxes; skip it here
                                // rather than print gibberish.
                                let Ok(name) = entry.file_name().into_string() else {
                                    continue;
                                };
                                if name.starts_with('.') {
                                    continue;
                                }
                                let is_dir = path.is_dir();
                                if is_dir {
                                    entries.push((true, name, path));
                                } else {
                                    let ext = path
                                        .extension()
                                        .map(|e| e.to_string_lossy().to_lowercase())
                                        .unwrap_or_default();
                                    if ["mp4", "mkv", "avi", "mov", "webm", "mp3", "flac", "ogg", "wav", "m4a", "m3u", "xspf"]
                                        .contains(&ext.as_str())
                                    {
                                        entries.push((false, name, path));
                                    }
                                }
                            }
                        }
                        entries.sort_by(|a, b| (b.0, a.1.to_lowercase()).cmp(&(a.0, b.1.to_lowercase())));
                        if entries.is_empty() {
                            ui.label(egui::RichText::new("Nothing playable here.").size(10.5).color(p.dim));
                        }
                        let mut go: Option<PathBuf> = None;
                        for (is_dir, name, path) in entries {
                            let (rect, resp) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 24.0),
                                Sense::click(),
                            );
                            if resp.hovered() {
                                ui.painter().rect_filled(rect, CornerRadius::same(4), p.hover);
                            }
                            skin::icon(
                                ui.painter(),
                                if is_dir { Icon::Folder } else { Icon::Film },
                                Rect::from_min_size(rect.min + vec2(4.0, 5.0), egui::Vec2::splat(14.0)),
                                if is_dir { p.muted } else { p.accent },
                                1.2,
                            );
                            ui.painter().text(
                                pos2(rect.left() + 26.0, rect.center().y),
                                Align2::LEFT_CENTER,
                                truncate(&name, 70),
                                skin::font(11.0),
                                p.text,
                            );
                            if resp.clicked() {
                                go = Some(path);
                            }
                        }
                        if let Some(path) = go {
                            if path.is_dir() {
                                self.browse_dir = path.to_string_lossy().to_string();
                            } else if folder_mode {
                                self.browse_dir = path
                                    .parent()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or(self.browse_dir.clone());
                            } else {
                                self.dlg_text = path.to_string_lossy().to_string();
                                self.open_media_file(&self.dlg_text.clone());
                            }
                        }
                    });
            });
    }

    fn dlg_open_url(&mut self, ctx: &egui::Context, p: &Palette) {
        shell(ctx, p, "Open a network stream", 560.0, |ui| {
            ui.label(
                egui::RichText::new("A media URL, an .m3u/.xspf link, or a playlist body served as text.")
                    .size(10.5)
                    .color(p.dim),
            );
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.dlg_text)
                    .hint_text("https://…")
                    .desired_width(480.0),
            );
            ui.add_space(4.0);
            if let Some(msg) = &self.dlg_msg {
                ui.label(egui::RichText::new(msg).size(10.5).color(p.bad));
            }
            ui.horizontal(|ui| {
                if skin::primary(ui, "Open", p).clicked() {
                    if self.dlg_text.trim().is_empty() {
                        self.dlg_msg = Some("Paste a link first.".into());
                    } else {
                        let url = self.dlg_text.clone();
                        self.open_url(&url);
                    }
                }
                if skin::ghost(ui, "Cancel", p).clicked() {
                    self.dialog = None;
                }
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "A link that answers with a playlist fills the queue; anything else opens as one stream. If the link cannot be read, playback still opens it with VLC.",
                )
                .size(10.0)
                .color(p.dim),
            );
        });
    }

    fn dlg_open_disc(&mut self, ctx: &egui::Context, p: &Palette) {
        if self.dlg_text.is_empty() {
            self.dlg_text = "/dev/sr0".into();
        }
        shell(ctx, p, "Open a disc", 460.0, |ui| {
            ui.label(egui::RichText::new("Device").size(11.0).color(p.muted));
            ui.add(egui::TextEdit::singleline(&mut self.dlg_text).desired_width(340.0));
            ui.label(
                egui::RichText::new("DVD, audio CD and Blu-ray are one job each; convert and stream-out live under Tools.")
                    .size(10.0)
                    .color(p.dim),
            );
            ui.horizontal(|ui| {
                if skin::primary(ui, "Open disc", p).clicked() {
                    let dev = self.dlg_text.clone();
                    self.dialog = None;
                    self.open_path(&format!("dvd://{dev}"), true);
                }
                if skin::ghost(ui, "Cancel", p).clicked() {
                    self.dialog = None;
                }
            });
        });
    }

    fn dlg_capture(&mut self, ctx: &egui::Context, p: &Palette) {
        if self.dlg_text.is_empty() {
            self.dlg_text = "/dev/video0".into();
        }
        shell(ctx, p, "Open a capture device", 460.0, |ui| {
            ui.label(
                egui::RichText::new("Webcam, capture card or screen. This is its own dialog, not a tab in a media factory.")
                    .size(10.5)
                    .color(p.dim),
            );
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("capture-dev")
                    .selected_text(&self.dlg_text)
                    .show_ui(ui, |ui| {
                        for dev in ["/dev/video0", "/dev/video1", "/dev/video2"] {
                            ui.selectable_value(&mut self.dlg_text, dev.to_string(), dev);
                        }
                    });
            });
            ui.horizontal(|ui| {
                if skin::primary(ui, "Open device", p).clicked() {
                    let dev = self.dlg_text.clone();
                    self.dialog = None;
                    self.open_path(&format!("v4l2://{dev}"), true);
                }
                if skin::ghost(ui, "Cancel", p).clicked() {
                    self.dialog = None;
                }
            });
        });
    }

    // -----------------------------------------------------------------------
    // Playlists
    // -----------------------------------------------------------------------

    fn dlg_new_playlist(&mut self, ctx: &egui::Context, p: &Palette) {
        shell(ctx, p, "New playlist", 480.0, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Name").size(11.0).color(p.muted));
                ui.add(
                    egui::TextEdit::singleline(&mut self.dlg_text)
                        .hint_text("Sunday films")
                        .desired_width(280.0),
                );
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Pick titles now, or leave it empty and add from any item later.")
                    .size(10.0)
                    .color(p.dim),
            );
            ui.add_space(4.0);
            let items: Vec<(String, String)> = self
                .library
                .items
                .iter()
                .take(10)
                .map(|i| (i.path.clone(), i.title.clone()))
                .collect();
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    for (path, title) in items {
                        let mut on = self.dlg_sel.contains(&path);
                        if ui.checkbox(&mut on, truncate(&title, 56)).changed() {
                            if on {
                                self.dlg_sel.push(path);
                            } else {
                                self.dlg_sel.retain(|p| p != &path);
                            }
                        }
                    }
                });
            if let Some(msg) = &self.dlg_msg {
                ui.label(egui::RichText::new(msg).size(10.5).color(p.bad));
            }
            ui.horizontal(|ui| {
                if skin::primary(ui, "Create playlist", p).clicked() {
                    let name = self.dlg_text.trim().to_string();
                    if name.is_empty() {
                        self.dlg_msg = Some("Give the playlist a name.".into());
                    } else {
                        self.playlists.push(crate::model::Playlist {
                            id: new_id("pl"),
                            name,
                            items: self.dlg_sel.clone(),
                            created: now_ms(),
                            modified: now_ms(),
                        });
                        self.dlg_text.clear();
                        self.dlg_sel.clear();
                        self.dialog = None;
                        self.section = crate::app::Section::Playlists;
                        self.touch();
                        self.save_all();
                        self.toast_now("Playlist created");
                    }
                }
                if skin::ghost(ui, "Cancel", p).clicked() {
                    self.dialog = None;
                }
            });
        });
    }

    fn dlg_add_to_playlist(&mut self, ctx: &egui::Context, p: &Palette, path: &str) {
        let title = self.library.title_for(path);
        shell(ctx, p, "Add to playlist", 420.0, |ui| {
            ui.label(
                egui::RichText::new(format!("“{}”", truncate(&title, 40)))
                    .size(12.0)
                    .color(p.text),
            );
            ui.add_space(4.0);
            let playlists = self.playlists.clone();
            if playlists.is_empty() {
                ui.label(
                    egui::RichText::new("No playlists yet — make one here.")
                        .size(10.5)
                        .color(p.dim),
                );
            }
            for pl in &playlists {
                if ui
                    .selectable_label(pl.items.contains(&path.to_string()), format!("{} · {}", pl.name, pl.items.len()))
                    .clicked()
                {
                    if let Some(target) = self.playlists.iter_mut().find(|x| x.id == pl.id) {
                        if !target.items.contains(&path.to_string()) {
                            target.items.push(path.to_string());
                            target.modified = now_ms();
                        }
                    }
                    self.touch();
                    self.save_all();
                    self.toast_now(format!("Added to “{}”", pl.name));
                    self.dialog = None;
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if skin::ghost(ui, "+ New playlist…", p).clicked() {
                    self.dlg_text.clear();
                    self.dlg_sel = vec![path.to_string()];
                    self.dialog = Some(Dialog::NewPlaylist);
                }
                if skin::ghost(ui, "Cancel", p).clicked() {
                    self.dialog = None;
                }
            });
        });
    }

    fn dlg_edit_playlist(&mut self, ctx: &egui::Context, p: &Palette, id: &str) {
        shell(ctx, p, "Rename playlist", 380.0, |ui| {
            ui.add(egui::TextEdit::singleline(&mut self.dlg_text).desired_width(300.0));
            ui.horizontal(|ui| {
                if skin::primary(ui, "Rename", p).clicked() {
                    let name = self.dlg_text.trim().to_string();
                    if let Some(pl) = self.playlists.iter_mut().find(|x| x.id == id) {
                        if !name.is_empty() {
                            pl.name = name;
                            pl.modified = now_ms();
                        }
                    }
                    self.touch();
                    self.save_all();
                    self.dialog = None;
                }
                if skin::ghost(ui, "Cancel", p).clicked() {
                    self.dialog = None;
                }
            });
        });
    }

    // -----------------------------------------------------------------------
    // Subtitles
    // -----------------------------------------------------------------------

    fn dlg_subtitle_batch(&mut self, ctx: &egui::Context, p: &Palette) {
        let current_folder = self
            .now
            .as_ref()
            .and_then(|n| Path::new(&n.path).parent().map(|x| x.to_path_buf()));
        let folder = current_folder.clone().unwrap_or_else(|| PathBuf::from(&self.browse_dir));
        let report = match_folder_subtitles(&folder);
        shell(ctx, p, "Add subtitles", 620.0, |ui| {
            ui.horizontal(|ui| {
                for (id, label) in [("file", "This file"), ("folder", "This folder"), ("playlist", "This playlist")] {
                    if skin::chip(ui, label, self.subtitle_scope == id, p).clicked() {
                        self.subtitle_scope = id.into();
                    }
                }
            });
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("Matching “{}”", folder.display()))
                    .size(10.0)
                    .color(p.dim)
                    .monospace(),
            );
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(p.video_bg)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            skin::section(ui, &format!("Matched · {}", report.matched.len()), p);
                            for (media, sub) in &report.matched {
                                ui.horizontal(|ui| {
                                    skin::icon(ui.painter(), Icon::Check, Rect::from_min_size(ui.cursor().min + vec2(0.0, 3.0), egui::Vec2::splat(11.0)), p.good, 1.3);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}  pairs with  {}",
                                            file_name(media),
                                            file_name(sub)
                                        ))
                                        .size(10.0)
                                        .color(p.muted),
                                    );
                                });
                            }
                            if !report.unmatched_media.is_empty() {
                                skin::section(ui, &format!("No match · {}", report.unmatched_media.len()), p);
                                for m in &report.unmatched_media {
                                    ui.label(
                                        egui::RichText::new(format!("{} — choose one below", file_name(m)))
                                            .size(10.0)
                                            .color(p.warn_or(p)),
                                    );
                                }
                            }
                            if !report.ambiguous.is_empty() {
                                skin::section(ui, &format!("Ambiguous · {}", report.ambiguous.len()), p);
                                for m in &report.ambiguous {
                                    ui.label(
                                        egui::RichText::new(format!("{} — more than one candidate", file_name(m)))
                                            .size(10.0)
                                            .color(p.warn_or(p)),
                                    );
                                }
                                for m in &report.ambiguous {
                                    if let Some(path) = Path::new(m).to_str() {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(egui::RichText::new(file_name(m)).size(9.5).color(p.dim));
                                            for c in find_subtitle(Path::new(path), &self.settings.subtitle_precision).candidates {
                                                if skin::chip(ui, &file_name(&c.to_string_lossy()), false, p).clicked() {
                                                    if let Some(item) = self.library.get_mut(path) {
                                                        item.subtitle_path = Some(c.to_string_lossy().to_string());
                                                    }
                                                    self.touch();
                                                    self.save_library();
                                                    self.toast_now("Subtitle chosen for that item");
                                                }
                                            }
                                        });
                                    }
                                }
                            }
                            if report.matched.is_empty() && report.unmatched_media.is_empty() {
                                ui.label(
                                    egui::RichText::new("No videos found in this folder.")
                                        .size(10.5)
                                        .color(p.dim),
                                );
                            }
                        });
                });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if skin::primary(ui, "Apply matches", p).clicked() {
                    let mut applied = 0;
                    match self.subtitle_scope.as_str() {
                        "file" => {
                            if let Some(now) = &self.now {
                                let path = now.path.clone();
                                let found = find_subtitle(Path::new(&path), &self.settings.subtitle_precision);
                                if let Some(sub) = found.chosen {
                                    self.select_subtitle(Some(sub));
                                    applied = 1;
                                }
                            }
                        }
                        "playlist" => {
                            let paths: Vec<String> = self
                                .playlists
                                .iter()
                                .find(|pl| pl.items.iter().any(|p| self.now.as_ref().map(|n| &n.path == p).unwrap_or(false)))
                                .map(|pl| pl.items.clone())
                                .unwrap_or_default();
                            for path in paths {
                                let found = find_subtitle(Path::new(&path), &self.settings.subtitle_precision);
                                if let Some(sub) = found.chosen {
                                    if let Some(item) = self.library.get_mut(&path) {
                                        item.subtitle_path = Some(sub.to_string_lossy().to_string());
                                        applied += 1;
                                    }
                                }
                            }
                        }
                        _ => {
                            for (media, sub) in &report.matched {
                                if let Some(item) = self.library.get_mut(media) {
                                    item.subtitle_path = Some(sub.clone());
                                    applied += 1;
                                }
                            }
                        }
                    }
                    self.touch();
                    self.save_library();
                    self.toast_now(format!("{applied} subtitle file(s) attached"));
                    self.dialog = None;
                }
                if skin::ghost(ui, "Close", p).clicked() {
                    self.dialog = None;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("Matched by filename; nothing is committed until Apply.")
                            .size(9.5)
                            .color(p.dim),
                    );
                });
            });
        });
    }

    fn dlg_subtitle_search(&mut self, ctx: &egui::Context, p: &Palette) {
        shell(ctx, p, "Find subtitles online", 500.0, |ui| {
            ui.label(
                egui::RichText::new("OpenSubtitles needs an API key of your own — it cannot ship in the app.")
                    .size(10.5)
                    .color(p.dim),
            );
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("API key").size(11.0).color(p.muted));
                ui.add(
                    egui::TextEdit::singleline(&mut self.dlg_text2)
                        .password(true)
                        .hint_text("kept for this session only")
                        .desired_width(240.0),
                );
            });
            ui.add_space(4.0);
            if !self.settings.subtitle_online_enabled {
                ui.label(
                    egui::RichText::new("Online lookup is off in Settings, under Privacy.")
                        .size(10.5)
                        .color(p.warn_or(p)),
                );
            }
            if let Some(msg) = &self.dlg_msg {
                ui.label(egui::RichText::new(msg).size(10.5).color(p.muted));
            }
            ui.horizontal(|ui| {
                if skin::primary(ui, "Search", p).clicked() {
                    self.dlg_msg = Some(
                        "No request was sent: this build has no provider client. Playback and your subtitles are untouched."
                            .into(),
                    );
                }
                if skin::ghost(ui, "Close", p).clicked() {
                    self.dialog = None;
                }
            });
            ui.label(
                egui::RichText::new("Fail-open by design: no network, no key or a rate limit changes nothing on screen.")
                    .size(9.5)
                    .color(p.dim),
            );
        });
    }

    // -----------------------------------------------------------------------
    // Convert and stream out
    // -----------------------------------------------------------------------

    fn dlg_convert(&mut self, ctx: &egui::Context, p: &Palette) {
        shell(ctx, p, "Convert to a new file", 640.0, |ui| {
            ui.label(
                egui::RichText::new(
                    "One job: a copy in another format. The original is read once and never written.",
                )
                .size(10.5)
                .color(p.dim),
            );
            ui.add_space(8.0);
            skin::section(ui, "From", p);
            ui.add(
                egui::TextEdit::singleline(&mut self.convert_in)
                    .hint_text("/path/to/video.mp4")
                    .desired_width(ui.available_width()),
            );
            if !self.convert_in.trim().is_empty() {
                ui.label(
                    egui::RichText::new(format!(
                        "File · {}",
                        crate::tools::file_label(&self.convert_in)
                    ))
                    .size(9.5)
                    .color(p.muted),
                );
            }
            ui.add_space(8.0);
            skin::section(ui, "Format held in the file", p);
            let presets = crate::tools::conversion_presets();
            let mut chosen = self.convert_preset.min(presets.len().saturating_sub(1));
            for (index, preset) in presets.iter().enumerate() {
                let selected = chosen == index;
                ui.horizontal(|ui| {
                    if skin::chip(ui, &preset.name, selected, p).clicked() {
                        chosen = index;
                    }
                    ui.label(
                        egui::RichText::new(&preset.detail)
                            .size(9.5)
                            .color(if selected { p.muted } else { p.dim }),
                    );
                });
            }
            if chosen != self.convert_preset {
                self.convert_preset = chosen;
                if self.convert_out.trim().is_empty()
                    || self.convert_out.contains("(converted)")
                {
                    self.convert_out = crate::model::suggested_convert_path(
                        &self.convert_in,
                        presets[chosen].ext,
                    );
                }
            }
            ui.add_space(8.0);
            skin::section(ui, "Save as", p);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.convert_out)
                        .hint_text("/path/to/new file.mp4")
                        .desired_width(ui.available_width() - 96.0),
                );
                if skin::ghost(ui, "Suggest", p).clicked() {
                    self.convert_out = crate::model::suggested_convert_path(
                        &self.convert_in,
                        presets[chosen].ext,
                    );
                }
            });
            if !self.convert_out.trim().is_empty() {
                ui.label(
                    egui::RichText::new(format!(
                        "New file · {}",
                        crate::tools::file_label(&self.convert_out)
                    ))
                    .size(9.5)
                    .color(p.muted),
                );
            }
            ui.add_space(6.0);

            if let Some(msg) = &self.dlg_msg {
                ui.label(egui::RichText::new(msg).size(10.5).color(p.bad));
            }

            if let Some(job) = &self.job {
                ui.add_space(4.0);
                egui::Frame::new()
                    .fill(p.panel2)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("VLC is working in the background")
                                .size(11.0)
                                .color(p.text),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {} s elapsed · you can keep watching",
                                job.label,
                                job.elapsed_secs()
                            ))
                            .size(10.0)
                            .color(p.dim),
                        );
                    });
                ui.add_space(6.0);
            }

            ui.horizontal(|ui| {
                let busy = self.job.is_some();
                if busy {
                    if skin::primary(ui, "Stop the job", p).clicked() {
                        self.stop_job();
                    }
                } else if skin::primary(ui, "Start converting", p).clicked() {
                    let preset = crate::tools::conversion_presets()
                        .into_iter()
                        .nth(self.convert_preset)
                        .unwrap();
                    match crate::tools::start_convert(
                        &self.convert_in,
                        &self.convert_out,
                        &preset,
                    ) {
                        Ok(job) => {
                            self.job = Some(job);
                            self.dlg_msg = None;
                            self.toast_now("Converting in the background — playback is untouched");
                        }
                        Err(e) => self.dlg_msg = Some(e),
                    }
                }
                if skin::ghost(ui, "Close", p).clicked() {
                    self.dialog = None;
                }
                ui.label(
                    egui::RichText::new("Runs the installed VLC as a separate process")
                        .size(9.5)
                        .color(p.dim),
                );
            });
        });
    }

    fn dlg_stream(&mut self, ctx: &egui::Context, p: &Palette) {
        shell(ctx, p, "Stream out to the network", 620.0, |ui| {
            ui.label(
                egui::RichText::new(
                    "One job: a local address other devices can open. Nothing is uploaded to a service.",
                )
                .size(10.5)
                .color(p.dim),
            );
            ui.add_space(8.0);
            skin::section(ui, "What to stream", p);
            ui.add(
                egui::TextEdit::singleline(&mut self.stream_in)
                    .hint_text("A file, or an address you already play")
                    .desired_width(ui.available_width()),
            );
            if self.stream_in.contains("://") {
                let shown = self.stream_in.clone();
                ui.label(
                    egui::RichText::new(format!("Link · {shown}"))
                        .size(9.5)
                        .color(p.muted),
                );
            } else if !self.stream_in.trim().is_empty() {
                ui.label(
                    egui::RichText::new(format!(
                        "File · {}",
                        crate::tools::file_label(&self.stream_in)
                    ))
                    .size(9.5)
                    .color(p.muted),
                );
            }
            ui.add_space(8.0);
            skin::section(ui, "Port", p);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.stream_port)
                        .hint_text("8080")
                        .desired_width(90.0),
                );
                if skin::chip(ui, "Shrink to 2.5 Mbit/s H.264", self.stream_transcode, p).clicked() {
                    self.stream_transcode = !self.stream_transcode;
                }
                ui.label(
                    egui::RichText::new(if self.stream_transcode {
                        "Smaller stream; the picture keeps its shape"
                    } else {
                        "Off keeps the original quality"
                    })
                    .size(9.5)
                    .color(p.dim),
                );
            });
            ui.add_space(6.0);
            if let Some(msg) = &self.dlg_msg {
                ui.label(egui::RichText::new(msg).size(10.5).color(p.bad));
            }

            if let Some(job) = &self.job {
                ui.add_space(4.0);
                egui::Frame::new()
                    .fill(p.panel2)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Live now")
                                .size(11.0)
                                .color(p.accent),
                        );
                        if let Some(url) = &job.url {
                            ui.label(
                                egui::RichText::new(url)
                                    .size(13.0)
                                    .monospace()
                                    .color(p.text),
                            );
                        }
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · running for {} s",
                                job.label,
                                job.elapsed_secs()
                            ))
                            .size(9.5)
                            .color(p.dim),
                        );
                    });
                ui.add_space(6.0);
            }

            ui.horizontal(|ui| {
                if self.job.is_some() {
                    if skin::primary(ui, "Stop streaming", p).clicked() {
                        self.stop_job();
                    }
                } else if skin::primary(ui, "Start streaming", p).clicked() {
                    let port: u16 = self.stream_port.trim().parse().unwrap_or(0);
                    if port < 1024 {
                        self.dlg_msg =
                            Some("Use a port from 1024 to 65535 — below 1024 needs administrator rights".into());
                    } else {
                        match crate::tools::start_stream(
                            &self.stream_in,
                            port,
                            self.stream_transcode,
                        ) {
                            Ok(job) => {
                                self.job = Some(job);
                                self.dlg_msg = None;
                                self.toast_now(format!("Streaming on port {port}"));
                            }
                            Err(e) => self.dlg_msg = Some(e),
                        }
                    }
                }
                if skin::ghost(ui, "Close", p).clicked() {
                    self.dialog = None;
                }
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "The stream repeats the file. Open the address on a TV, phone or another computer on the same network.",
                )
                .size(9.5)
                .color(p.dim),
            );
        });
    }

    fn dlg_presets(&mut self, ctx: &egui::Context, p: &Palette) {
        shell(ctx, p, "Subtitle and audio presets", 560.0, |ui| {
            ui.label(
                egui::RichText::new("A preset stores delay, style, preferred tracks and filter for a folder or show.")
                    .size(10.5)
                    .color(p.dim),
            );
            ui.add_space(6.0);
            let presets = self.presets.clone();
            if presets.is_empty() {
                ui.label(egui::RichText::new("No presets yet.").size(11.0).color(p.dim));
            }
            for preset in presets {
                egui::Frame::new()
                    .fill(p.panel2)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&preset.name).size(12.0).color(p.text));
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} · delay {:+.0} ms · size {:.1}%",
                                        truncate(&preset.match_value, 46),
                                        preset.delay_ms as f32,
                                        preset.size_pct
                                    ))
                                    .size(9.5)
                                    .color(p.dim),
                                );
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if skin::icon_button(ui, Icon::Trash, 22.0, false, "Delete preset", p).clicked() {
                                    self.presets.retain(|x| x.id != preset.id);
                                    self.touch();
                                    self.save_all();
                                }
                                if skin::ghost(ui, "Apply", p).clicked() {
                                    if let Some(now) = &mut self.now {
                                        now.style.delay_ms = preset.delay_ms;
                                        now.style.size_pct = preset.size_pct;
                                        now.style.outline = preset.outline;
                                        now.style.position_pct = preset.position_pct;
                                        now.style.scale = preset.scale;
                                        now.preset_name = Some(preset.name.clone());
                                        self.toast_now(format!("Preset “{}” applied", preset.name));
                                    } else {
                                        self.toast_now("Open an episode first");
                                    }
                                }
                            });
                        });
                    });
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if skin::ghost(ui, "Save the current look…", p).clicked() {
                    self.preset_save_name = self
                        .now
                        .as_ref()
                        .map(|n| n.show.clone())
                        .unwrap_or_else(|| "Show preset".into());
                    self.dialog = None;
                    self.sidebar_open = true;
                    self.sidebar_tab = 0;
                    self.toast_now("Name the preset in the sidebar");
                }
                if skin::ghost(ui, "Close", p).clicked() {
                    self.dialog = None;
                }
            });
        });
    }

    // -----------------------------------------------------------------------
    // Shortcuts
    // -----------------------------------------------------------------------

    fn dlg_hotkeys(&mut self, ctx: &egui::Context, p: &Palette) {
        if let Some(action) = self.rebinding.clone() {
            if let Some(combo) = capture_combo(ctx) {
                self.settings.hotkeys.insert(action.clone(), combo.clone());
                self.rebinding = None;
                self.touch();
                self.toast_now(format!("{action} · {combo}"));
            } else if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.rebinding = None;
            }
        }
        let conflicts = hotkey_conflicts(&self.settings);
        shell(ctx, p, "Keyboard shortcuts", 620.0, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.hotkey_search)
                        .hint_text("Search actions or keys")
                        .desired_width(280.0),
                );
                if skin::ghost(ui, "Reset all", p).clicked() {
                    self.settings.hotkeys = crate::model::Settings::default().hotkeys;
                    self.touch();
                    self.toast_now("All shortcuts reset");
                }
            });
            ui.add_space(4.0);
            if !conflicts.is_empty() {
                ui.label(
                    egui::RichText::new(format!(
                        "{} conflict{}: {}",
                        conflicts.len(),
                        if conflicts.len() == 1 { "" } else { "s" },
                        conflicts.join(" · ")
                    ))
                    .size(10.0)
                    .color(p.warn_or(p)),
                );
            }
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    let actions: Vec<(String, String)> = self
                        .settings
                        .hotkeys
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    for (action, combo) in actions {
                        if !self.hotkey_search.trim().is_empty() {
                            let q = self.hotkey_search.to_lowercase();
                            if !action.to_lowercase().contains(&q) && !combo.to_lowercase().contains(&q) {
                                continue;
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&action).size(11.5).color(p.text));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if skin::icon_button(ui, Icon::Close, 20.0, false, "Reset this action", p).clicked() {
                                    self.settings.hotkeys.insert(action.clone(), String::new());
                                    self.touch();
                                }
                                let rebinding = self.rebinding.as_deref() == Some(action.as_str());
                                let label = if rebinding { "Press keys…" } else { &combo };
                                if skin::chip(ui, label, rebinding, p).clicked() {
                                    self.rebinding = Some(action.clone());
                                }
                                if conflicts.iter().any(|c| c == &combo) {
                                    skin::icon(ui.painter(), Icon::Info, Rect::from_min_size(ui.cursor().min, egui::Vec2::splat(12.0)), p.warn_or(p), 1.2);
                                }
                            });
                        });
                    }
                });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if skin::ghost(ui, "Done", p).clicked() {
                    self.rebinding = None;
                    self.dialog = None;
                }
                ui.label(
                    egui::RichText::new("Conflicts are detected live; Esc cancels a rebind, not the window.")
                        .size(9.5)
                        .color(p.dim),
                );
            });
        });
    }

    // -----------------------------------------------------------------------
    // Layout and profiles
    // -----------------------------------------------------------------------

    fn dlg_customise(&mut self, ctx: &egui::Context, p: &Palette) {
        shell_fixed(ctx, p, "Customise controls and profiles", 720.0, 600.0, |ui| {
            ui.label(
                egui::RichText::new("The default bar is complete. This editor is opt-in — nothing is hidden by default.")
                    .size(10.5)
                    .color(p.dim),
            );
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                // ---- left: the live bar preview and the ordered control list
                ui.vertical(|ui| {
                    ui.set_width(400.0);
                    let (preview, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
                    ui.painter().rect_filled(preview, CornerRadius::same(8), p.video_bg);
                    let mut x = preview.left() + 10.0;
                    for id in self.settings.layout.controls.clone() {
                        if let Some((_, label)) =
                            crate::model::ALL_CONTROLS.iter().find(|(k, _)| *k == id)
                        {
                            if x + 90.0 > preview.right() {
                                break;
                            }
                            let icon = icon_for_control(&id);
                            skin::icon(
                                ui.painter(),
                                icon,
                                Rect::from_center_size(
                                    pos2(x + 9.0, preview.center().y),
                                    egui::Vec2::splat(16.0),
                                ),
                                if id == "play" { p.accent } else { p.muted },
                                1.4,
                            );
                            ui.painter().text(
                                pos2(x + 20.0, preview.center().y),
                                Align2::LEFT_CENTER,
                                truncate(label, 10),
                                skin::font(9.0),
                                p.dim,
                            );
                            x += 82.0;
                        }
                    }
                    ui.add_space(6.0);
                    skin::section(ui, "Bar order", p);
                    egui::ScrollArea::vertical()
                        .id_salt("bar-order")
                        .max_height(380.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            let mut order = self.settings.layout.controls.clone();
                            let mut changed = false;
                            for (index, id) in order.clone().iter().enumerate() {
                                let label = crate::model::ALL_CONTROLS
                                    .iter()
                                    .find(|(k, _)| k == id)
                                    .map(|(_, v)| *v)
                                    .unwrap_or(id.as_str());
                                ui.horizontal(|ui| {
                                    ui.set_width(430.0);
                                    ui.label(
                                        egui::RichText::new(label).size(11.5).color(p.text),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let mut on = true;
                                            let cb = ui.checkbox(&mut on, "");
                                            cb.on_hover_text("Show on the control bar");
                                            if !on {
                                                order.remove(index);
                                                changed = true;
                                            }
                                            if skin::icon_button(
                                                ui, Icon::ChevronUp, 20.0, false, "Move up", p,
                                            )
                                            .clicked()
                                                && index > 0
                                            {
                                                order.swap(index, index - 1);
                                                changed = true;
                                            }
                                            if skin::icon_button(
                                                ui, Icon::ChevronDown, 20.0, false, "Move down", p,
                                            )
                                            .clicked()
                                                && index + 1 < order.len()
                                            {
                                                order.swap(index, index + 1);
                                                changed = true;
                                            }
                                        },
                                    );
                                });
                            }
                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                for (id, label) in crate::model::ALL_CONTROLS {
                                    if !order.iter().any(|c| c == id)
                                        && skin::chip(ui, &format!("+ {label}"), false, p)
                                            .clicked()
                                    {
                                        order.push((*id).to_string());
                                        changed = true;
                                    }
                                }
                            });
                            if changed {
                                self.settings.layout.controls = order;
                                self.touch();
                            }
                        });
                });
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);
                // ---- right: profiles, always visible, never behind the list
                ui.vertical(|ui| {
                    ui.set_width(240.0);
                    skin::section(ui, "Profiles", p);
                    ui.label(
                        egui::RichText::new("A profile keeps this bar and the look with it.")
                            .size(9.5)
                            .color(p.dim),
                    );
                    ui.add_space(2.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.profile_name)
                            .hint_text("Profile name")
                            .desired_width(216.0),
                    );
                    if skin::primary(ui, "Save profile", p).clicked() {
                        let name = if self.profile_name.trim().is_empty() {
                            format!("Profile {}", self.profiles.len() + 1)
                        } else {
                            self.profile_name.trim().to_string()
                        };
                        let profile = Profile {
                            name: name.clone(),
                            layout: self.settings.layout.clone(),
                            overrides: SettingsPatch {
                                theme: Some(self.settings.theme.clone()),
                                accent: Some(self.settings.accent),
                                volume: Some(self.settings.volume),
                                subtitle_size: Some(self.settings.subtitle_style.size_pct),
                                subtitle_outline: Some(self.settings.subtitle_style.outline),
                                subtitle_position: Some(
                                    self.settings.subtitle_style.position_pct,
                                ),
                            },
                            created: now_ms(),
                            schema_version: 2,
                        };
                        self.profiles.retain(|x| x.name != name);
                        self.profiles.push(profile);
                        self.settings.active_profile = name.clone();
                        self.touch();
                        self.save_all();
                        self.toast_now(format!("Profile “{name}” saved"));
                    }
                    ui.add_space(6.0);
                    egui::ScrollArea::vertical()
                        .id_salt("profiles")
                        .max_height(300.0)
                        .show(ui, |ui| {
                            let profiles = self.profiles.clone();
                            if profiles.is_empty() {
                                ui.label(
                                    egui::RichText::new("No profiles yet — the default bar is enough.")
                                        .size(10.0)
                                        .color(p.dim),
                                );
                            }
                            for profile in &profiles {
                                let active = self.settings.active_profile == profile.name;
                                egui::Frame::new()
                                    .fill(if active { p.accent_soft } else { p.panel2 })
                                    .corner_radius(CornerRadius::same(8))
                                    .inner_margin(egui::Margin::same(8))
                                    .show(ui, |ui| {
                                        ui.set_width(228.0);
                                        ui.label(
                                            egui::RichText::new(truncate(&profile.name, 24))
                                                .size(11.0)
                                                .color(if active { p.accent } else { p.text }),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} controls{}",
                                                profile.layout.controls.len(),
                                                if active { " · in use" } else { "" }
                                            ))
                                            .size(9.0)
                                            .color(p.dim),
                                        );
                                        ui.horizontal(|ui| {
                                            if skin::ghost(ui, "Apply", p).clicked() {
                                                self.apply_profile(&profile.name);
                                            }
                                            if skin::icon_button(
                                                ui, Icon::Trash, 20.0, false, "Delete profile", p,
                                            )
                                            .clicked()
                                            {
                                                self.profiles.retain(|x| x.name != profile.name);
                                                if self.settings.active_profile == profile.name {
                                                    self.settings.active_profile.clear();
                                                }
                                                self.touch();
                                                self.save_all();
                                            }
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                        });
                    ui.add_space(6.0);
                    skin::divider(ui, p);
                    if self.export_path.is_empty() {
                        self.export_path =
                            data_dir().join("profiles.json").to_string_lossy().to_string();
                    }
                    ui.label(
                        egui::RichText::new("One file for import and export")
                            .size(9.5)
                            .color(p.dim),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.export_path)
                            .desired_width(216.0)
                            .hint_text("profiles.json"),
                    );
                    ui.horizontal(|ui| {
                        if skin::ghost(ui, "Export", p).clicked() {
                            let path = PathBuf::from(&self.export_path);
                            let text =
                                serde_json::to_string_pretty(&self.profiles).unwrap_or_default();
                            if std::fs::write(&path, text).is_ok() {
                                self.toast_now("Profiles exported");
                            } else {
                                self.toast_now("Could not write that path");
                            }
                        }
                        if skin::ghost(ui, "Import", p).clicked() {
                            let path = PathBuf::from(&self.export_path);
                            if let Ok(text) = std::fs::read_to_string(&path) {
                                if let Ok(mut loaded) = serde_json::from_str::<Vec<Profile>>(&text)
                                {
                                    self.profiles.append(&mut loaded);
                                    self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
                                    self.profiles.dedup_by(|a, b| a.name == b.name);
                                    self.touch();
                                    self.save_all();
                                    self.toast_now("Profiles imported");
                                }
                            } else {
                                self.toast_now("No profile file at that path");
                            }
                        }
                    });
                    ui.add_space(4.0);
                    if skin::ghost(ui, "Done", p).clicked() {
                        self.dialog = None;
                    }
                });
            });
        });
    }

    fn apply_profile(&mut self, name: &str) {
        let Some(profile) = self.profiles.iter().find(|x| x.name == name).cloned() else {
            return;
        };
        self.settings.layout = profile.layout.clone();
        if let Some(theme) = &profile.overrides.theme {
            self.settings.theme = theme.clone();
        }
        if let Some(accent) = profile.overrides.accent {
            self.settings.accent = accent;
        }
        if let Some(volume) = profile.overrides.volume {
            self.settings.volume = volume;
        }
        if let Some(size) = profile.overrides.subtitle_size {
            self.settings.subtitle_style.size_pct = size;
            if let Some(now) = &mut self.now {
                now.style.size_pct = size;
            }
        }
        self.settings.active_profile = profile.name.clone();
        self.touch();
        self.save_all();
        self.toast_now(format!("Profile “{name}” applied"));
    }

    // -----------------------------------------------------------------------
    // Relink / about / onboarding
    // -----------------------------------------------------------------------

    fn dlg_relink(&mut self, ctx: &egui::Context, p: &Palette, old: &str) {
        let name = Path::new(old)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut candidates: Vec<PathBuf> = Vec::new();
        for folder in &self.library.folders {
            let mut found = Vec::new();
            crate::model::scan_folder(Path::new(folder), 0, &mut |path, _| {
                if path.file_name().map(|n| n.to_string_lossy() == name).unwrap_or(false) {
                    found.push(path.to_path_buf());
                }
            });
            candidates.extend(found);
        }
        shell(ctx, p, "Relink a moved file", 560.0, |ui| {
            ui.label(
                egui::RichText::new(format!("“{}” is not at its old path.", name))
                    .size(11.5)
                    .color(p.text),
            );
            ui.label(
                egui::RichText::new("Choose the file again; its position, subtitles and bookmarks stay.")
                    .size(10.0)
                    .color(p.dim),
            );
            ui.add_space(6.0);
            if candidates.is_empty() {
                ui.label(egui::RichText::new("No file with that name is in the linked folders.").size(10.5).color(p.warn_or(p)));
            }
            for cand in candidates {
                let label = cand.to_string_lossy().to_string();
                if ui.button(truncate(&label, 76)).clicked() {
                    self.relink(old, &label);
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.dlg_text)
                        .hint_text("or type the new path")
                        .desired_width(340.0),
                );
                if skin::primary(ui, "Relink", p).clicked() {
                    let path = self.dlg_text.clone();
                    self.relink(old, &path);
                }
            });
            if let Some(msg) = &self.dlg_msg {
                ui.label(egui::RichText::new(msg).size(10.5).color(p.bad));
            }
            ui.horizontal(|ui| {
                if skin::ghost(ui, "Forget this item", p).clicked() {
                    self.library.items.retain(|i| i.path != old);
                    self.touch();
                    self.save_library();
                    self.dialog = None;
                }
                if skin::ghost(ui, "Cancel", p).clicked() {
                    self.dialog = None;
                }
            });
        });
    }

    fn dlg_about(&mut self, ctx: &egui::Context, p: &Palette) {
        shell(ctx, p, "About VLCCraft", 480.0, |ui| {
            ui.label(egui::RichText::new("VLCCraft").size(22.0).strong().color(p.text));
            ui.label(
                egui::RichText::new("A calm skin over the VLC you already have.")
                    .size(11.5)
                    .color(p.muted),
            );
            ui.add_space(8.0);
            for (k, v) in [
                ("Player", self.libvlc_version.clone().unwrap_or_else(|| "libvlc unavailable".into())),
                ("Engine", "system libvlc, loaded at runtime".into()),
                ("System", crate::platform::os_name().to_string()),
                ("Interface", "Rust + egui".into()),
                ("VLC files touched", "none".into()),
                ("Library", format!("{} · {}", plural(self.library.items.len(), "item"), data_dir().display())),
            ] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(k).size(10.5).color(p.dim));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(v).size(10.0).color(p.muted).monospace());
                    });
                });
            }
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(
                    "The skin never edits VLC's library, plugins folder or launcher. Package updates replace VLC freely; VLCCraft keeps loading whichever libvlc is installed.",
                )
                .size(10.0)
                .color(p.dim),
            );
            ui.horizontal(|ui| {
                if skin::ghost(ui, "Shortcuts", p).clicked() {
                    self.dialog = Some(Dialog::Hotkeys);
                }
                if skin::ghost(ui, "Close", p).clicked() {
                    self.dialog = None;
                }
            });
        });
    }

    fn dlg_onboarding(&mut self, ctx: &egui::Context, p: &Palette) {
        shell(ctx, p, "Welcome to VLCCraft", 620.0, |ui| {
            ui.label(
                egui::RichText::new("Pick a look, learn five keys, then watch something.")
                    .size(13.0)
                    .color(p.text),
            );
            ui.add_space(8.0);
            skin::section(ui, "Theme", p);
            ui.horizontal_wrapped(|ui| {
                for theme in skin::THEMES {
                    if skin::chip(ui, theme, &self.settings.theme == theme, p).clicked() {
                        self.settings.theme = theme.to_string();
                        self.touch();
                    }
                }
            });
            ui.add_space(6.0);
            skin::section(ui, "Accent", p);
            ui.horizontal(|ui| {
                for (name, rgb) in skin::ACCENTS {
                    let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
                    ui.painter().circle_filled(
                        rect.center(),
                        10.0,
                        Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                    );
                    if self.settings.accent == *rgb {
                        ui.painter().circle_stroke(rect.center(), 12.0, Stroke::new(1.5, p.text));
                    }
                    if resp.clicked() {
                        self.settings.accent = *rgb;
                        self.touch();
                    }
                    resp.on_hover_text(*name);
                }
            });
            ui.add_space(8.0);
            skin::section(ui, "The five keys", p);
            for (key, action) in [
                ("Space", "Play or pause"),
                ("Left / Right", "Back or forward five seconds"),
                ("F", "Full screen; Esc comes back"),
                ("S", "Sidebar: subtitles, audio, video, playback"),
                ("Ctrl+B", "Drop a bookmark where you are"),
            ] {
                ui.horizontal(|ui| {
                    skin::badge(ui, key, p);
                    ui.label(egui::RichText::new(action).size(11.0).color(p.muted));
                });
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if skin::primary(ui, "Open my library", p).clicked() {
                    self.settings.onboarding_done = true;
                    self.dialog = None;
                    self.view = View::Library;
                    self.touch();
                    self.save_all();
                }
                if skin::ghost(ui, "Skip", p).clicked() {
                    self.settings.onboarding_done = true;
                    self.dialog = None;
                    self.touch();
                    self.save_all();
                }
            });
        });
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn shell<R>(
    ctx: &egui::Context,
    p: &Palette,
    title: &str,
    width: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    egui::Window::new(title)
        .id(Id::new(title))
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .title_bar(true)
        .frame(
            egui::Frame::new()
                .fill(p.panel)
                .stroke(Stroke::new(1.0, p.line))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(14))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 14],
                    blur: 40,
                    spread: 0,
                    color: Color32::from_black_alpha(150),
                }),
        )
        .show(ctx, |ui| {
            ui.set_width(width);
            add(ui)
        })
        .and_then(|r| r.inner)
}

/// Same chrome, but with a fixed height so a scrolling list cannot be cut by
/// the window guessing its own size from the scroll area.
fn shell_fixed<R>(
    ctx: &egui::Context,
    p: &Palette,
    title: &str,
    width: f32,
    height: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    egui::Window::new(title)
        .id(Id::new(title))
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .fixed_size(vec2(width, height))
        .frame(
            egui::Frame::new()
                .fill(p.panel)
                .stroke(Stroke::new(1.0, p.line))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(14))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 14],
                    blur: 40,
                    spread: 0,
                    color: Color32::from_black_alpha(150),
                }),
        )
        .show(ctx, |ui| add(ui))
        .and_then(|r| r.inner)
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>())
    } else {
        s.to_string()
    }
}

fn icon_for_control(id: &str) -> Icon {
    match id {
        "previous" => Icon::Prev,
        "play" => Icon::Play,
        "next" => Icon::Next,
        "time" => Icon::Clock,
        "subtitles" => Icon::Subtitle,
        "audio" => Icon::Speaker,
        "queue" => Icon::Queue,
        "volume" => Icon::Volume,
        "bookmark" => Icon::Bookmark,
        "pip" => Icon::Pip,
        "fullscreen" => Icon::Fullscreen,
        "settings" => Icon::Settings,
        _ => Icon::Panel,
    }
}

fn capture_combo(ctx: &egui::Context) -> Option<String> {
    let events = ctx.input(|i| i.events.clone());
    for event in events {
        if let egui::Event::Key {
            key,
            modifiers,
            pressed: true,
            ..
        } = event
        {
            if key == egui::Key::Escape {
                return None;
            }
            let mut parts: Vec<String> = Vec::new();
            if modifiers.ctrl || modifiers.command {
                parts.push("Ctrl".into());
            }
            if modifiers.shift {
                parts.push("Shift".into());
            }
            if modifiers.alt {
                parts.push("Alt".into());
            }
            parts.push(key_label(key));
            return Some(parts.join("+"));
        }
    }
    None
}

fn key_label(key: egui::Key) -> String {
    match key {
        egui::Key::ArrowLeft => "Left".into(),
        egui::Key::ArrowRight => "Right".into(),
        egui::Key::ArrowUp => "Up".into(),
        egui::Key::ArrowDown => "Down".into(),
        egui::Key::Space => "Space".into(),
        egui::Key::Enter => "Enter".into(),
        egui::Key::Escape => "Esc".into(),
        egui::Key::OpenBracket => "[".into(),
        egui::Key::CloseBracket => "]".into(),
        egui::Key::Comma => ",".into(),
        egui::Key::Period => ".".into(),
        egui::Key::Minus => "-".into(),
        egui::Key::Equals => "=".into(),
        other => format!("{other:?}"),
    }
}

fn hotkey_conflicts(settings: &crate::model::Settings) -> Vec<String> {
    let mut seen: Vec<(String, usize)> = Vec::new();
    for combo in settings.hotkeys.values() {
        if combo.is_empty() {
            continue;
        }
        if let Some(entry) = seen.iter_mut().find(|(c, _)| c == combo) {
            entry.1 += 1;
        } else {
            seen.push((combo.clone(), 1));
        }
    }
    seen.into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(c, n)| format!("{c} ×{n}"))
        .collect()
}

#[allow(dead_code)]
fn _unused(_: Bookmark, _: Preset, _: SettingsCategory) {}
