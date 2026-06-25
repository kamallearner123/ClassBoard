use serde::{Deserialize, Serialize};

// ── Primitives ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Default for Color {
    fn default() -> Self {
        Color { r: 0.05, g: 0.05, b: 0.05, a: 1.0 }
    }
}

// ── Tools ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tool {
    #[default]
    Pen,
    Eraser,
    Fill,
    Spray,
    Select,
    Text,
    Sticky,
    Highlighter,
    Line,
    Rectangle,
    Circle,
    Arrow,
    Star,
    Heart,
    Triangle,
    Diamond,
    Cloud,
    Database,
    Actor,
    UmlClass,
    RoundedRect,
}

// ── Drawing elements ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stroke {
    pub points: Vec<Point>,
    pub color: Color,
    pub width: f64,
    #[serde(default)]
    pub is_highlighter: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ShapeKind {
    Line,
    Rectangle,
    Circle,
    Arrow,
    Star,
    Heart,
    Triangle,
    Diamond,
    Cloud,
    Database,
    Actor,
    UmlClass,
    RoundedRect,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spray {
    pub points: Vec<Point>,
    pub color: Color,
    pub radius: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shape {
    pub kind: ShapeKind,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub color: Color,
    pub width: f64,
}

/// A pasted/inserted image on the canvas.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasImage {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Raw PNG bytes of the image.
    pub png_data: Vec<u8>,
}

/// A table inserted on the canvas.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasText {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub font_family: String,
    pub font_size: f64,
    pub color: Color,
    #[serde(default)]
    pub bg_color: Option<Color>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasTable {
    pub x: f64,
    pub y: f64,
    pub rows: usize,
    pub cols: usize,
    pub cell_w: f64,
    pub cell_h: f64,
    /// cells[row][col]
    pub cells: Vec<Vec<String>>,
}

impl CanvasTable {
    pub fn new(rows: usize, cols: usize, x: f64, y: f64) -> Self {
        CanvasTable {
            x, y, rows, cols,
            cell_w: 120.0,
            cell_h: 36.0,
            cells: vec![vec![String::new(); cols]; rows],
        }
    }

    pub fn total_w(&self) -> f64 { self.cols as f64 * self.cell_w }
    pub fn total_h(&self) -> f64 { self.rows as f64 * self.cell_h }

    /// Returns (row, col) if the point hits a cell.
    pub fn hit_cell(&self, px: f64, py: f64) -> Option<(usize, usize)> {
        if px < self.x || py < self.y
            || px > self.x + self.total_w()
            || py > self.y + self.total_h()
        {
            return None;
        }
        let col = ((px - self.x) / self.cell_w) as usize;
        let row = ((py - self.y) / self.cell_h) as usize;
        if col < self.cols && row < self.rows { Some((row, col)) } else { None }
    }
}

// ── Note ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Note {
    pub name: String,
    pub bg_color: Option<Color>,
    pub rule_gap: Option<f64>,
    pub strokes: Vec<Stroke>,
    pub shapes: Vec<Shape>,
    pub images: Vec<CanvasImage>,
    pub tables: Vec<CanvasTable>,
    pub texts: Vec<CanvasText>,
    pub sprays: Vec<Spray>,
    /// Undo stack — not persisted.
    #[serde(skip)]
    pub undo_stack: Vec<(
        Option<Color>,
        Option<f64>,
        Vec<Stroke>,
        Vec<Shape>,
        Vec<CanvasImage>,
        Vec<CanvasTable>,
        Vec<CanvasText>,
        Vec<Spray>
    )>,
    /// Redo stack — not persisted.
    #[serde(skip)]
    pub redo_stack: Vec<(
        Option<Color>,
        Option<f64>,
        Vec<Stroke>,
        Vec<Shape>,
        Vec<CanvasImage>,
        Vec<CanvasTable>,
        Vec<CanvasText>,
        Vec<Spray>
    )>,
    #[serde(skip)]
    pub revision: u64,
}

impl Note {
    pub fn new(name: impl Into<String>) -> Self {
        Note { name: name.into(), ..Default::default() }
    }

    pub fn push_undo(&mut self) {
        self.revision += 1;
        self.undo_stack.push((
            self.bg_color.clone(),
            self.rule_gap.clone(),
            self.strokes.clone(),
            self.shapes.clone(),
            self.images.clone(),
            self.tables.clone(),
            self.texts.clone(),
            self.sprays.clone(),
        ));
        if self.undo_stack.len() > 50 { self.undo_stack.remove(0); }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        self.revision += 1;
        if let Some((bg, gap, s, sh, im, tb, tx, sp)) = self.undo_stack.pop() {
            self.redo_stack.push((
                self.bg_color.clone(),
                self.rule_gap.clone(),
                self.strokes.clone(),
                self.shapes.clone(),
                self.images.clone(),
                self.tables.clone(),
                self.texts.clone(),
                self.sprays.clone(),
            ));
            if self.redo_stack.len() > 50 { self.redo_stack.remove(0); }

            self.bg_color = bg;
            self.rule_gap = gap;
            self.strokes = s;
            self.shapes = sh;
            self.images = im;
            self.tables = tb;
            self.texts = tx;
            self.sprays = sp;
        }
    }

    pub fn redo(&mut self) {
        self.revision += 1;
        if let Some((bg, gap, s, sh, im, tb, tx, sp)) = self.redo_stack.pop() {
            self.undo_stack.push((
                self.bg_color.clone(),
                self.rule_gap.clone(),
                self.strokes.clone(),
                self.shapes.clone(),
                self.images.clone(),
                self.tables.clone(),
                self.texts.clone(),
                self.sprays.clone(),
            ));
            if self.undo_stack.len() > 50 { self.undo_stack.remove(0); }

            self.bg_color = bg;
            self.rule_gap = gap;
            self.strokes = s;
            self.shapes = sh;
            self.images = im;
            self.tables = tb;
            self.texts = tx;
            self.sprays = sp;
        }
    }
}

// ── Session ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub notes: Vec<Note>,
    pub current_note_idx: usize,
    /// Path to the file this session was last saved to (not persisted).
    #[serde(skip)]
    pub save_path: Option<String>,
}

impl Session {
    pub fn new(name: impl Into<String>) -> Self {
        Session {
            name: name.into(),
            notes: vec![Note::new("Note 1")],
            current_note_idx: 0,
            save_path: None,
        }
    }
}

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct AppState {
    pub sessions: Vec<Session>,
    pub current_session_idx: usize,
    pub current_tool: Tool,
    pub current_color: Color,
    pub line_width: f64,
    pub zoom_level: f64,
    pub canvas_width: f64,
    pub canvas_height: f64,
    pub clipboard_note: Note,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            sessions: vec![Session::new("Session 1")],
            current_session_idx: 0,
            current_tool: Tool::Pen,
            current_color: Color::default(),
            line_width: 2.0,
            zoom_level: 1.0,
            canvas_width: 1600.0,
            canvas_height: 1200.0,
            clipboard_note: Note::default(),
        }
    }
}

impl AppState {
    pub fn current_session(&self) -> &Session {
        &self.sessions[self.current_session_idx]
    }

    pub fn current_note(&self) -> &Note {
        let s = self.current_session();
        &s.notes[s.current_note_idx]
    }

    pub fn current_note_mut(&mut self) -> &mut Note {
        let si = self.current_session_idx;
        let ni = self.sessions[si].current_note_idx;
        &mut self.sessions[si].notes[ni]
    }
}
