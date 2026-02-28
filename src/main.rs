#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod game;
mod stats;
mod words;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Key, Pos2, Rect,
    RichText, Shape, Stroke, StrokeKind, Vec2,
};
use game::{GameState, GameStatus, TileState, MAX_GUESSES, WORD_LENGTH};
use rand::Rng;
use stats::ProfileManager;
use std::f32::consts::PI;
use std::time::Instant;

// ── Animation ─────────────────────────────────────────────────────────────────
const FLIP_STAGGER:  f32 = 0.09;
const FLIP_DURATION: f32 = 0.35;
const TOTAL_ANIM:    f32 = FLIP_STAGGER * (WORD_LENGTH as f32 - 1.0) + FLIP_DURATION;

// ── Base layout (at scale = 1.0) ──────────────────────────────────────────────
const BASE_TILE:       f32 = 62.0;
const BASE_GAP:        f32 = 5.0;
const BASE_KEY_W:      f32 = 43.0;
const BASE_KEY_H:      f32 = 58.0;
const BASE_KEY_GAP:    f32 = 6.0;
const BASE_ENTER_W:    f32 = 65.5;
const BASE_HEADER:     f32 = 56.0;
const BASE_BOTTOM_BAR: f32 = 62.0; // reserved below keyboard for the New Game button

const IDEAL_W: f32 = BASE_ENTER_W * 2.0 + BASE_KEY_W * 7.0 + BASE_KEY_GAP * 8.0 + 40.0;
const IDEAL_H: f32 = BASE_HEADER + 14.0
    + (BASE_TILE * MAX_GUESSES as f32 + BASE_GAP * (MAX_GUESSES - 1) as f32)
    + 20.0
    + (BASE_KEY_H * 3.0 + BASE_KEY_GAP * 2.0)
    + 20.0
    + BASE_BOTTOM_BAR;

// ── Colors ────────────────────────────────────────────────────────────────────
const C_BG:           Color32 = Color32::from_rgb(18, 18, 19);
const C_BORDER_EMPTY: Color32 = Color32::from_rgb(58, 58, 60);
const C_BORDER_TYPED: Color32 = Color32::from_rgb(134, 134, 134);
const C_CORRECT:      Color32 = Color32::from_rgb(83, 141, 78);
const C_PRESENT:      Color32 = Color32::from_rgb(181, 159, 59);
const C_ABSENT:       Color32 = Color32::from_rgb(58, 58, 60);
const C_KEY:          Color32 = Color32::from_rgb(129, 131, 132);
const C_WHITE:        Color32 = Color32::WHITE;
const C_DIVIDER:      Color32 = Color32::from_rgb(58, 58, 60);
const C_BAR_EMPTY:    Color32 = Color32::from_rgb(40, 40, 43);

// ── Color helpers ─────────────────────────────────────────────────────────────
fn tile_bg(s: TileState) -> Color32 {
    match s { TileState::Correct => C_CORRECT, TileState::Present => C_PRESENT,
              TileState::Absent  => C_ABSENT,  _                  => C_BG }
}
fn tile_border(s: TileState) -> Color32 {
    match s { TileState::Empty   => C_BORDER_EMPTY, TileState::Typing  => C_BORDER_TYPED,
              TileState::Correct => C_CORRECT,       TileState::Present => C_PRESENT,
              TileState::Absent  => C_ABSENT }
}
fn key_bg(s: Option<&TileState>) -> Color32 {
    match s { Some(TileState::Correct) => C_CORRECT, Some(TileState::Present) => C_PRESENT,
              Some(TileState::Absent)  => C_ABSENT,  _                        => C_KEY }
}
fn lighten(c: Color32, amt: u16) -> Color32 {
    Color32::from_rgb(
        (c.r() as u16 + amt).min(255) as u8,
        (c.g() as u16 + amt).min(255) as u8,
        (c.b() as u16 + amt).min(255) as u8,
    )
}

// ── Icon drawing (painter primitives — no font dependency) ────────────────────

/// Small bar-chart icon (for the Stats header button)
fn icon_barchart(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let bw = size * 0.22;
    let gap = size * 0.10;
    let heights = [0.55_f32, 1.0, 0.72];
    let total_w = bw * 3.0 + gap * 2.0;
    let x0 = center.x - total_w * 0.5;
    let bottom = center.y + size * 0.5;
    for (i, &h) in heights.iter().enumerate() {
        let x = x0 + i as f32 * (bw + gap);
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(x, bottom - size * h), Pos2::new(x + bw, bottom)),
            CornerRadius::same(1), color);
    }
}

/// Small person / profile icon (for the Profiles header button)
fn icon_person(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    // Head
    let head_r = size * 0.22;
    let head_c = Pos2::new(center.x, center.y - size * 0.14);
    painter.circle_filled(head_c, head_r, color);
    // Shoulders (rounded rectangle)
    let sw = size * 0.52;
    let sh = size * 0.30;
    let sy = head_c.y + head_r + size * 0.06;
    painter.rect_filled(
        Rect::from_center_size(Pos2::new(center.x, sy + sh * 0.5), Vec2::new(sw, sh)),
        CornerRadius::same((sw * 0.18) as u8), color);
}

/// Backspace / delete icon — drawn in place of the ⌫ glyph
fn icon_backspace(painter: &egui::Painter, key_rect: Rect, color: Color32, bg: Color32) {
    let cx = key_rect.center().x;
    let cy = key_rect.center().y;
    let iw = key_rect.width()  * 0.54;
    let ih = key_rect.height() * 0.30;
    let x0 = cx - iw * 0.5;
    let x1 = cx + iw * 0.5;
    let y0 = cy - ih * 0.5;
    let y1 = cy + ih * 0.5;
    let notch = ih * 0.52;

    // Pentagon fill
    let pts = vec![
        Pos2::new(x0 + notch, y0),
        Pos2::new(x1, y0),
        Pos2::new(x1, y1),
        Pos2::new(x0 + notch, y1),
        Pos2::new(x0, cy),
    ];
    painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));

    // ✕ inside
    let xs = ih * 0.20;
    let xc = Pos2::new(cx + notch * 0.10, cy);
    let sw = Stroke::new((ih * 0.14).max(1.0), bg);
    painter.line_segment([Pos2::new(xc.x - xs, xc.y - xs), Pos2::new(xc.x + xs, xc.y + xs)], sw);
    painter.line_segment([Pos2::new(xc.x + xs, xc.y - xs), Pos2::new(xc.x - xs, xc.y + xs)], sw);
}

/// Header button: icon above small label. Returns true if clicked.
fn header_btn(
    painter: &egui::Painter,
    ctx:     &egui::Context,
    rect:    Rect,
    icon:    impl Fn(&egui::Painter, Pos2, f32, Color32),
    label:   &str,
    active:  bool,
    scale:   f32,
) -> bool {
    let hov = ctx.input(|i| i.pointer.hover_pos().map_or(false, |p| rect.contains(p)));
    let col = if active || hov { C_WHITE } else { Color32::from_rgb(140, 140, 143) };

    let icon_sz = 17.0 * scale;
    let icon_c  = Pos2::new(rect.center().x, rect.center().y - 7.0 * scale);
    icon(painter, icon_c, icon_sz, col);
    painter.text(
        Pos2::new(rect.center().x, icon_c.y + icon_sz * 0.5 + 4.0 * scale),
        Align2::CENTER_TOP, label, FontId::proportional(9.0 * scale), col);

    ctx.input(|i| hov && i.pointer.primary_released())
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Rusdle")
            .with_inner_size([500.0, 750.0])
            .with_min_inner_size([360.0, 560.0])
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native("Rusdle", options, Box::new(|cc| {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Ok(Box::new(RusdleApp::new()))
    }))
}

// ── App struct ────────────────────────────────────────────────────────────────

struct RusdleApp {
    game: GameState,
    seed: u64,

    anim_row:     Option<usize>,
    anim_elapsed: f32,

    profiles:       ProfileManager,
    game_start:     Option<Instant>,
    stats_recorded: bool,

    show_stats:       bool,
    show_profiles:    bool,
    new_profile_name: String,
    profile_error:    Option<String>,
}

impl RusdleApp {
    fn new() -> Self {
        let seed = rand::thread_rng().gen::<u64>();
        Self {
            game: GameState::new(seed),
            seed,
            anim_row:         None,
            anim_elapsed:     0.0,
            profiles:         ProfileManager::load(),
            game_start:       None,
            stats_recorded:   false,
            show_stats:       false,
            show_profiles:    false,
            new_profile_name: String::new(),
            profile_error:    None,
        }
    }

    fn next_game(&mut self) {
        // Truly random word every time
        self.seed           = rand::thread_rng().gen::<u64>();
        self.game.reset(self.seed);
        self.anim_row       = None;
        self.anim_elapsed   = 0.0;
        self.game_start     = None;
        self.stats_recorded = false;
    }

    fn record_stats(&mut self) {
        if self.stats_recorded { return; }
        self.stats_recorded = true;
        let elapsed  = self.game_start.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        let won      = self.game.status == GameStatus::Won;
        let guesses  = self.game.current_row as u32;
        let first: Option<String> = if self.game.current_row > 0 {
            let w: String = self.game.guesses[0].iter().map(|t| t.ch).collect();
            if w.trim_matches(' ').len() == WORD_LENGTH { Some(w) } else { None }
        } else { None };
        self.profiles.active_mut().stats.record_game(won, guesses, elapsed, first.as_deref());
        self.profiles.save();
    }
}

// ── Tile drawing ──────────────────────────────────────────────────────────────

fn flip_squish(p: f32) -> f32 {
    if p <= 0.0 { return 1.0; }
    if p >= 1.0 { return 1.0; }
    (if p < 0.5 { (p * PI).cos() } else { ((p - 0.5) * PI).sin() }).max(0.0)
}

fn draw_tile(painter: &egui::Painter, full: Rect, tile: &game::Tile, flip: Option<f32>, scale: f32) {
    let cr = CornerRadius::same((3.0 * scale).max(1.0) as u8);
    let sw = (2.0 * scale.min(1.0)).max(1.0);

    let (bg, border, show_ch) = match flip {
        Some(t) if t < 0.5 => (C_BG, C_BORDER_TYPED, t < 0.02),
        Some(_)            => (tile_bg(tile.state), tile_border(tile.state), true),
        None               => (tile_bg(tile.state), tile_border(tile.state), true),
    };
    let squish = flip.map(flip_squish).unwrap_or(1.0);
    let c  = full.center();
    let r  = Rect::from_center_size(c, Vec2::new(full.width() * squish, full.height()));

    if r.width() >= 1.0 {
        painter.rect_filled(r, cr, bg);
        painter.rect_stroke(r, cr, Stroke::new(sw, border), StrokeKind::Middle);
    }
    if show_ch && tile.ch != ' ' {
        painter.text(c, Align2::CENTER_CENTER,
            tile.ch.to_ascii_uppercase().to_string(),
            FontId::proportional(full.width() * 0.54), C_WHITE);
    }
}

// ── eframe::App ───────────────────────────────────────────────────────────────

impl eframe::App for RusdleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.unstable_dt.min(0.05));
        self.game.tick(dt);

        // ── Flip animation ────────────────────────────────────────────────────
        if self.anim_row.is_some() {
            self.anim_elapsed += dt;
            ctx.request_repaint();
            if self.anim_elapsed >= TOTAL_ANIM {
                self.anim_row = None;
                // deferred_message is only set on win/loss, so stats are recorded only then
                if let Some(msg) = self.game.deferred_message.take() {
                    self.game.message       = Some(msg);
                    self.game.message_timer = 3.5;
                    self.record_stats();
                }
            }
        }
        if self.game.message_timer > 0.0 { ctx.request_repaint(); }

        let panels_open = self.show_stats || self.show_profiles;

        // ── Physical keyboard ─────────────────────────────────────────────────
        if self.game.status == GameStatus::Playing && !panels_open {
            ctx.input(|i| {
                for ev in &i.events {
                    if let egui::Event::Key { key, pressed: true, modifiers, .. } = ev {
                        if modifiers.ctrl || modifiers.alt || modifiers.mac_cmd { continue; }
                        match key {
                            Key::Enter => {
                                let before = self.game.current_row;
                                self.game.submit_guess();
                                if self.game.current_row > before {
                                    self.anim_row     = Some(before);
                                    self.anim_elapsed = 0.0;
                                }
                            }
                            Key::Backspace => self.game.delete_letter(),
                            k => {
                                let ch = match k {
                                    Key::A=>'a', Key::B=>'b', Key::C=>'c', Key::D=>'d',
                                    Key::E=>'e', Key::F=>'f', Key::G=>'g', Key::H=>'h',
                                    Key::I=>'i', Key::J=>'j', Key::K=>'k', Key::L=>'l',
                                    Key::M=>'m', Key::N=>'n', Key::O=>'o', Key::P=>'p',
                                    Key::Q=>'q', Key::R=>'r', Key::S=>'s', Key::T=>'t',
                                    Key::U=>'u', Key::V=>'v', Key::W=>'w', Key::X=>'x',
                                    Key::Y=>'y', Key::Z=>'z', _ => continue,
                                };
                                if self.game_start.is_none() { self.game_start = Some(Instant::now()); }
                                self.game.type_letter(ch);
                            }
                        }
                    }
                }
            });
        }

        // ── Central panel ─────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(C_BG))
            .show(ctx, |ui| {
                let full    = ui.available_rect_before_wrap();
                let painter = ui.painter();

                // Unified scale — every element scales together
                let scale = ((full.width() / IDEAL_W)
                    .min(full.height() / IDEAL_H)
                    .min(1.2))
                    .max(0.35);

                let tile       = BASE_TILE       * scale;
                let gap        = BASE_GAP        * scale;
                let key_w      = BASE_KEY_W      * scale;
                let key_h      = BASE_KEY_H      * scale;
                let key_gap    = BASE_KEY_GAP    * scale;
                let enter_w    = BASE_ENTER_W    * scale;
                let header_h   = BASE_HEADER     * scale;
                let bottom_bar = BASE_BOTTOM_BAR * scale;

                // ── Header ────────────────────────────────────────────────────
                let div_y = full.min.y + header_h;
                painter.text(
                    Pos2::new(full.center().x, full.min.y + header_h * 0.5),
                    Align2::CENTER_CENTER, "Rusdle",
                    FontId::proportional(30.0 * scale), C_WHITE);
                painter.line_segment(
                    [Pos2::new(full.min.x, div_y), Pos2::new(full.max.x, div_y)],
                    Stroke::new(1.0, C_DIVIDER));

                // Header icon buttons
                let btn_w = 64.0 * scale;
                let profiles_rect = Rect::from_min_size(
                    Pos2::new(full.min.x + 4.0 * scale, full.min.y),
                    Vec2::new(btn_w, header_h));
                let stats_rect = Rect::from_min_size(
                    Pos2::new(full.max.x - btn_w - 4.0 * scale, full.min.y),
                    Vec2::new(btn_w, header_h));

                if header_btn(painter, ctx, profiles_rect, icon_person, "PROFILES",
                              self.show_profiles, scale) {
                    self.show_profiles = !self.show_profiles;
                    self.show_stats    = false;
                    self.profile_error = None;
                }
                if header_btn(painter, ctx, stats_rect, icon_barchart, "STATS",
                              self.show_stats, scale) {
                    self.show_stats    = !self.show_stats;
                    self.show_profiles = false;
                }

                // Active profile name (tiny, bottom of header)
                painter.text(
                    Pos2::new(full.max.x - btn_w * 0.5 - 4.0 * scale, div_y - 9.0 * scale),
                    Align2::CENTER_CENTER,
                    &self.profiles.active_name.clone(),
                    FontId::proportional(8.5 * scale),
                    Color32::from_rgb(90, 90, 93));

                // ── Vertical layout ───────────────────────────────────────────
                let kbd_h    = key_h * 3.0 + key_gap * 2.0;
                let grid_h   = tile * MAX_GUESSES as f32 + gap * (MAX_GUESSES - 1) as f32;
                let grid_w   = tile * WORD_LENGTH as f32  + gap * (WORD_LENGTH  - 1) as f32;
                let gap_mid  = 16.0 * scale;
                let content_top  = div_y + 10.0 * scale;
                let content_bot  = full.max.y - bottom_bar;
                let block_h  = grid_h + gap_mid + kbd_h;
                let v_off    = ((content_bot - content_top - block_h) * 0.5).max(0.0);
                let grid_top = content_top + v_off;
                let kbd_top  = grid_top + grid_h + gap_mid;

                // ── Toast ─────────────────────────────────────────────────────
                if let Some(ref msg) = self.game.message.clone() {
                    let galley = painter.layout_no_wrap(
                        msg.clone(), FontId::proportional(14.0 * scale), Color32::BLACK);
                    let pad   = Vec2::new(14.0 * scale, 7.0 * scale);
                    let size  = galley.size() + pad * 2.0;
                    let toast = Rect::from_center_size(
                        Pos2::new(full.center().x, grid_top - size.y * 0.5 - 8.0 * scale), size);
                    painter.rect_filled(toast, CornerRadius::same(4), C_WHITE);
                    painter.text(toast.center(), Align2::CENTER_CENTER,
                        msg.as_str(), FontId::proportional(14.0 * scale), Color32::BLACK);
                }

                // ── Tile grid ─────────────────────────────────────────────────
                let grid_x = full.center().x - grid_w * 0.5;
                for row in 0..MAX_GUESSES {
                    for col in 0..WORD_LENGTH {
                        let tile_obj = &self.game.guesses[row][col];
                        let x = grid_x + col as f32 * (tile + gap);
                        let y = grid_top + row as f32 * (tile + gap);
                        let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(tile));
                        let flip = if self.anim_row == Some(row) {
                            let start = col as f32 * FLIP_STAGGER;
                            Some(((self.anim_elapsed - start) / FLIP_DURATION).clamp(0.0, 1.0))
                        } else { None };
                        draw_tile(painter, rect, tile_obj, flip, scale);
                    }
                }

                // ── On-screen keyboard ────────────────────────────────────────
                let kbd_rows: [&[&str]; 3] = [
                    &["Q","W","E","R","T","Y","U","I","O","P"],
                    &["A","S","D","F","G","H","J","K","L"],
                    &["ENTER","Z","X","C","V","B","N","M","DEL"],
                ];
                for (ri, row_keys) in kbd_rows.iter().enumerate() {
                    let row_w: f32 = row_keys.iter().enumerate().map(|(ki, k)| {
                        let w = if k.len() > 1 { enter_w } else { key_w };
                        w + if ki + 1 < row_keys.len() { key_gap } else { 0.0 }
                    }).sum();

                    let mut cur_x = full.center().x - row_w * 0.5;
                    let row_y = kbd_top + ri as f32 * (key_h + key_gap);

                    for &label in row_keys.iter() {
                        let is_wide  = label.len() > 1;
                        let w        = if is_wide { enter_w } else { key_w };
                        let key_rect = Rect::from_min_size(Pos2::new(cur_x, row_y), Vec2::new(w, key_h));
                        let cr       = CornerRadius::same((4.0 * scale).max(2.0) as u8);

                        let ch    = label.chars().next().unwrap().to_ascii_lowercase();
                        let state = if is_wide { None } else { self.game.keyboard.get(&ch) };
                        let bg    = key_bg(state);

                        let hov = !panels_open && ctx.input(|i| {
                            i.pointer.hover_pos().map_or(false, |p| key_rect.contains(p))
                        });
                        let draw_bg = if hov { lighten(bg, 25) } else { bg };

                        painter.rect_filled(key_rect, cr, draw_bg);

                        if label == "DEL" {
                            // Drawn icon instead of text glyph
                            icon_backspace(painter, key_rect, C_WHITE, draw_bg);
                        } else {
                            let font_sz = if label == "ENTER" { 11.5 * scale } else { 14.0 * scale };
                            painter.text(key_rect.center(), Align2::CENTER_CENTER, label,
                                FontId::proportional(font_sz), C_WHITE);
                        }

                        if !panels_open {
                            let clicked = ctx.input(|i| hov && i.pointer.primary_released());
                            if clicked && self.game.status == GameStatus::Playing {
                                match label {
                                    "ENTER" => {
                                        let before = self.game.current_row;
                                        self.game.submit_guess();
                                        if self.game.current_row > before {
                                            self.anim_row     = Some(before);
                                            self.anim_elapsed = 0.0;
                                        }
                                    }
                                    "DEL" => self.game.delete_letter(),
                                    _ => {
                                        if self.game_start.is_none() {
                                            self.game_start = Some(Instant::now());
                                        }
                                        self.game.type_letter(ch);
                                    }
                                }
                            }
                        }
                        cur_x += w + key_gap;
                    }
                }

                // ── Bottom bar: New Game button ───────────────────────────────
                if self.game.status != GameStatus::Playing {
                    let bw = 150.0 * scale;
                    let bh = 44.0  * scale;
                    let bar_center_y = content_bot + bottom_bar * 0.5;
                    let br = Rect::from_center_size(
                        Pos2::new(full.center().x, bar_center_y), Vec2::new(bw, bh));
                    let bhov = !panels_open && ctx.input(|i| {
                        i.pointer.hover_pos().map_or(false, |p| br.contains(p))
                    });
                    painter.rect_filled(br, CornerRadius::same(4),
                        if bhov { lighten(C_CORRECT, 18) } else { C_CORRECT });
                    painter.text(br.center(), Align2::CENTER_CENTER,
                        "New Game", FontId::proportional(14.0 * scale), C_WHITE);
                    if ctx.input(|i| bhov && i.pointer.primary_released()) {
                        self.next_game();
                    }
                }

                ui.allocate_rect(full, egui::Sense::hover());
            });

        // ── Stats window (draggable) ──────────────────────────────────────────
        if self.show_stats {
            let stats = self.profiles.active().stats.clone();
            egui::Window::new("  STATISTICS  ")
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.screen_rect().center() - Vec2::new(175.0, 220.0))
                .constrain(true)
                .min_width(340.0)
                .max_width(360.0)
                .show(ctx, |ui| {
                    ui.set_min_width(340.0);
                    ui.add_space(6.0);

                    // Four summary numbers
                    ui.horizontal(|ui| {
                        for (val, label) in [
                            (stats.games_played.to_string(),     "Played"),
                            (format!("{:.0}%", stats.win_pct()), "Win %"),
                            (stats.current_streak.to_string(),   "Streak"),
                            (stats.max_streak.to_string(),        "Max Streak"),
                        ] {
                            ui.vertical_centered(|ui| {
                                ui.set_min_width(78.0);
                                ui.label(RichText::new(val).size(28.0).strong().color(C_WHITE));
                                ui.label(RichText::new(label).size(11.0)
                                    .color(Color32::from_rgb(155, 155, 158)));
                            });
                        }
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    ui.label(RichText::new("GUESS DISTRIBUTION").size(12.0).strong()
                        .color(Color32::from_rgb(155, 155, 158)));
                    ui.add_space(6.0);

                    let max_count = stats.guess_dist.iter().max().copied().unwrap_or(0);
                    let winning_row = if self.game.status == GameStatus::Won {
                        Some(self.game.current_row.saturating_sub(1))
                    } else { None };

                    for (i, &count) in stats.guess_dist.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{}", i + 1)).size(13.0).color(C_WHITE));
                            ui.add_space(4.0);

                            let bar_max_w = 220.0_f32;
                            let row_h    = 22.0_f32;
                            let (row_rect, _) = ui.allocate_exact_size(
                                Vec2::new(bar_max_w + 28.0, row_h), egui::Sense::hover());
                            let p = ui.painter();

                            p.rect_filled(
                                Rect::from_min_size(row_rect.min, Vec2::new(bar_max_w, row_h)),
                                CornerRadius::same(2), C_BAR_EMPTY);

                            let fill_w = if max_count > 0 && count > 0 {
                                (count as f32 / max_count as f32 * bar_max_w).max(28.0)
                            } else { 0.0 };
                            if fill_w > 0.0 {
                                let bar_col = if winning_row == Some(i) { C_PRESENT } else { C_CORRECT };
                                p.rect_filled(
                                    Rect::from_min_size(row_rect.min, Vec2::new(fill_w, row_h)),
                                    CornerRadius::same(2), bar_col);
                                p.text(
                                    Pos2::new(row_rect.min.x + fill_w - 6.0, row_rect.center().y),
                                    Align2::RIGHT_CENTER,
                                    format!("{}", count),
                                    FontId::proportional(13.0), C_WHITE);
                            } else {
                                p.text(Pos2::new(row_rect.min.x + 6.0, row_rect.center().y),
                                    Align2::LEFT_CENTER, "0",
                                    FontId::proportional(13.0),
                                    Color32::from_rgb(100, 100, 103));
                            }
                        });
                        ui.add_space(3.0);
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    egui::Grid::new("extra_stats").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                        let dim = Color32::from_rgb(155, 155, 158);
                        macro_rules! stat_row {
                            ($label:expr, $val:expr) => {
                                ui.label(RichText::new($label).size(13.0).color(dim));
                                ui.label(RichText::new($val).size(13.0).strong().color(C_WHITE));
                                ui.end_row();
                            }
                        }
                        stat_row!("Fastest Win",  stats.fastest_fmt().unwrap_or_else(|| "—".into()));
                        stat_row!("Avg. Guesses", stats.avg_guesses().map(|a| format!("{:.2}", a))
                                                      .unwrap_or_else(|| "—".into()));
                        stat_row!("Time Played",  stats.total_time_fmt());
                        stat_row!("Fav. Starter", stats.favorite_start().unwrap_or_else(|| "—".into()));
                    });

                    ui.add_space(10.0);
                    ui.vertical_centered(|ui| {
                        if ui.button("  Close  ").clicked() { self.show_stats = false; }
                    });
                    ui.add_space(4.0);
                });
        }

        // ── Profiles window (draggable) ───────────────────────────────────────
        if self.show_profiles {
            egui::Window::new("  PROFILES  ")
                .collapsible(false)
                .resizable(false)
                .default_pos(ctx.screen_rect().center() - Vec2::new(160.0, 180.0))
                .constrain(true)
                .min_width(300.0)
                .max_width(340.0)
                .show(ctx, |ui| {
                    ui.set_min_width(300.0);
                    ui.add_space(6.0);

                    let list: Vec<(String, u32, u32, bool)> = self.profiles.profiles.iter()
                        .map(|p| (p.name.clone(), p.stats.games_played, p.stats.games_won,
                                  p.name == self.profiles.active_name))
                        .collect();

                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (name, played, won, is_active) in &list {
                            let win_pct = if *played > 0 {
                                *won as f32 / *played as f32 * 100.0
                            } else { 0.0 };

                            egui::Frame::default()
                                .fill(if *is_active { Color32::from_rgb(28, 38, 28) }
                                      else { Color32::TRANSPARENT })
                                .inner_margin(egui::Margin::symmetric(8, 5))
                                .corner_radius(CornerRadius::same(4))
                                .show(ui, |ui| {
                                    ui.set_min_width(280.0);

                                    // Row 1: dot + name | Switch / ✕ / Active on right
                                    ui.horizontal(|ui| {
                                        ui.colored_label(
                                            if *is_active { C_CORRECT }
                                            else { Color32::from_rgb(80, 80, 83) },
                                            if *is_active { "●" } else { "○" });
                                        ui.label(RichText::new(name).strong()
                                            .size(14.0).color(C_WHITE));

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if !is_active && list.len() > 1 {
                                                if ui.small_button("✕").clicked() {
                                                    self.profiles.delete_profile(name);
                                                    self.profiles.save();
                                                }
                                            }
                                            if !is_active {
                                                if ui.small_button("Switch").clicked() {
                                                    self.profiles.switch_to(name);
                                                    self.profiles.save();
                                                    self.next_game();
                                                }
                                            } else {
                                                ui.label(RichText::new("Active")
                                                    .size(11.0).color(C_CORRECT));
                                            }
                                        });
                                    });

                                    // Row 2: stats summary
                                    ui.label(RichText::new(
                                        format!("    {} played  ·  {:.0}% wins", played, win_pct)
                                    ).size(11.0).color(Color32::from_rgb(120, 120, 124)));
                                });
                            ui.add_space(3.0);
                        }
                    });

                    ui.separator();
                    ui.add_space(4.0);
                    ui.label(RichText::new("New profile").size(12.0)
                        .color(Color32::from_rgb(155, 155, 158)));
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        let field = egui::TextEdit::singleline(&mut self.new_profile_name)
                            .hint_text("Name…")
                            .desired_width(160.0);
                        let resp = ui.add(field);
                        let submit = ui.button("Create").clicked()
                            || (resp.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
                        if submit {
                            match self.profiles.create_profile(&self.new_profile_name.clone()) {
                                Ok(()) => {
                                    self.new_profile_name.clear();
                                    self.profile_error = None;
                                    self.profiles.save();
                                    self.next_game();
                                }
                                Err(e) => self.profile_error = Some(e.to_string()),
                            }
                        }
                    });

                    if let Some(ref err) = self.profile_error.clone() {
                        ui.label(RichText::new(err).size(11.0)
                            .color(Color32::from_rgb(200, 80, 80)));
                    }

                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        if ui.button("  Close  ").clicked() { self.show_profiles = false; }
                    });
                    ui.add_space(4.0);
                });
        }
    }
}
