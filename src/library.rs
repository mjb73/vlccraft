//! The library surfaces: home, videos, music, streams and playlists.

use crate::app::{sorted_items, App, Dialog, Section, Sort, View, ViewMode};
use crate::model::{format_time, plural, playlist_to_m3u, LibraryItem};
use crate::skin::{self, Icon, Palette};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

/// What the single button on an empty shelf does.
#[derive(Clone, Copy)]
enum EmptyAction {
    AddFolder,
    AddStream,
    NewPlaylist,
    None,
}

impl App {
    pub fn library_view(&mut self, ctx: &egui::Context, p: &Palette) {
        if self.now.is_some() && self.compact_library && self.view == View::Library {
            // Compact mode is a rail-friendly list, not a different library.
        }
        egui::SidePanel::left("rail")
            .exact_width(226.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(p.panel)
                    .inner_margin(egui::Margin::symmetric(12, 12))
                    .stroke(Stroke::new(1.0, p.line_soft)),
            )
            .show(ctx, |ui| {
                self.library_rail(ui, p);
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg).inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                self.library_toolbar(ui, p);
                skin::divider(ui, p);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(22, 16))
                            .show(ui, |ui| {
                                match self.section {
                                    Section::Home => self.home_surface(ui, p),
                                    Section::Videos => self.videos_surface(ui, p),
                                    Section::Music => self.music_surface(ui, p),
                                    Section::Streams => self.streams_surface(ui, p),
                                    Section::Playlists => self.playlists_surface(ui, p),
                                    Section::Folder(i) => self.folder_surface(ui, p, i),
                                }
                                ui.add_space(30.0);
                            });
                    });
            });
    }

    fn library_rail(&mut self, ui: &mut Ui, p: &Palette) {
        ui.add_space(2.0);
        let entries: [(Section, Icon, &str); 5] = [
            (Section::Home, Icon::Library, "Home"),
            (Section::Videos, Icon::Film, "Videos"),
            (Section::Music, Icon::Music, "Music"),
            (Section::Streams, Icon::Stream, "Streams"),
            (Section::Playlists, Icon::Queue, "Playlists"),
        ];
        for (section, icon, label) in entries {
            let active = self.section == section;
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
            if active {
                ui.painter().rect_filled(rect, CornerRadius::same(8), p.panel2);
                ui.painter().rect_filled(
                    Rect::from_min_size(pos2(rect.left(), rect.top() + 8.0), vec2(2.5, 18.0)),
                    CornerRadius::same(2),
                    p.accent,
                );
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, CornerRadius::same(8), p.hover);
            }
            skin::icon(
                ui.painter(),
                icon,
                Rect::from_min_size(pos2(rect.left() + 10.0, rect.top() + 9.0), Vec2::splat(16.0)),
                if active { p.accent } else { p.muted },
                1.4,
            );
            ui.painter().text(
                pos2(rect.left() + 36.0, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                skin::font(12.8),
                if active { p.text } else { p.muted },
            );
            if resp.clicked() {
                self.section = section;
                self.playlist_open = None;
                self.folder_open = None;
            }
        }

        ui.add_space(16.0);
        skin::section(ui, "Pinned", p);
        ui.add_space(2.0);
        let playlists = self.playlists.clone();
        for pl in playlists.iter().take(3) {
            if skin::chip(ui, &pl.name, false, p).clicked() {
                self.section = Section::Playlists;
                self.playlist_open = Some(pl.id.clone());
            }
        }
        let folders = self.library.folders.clone();
        for (i, folder) in folders.iter().take(3).enumerate() {
            let name = std::path::Path::new(folder)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| folder.clone());
            if skin::chip(ui, &name, matches!(self.section, Section::Folder(x) if x == i), p).clicked() {
                self.section = Section::Folder(i);
                self.folder_open = Some(i);
            }
        }
        if playlists.is_empty() && folders.is_empty() {
            ui.label(
                egui::RichText::new("Pin a show or a playlist and it stays here.")
                    .size(10.0)
                    .color(p.dim),
            );
        }

        ui.add_space(16.0);
        skin::section(ui, "Recent", p);
        let recents: Vec<LibraryItem> = {
            let mut items: Vec<LibraryItem> = self
                .library
                .items
                .iter()
                .filter(|i| i.position_ms > 0)
                .cloned()
                .collect();
            items.sort_by(|a, b| b.added_at.cmp(&a.added_at));
            items.into_iter().take(4).collect()
        };
        for item in recents {
            let title = truncate(&item.title, 22);
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, CornerRadius::same(6), p.hover);
            }
            ui.painter().circle_filled(
                pos2(rect.left() + 6.0, rect.center().y),
                2.0,
                p.accent,
            );
            ui.painter().text(
                pos2(rect.left() + 16.0, rect.center().y),
                Align2::LEFT_CENTER,
                title,
                skin::font(11.0),
                p.muted,
            );
            if resp.clicked() {
                self.open_path(&item.path, true);
            }
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!(
                    "{} · {}",
                    plural(self.library.items.len(), "title"),
                    plural(self.library.folders.len(), "folder")
                ))
                .size(9.5)
                .color(p.dim),
            );
            ui.add_space(4.0);
            if skin::ghost(ui, "  Add folder", p).clicked() {
                self.dlg_text.clear();
                self.dlg_msg = None;
                self.dialog = Some(Dialog::OpenFolder);
            }
        });
    }

    fn library_toolbar(&mut self, ui: &mut Ui, p: &Palette) {
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(22, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (title, sub) = match self.section {
                        Section::Home => ("Home".to_string(), "Continue where you left off".to_string()),
                        Section::Videos => ("Videos".to_string(), format!("{} titles", self.library.items.iter().filter(|i| i.kind == "video").count())),
                        Section::Music => ("Music".to_string(), format!("{} tracks", self.library.items.iter().filter(|i| i.kind == "audio").count())),
                        Section::Streams => ("Streams".to_string(), "Links and playlists".to_string()),
                        Section::Playlists => ("Playlists".to_string(), format!("{} saved", self.playlists.len())),
                        Section::Folder(i) => {
                            let name = self
                                .library
                                .folders
                                .get(i)
                                .and_then(|f| std::path::Path::new(f).file_name())
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Folder".into());
                            (name, "Folder".to_string())
                        }
                    };
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(title).size(20.0).strong().color(p.text));
                        ui.label(egui::RichText::new(sub).size(10.5).color(p.dim));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // The one orange button says what this shelf is for;
                        // "Add folder" on the playlists page was a dead end.
                        let (label, action) = match self.section {
                            Section::Streams => ("Add stream", 2u8),
                            Section::Playlists => ("New playlist", 1u8),
                            _ => ("Add folder", 0u8),
                        };
                        if skin::primary(ui, label, p).clicked() {
                            match action {
                                2 => {
                                    self.dlg_text.clear();
                                    self.dlg_msg = None;
                                    self.dialog = Some(Dialog::OpenUrl);
                                }
                                1 => {
                                    self.dlg_text.clear();
                                    self.dlg_sel.clear();
                                    self.dialog = Some(Dialog::NewPlaylist);
                                }
                                _ => {
                                    self.dlg_text.clear();
                                    self.dlg_msg = None;
                                    self.dialog = Some(Dialog::OpenFolder);
                                }
                            }
                        }
                        // Search
                        ui.add_sized(
                            vec2(190.0, 28.0),
                            egui::TextEdit::singleline(&mut self.search)
                                .hint_text("Search titles and shows")
                                .margin(vec2(8.0, 5.0)),
                        );
                        // View mode
                        let mode = self.view_mode;
                        if skin::icon_button(ui, Icon::Grid, 28.0, mode == ViewMode::Grid, "Grid", p).clicked() {
                            self.view_mode = ViewMode::Grid;
                        }
                        if skin::icon_button(ui, Icon::List, 28.0, mode == ViewMode::List, "List", p).clicked() {
                            self.view_mode = ViewMode::List;
                        }
                        if skin::icon_button(ui, Icon::Details, 28.0, mode == ViewMode::Details, "Details", p).clicked() {
                            self.view_mode = ViewMode::Details;
                        }
                        // Sort, on the surface (LIB-3)
                        egui::ComboBox::from_id_salt("sort")
                            .selected_text(self.sort.label())
                            .width(140.0)
                            .show_ui(ui, |ui| {
                                for sort in [Sort::Name, Sort::Added, Sort::Duration, Sort::UnwatchedFirst, Sort::WatchedFirst] {
                                    if ui.selectable_label(self.sort == sort, sort.label()).clicked() {
                                        self.sort = sort;
                                    }
                                }
                            });
                    });
                });
            });
    }

    // -----------------------------------------------------------------------
    // Home
    // -----------------------------------------------------------------------

    fn home_surface(&mut self, ui: &mut Ui, p: &Palette) {

        // The one place the person keeps coming back to: say plainly which
        // player opens their files, and let them change it in one click.
        if self.settings.default_hint
            && self.defaults.available
            && !(self.defaults.video_ours && self.defaults.audio_ours)
        {
            egui::Frame::new()
                .fill(p.panel)
                .stroke(Stroke::new(1.0, p.line_soft))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::symmetric(14, 11))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (badge, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                        ui.painter().rect_filled(badge, CornerRadius::same(8), p.accent_soft);
                        skin::icon(ui.painter(), Icon::Play, badge.shrink(9.0), p.accent, 1.5);
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("Make VLCCraft the player that opens your files")
                                    .size(12.4)
                                    .strong()
                                    .color(p.text),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "Double-clicked video still opens in {}. One click hands video and music over; nothing in the system is touched.",
                                    self.defaults.video_name
                                ))
                                .size(10.2)
                                .color(p.muted),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if skin::icon_button(ui, Icon::Close, 24.0, false, "Not now", p).clicked() {
                                self.settings.default_hint = false;
                                self.touch();
                            }
                            if skin::primary(ui, "Set it up", p).clicked() {
                                self.view = View::Settings;
                                self.settings_cat = crate::app::SettingsCategory::General;
                                self.settings_search.clear();
                                self.settings_highlight = Some("default.player".into());
                                self.refresh_defaults();
                            }
                        });
                    });
                });
            ui.add_space(16.0);
        }

        let mut continue_item: Option<LibraryItem> = self
            .library
            .items
            .iter()
            .filter(|i| !i.watched && i.position_ms > 2_000 && i.progress() < 0.95)
            .max_by_key(|i| i.added_at)
            .cloned();
        if continue_item.is_none() {
            continue_item = self.library.items.iter().max_by_key(|i| i.added_at).cloned();
        }

        if let Some(ref item) = continue_item {
            let width = ui.available_width();
            let (rect, resp) = ui.allocate_exact_size(vec2(width, 196.0), Sense::click());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, CornerRadius::same(14), p.panel);
            painter.rect_stroke(rect, CornerRadius::same(14), Stroke::new(1.0, p.line_soft), StrokeKind::Inside);
            let art = Rect::from_min_size(rect.min + vec2(10.0, 10.0), vec2(300.0, 176.0));
            skin::poster(&painter, art, skin::seed_for(&item.path), p);
            if resp.hovered() {
                skin::play_badge(&painter, art.center(), 26.0, p);
            }
            let text_x = art.right() + 26.0;
            painter.text(
                pos2(text_x, rect.top() + 36.0),
                Align2::LEFT_CENTER,
                if item.progress() > 0.02 { "CONTINUE WATCHING" } else { "START HERE" },
                skin::font(9.5),
                p.accent,
            );
            painter.text(
                pos2(text_x, rect.top() + 66.0),
                Align2::LEFT_CENTER,
                truncate(&item.title, 40),
                skin::font(24.0),
                p.text,
            );
            painter.text(
                pos2(text_x, rect.top() + 94.0),
                Align2::LEFT_CENTER,
                format!(
                    "{} · {} · {} left",
                    item.show,
                    format_time(item.duration_ms),
                    format_time((item.duration_ms - item.position_ms).max(0))
                ),
                skin::font(11.5),
                p.muted,
            );
            let btn_w = 118.0;
            let bar_right = rect.right() - 30.0 - btn_w - 20.0;
            let bar = Rect::from_min_size(
                pos2(text_x, rect.top() + 128.0),
                vec2((bar_right - text_x).max(120.0), 5.0),
            );
            painter.rect_filled(bar, CornerRadius::same(3), p.line);
            painter.rect_filled(
                Rect::from_min_size(bar.min, vec2(bar.width() * item.progress(), 5.0)),
                CornerRadius::same(3),
                p.accent,
            );
            painter.text(
                pos2(text_x, rect.top() + 152.0),
                Align2::LEFT_CENTER,
                if item.position_ms > 2_000 {
                    format!("Resume at {}", format_time(item.position_ms))
                } else {
                    format!("Begin at 0:00")
                },
                skin::font(11.0),
                p.dim,
            );
            // One clear action so the card is not just a big link.
            let resume_at = if item.position_ms > 2_000 {
                "Resume"
            } else {
                "Play"
            };
            let mut action = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_size(
                        pos2(rect.right() - 30.0 - btn_w, rect.top() + 104.0),
                        vec2(btn_w, 32.0),
                    ))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            action.vertical_centered(|ui| {
                if skin::primary(ui, resume_at, p).clicked() {
                    self.open_path(&item.path, true);
                }
            });
            if resp.clicked() {
                self.open_path(&item.path, true);
            }
        } else {
            self.empty_state(ui, p, Icon::Library, "Your library is empty", "Add a folder and VLCCraft indexes it recursively — one action, no wizards.", EmptyAction::AddFolder);
        }

        ui.add_space(24.0);
        let mut recent: Vec<LibraryItem> = sorted_items(&self.library.items, Sort::Added)
            .into_iter()
            .take(6)
            .collect();
        recent.retain(|i| continue_item.as_ref().map(|c| c.path != i.path).unwrap_or(true));
        self.row_header(ui, p, "Recently added", "Everything indexed from your folders", Some(("All videos", Section::Videos)));
        self.card_row(ui, p, &recent);

        ui.add_space(22.0);
        self.row_header(ui, p, "Your playlists", "Collections you keep, not a temporary queue", Some(("All playlists", Section::Playlists)));
        let playlists = self.playlists.clone();
        ui.horizontal_wrapped(|ui| {
            for pl in playlists.iter().take(4) {
                let (rect, resp) = ui.allocate_exact_size(vec2(220.0, 76.0), Sense::click());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, CornerRadius::same(10), p.panel);
                painter.rect_stroke(rect, CornerRadius::same(10), Stroke::new(1.0, p.line_soft), StrokeKind::Inside);
                skin::icon(
                    &painter,
                    Icon::Queue,
                    Rect::from_min_size(rect.min + vec2(12.0, 12.0), Vec2::splat(20.0)),
                    p.accent,
                    1.5,
                );
                painter.text(
                    pos2(rect.left() + 42.0, rect.top() + 24.0),
                    Align2::LEFT_CENTER,
                    truncate(&pl.name, 20),
                    skin::font(12.5),
                    p.text,
                );
                painter.text(
                    pos2(rect.left() + 12.0, rect.top() + 54.0),
                    Align2::LEFT_CENTER,
                    plural(pl.items.len(), "item"),
                    skin::font(10.0),
                    p.dim,
                );
                if resp.hovered() {
                    painter.rect_stroke(rect, CornerRadius::same(10), Stroke::new(1.0, p.accent), StrokeKind::Inside);
                }
                if resp.clicked() {
                    self.section = Section::Playlists;
                    self.playlist_open = Some(pl.id.clone());
                }
            }
            if skin::ghost(ui, "+ New playlist", p).clicked() {
                self.dlg_text.clear();
                self.dlg_sel.clear();
                self.dialog = Some(Dialog::NewPlaylist);
            }
        });

        ui.add_space(22.0);
        self.row_header(ui, p, "Streams", "Links, not downloads", Some(("All streams", Section::Streams)));
        let streams = self.library.streams.clone();
        if streams.is_empty() {
            ui.label(
                egui::RichText::new("Add a radio station or a channel list from Streams.")
                    .size(11.0)
                    .color(p.dim),
            );
        } else {
            for url in streams.iter().take(4) {
                ui.horizontal(|ui| {
                    skin::icon(ui.painter(), Icon::Stream, Rect::from_min_size(ui.cursor().min, Vec2::splat(14.0)), p.accent, 1.3);
                    ui.add_space(4.0);
                    if ui.link(truncate(url, 60)).clicked() {
                        self.open_url(url);
                    }
                });
            }
        }
    }

    fn row_header(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        title: &str,
        sub: &str,
        all: Option<(&str, Section)>,
    ) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(title).size(15.0).strong().color(p.text));
                ui.label(egui::RichText::new(sub).size(10.0).color(p.dim));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((label, section)) = all {
                    if skin::ghost(ui, label, p).clicked() {
                        self.section = section;
                    }
                }
            });
        });
        ui.add_space(8.0);
    }

    fn card_row(&mut self, ui: &mut Ui, p: &Palette, items: &[LibraryItem]) {
        if items.is_empty() {
            ui.label(egui::RichText::new("Nothing here yet.").size(11.0).color(p.dim));
            return;
        }
        let (_, width) = grid_metrics(ui.available_width(), 236.0, 4);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(GRID_GAP, 18.0);
            for item in items.iter().take(8) {
                self.media_card(ui, p, item, width);
            }
        });
    }

    /// One shelf tile: the picture carries the duration and how far you got,
    /// the two lines under it carry the name and the show.
    fn media_card(&mut self, ui: &mut Ui, p: &Palette, item: &LibraryItem, width: f32) {
        let art_h = (width * 0.5625).round();
        let (rect, resp) = ui.allocate_exact_size(vec2(width, art_h + 48.0), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let art = Rect::from_min_size(rect.min, vec2(width, art_h));
        let radius = CornerRadius::same(11);
        skin::poster(&painter, art, skin::seed_for(&item.path), p);

        let hovered = resp.hovered() || resp.dragged();
        let progress = item.progress();

        if hovered {
            painter.rect_filled(art, radius, Color32::from_black_alpha(64));
            skin::play_badge(&painter, art.center(), 23.0, p);
            painter.rect_stroke(art, radius, Stroke::new(1.5, p.accent), StrokeKind::Inside);
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        } else if item.watched {
            let badge = Rect::from_min_size(
                pos2(art.right() - 58.0, art.top() + 8.0),
                vec2(50.0, 18.0),
            );
            painter.rect_filled(badge, CornerRadius::same(4), Color32::from_black_alpha(150));
            painter.text(
                badge.center(),
                Align2::CENTER_CENTER,
                "WATCHED",
                skin::font(8.5),
                Color32::WHITE,
            );
        }

        if !hovered && item.duration_ms > 0 {
            let duration = format_time(item.duration_ms);
            let galley = painter.layout_no_wrap(duration, skin::mono(9.5), Color32::from_white_alpha(238));
            let pill = Rect::from_min_size(
                pos2(art.right() - galley.size().x - 18.0, art.bottom() - 25.0),
                vec2(galley.size().x + 10.0, 17.0),
            );
            painter.rect_filled(pill, CornerRadius::same(4), Color32::from_black_alpha(168));
            painter.galley(pill.min + vec2(5.0, 2.0), galley, Color32::TRANSPARENT);
        }

        if progress > 0.0 {
            let track = Rect::from_min_size(
                pos2(art.left() + 7.0, art.bottom() - 8.0),
                vec2(art.width() - 14.0, 3.0),
            );
            painter.rect_filled(track, CornerRadius::same(2), Color32::from_black_alpha(120));
            painter.rect_filled(
                Rect::from_min_size(track.min, vec2(track.width() * progress, 3.0)),
                CornerRadius::same(2),
                p.accent,
            );
        }

        let chars = ((width / 6.9) as usize).max(10);
        painter.text(
            pos2(rect.left(), art.bottom() + 17.0),
            Align2::LEFT_CENTER,
            truncate(&item.title, chars),
            skin::font(12.4),
            if hovered { p.accent } else { p.text },
        );
        let meta = if item.watched {
            format!("{} · Watched", item.show)
        } else if progress > 0.02 && item.duration_ms > item.position_ms {
            format!(
                "{} · {} left",
                item.show,
                format_time(item.duration_ms - item.position_ms)
            )
        } else {
            item.show.clone()
        };
        painter.text(
            pos2(rect.left(), art.bottom() + 35.0),
            Align2::LEFT_CENTER,
            truncate(&meta, chars + 6),
            skin::font(10.0),
            p.dim,
        );

        if resp.drag_started() {
            resp.dnd_set_drag_payload(item.path.clone());
        }
        let menu = resp.context_menu(|ui| self.item_menu(ui, p, &item.path));
        let _ = menu;
        if resp.clicked() {
            self.open_path(&item.path, true);
        }
        resp.on_hover_text("Click to play · drag into the queue");
    }

    fn item_menu(&mut self, ui: &mut Ui, p: &Palette, path: &str) {
        if ui.button("Play now").clicked() {
            self.open_path(path, true);
            ui.close();
        }
        if ui.button("Play next").clicked() {
            let idx = (self.queue.index + 1).min(self.queue.items.len());
            self.queue.items.insert(idx, path.to_string());
            self.toast_now("Playing next");
            self.touch();
            ui.close();
        }
        if ui.button("Add to queue").clicked() {
            self.enqueue(vec![path.to_string()]);
            ui.close();
        }
        if ui.button("Add to playlist…").clicked() {
            self.dialog = Some(Dialog::AddToPlaylist(path.to_string()));
            ui.close();
        }
        ui.separator();
        let watched = self.library.get(path).map(|i| i.watched).unwrap_or(false);
        if ui.button(if watched { "Mark unwatched" } else { "Mark watched" }).clicked() {
            self.mark_watched(path, !watched);
            ui.close();
        }
        if ui.button("Clear resume point").clicked() {
            self.clear_resume(path);
            ui.close();
        }
        if ui.button("Video tracks…").clicked() {
            self.popup = Some(crate::app::Popup::VideoTracks);
            self.popup_anchor = ui.ctx().input(|i| i.pointer.hover_pos().unwrap_or(pos2(400.0, 300.0)));
            ui.close();
        }
        ui.separator();
        ui.label(egui::RichText::new("Also in the menu bar").size(9.5).color(p.dim));
    }

    // -----------------------------------------------------------------------
    // Videos
    // -----------------------------------------------------------------------

    fn videos_surface(&mut self, ui: &mut Ui, p: &Palette) {
        ui.horizontal(|ui| {
            for (id, label) in [("all", "All"), ("unwatched", "Unwatched"), ("watched", "Watched")] {
                if skin::chip(ui, label, self.library_filter != "all" && self.library_filter == id, p).clicked() {
                    self.library_filter = id.into();
                }
            }
            if self.library_filter == "all" {
                // "All" is active by default; the chip above only highlights filters.
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} of {}",
                        self.filtered_videos().len(),
                        self.library.items.iter().filter(|i| i.kind == "video").count()
                    ))
                    .size(10.5)
                    .color(p.dim),
                );
            });
        });
        ui.add_space(10.0);

        let items = self.filtered_videos();
        if items.is_empty() {
            self.empty_state(ui, p, Icon::Film, "No videos yet", "Add a folder with your shows and films. Subtitles beside the video are matched by name.", EmptyAction::AddFolder);
            return;
        }
        match self.view_mode {
            ViewMode::Grid => {
                let (_, width) = grid_metrics(ui.available_width(), 208.0, 6);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(GRID_GAP, 18.0);
                    for item in items.iter() {
                        self.media_card(ui, p, item, width);
                    }
                });
            }
            ViewMode::List => {
                for item in items.iter() {
                    self.media_row(ui, p, item);
                }
            }
            ViewMode::Details => {
                self.details_table(ui, p, &items);
            }
        }
    }

    fn filtered_videos(&self) -> Vec<LibraryItem> {
        let search = self.search.to_lowercase();
        let mut list: Vec<LibraryItem> = self
            .library
            .items
            .iter()
            .filter(|i| i.kind == "video")
            .filter(|i| match self.library_filter.as_str() {
                "watched" => i.watched,
                "unwatched" => !i.watched,
                _ => true,
            })
            .filter(|i| {
                search.is_empty()
                    || i.title.to_lowercase().contains(&search)
                    || i.show.to_lowercase().contains(&search)
            })
            .cloned()
            .collect();
        list = sorted_items(&list, self.sort);
        list
    }

    fn media_row(&mut self, ui: &mut Ui, p: &Palette, item: &LibraryItem) {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 62.0), Sense::click());
        let painter = ui.painter_at(rect);
        if resp.hovered() {
            painter.rect_filled(rect, CornerRadius::same(8), p.panel);
        }
        let art = Rect::from_min_size(pos2(rect.left() + 8.0, rect.top() + 8.0), vec2(76.0, 46.0));
        skin::poster(&painter, art, skin::seed_for(&item.path), p);
        painter.text(
            pos2(art.right() + 14.0, rect.top() + 24.0),
            Align2::LEFT_CENTER,
            truncate(&item.title, 52),
            skin::font(12.8),
            p.text,
        );
        painter.text(
            pos2(art.right() + 14.0, rect.top() + 42.0),
            Align2::LEFT_CENTER,
            format!("{} · {}", item.show, if item.duration_ms > 0 { format_time(item.duration_ms) } else { "—".into() }),
            skin::font(10.0),
            p.dim,
        );
        if item.progress() > 0.0 {
            let bar = Rect::from_min_size(pos2(art.right() + 14.0, rect.top() + 50.0), vec2(180.0, 3.0));
            painter.rect_filled(bar, CornerRadius::same(2), p.line);
            painter.rect_filled(
                Rect::from_min_size(bar.min, vec2(bar.width() * item.progress(), 3.0)),
                CornerRadius::same(2),
                p.accent,
            );
        }
        if item.watched {
            painter.text(
                pos2(rect.right() - 70.0, rect.center().y),
                Align2::RIGHT_CENTER,
                "Watched",
                skin::font(10.0),
                p.good,
            );
        }
        if resp.clicked() {
            self.open_path(&item.path, true);
        }
        if resp.drag_started() {
            resp.dnd_set_drag_payload(item.path.clone());
        }
        resp.context_menu(|ui| self.item_menu(ui, p, &item.path));
    }

    fn details_table(&mut self, ui: &mut Ui, p: &Palette, items: &[LibraryItem]) {
        let widths = [ui.available_width() - 330.0, 110.0, 90.0, 130.0];
        ui.horizontal(|ui| {
            for (label, w) in ["Name", "Show", "Length", "Status"].iter().zip(widths) {
                ui.add_sized(vec2(w, 20.0), egui::Label::new(egui::RichText::new(*label).size(10.0).color(p.dim).strong()));
            }
        });
        skin::divider(ui, p);
        for item in items {
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
            let painter = ui.painter_at(rect);
            if resp.hovered() {
                painter.rect_filled(rect, CornerRadius::same(5), p.panel);
            }
            let mut x = rect.left();
            painter.text(pos2(x + 4.0, rect.center().y), Align2::LEFT_CENTER, truncate(&item.title, 46), skin::font(11.8), p.text);
            x += widths[0];
            painter.text(pos2(x, rect.center().y), Align2::LEFT_CENTER, truncate(&item.show, 14), skin::font(11.0), p.muted);
            x += widths[1];
            painter.text(pos2(x, rect.center().y), Align2::LEFT_CENTER, format_time(item.duration_ms), skin::mono(10.5), p.muted);
            x += widths[2];
            let status = if item.watched { "Watched" } else if item.position_ms > 2000 { "In progress" } else { "Unwatched" };
            painter.text(pos2(x, rect.center().y), Align2::LEFT_CENTER, status, skin::font(10.5), if item.watched { p.good } else { p.dim });
            if resp.clicked() {
                self.open_path(&item.path, true);
            }
            resp.context_menu(|ui| self.item_menu(ui, p, &item.path));
        }
    }

    fn folder_surface(&mut self, ui: &mut Ui, p: &Palette, index: usize) {
        let Some(folder) = self.library.folders.get(index).cloned() else {
            self.empty_state(ui, p, Icon::Folder, "Folder missing", "It may have moved. Add it again from the library.", EmptyAction::AddFolder);
            return;
        };
        ui.label(egui::RichText::new(&folder).size(10.0).color(p.dim).monospace());
        ui.add_space(8.0);
        let mut items: Vec<LibraryItem> = self
            .library
            .items
            .iter()
            .filter(|i| i.path.starts_with(&folder))
            .cloned()
            .collect();
        items = sorted_items(&items, self.sort);
        if self.search.len() > 0 {
            let s = self.search.to_lowercase();
            items.retain(|i| i.title.to_lowercase().contains(&s) || i.show.to_lowercase().contains(&s));
        }
        if items.is_empty() {
            self.empty_state(ui, p, Icon::Film, "Nothing here yet", "This folder has no videos or music the library recognises.", EmptyAction::None);
            return;
        }
        // Shows first, as folders within the folder.
        let mut shows: Vec<String> = items.iter().map(|i| i.show.clone()).collect();
        shows.sort();
        shows.dedup();
        if shows.len() > 1 {
            skin::section(ui, "Shows in this folder", p);
            ui.horizontal_wrapped(|ui| {
                for show in &shows {
                    if skin::chip(ui, show, false, p).clicked() {
                        self.search = show.clone();
                    }
                }
            });
            ui.add_space(10.0);
        }
        match self.view_mode {
            ViewMode::Grid => {
                let (_, width) = grid_metrics(ui.available_width(), 208.0, 6);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(GRID_GAP, 18.0);
                    for item in &items {
                        self.media_card(ui, p, item, width);
                    }
                });
            }
            _ => {
                for item in &items {
                    self.media_row(ui, p, item);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Music
    // -----------------------------------------------------------------------

    fn music_surface(&mut self, ui: &mut Ui, p: &Palette) {
        ui.horizontal(|ui| {
            for group in ["Songs", "Albums", "Artists", "Genres"] {
                if skin::chip(ui, group, self.music_group == group, p).clicked() {
                    self.music_group = group.into();
                }
            }
        });
        ui.add_space(10.0);
        let mut tracks: Vec<LibraryItem> = self
            .library
            .items
            .iter()
            .filter(|i| i.kind == "audio")
            .cloned()
            .collect();
        tracks = sorted_items(&tracks, self.sort);
        if tracks.is_empty() {
            self.empty_state(ui, p, Icon::Music, "No music yet", "Add a folder of songs; the library groups it by album, artist and genre.", EmptyAction::AddFolder);
            return;
        }
        if self.music_group == "Songs" {
            for item in &tracks {
                self.media_row(ui, p, item);
            }
            return;
        }
        let mut groups: Vec<(String, Vec<LibraryItem>)> = Vec::new();
        for t in tracks {
            let key = match self.music_group.as_str() {
                "Albums" | "Artists" => t.show.clone(),
                _ => "Unknown genre".to_string(),
            };
            if let Some((_, list)) = groups.iter_mut().find(|(k, _)| k == &key) {
                list.push(t);
            } else {
                groups.push((key, vec![t]));
            }
        }
        let (_, width) = grid_metrics(ui.available_width(), 196.0, 5);
        let art_h = (width * 0.74).round();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(GRID_GAP, 18.0);
            for (name, list) in groups {
                let (rect, resp) = ui.allocate_exact_size(vec2(width, art_h + 42.0), Sense::click());
                let painter = ui.painter_at(rect);
                let art = Rect::from_min_size(rect.min, vec2(width, art_h));
                skin::poster(&painter, art, skin::seed_for(&name), p);
                skin::icon(&painter, Icon::Music, Rect::from_center_size(art.center(), Vec2::splat(34.0)), Color32::from_white_alpha(150), 1.5);
                painter.text(pos2(rect.left(), art.bottom() + 16.0), Align2::LEFT_CENTER, truncate(&name, ((width / 6.9) as usize).max(10)), skin::font(12.4), p.text);
                painter.text(pos2(rect.left(), art.bottom() + 33.0), Align2::LEFT_CENTER, format!("{} tracks", list.len()), skin::font(10.0), p.dim);
                if resp.clicked() {
                    self.queue.items = list.iter().map(|i| i.path.clone()).collect();
                    self.queue.index = 0;
                    if let Some(first) = self.queue.items.first().cloned() {
                        self.open_path(&first, true);
                    }
                }
                if resp.hovered() {
                    painter.rect_filled(art, CornerRadius::same(11), Color32::from_black_alpha(50));
                    skin::play_badge(&painter, art.center(), 22.0, p);
                    painter.rect_stroke(art, CornerRadius::same(11), Stroke::new(1.5, p.accent), StrokeKind::Inside);
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }
        });
    }

    // -----------------------------------------------------------------------
    // Streams
    // -----------------------------------------------------------------------

    fn streams_surface(&mut self, ui: &mut Ui, p: &Palette) {
        ui.horizontal(|ui| {
            if skin::primary(ui, "Add stream", p).clicked() {
                self.dlg_text.clear();
                self.dlg_msg = None;
                self.dialog = Some(Dialog::OpenUrl);
            }
            ui.label(
                egui::RichText::new("A URL that returns a playlist becomes a queue, not one item.")
                    .size(10.0)
                    .color(p.dim),
            );
        });
        ui.add_space(12.0);
        let streams = self.library.streams.clone();
        if streams.is_empty() {
            self.empty_state(ui, p, Icon::Stream, "No streams saved", "Radio, a channel list or a direct link. Each one is its own dialog, not an all-in-one media factory.", EmptyAction::AddStream);
            return;
        }
        for url in streams {
            ui.horizontal(|ui| {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, CornerRadius::same(8), p.panel);
                skin::icon(&painter, Icon::Stream, Rect::from_min_size(rect.min + vec2(12.0, 14.0), Vec2::splat(18.0)), p.accent, 1.4);
                painter.text(pos2(rect.left() + 42.0, rect.center().y), Align2::LEFT_CENTER, truncate(&url, 90), skin::mono(10.5), p.text);
                if resp.clicked() {
                    self.open_url(&url);
                }
                resp.context_menu(|ui| {
                    if ui.button("Play").clicked() {
                        self.open_url(&url);
                        ui.close();
                    }
                    if ui.button("Remove from streams").clicked() {
                        self.library.streams.retain(|s| s != &url);
                        self.touch();
                        self.save_library();
                        ui.close();
                    }
                });
            });
        }
    }

    // -----------------------------------------------------------------------
    // Playlists
    // -----------------------------------------------------------------------

    fn playlists_surface(&mut self, ui: &mut Ui, p: &Palette) {
        if let Some(id) = self.playlist_open.clone() {
            self.playlist_detail(ui, p, &id);
            return;
        }
        ui.horizontal(|ui| {
            if skin::primary(ui, "New playlist", p).clicked() {
                self.dlg_text.clear();
                self.dlg_sel.clear();
                self.dialog = Some(Dialog::NewPlaylist);
            }
            if skin::ghost(ui, "Import .m3u", p).clicked() {
                self.dlg_text.clear();
                self.dlg_msg = None;
                self.dialog = Some(Dialog::OpenFile);
                self.dlg_text2 = "m3u".into();
            }
        });
        ui.add_space(12.0);
        let playlists = self.playlists.clone();
        if playlists.is_empty() {
            self.empty_state(ui, p, Icon::Queue, "No playlists yet", "A playlist is your collection — the queue is what plays next. They are separate things.", EmptyAction::NewPlaylist);
            return;
        }
        let (_, width) = grid_metrics(ui.available_width(), 208.0, 5);
        let art_h = (width * 0.5625).round();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(GRID_GAP, 18.0);
            for pl in playlists {
                let (rect, resp) = ui.allocate_exact_size(vec2(width, art_h + 48.0), Sense::click());
                let painter = ui.painter_at(rect);
                let art = Rect::from_min_size(rect.min, vec2(width, art_h));
                skin::poster(&painter, art, skin::seed_for(&pl.name), p);
                skin::icon(
                    &painter,
                    Icon::Queue,
                    Rect::from_min_size(art.min + vec2(10.0, 10.0), Vec2::splat(16.0)),
                    Color32::from_white_alpha(200),
                    1.4,
                );
                painter.text(pos2(rect.left(), art.bottom() + 17.0), Align2::LEFT_CENTER, truncate(&pl.name, ((width / 6.9) as usize).max(10)), skin::font(12.4), p.text);
                painter.text(pos2(rect.left(), art.bottom() + 35.0), Align2::LEFT_CENTER, format!("{} · {}", plural(pl.items.len(), "item"), if pl.items.is_empty() { "empty" } else { "ready" }), skin::font(10.0), p.dim);
                if resp.hovered() {
                    painter.rect_filled(art, CornerRadius::same(11), Color32::from_black_alpha(56));
                    skin::play_badge(&painter, art.center(), 22.0, p);
                    painter.rect_stroke(art, CornerRadius::same(11), Stroke::new(1.5, p.accent), StrokeKind::Inside);
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    self.playlist_open = Some(pl.id.clone());
                }
                resp.context_menu(|ui| {
                    if ui.button("Play").clicked() {
                        self.play_playlist(&pl.id);
                        ui.close();
                    }
                    if ui.button("Enqueue").clicked() {
                        self.enqueue(pl.items.clone());
                        ui.close();
                    }
                    if ui.button("Delete playlist").clicked() {
                        self.playlists.retain(|x| x.id != pl.id);
                        self.touch();
                        self.save_all();
                        ui.close();
                    }
                });
            }
        });
    }

    fn playlist_detail(&mut self, ui: &mut Ui, p: &Palette, id: &str) {
        let Some(pl) = self.playlists.iter().find(|x| x.id == id).cloned() else {
            self.playlist_open = None;
            return;
        };
        ui.horizontal(|ui| {
            if skin::icon_button(ui, Icon::ArrowLeft, 28.0, false, "All playlists", p).clicked() {
                self.playlist_open = None;
            }
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(&pl.name).size(20.0).strong().color(p.text));
                ui.label(egui::RichText::new(plural(pl.items.len(), "item")).size(10.5).color(p.dim));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if skin::primary(ui, "Play all", p).clicked() {
                    self.play_playlist(id);
                }
                if skin::ghost(ui, "Enqueue", p).clicked() {
                    self.enqueue(pl.items.clone());
                }
                if skin::ghost(ui, "Export .m3u", p).clicked() {
                    let target = PathBuf::from(data_dir()).join(format!("{}.m3u", pl.name.replace(' ', "_")));
                    let text = playlist_to_m3u(&pl.items);
                    if std::fs::write(&target, text).is_ok() {
                        self.toast_now(format!("Exported {}", target.display()));
                    }
                }
                if skin::ghost(ui, "Duplicate", p).clicked() {
                    let mut copy = pl.clone();
                    copy.id = crate::model::new_id("pl");
                    copy.name = format!("{} copy", pl.name);
                    copy.created = crate::model::now_ms();
                    copy.modified = crate::model::now_ms();
                    self.playlists.push(copy);
                    self.touch();
                    self.save_all();
                }
                if skin::ghost(ui, "Rename", p).clicked() {
                    self.dlg_text = pl.name.clone();
                    self.dialog = Some(Dialog::EditPlaylist(id.to_string()));
                }
            });
        });
        ui.add_space(12.0);
        if pl.items.is_empty() {
            self.empty_state(ui, p, Icon::Queue, "This playlist is empty", "Add items from the library, or drag titles here from the queue.", EmptyAction::None);
            return;
        }
        let mut items = pl.items.clone();
        let mut changed = false;
        let mut idx = 0;
        while idx < items.len() {
            let path = items[idx].clone();
            let title = self.library.title_for(&path);
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
            let painter = ui.painter_at(rect);
            if resp.hovered() {
                painter.rect_filled(rect, CornerRadius::same(6), p.panel);
            }
            painter.text(pos2(rect.left() + 10.0, rect.center().y), Align2::LEFT_CENTER, format!("{}", idx + 1), skin::mono(10.0), p.dim);
            painter.text(pos2(rect.left() + 40.0, rect.center().y), Align2::LEFT_CENTER, truncate(&title, 60), skin::font(12.0), p.text);
            let mut action = 0u8;
            let mut x = rect.right() - 8.0;
            for icon in [Icon::Trash, Icon::ChevronDown, Icon::ChevronUp] {
                x -= 26.0;
                let ar = Rect::from_center_size(pos2(x, rect.center().y), Vec2::splat(22.0));
                let r = ui.interact(ar, Id::new(("pl-item", id, idx, x as i32)), Sense::click());
                if r.clicked() {
                    action = match icon {
                        Icon::Trash => 1,
                        Icon::ChevronDown => 3,
                        _ => 2,
                    };
                }
                skin::icon(&painter, icon, ar.shrink(3.0), p.muted, 1.3);
            }
            if resp.clicked() {
                let q: Vec<String> = items.clone();
                self.queue.items = q;
                self.queue.index = idx;
                self.open_path(&path, true);
            }
            match action {
                1 => {
                    items.remove(idx);
                    changed = true;
                    continue;
                }
                2 if idx > 0 => {
                    items.swap(idx, idx - 1);
                    changed = true;
                }
                3 if idx + 1 < items.len() => {
                    items.swap(idx, idx + 1);
                    changed = true;
                }
                _ => {}
            }
            idx += 1;
        }
        if changed {
            if let Some(target) = self.playlists.iter_mut().find(|x| x.id == id) {
                target.items = items;
                target.modified = crate::model::now_ms();
            }
            self.touch();
            self.save_all();
        }
    }

    // -----------------------------------------------------------------------
    // Compact library while watching (LIB-6)
    // -----------------------------------------------------------------------

    pub fn compact_library_panel(&mut self, ctx: &egui::Context, p: &Palette) {
        egui::SidePanel::left("compact-library")
            .exact_width(268.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(p.panel)
                    .inner_margin(egui::Margin::ZERO)
                    .stroke(Stroke::new(1.0, p.line_soft)),
            )
            .show(ctx, |ui| match self.section {
                Section::Playlists => self.playlists_surface(ui, p),
                _ => self.videos_surface(ui, p),
            });
    }

    /// An empty shelf: the icon says which shelf it is, the sentence says how
    /// to fill it, and the one button starts that. Nothing else competes.
    fn empty_state(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        icon: Icon,
        title: &str,
        body: &str,
        action: EmptyAction,
    ) {
        ui.add_space(14.0);
        let col_w = ui.available_width().min(720.0);
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 244.0), Sense::hover());
        let rect = Rect::from_center_size(row.center(), vec2(col_w, 244.0));
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::same(16), p.panel);
        painter.rect_stroke(
            rect,
            CornerRadius::same(16),
            Stroke::new(1.0, p.line_soft),
            StrokeKind::Inside,
        );

        let disc = pos2(rect.center().x, rect.top() + 62.0);
        painter.circle_filled(disc, 31.0, p.accent_soft);
        skin::icon(
            &painter,
            icon,
            Rect::from_center_size(disc, Vec2::splat(26.0)),
            p.accent,
            1.6,
        );
        painter.text(
            pos2(rect.center().x, rect.top() + 116.0),
            Align2::CENTER_CENTER,
            title,
            skin::font(16.0),
            p.text,
        );
        let galley = painter.layout(
            body.to_string(),
            skin::font(11.0),
            p.muted,
            (col_w - 90.0).max(180.0),
        );
        painter.galley(
            pos2(rect.center().x - galley.size().x / 2.0, rect.top() + 136.0),
            galley,
            Color32::TRANSPARENT,
        );

        let (label, icon_w) = match action {
            EmptyAction::AddFolder => ("Add folder", 96.0),
            EmptyAction::AddStream => ("Add stream", 96.0),
            EmptyAction::NewPlaylist => ("New playlist", 110.0),
            EmptyAction::None => ("", 0.0),
        };
        if !label.is_empty() {
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_center_size(
                        pos2(rect.center().x, rect.top() + 196.0),
                        vec2(icon_w, 34.0),
                    ))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            child.vertical_centered(|ui| {
                if skin::primary(ui, label, p).clicked() {
                    match action {
                        EmptyAction::AddFolder => {
                            self.dlg_text.clear();
                            self.dlg_msg = None;
                            self.dialog = Some(Dialog::OpenFolder);
                        }
                        EmptyAction::AddStream => {
                            self.dlg_text.clear();
                            self.dlg_msg = None;
                            self.dialog = Some(Dialog::OpenUrl);
                        }
                        EmptyAction::NewPlaylist => {
                            self.dlg_text.clear();
                            self.dlg_sel.clear();
                            self.dialog = Some(Dialog::NewPlaylist);
                        }
                        EmptyAction::None => {}
                    }
                }
            });
        }
    }
}

/// Space between two tiles in a shelf.
const GRID_GAP: f32 = 18.0;

/// How many tiles fit across the window, and how wide each one has to be so
/// the row ends exactly at the right edge. A fixed tile width leaves a gap on
/// wide windows and squeezes the last column on narrow ones; this does not.
fn grid_metrics(avail: f32, min_width: f32, max_cols: usize) -> (usize, f32) {
    let avail = avail.max(180.0);
    let cols = (((avail + GRID_GAP) / (min_width + GRID_GAP)).floor() as usize)
        .clamp(1, max_cols.max(1));
    let width = ((avail - GRID_GAP * (cols.saturating_sub(1)) as f32) / cols as f32).max(120.0);
    (cols, width)
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>())
    } else {
        s.to_string()
    }
}

use std::path::PathBuf;
use crate::model::data_dir;

#[cfg(test)]
mod scan_tests {
    use super::*;
    #[test]
    fn demo_folder_has_three_titles() {
        let media = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/media");
        // The demo clips are deliberately not in the repository, so this only
        // runs where they exist: a fresh checkout, or CI, has nothing to scan
        // and says so rather than failing.
        if !media.join("Films/Desert Road.mp4").exists() {
            eprintln!("skipped: assets/media is not in this checkout");
            return;
        }
        let mut lib = crate::model::Library::default();
        let added = lib.ensure_folder(&media);
        for i in &lib.items {
            eprintln!("ITEM {} <- {}", i.title, i.path);
        }
        assert_eq!(added, 3);
    }
}
