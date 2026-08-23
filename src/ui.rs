//! Quiet, read-only secondary surfaces for history, corpus, and preferences.
//! UI text uses the bundled grotesque; Tom's handwriting never appears here.

use ab_glyph::FontRef;

use crate::fb::{BBox, SCREEN_H, SCREEN_W};
use crate::memory::MemoryStore;
use crate::oracle::ContextSnapshot;
use crate::preferences::{Mode, Preferences};
use crate::script;
use crate::surface::{Surface, BLACK, WHITE};

pub const UI_FONT_TTF: &[u8] = include_bytes!("../fonts/LiberationSans-Regular.ttf");
pub const PANEL_W: usize = SCREEN_W * 42 / 100;
const LABEL_PX: f32 = 32.0;
const TITLE_PX: f32 = 64.0;
const PAD: usize = 36;
const BLUE: u16 = 0x0335;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawerKind { History, Corpus }

pub struct Drawer {
    pub kind: DrawerKind,
    pub selection: Option<usize>,
    pub scroll: i32,
    pub expanded: bool,
    saved: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Close,
    History,
    Corpus,
    Replay(u64),
    Send,
    Erase,
    NewPage,
    Sleep,
    Settings,
    Dismiss,
    SetMode(Mode),
    ToggleIdle,
}

impl Drawer {
    pub fn open(surf: &Surface, kind: DrawerKind, selection: Option<usize>, scroll: i32) -> Self {
        Self { kind, selection, scroll, expanded: false,
            saved: surf.copy_rect(0, 0, PANEL_W, SCREEN_H) }
    }

    pub fn close(self, surf: &mut Surface) -> BBox {
        surf.paste_rect(0, 0, PANEL_W, SCREEN_H, &self.saved);
        panel_region()
    }

    pub fn scroll_by(&mut self, delta: i32) {
        self.scroll = (self.scroll + delta.signum()).max(0);
    }

    pub fn tap(&mut self, x: i32, y: i32, store: &Option<MemoryStore>) -> Action {
        if x < 0 || x >= PANEL_W as i32 { return Action::Close; }
        if y < 105 {
            if x < 100 { return Action::Close; }
            if x < PANEL_W as i32 / 2 { return Action::History; }
            return Action::Corpus;
        }
        if self.kind == DrawerKind::Corpus { return Action::None; }
        let Some(s) = store else { return Action::None };
        let rows = s.conversation_rows();
        if self.expanded && y > SCREEN_H as i32 - 150 {
            return self.selection.and_then(|i| rows.get(i)).map(|r| Action::Replay(r.id)).unwrap_or(Action::None);
        }
        let visible = 7usize;
        let base = rows.len().saturating_sub(visible + self.scroll as usize);
        let shown = rows.len().saturating_sub(base).min(visible);
        let y0 = 135 + (visible - shown) * 220;
        if y < y0 as i32 { return Action::None; }
        let i = base + ((y as usize - y0) / 220);
        if i < rows.len() {
            if self.selection == Some(i) { self.expanded = !self.expanded; }
            else { self.selection = Some(i); self.expanded = false; }
        }
        Action::None
    }
}

pub fn draw_drawer(surf: &mut Surface, font: &FontRef, store: &Option<MemoryStore>,
    snapshot: &ContextSnapshot, drawer: &Drawer) {
    surf.fill_rect(0, 0, PANEL_W, SCREEN_H, WHITE);
    surf.fill_rect(PANEL_W - 2, 0, 2, SCREEN_H, BLACK);
    text(surf, font, "×", LABEL_PX, PAD, 36, BLACK);
    text(surf, font, "HISTORY", LABEL_PX, 105, 36,
        if drawer.kind == DrawerKind::History { BLUE } else { BLACK });
    text(surf, font, "CORPUS", LABEL_PX, PANEL_W / 2 + 18, 36,
        if drawer.kind == DrawerKind::Corpus { BLUE } else { BLACK });
    rule(surf, 0, 104, PANEL_W, 2);
    match drawer.kind {
        DrawerKind::History => draw_history(surf, font, store, drawer),
        DrawerKind::Corpus => draw_corpus(surf, font, store, snapshot, drawer.scroll),
    }
}

fn draw_history(surf: &mut Surface, font: &FontRef, store: &Option<MemoryStore>, drawer: &Drawer) {
    let Some(store) = store else {
        text(surf, font, "MEMORY DISABLED", TITLE_PX, PAD, 170, BLACK);
        return;
    };
    let rows = store.conversation_rows();
    if rows.is_empty() {
        text(surf, font, "NO CONVERSATIONS YET", LABEL_PX, PAD, 170, BLACK);
        return;
    }
    if drawer.expanded {
        let Some(row) = drawer.selection.and_then(|i| rows.get(i)) else { return };
        text(surf, font, &row.date, LABEL_PX, PAD, 145, BLACK);
        text(surf, font, "YOU", LABEL_PX, PAD, 210, BLACK);
        let mut y = wrapped(surf, font, &row.transcript, LABEL_PX, PAD, 255, PANEL_W - 2 * PAD, BLACK, 8);
        y += 30;
        text(surf, font, "TOM", LABEL_PX, PAD, y, BLUE);
        wrapped(surf, font, &row.reply, LABEL_PX, PAD, y + 45, PANEL_W - 2 * PAD, BLACK, 12);
        rule(surf, PAD, SCREEN_H - 160, PANEL_W - 2 * PAD, 2);
        text(surf, font, "REPLAY ON PAGE", LABEL_PX, PAD, SCREEN_H - 115, BLUE);
        return;
    }
    let visible = 7usize;
    let start = rows.len().saturating_sub(visible + drawer.scroll as usize);
    let shown = rows.len().saturating_sub(start).min(visible);
    let mut y = 135usize + (visible - shown) * 220;
    for (i, row) in rows.iter().enumerate().skip(start).take(visible) {
        text(surf, font, &row.date, LABEL_PX, PAD, y, BLACK);
        text(surf, font, "YOU", LABEL_PX, PAD, y + 43, BLACK);
        text(surf, font, if row.preview.is_empty() { "(NO TRANSCRIPT)" } else { &row.preview },
            LABEL_PX, PAD + 82, y + 43, BLACK);
        text(surf, font, "TOM", LABEL_PX, PAD, y + 91, BLUE);
        text(surf, font, &one_line(&row.reply, 44), LABEL_PX, PAD + 82, y + 91, BLACK);
        if drawer.selection == Some(i) { surf.fill_rect(12, y - 5, 5, 128, BLUE); }
        rule(surf, PAD, y + 145, PANEL_W - 2 * PAD, 1);
        y += 220;
    }
}

fn draw_corpus(surf: &mut Surface, font: &FontRef, store: &Option<MemoryStore>, snap: &ContextSnapshot, scroll: i32) {
    let mut y = 145i32 - scroll * 120;
    let stats = store.as_ref().map(|s| s.stats()).unwrap_or_default();
    section(surf, font, "LOCAL MEMORY", &mut y);
    line(surf, font, &format!("STATE  {}", if store.is_some() { "ENABLED" } else { "DISABLED" }), &mut y);
    line(surf, font, &format!("STORED TURNS  {} / 400", stats.count), &mut y);
    line(surf, font, &format!("OLDEST  {}", stats.oldest.map(crate::memory::spoken_date).unwrap_or_else(|| "—".into())), &mut y);
    line(surf, font, &format!("NEWEST  {}", stats.newest.map(crate::memory::spoken_date).unwrap_or_else(|| "—".into())), &mut y);
    line(surf, font, "SEARCH  ALL LOCAL ENTRIES", &mut y);
    if let Some(store) = store {
        let entries = store.search("");
        for row in entries.iter().rev().take(8).rev() {
            line(surf, font, &format!("{}  {}", row.date, one_line(&row.preview, 38)), &mut y);
        }
    }
    y += 28;
    section(surf, font, "MODEL CONTEXT", &mut y);
    line(surf, font, &format!("PROVIDER  {}", snap.provider), &mut y);
    line(surf, font, &format!("MODEL  {}", snap.model), &mut y);
    line(surf, font, "RECENT DIALOGUE — EXACT", &mut y);
    for (you, tom) in &snap.context.history {
        context_text(surf, font, &format!("YOU  {you}"), &mut y);
        context_text(surf, font, &format!("TOM  {tom}"), &mut y);
    }
    line(surf, font, "CATALOG — EXACT", &mut y);
    for (i, row) in snap.context.catalog_lines.iter().enumerate() {
        let id = snap.context.catalog_ids.get(i).copied().unwrap_or(0);
        context_text(surf, font, row, &mut y);
        line(surf, font, &format!("SELECTED ID  {id}"), &mut y);
    }
    y += 25;
    wrapped(surf, font, "API CREDENTIALS AND UNCONFIGURED EXTERNAL KNOWLEDGE ARE NOT INCLUDED.",
        LABEL_PX, PAD, y.max(110) as usize, PANEL_W - 2 * PAD, BLACK, 4);
}

fn section(surf: &mut Surface, font: &FontRef, label: &str, y: &mut i32) {
    if *y > 105 && *y < SCREEN_H as i32 { text(surf, font, label, TITLE_PX, PAD, *y as usize, BLACK); }
    *y += 92;
}
fn line(surf: &mut Surface, font: &FontRef, value: &str, y: &mut i32) {
    if *y > 105 && *y < SCREEN_H as i32 - 40 { text(surf, font, value, LABEL_PX, PAD, *y as usize, BLACK); }
    *y += 48;
}

fn context_text(surf: &mut Surface, font: &FontRef, value: &str, y: &mut i32) {
    for part in script::wrap(font, value, LABEL_PX, (PANEL_W - 2 * PAD) as f32) {
        line(surf, font, &part, y);
    }
}

pub fn draw_controls(surf: &mut Surface, font: &FontRef, reply_visible: bool) -> Vec<u8> {
    let h = 82;
    let saved = surf.copy_rect(0, 0, SCREEN_W, h);
    surf.fill_rect(0, 0, SCREEN_W, h, WHITE);
    let labels = if reply_visible {
        ["DISMISS", "ERASE", "NEW PAGE", "HISTORY", "CORPUS", "SLEEP", "SETTINGS"]
    } else {
        ["SEND", "ERASE", "NEW PAGE", "HISTORY", "CORPUS", "SLEEP", "SETTINGS"]
    };
    let w = SCREEN_W / labels.len();
    for (i, label) in labels.iter().enumerate() {
        if i > 0 { surf.fill_rect(i * w, 0, 1, h, BLACK); }
        full_text(surf, font, label, LABEL_PX, i * w + 12, 25, if i == 3 || i == 4 { BLUE } else { BLACK });
    }
    rule(surf, 0, h - 2, SCREEN_W, 2);
    saved
}

pub fn control_action(x: i32, y: i32, reply_visible: bool) -> Action {
    if y < 0 || y >= 82 || x < 0 || x >= SCREEN_W as i32 { return Action::None; }
    match x as usize / (SCREEN_W / 7) {
        0 if reply_visible => Action::Dismiss,
        0 => Action::Send,
        1 => Action::Erase,
        2 => Action::NewPage,
        3 => Action::History,
        4 => Action::Corpus,
        5 => Action::Sleep,
        _ => Action::Settings,
    }
}

pub fn restore_controls(surf: &mut Surface, saved: &[u8]) {
    surf.paste_rect(0, 0, SCREEN_W, 82, saved);
}

pub fn draw_settings(surf: &mut Surface, font: &FontRef, prefs: Preferences) -> Vec<u8> {
    let saved = surf.copy_rect(0, 0, PANEL_W, SCREEN_H);
    surf.fill_rect(0, 0, PANEL_W, SCREEN_H, WHITE);
    surf.fill_rect(PANEL_W - 2, 0, 2, SCREEN_H, BLACK);
    text(surf, font, "×", LABEL_PX, PAD, 36, BLACK);
    text(surf, font, "SETTINGS", TITLE_PX, PAD, 145, BLACK);
    rule(surf, PAD, 245, PANEL_W - 2 * PAD, 2);
    text(surf, font, "STEALTH", LABEL_PX, PAD, 310, if prefs.mode == Mode::Stealth { BLUE } else { BLACK });
    text(surf, font, "GUIDED", LABEL_PX, PAD, 390, if prefs.mode == Mode::Guided { BLUE } else { BLACK });
    text(surf, font, "OPTIONAL IDLE-SEND", LABEL_PX, PAD, 510, BLACK);
    text(surf, font, if prefs.idle_send_ms == 0 { "OFF" } else { "ON" }, LABEL_PX, PAD, 570,
        if prefs.idle_send_ms == 0 { BLACK } else { BLUE });
    saved
}

pub fn settings_action(x: i32, y: i32) -> Action {
    if x < 0 || x >= PANEL_W as i32 || y < 100 { return Action::Close; }
    match y { 280..=365 => Action::SetMode(Mode::Stealth), 366..=460 => Action::SetMode(Mode::Guided),
        500..=650 => Action::ToggleIdle, _ => Action::None }
}

fn panel_region() -> BBox {
    let mut b = BBox::empty(); b.add(0, 0, 0); b.add(PANEL_W as i32 - 1, SCREEN_H as i32 - 1, 0); b
}
fn rule(s: &mut Surface, x: usize, y: usize, w: usize, h: usize) { s.fill_rect(x, y, w, h, BLACK); }
fn one_line(s: &str, max: usize) -> String { s.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(max).collect() }

fn text(surf: &mut Surface, font: &FontRef, value: &str, px: f32, x: usize, y: usize, color: u16) {
    render_text(surf, font, value, px, x, y, color, PANEL_W);
}

fn full_text(surf: &mut Surface, font: &FontRef, value: &str, px: f32, x: usize, y: usize, color: u16) {
    render_text(surf, font, value, px, x, y, color, SCREEN_W);
}

fn render_text(surf: &mut Surface, font: &FontRef, value: &str, px: f32, x: usize, y: usize, color: u16, limit_x: usize) {
    let raster = script::rasterize_line(font, value, px);
    for row in 0..raster.height {
        if y + row >= SCREEN_H { break; }
        for col in 0..raster.width {
            if x + col >= limit_x { break; }
            if raster.mask[row * raster.width + col] { surf.put_px((x + col) as i32, (y + row) as i32, color); }
        }
    }
}

fn wrapped(surf: &mut Surface, font: &FontRef, value: &str, px: f32, x: usize, y: usize,
    width: usize, color: u16, max_lines: usize) -> usize {
    let lines = script::wrap(font, value, px, width as f32);
    let mut yy = y;
    for line in lines.iter().take(max_lines) { text(surf, font, line, px, x, yy, color); yy += 42; }
    yy
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::PixFmt;
    #[test]
    fn controls_use_fixed_hit_regions() {
        assert_eq!(control_action(10, 20, false), Action::Send);
        assert_eq!(control_action((SCREEN_W * 3 / 7 + 2) as i32, 20, false), Action::History);
        assert_eq!(control_action(10, 100, false), Action::None);
    }

    #[test]
    fn drawer_touch_cannot_create_ink_and_reopen_state_is_preserved() {
        let mut bytes = vec![0xff; SCREEN_W * SCREEN_H * 4];
        let ptr = bytes.as_mut_ptr();
        let mut surf = Surface::new(ptr, bytes.len(), SCREEN_W, SCREEN_H, SCREEN_W * 4, PixFmt::Rgb32);
        let mut drawer = Drawer::open(&surf, DrawerKind::History, Some(3), 2);
        let ink = crate::ink::Ink::new();
        assert_eq!(drawer.tap(200, 500, &None), Action::None);
        assert!(ink.is_empty(), "touch routing must not add page ink");
        let selection = drawer.selection;
        let scroll = drawer.scroll;
        drawer.close(&mut surf);
        let reopened = Drawer::open(&surf, DrawerKind::History, selection, scroll);
        assert_eq!(reopened.selection, Some(3));
        assert_eq!(reopened.scroll, 2);
    }
}
