mod state;
mod canvas;

use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, ColorButton,
    DrawingArea, EventControllerKey, GestureDrag,
    Label, MenuButton, Orientation, Paned, Popover, Scale,
    ScrolledWindow, Separator, ToggleButton, CssProvider, HeaderBar,
};
use gtk::gdk::RGBA;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{HashSet, HashMap};

use state::{AppState, Color, Shape, ShapeKind, Stroke, Point, Tool, CanvasImage, CanvasTable, CanvasText, Spray};
use canvas::{draw_stroke, draw_shape, draw_canvas_image, draw_canvas_text, draw_spray};

/// Load the elegant light theme CSS into the GTK display.
fn apply_css() {
    let css = r#"
    window {
        background: radial-gradient(circle at top left, #f8fafc 0%, #e2e8f0 100%);
    }
    .toolbar {
        background: rgba(241, 245, 249, 0.95);
        border-bottom: 1px solid rgba(0, 0, 0, 0.08);
        padding: 6px 12px;
        box-shadow: 0 2px 10px rgba(0,0,0,0.03);
    }
    .toolbar button {
        background: transparent;
        border: 1px solid transparent;
        border-radius: 8px;
        color: #475569;
        font-weight: 500;
        padding: 6px 12px;
        transition: all 0.2s ease;
    }
    .toolbar button:hover {
        background: #f1f5f9;
        border-color: #cbd5e1;
        color: #0f172a;
    }
    .toolbar button:active,
    .toolbar button:checked {
        background: linear-gradient(135deg, #3b82f6 0%, #8b5cf6 100%);
        color: white;
        border-color: transparent;
        box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);
    }
    .sidebar {
        background: rgba(241, 245, 249, 0.90);
        border-right: 1px solid rgba(0, 0, 0, 0.08);
    }
    .sidebar button.selected {
        background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%);
        color: white;
        font-weight: 600;
        border-radius: 8px;
        box-shadow: 0 2px 6px rgba(99, 102, 241, 0.3);
    }
    .sidebar button {
        background: transparent;
        border: none;
        border-radius: 8px;
        color: #475569;
        padding: 6px 10px;
        font-size: 13px;
        transition: all 0.2s;
    }
    .sidebar button:hover {
        background: #f1f5f9;
        color: #0f172a;
    }
    .sidebar .close-btn {
        color: #c0392b;
        min-width: 22px;
        min-height: 22px;
        padding: 2px 4px;
    }
    .sidebar .close-btn:hover {
        background: #fde8e8;
        color: #922b21;
    }
    .sidebar .rename-btn {
        color: #5b6acf;
        min-width: 22px;
        min-height: 22px;
        padding: 2px 4px;
    }
    .sidebar .add-btn {
        color: #27ae60;
        min-width: 22px;
        min-height: 22px;
        padding: 2px 4px;
    }
    .sidebar separator {
        background: #d1d5e0;
        min-height: 1px;
        margin: 2px 0;
    }
    colorbutton button {
        border-radius: 6px;
    }
    .tool-group {
        background: #f8fafc;
        border: 1px solid #e2e8f0;
        border-radius: 8px;
        padding: 4px;
        margin-left: 2px;
        margin-right: 2px;
        box-shadow: 0 1px 2px rgba(0,0,0,0.03);
    }
    "#;
    let provider = CssProvider::new();
    provider.load_from_data(css);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn update_window_title(window: &ApplicationWindow, app_state: &SharedApp) {
    let app = app_state.borrow();
    let session = &app.sessions[app.current_session_idx];
    let file_name = match &session.save_path {
        Some(path) => {
            std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path)
                .to_string()
        }
        None => format!("{} (Unsaved)", session.name),
    };
    let title = format!("✏️ roughnote - {}", file_name);

    window.set_title(Some(&title));

    if let Some(titlebar) = window.titlebar() {
        if let Some(header) = titlebar.downcast_ref::<gtk::HeaderBar>() {
            if let Some(widget) = header.title_widget() {
                if let Some(label) = widget.downcast_ref::<gtk::Label>() {
                    label.set_text(&title);
                }
            } else {
                let title_lbl = Label::new(Some(&title));
                title_lbl.add_css_class("title");
                title_lbl.set_margin_start(12);
                title_lbl.set_margin_end(12);
                header.set_title_widget(Some(&title_lbl));
            }
        }
    }
}

#[derive(Default)]
struct DrawState {
    drawing: bool,
    drag_start: Option<(f64, f64)>,
    current_stroke: Option<Stroke>,
    current_spray: Option<Spray>,
    preview_shape: Option<Shape>,

    // Selection state
    selection_rect: Option<(f64, f64, f64, f64)>,
    preview_selection: Option<(f64, f64, f64, f64)>,
    is_moving_selection: bool,
    
    selected_strokes: HashSet<usize>,
    selected_shapes: HashSet<usize>,
    selected_images: HashSet<usize>,
    selected_tables: HashSet<usize>,
    selected_texts: HashSet<usize>,

    // Original state before a move (for delta translation)
    moving_original_strokes: Vec<(usize, Stroke)>,
    moving_original_shapes: Vec<(usize, Shape)>,
    moving_original_images: Vec<(usize, CanvasImage)>,
    moving_original_tables: Vec<(usize, CanvasTable)>,
    moving_original_texts: Vec<(usize, CanvasText)>,

    // References to tool toggle buttons for keyboard shortcuts
    tool_buttons: HashMap<state::Tool, ToggleButton>,
}

type SharedApp = Rc<RefCell<AppState>>;
type SharedDraw = Rc<RefCell<DrawState>>;

fn main() {
    // We use the cairo renderer because GTK4's hardware acceleration (NGL/Vulkan) 
    // introduces massive texture-upload latency for full-screen Cairo DrawingAreas.
    unsafe {
        std::env::set_var("GSK_RENDERER", "cairo");
    }
    
    let app = Application::builder()
        .application_id("org.roughnote.roughnote")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("roughnote")
        .default_width(1920)
        .default_height(1080)
        .build();

    let app_state: SharedApp = Rc::new(RefCell::new(AppState::default()));
    let draw_state: SharedDraw = Rc::new(RefCell::new(DrawState::default()));

    let root = GtkBox::new(Orientation::Vertical, 0);
    
    // Professional HeaderBar
    let header = HeaderBar::new();
    header.set_show_title_buttons(true);
    let title_lbl = Label::new(Some("✏️ roughnote"));
    title_lbl.add_css_class("title");
    title_lbl.set_margin_start(12);
    title_lbl.set_margin_end(12);
    header.set_title_widget(Some(&title_lbl));
    window.set_titlebar(Some(&header));

    let da = DrawingArea::new();
    da.set_hexpand(true);
    da.set_vexpand(true);
    da.set_cursor_from_name(Some("crosshair"));

    let sidebar_list = GtkBox::new(Orientation::Vertical, 2);
    sidebar_list.set_margin_top(6);
    sidebar_list.set_margin_start(4);
    sidebar_list.set_margin_end(4);

    let toolbar = build_toolbar(&app_state, &draw_state, &da, &sidebar_list, &window);
    let toolbar_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .build();
    toolbar_scroll.set_child(Some(&toolbar));
    root.append(&toolbar_scroll);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_vexpand(true);

    let sidebar_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .build();
    sidebar_scroll.set_size_request(220, -1);
    sidebar_scroll.set_child(Some(&sidebar_list));
    
    let canvas_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .build();
    canvas_scroll.set_child(Some(&da));
    
    paned.set_start_child(Some(&sidebar_scroll));
    paned.set_end_child(Some(&canvas_scroll));
    paned.set_position(220);
    root.append(&paned);

    setup_canvas(&da, &app_state, &draw_state);
    setup_keyboard(&window, &app_state, &da, &draw_state, &sidebar_list);

    rebuild_sidebar(&sidebar_list, &app_state, &da, &window);

    // Apply elegant light theme
    apply_css();

    // Mark sidebar with CSS class
    sidebar_list.add_css_class("sidebar");

    window.set_child(Some(&root));
    window.present();
}

fn setup_keyboard(
    window: &ApplicationWindow,
    app_state: &SharedApp,
    da: &DrawingArea,
    draw_state: &SharedDraw,
    sidebar_list: &GtkBox,
) {
    let ctrl = EventControllerKey::new();
    ctrl.connect_key_pressed({
        let app_state = app_state.clone();
        let da = da.clone();
        let ds = draw_state.clone();
        let sidebar_list = sidebar_list.clone();
        let window_clone = window.clone();
        move |_, key, _, mods| {
            let is_editing = if let Some(focus) = gtk::prelude::GtkWindowExt::focus(&window_clone) {
                let type_name = focus.type_().name();
                focus.is::<gtk::Editable>() || type_name == "GtkTextView" || type_name == "GtkText"
            } else {
                false
            };

            let has_ctrl = mods.contains(gtk::gdk::ModifierType::CONTROL_MASK);
            let has_shift = mods.contains(gtk::gdk::ModifierType::SHIFT_MASK);

            if has_ctrl {
                // Ctrl + Z (Undo) / Ctrl + Shift + Z (Redo)
                if key == gtk::gdk::Key::z || key == gtk::gdk::Key::Z {
                    if has_shift {
                        state::Note::redo(app_state.borrow_mut().current_note_mut());
                    } else {
                        state::Note::undo(app_state.borrow_mut().current_note_mut());
                    }
                    da.queue_draw();
                    return gtk::glib::Propagation::Stop;
                }

                // Ctrl + Y (Redo)
                if key == gtk::gdk::Key::y || key == gtk::gdk::Key::Y {
                    state::Note::redo(app_state.borrow_mut().current_note_mut());
                    da.queue_draw();
                    return gtk::glib::Propagation::Stop;
                }

                // Ctrl + C (Copy)
                if key == gtk::gdk::Key::c || key == gtk::gdk::Key::C {
                    if !is_editing {
                        let d = ds.borrow();
                        if !d.selected_strokes.is_empty() || !d.selected_shapes.is_empty() || !d.selected_images.is_empty() || !d.selected_tables.is_empty() || !d.selected_texts.is_empty() {
                            let mut app = app_state.borrow_mut();
                            let note = app.current_note_mut();
                            
                            let mut clip = state::Note::default();
                            for idx in &d.selected_strokes { if let Some(s) = note.strokes.get(*idx) { clip.strokes.push(s.clone()); } }
                            for idx in &d.selected_shapes { if let Some(sh) = note.shapes.get(*idx) { clip.shapes.push(sh.clone()); } }
                            for idx in &d.selected_images { if let Some(img) = note.images.get(*idx) { clip.images.push(img.clone()); } }
                            for idx in &d.selected_tables { if let Some(tbl) = note.tables.get(*idx) { clip.tables.push(tbl.clone()); } }
                            for idx in &d.selected_texts { if let Some(txt) = note.texts.get(*idx) { clip.texts.push(txt.clone()); } }
                            
                            app.clipboard_note = clip;
                        }
                        return gtk::glib::Propagation::Stop;
                    }
                }

                // Ctrl + X (Cut)
                if key == gtk::gdk::Key::x || key == gtk::gdk::Key::X {
                    if !is_editing {
                        let mut d = ds.borrow_mut();
                        if !d.selected_strokes.is_empty() || !d.selected_shapes.is_empty() || !d.selected_images.is_empty() || !d.selected_tables.is_empty() || !d.selected_texts.is_empty() {
                            let mut app = app_state.borrow_mut();
                            let note = app.current_note_mut();
                            note.push_undo();
                            
                            let mut clip = state::Note::default();
                            
                            let mut del_s: Vec<_> = d.selected_strokes.iter().copied().collect(); del_s.sort_unstable_by(|a,b| b.cmp(a));
                            let mut del_sh: Vec<_> = d.selected_shapes.iter().copied().collect(); del_sh.sort_unstable_by(|a,b| b.cmp(a));
                            let mut del_img: Vec<_> = d.selected_images.iter().copied().collect(); del_img.sort_unstable_by(|a,b| b.cmp(a));
                            let mut del_tbl: Vec<_> = d.selected_tables.iter().copied().collect(); del_tbl.sort_unstable_by(|a,b| b.cmp(a));
                            let mut del_txt: Vec<_> = d.selected_texts.iter().copied().collect(); del_txt.sort_unstable_by(|a,b| b.cmp(a));
                            
                            for i in del_s { clip.strokes.push(note.strokes.remove(i)); }
                            for i in del_sh { clip.shapes.push(note.shapes.remove(i)); }
                            for i in del_img { clip.images.push(note.images.remove(i)); }
                            for i in del_tbl { clip.tables.push(note.tables.remove(i)); }
                            for i in del_txt { clip.texts.push(note.texts.remove(i)); }
                            
                            app.clipboard_note = clip;
                            
                            d.selection_rect = None;
                            d.selected_strokes.clear();
                            d.selected_shapes.clear();
                            d.selected_images.clear();
                            d.selected_tables.clear();
                            d.selected_texts.clear();
                            
                            da.queue_draw();
                        }
                        return gtk::glib::Propagation::Stop;
                    }
                }

                // Ctrl + V (Paste)
                if key == gtk::gdk::Key::v || key == gtk::gdk::Key::V {
                    if !is_editing {
                        let mut has_internal = false;
                        {
                            let mut app = app_state.borrow_mut();
                            let clip = app.clipboard_note.clone();
                            if !clip.strokes.is_empty() || !clip.shapes.is_empty() || !clip.images.is_empty() || !clip.tables.is_empty() || !clip.texts.is_empty() {
                                let note = app.current_note_mut();
                                note.push_undo();
                                for mut s in clip.strokes { for p in &mut s.points { p.x += 20.0; p.y += 20.0; } note.strokes.push(s); }
                                for mut sh in clip.shapes { sh.x1 += 20.0; sh.y1 += 20.0; sh.x2 += 20.0; sh.y2 += 20.0; note.shapes.push(sh); }
                                for mut img in clip.images { img.x += 20.0; img.y += 20.0; note.images.push(img); }
                                for mut tb in clip.tables { tb.x += 20.0; tb.y += 20.0; note.tables.push(tb); }
                                for mut tx in clip.texts { tx.x += 20.0; tx.y += 20.0; note.texts.push(tx); }
                                has_internal = true;
                            }
                        }
                        if has_internal {
                            let mut d = ds.borrow_mut();
                            d.selection_rect = None;
                            d.selected_strokes.clear();
                            d.selected_shapes.clear();
                            d.selected_images.clear();
                            d.selected_tables.clear();
                            d.selected_texts.clear();
                            da.queue_draw();
                            return gtk::glib::Propagation::Stop;
                        }

                        // Fallback to system image clipboard
                        let clipboard = window_clone.clipboard();
                        let as_c = app_state.clone();
                        let da_c = da.clone();
                        clipboard.read_texture_async(gtk::gio::Cancellable::NONE, move |result| {
                            if let Ok(Some(texture)) = result {
                                let tmp = "/tmp/sp_paste.png";
                                if texture.save_to_png(tmp).is_ok() {
                                    if let Ok(bytes) = std::fs::read(tmp) {
                                        std::fs::remove_file(tmp).ok();
                                        let mut app = as_c.borrow_mut();
                                        let note = app.current_note_mut();
                                        note.push_undo();
                                        note.images.push(state::CanvasImage {
                                            x: 20.0,
                                            y: 20.0,
                                            width: texture.width() as f64,
                                            height: texture.height() as f64,
                                            png_data: bytes,
                                        });
                                    }
                                }
                                da_c.queue_draw();
                            }
                        });
                        return gtk::glib::Propagation::Stop;
                    }
                }

                // Ctrl + S (Save) / Ctrl + Shift + S (Save As)
                if key == gtk::gdk::Key::s || key == gtk::gdk::Key::S {
                    let win = window_clone.clone();
                    let as_ = app_state.clone();
                    if has_shift {
                        save_as_dialog(&as_, &win);
                    } else {
                        let path = as_.borrow().sessions[as_.borrow().current_session_idx].save_path.clone();
                        match path {
                            Some(p) => {
                                do_save_session(&as_, &p);
                                update_window_title(&win, &as_);
                            }
                            None => save_as_dialog(&as_, &win),
                        }
                    }
                    return gtk::glib::Propagation::Stop;
                }

                // Ctrl + O (Open Session)
                if key == gtk::gdk::Key::o || key == gtk::gdk::Key::O {
                    open_dialog(&app_state, &sidebar_list, &da, &window_clone);
                    return gtk::glib::Propagation::Stop;
                }

                // Ctrl + N (New Session)
                if key == gtk::gdk::Key::n || key == gtk::gdk::Key::N {
                    let idx = {
                        let mut s = app_state.borrow_mut();
                        let n = s.sessions.len() + 1;
                        s.sessions.push(state::Session::new(format!("Session {n}")));
                        s.sessions.len() - 1
                    };
                    app_state.borrow_mut().current_session_idx = idx;
                    rebuild_sidebar(&sidebar_list, &app_state, &da, &window_clone);
                    da.queue_draw();
                    return gtk::glib::Propagation::Stop;
                }

                // Ctrl + T (New Note)
                if key == gtk::gdk::Key::t || key == gtk::gdk::Key::T {
                    {
                        let mut s = app_state.borrow_mut();
                        let si = s.current_session_idx;
                        let n = s.sessions[si].notes.len() + 1;
                        s.sessions[si].notes.push(state::Note::new(format!("Note {n}")));
                        let last = s.sessions[si].notes.len() - 1;
                        s.sessions[si].current_note_idx = last;
                    }
                    rebuild_sidebar(&sidebar_list, &app_state, &da, &window_clone);
                    da.queue_draw();
                    return gtk::glib::Propagation::Stop;
                }

                // Zoom shortcuts
                let is_plus = key == gtk::gdk::Key::plus || key == gtk::gdk::Key::equal || key == gtk::gdk::Key::KP_Add;
                let is_minus = key == gtk::gdk::Key::minus || key == gtk::gdk::Key::KP_Subtract;
                let is_zero = key == gtk::gdk::Key::_0 || key == gtk::gdk::Key::KP_0;

                if is_plus {
                    let mut app = app_state.borrow_mut();
                    app.zoom_level += 0.2;
                    da.set_size_request((app.canvas_width * app.zoom_level) as i32, (app.canvas_height * app.zoom_level) as i32);
                    da.queue_draw();
                    return gtk::glib::Propagation::Stop;
                }

                if is_minus {
                    let mut app = app_state.borrow_mut();
                    if app.zoom_level > 0.2 {
                        app.zoom_level -= 0.2;
                    }
                    da.set_size_request((app.canvas_width * app.zoom_level) as i32, (app.canvas_height * app.zoom_level) as i32);
                    da.queue_draw();
                    return gtk::glib::Propagation::Stop;
                }

                if is_zero {
                    let mut app = app_state.borrow_mut();
                    app.zoom_level = 1.0;
                    da.set_size_request((app.canvas_width * app.zoom_level) as i32, (app.canvas_height * app.zoom_level) as i32);
                    da.queue_draw();
                    return gtk::glib::Propagation::Stop;
                }
            } else {
                // Delete or Backspace key
                if key == gtk::gdk::Key::Delete || key == gtk::gdk::Key::BackSpace {
                    let mut d = ds.borrow_mut();
                    if !d.selected_strokes.is_empty() || !d.selected_shapes.is_empty() || !d.selected_images.is_empty() || !d.selected_tables.is_empty() || !d.selected_texts.is_empty() {
                        let mut app = app_state.borrow_mut();
                        let note = app.current_note_mut();
                        note.push_undo();
                        
                        let mut del_s: Vec<_> = d.selected_strokes.iter().copied().collect(); del_s.sort_unstable_by(|a,b| b.cmp(a));
                        let mut del_sh: Vec<_> = d.selected_shapes.iter().copied().collect(); del_sh.sort_unstable_by(|a,b| b.cmp(a));
                        let mut del_img: Vec<_> = d.selected_images.iter().copied().collect(); del_img.sort_unstable_by(|a,b| b.cmp(a));
                        let mut del_tbl: Vec<_> = d.selected_tables.iter().copied().collect(); del_tbl.sort_unstable_by(|a,b| b.cmp(a));
                        let mut del_txt: Vec<_> = d.selected_texts.iter().copied().collect(); del_txt.sort_unstable_by(|a,b| b.cmp(a));
                        
                        for i in del_s { note.strokes.remove(i); }
                        for i in del_sh { note.shapes.remove(i); }
                        for i in del_img { note.images.remove(i); }
                        for i in del_tbl { note.tables.remove(i); }
                        for i in del_txt { note.texts.remove(i); }
                        
                        d.selection_rect = None;
                        d.selected_strokes.clear();
                        d.selected_shapes.clear();
                        d.selected_images.clear();
                        d.selected_tables.clear();
                        d.selected_texts.clear();
                        
                        da.queue_draw();
                        return gtk::glib::Propagation::Stop;
                    }
                }

                // Tool selection shortcuts (when not editing text inputs)
                if !is_editing {
                    let tool_opt = if key == gtk::gdk::Key::p || key == gtk::gdk::Key::P {
                        Some(Tool::Pen)
                    } else if key == gtk::gdk::Key::e || key == gtk::gdk::Key::E {
                        Some(Tool::Eraser)
                    } else if key == gtk::gdk::Key::r || key == gtk::gdk::Key::R {
                        Some(Tool::Spray)
                    } else if key == gtk::gdk::Key::f || key == gtk::gdk::Key::F {
                        Some(Tool::Fill)
                    } else if key == gtk::gdk::Key::t || key == gtk::gdk::Key::T {
                        Some(Tool::Text)
                    } else if key == gtk::gdk::Key::s || key == gtk::gdk::Key::S || key == gtk::gdk::Key::v || key == gtk::gdk::Key::V {
                        Some(Tool::Select)
                    } else {
                        None
                    };

                    if let Some(tool) = tool_opt {
                        let btn_opt = {
                            let d = ds.borrow();
                            d.tool_buttons.get(&tool).cloned()
                        };
                        if let Some(btn) = btn_opt {
                            btn.set_active(true);
                            return gtk::glib::Propagation::Stop;
                        }
                    }
                }
            }
            gtk::glib::Propagation::Proceed
        }
    });
    window.add_controller(ctrl);
}


// ── Toolbar ───────────────────────────────────────────────────────────────────

fn build_toolbar(
    app_state: &SharedApp,
    draw_state: &SharedDraw,
    da: &DrawingArea,
    sidebar_list: &GtkBox,
    window: &ApplicationWindow,
) -> GtkBox {
    let bar = GtkBox::new(Orientation::Horizontal, 6);
    bar.set_margin_start(8);
    bar.set_margin_end(8);
    bar.set_margin_top(6);
    bar.set_margin_bottom(6);
    bar.add_css_class("toolbar");

    // ── Group 1: File Operations ──
    let file_group = GtkBox::new(Orientation::Horizontal, 2);
    file_group.add_css_class("tool-group");

    let new_sess = Button::with_label("+ Session");
    {
        let (as_, sl, da_) = (app_state.clone(), sidebar_list.clone(), da.clone());
        let win = window.clone();
        new_sess.connect_clicked(move |_| {
            let idx = {
                let mut s = as_.borrow_mut();
                let n = s.sessions.len() + 1;
                s.sessions.push(state::Session::new(format!("Session {n}")));
                s.sessions.len() - 1
            };
            as_.borrow_mut().current_session_idx = idx;
            rebuild_sidebar(&sl, &as_, &da_, &win);
            da_.queue_draw();
        });
    }
    file_group.append(&new_sess);

    let save_btn = Button::with_label("💾 Save");
    {
        let (as_, win) = (app_state.clone(), window.clone());
        save_btn.connect_clicked(move |_| {
            let path = as_.borrow().sessions[as_.borrow().current_session_idx].save_path.clone();
            match path {
                Some(p) => {
                    do_save_session(&as_, &p);
                    update_window_title(&win, &as_);
                }
                None => save_as_dialog(&as_, &win),
            }
        });
    }
    file_group.append(&save_btn);

    let save_as_btn = Button::with_label("Save As…");
    {
        let (as_, win) = (app_state.clone(), window.clone());
        save_as_btn.connect_clicked(move |_| { save_as_dialog(&as_, &win); });
    }
    file_group.append(&save_as_btn);

    let export_btn = MenuButton::new();
    export_btn.set_label("⬇️ Export");
    let export_pop = Popover::new();
    let export_box = GtkBox::new(Orientation::Vertical, 4);
    export_box.set_margin_top(8); export_box.set_margin_bottom(8);
    export_box.set_margin_start(8); export_box.set_margin_end(8);
    
    let exp_png = Button::with_label("Export Note as PNG");
    {
        let (as_, win) = (app_state.clone(), window.clone());
        exp_png.connect_clicked(move |_| { export_note_dialog(&as_, &win, "png"); });
    }
    let exp_jpg = Button::with_label("Export Note as JPEG");
    {
        let (as_, win) = (app_state.clone(), window.clone());
        exp_jpg.connect_clicked(move |_| { export_note_dialog(&as_, &win, "jpeg"); });
    }
    let exp_pdf = Button::with_label("Export Note as PDF");
    {
        let (as_, win) = (app_state.clone(), window.clone());
        exp_pdf.connect_clicked(move |_| { export_note_dialog(&as_, &win, "pdf"); });
    }
    let exp_sess_pdf = Button::with_label("Export Session as PDF");
    {
        let (as_, win) = (app_state.clone(), window.clone());
        exp_sess_pdf.connect_clicked(move |_| { export_session_dialog(&as_, &win); });
    }
    
    export_box.append(&exp_png);
    export_box.append(&exp_jpg);
    export_box.append(&exp_pdf);
    export_box.append(&Separator::new(Orientation::Horizontal));
    export_box.append(&exp_sess_pdf);
    
    export_pop.set_child(Some(&export_box));
    export_btn.set_popover(Some(&export_pop));
    file_group.append(&export_btn);

    let gdrive_btn = Button::with_label("☁️ GDrive");
    gdrive_btn.set_tooltip_text(Some("Save to Google Drive"));
    {
        let win = window.clone();
        gdrive_btn.connect_clicked(move |_| {
            let dialog = gtk::MessageDialog::builder()
                .transient_for(&win).modal(true).message_type(gtk::MessageType::Info).buttons(gtk::ButtonsType::Ok)
                .text("Google Drive Integration")
                .secondary_text("Placeholder for GDrive sync.").build();
            dialog.connect_response(|d, _| d.close());
            dialog.present();
        });
    }
    file_group.append(&gdrive_btn);

    let open_btn = Button::with_label("📂 Open");
    {
        let (as_, sl, da_, win) = (app_state.clone(), sidebar_list.clone(), da.clone(), window.clone());
        open_btn.connect_clicked(move |_| { open_dialog(&as_, &sl, &da_, &win); });
    }
    file_group.append(&open_btn);
    bar.append(&file_group);

    // ── Group 2: Tools ──
    let tools_group = GtkBox::new(Orientation::Horizontal, 2);
    tools_group.add_css_class("tool-group");

    let select_btn = ToggleButton::builder().label("⬚ Select").build();
    select_btn.set_tooltip_text(Some("Select, move, and delete items"));
    {
        let as_ = app_state.clone();
        let da_ = da.clone();
        select_btn.connect_toggled(move |b| {
            if b.is_active() { as_.borrow_mut().current_tool = Tool::Select; da_.set_cursor_from_name(Some("default")); }
        });
    }
    tools_group.append(&select_btn);

    let text_btn = ToggleButton::builder().label("T Text").build();
    text_btn.set_group(Some(&select_btn));
    {
        let (as_, ds_, da_) = (app_state.clone(), draw_state.clone(), da.clone());
        text_btn.connect_toggled(move |b| {
            if b.is_active() { as_.borrow_mut().current_tool = Tool::Text; ds_.borrow_mut().selection_rect = None; da_.set_cursor_from_name(Some("text")); da_.queue_draw(); }
        });
    }
    tools_group.append(&text_btn);

    let fill_btn = ToggleButton::builder().label("🪣 Fill").build();
    fill_btn.set_group(Some(&select_btn));
    {
        let (as_, ds_, da_) = (app_state.clone(), draw_state.clone(), da.clone());
        fill_btn.connect_toggled(move |b| {
            if b.is_active() { as_.borrow_mut().current_tool = Tool::Fill; ds_.borrow_mut().selection_rect = None; da_.set_cursor_from_name(Some("cell")); da_.queue_draw(); }
        });
    }
    tools_group.append(&fill_btn);

    let pen_btn = ToggleButton::builder().label("✏️ Pen").build();
    pen_btn.set_group(Some(&select_btn));
    pen_btn.set_active(true);
    {
        let (as_, ds_, da_) = (app_state.clone(), draw_state.clone(), da.clone());
        pen_btn.connect_toggled(move |b| {
            if b.is_active() { as_.borrow_mut().current_tool = Tool::Pen; ds_.borrow_mut().selection_rect = None; da_.set_cursor_from_name(Some("crosshair")); da_.queue_draw(); }
        });
    }
    tools_group.append(&pen_btn);

    let eraser_btn = ToggleButton::builder().label("🧹 Eraser").build();
    eraser_btn.set_group(Some(&pen_btn));
    {
        let (as_, ds_, da_) = (app_state.clone(), draw_state.clone(), da.clone());
        eraser_btn.connect_toggled(move |b| {
            if b.is_active() { as_.borrow_mut().current_tool = Tool::Eraser; ds_.borrow_mut().selection_rect = None; da_.set_cursor_from_name(Some("crosshair")); da_.queue_draw(); }
        });
    }
    tools_group.append(&eraser_btn);

    let spray_btn = ToggleButton::builder().label("💨 Spray").build();
    spray_btn.set_group(Some(&pen_btn));
    {
        let (as_, ds_, da_) = (app_state.clone(), draw_state.clone(), da.clone());
        spray_btn.connect_toggled(move |b| {
            if b.is_active() { as_.borrow_mut().current_tool = Tool::Spray; ds_.borrow_mut().selection_rect = None; da_.set_cursor_from_name(Some("crosshair")); da_.queue_draw(); }
        });
    }
    tools_group.append(&spray_btn);

    // Populate tool buttons reference mapping for keyboard shortcuts
    {
        let mut ds = draw_state.borrow_mut();
        ds.tool_buttons.insert(Tool::Select, select_btn.clone());
        ds.tool_buttons.insert(Tool::Text, text_btn.clone());
        ds.tool_buttons.insert(Tool::Fill, fill_btn.clone());
        ds.tool_buttons.insert(Tool::Pen, pen_btn.clone());
        ds.tool_buttons.insert(Tool::Eraser, eraser_btn.clone());
        ds.tool_buttons.insert(Tool::Spray, spray_btn.clone());
    }

    bar.append(&tools_group);

    // ── Group 3: Shapes ──
    let shapes_group = GtkBox::new(Orientation::Horizontal, 2);
    shapes_group.add_css_class("tool-group");

    let shapes_palette: Rc<RefCell<Option<gtk::Window>>> = Rc::new(RefCell::new(None));
    let shapes_btn = Button::with_label("🔷 Shapes");
    shapes_btn.set_tooltip_text(Some("Open Shapes Palette"));
    
    {
        let palette_ref = shapes_palette.clone();
        let parent_win = window.clone();
        let as_ = app_state.clone();
        let ds_ = draw_state.clone();
        let da_ = da.clone();
        let pen_btn_ = pen_btn.clone();
        
        shapes_btn.connect_clicked(move |_| {
            if let Some(win) = palette_ref.borrow().as_ref() {
                win.present();
                return;
            }

            let win = gtk::Window::builder()
                .title("Shapes Palette")
                .transient_for(&parent_win)
                .destroy_with_parent(true)
                .default_width(260)
                .default_height(400)
                .build();
                
            win.connect_close_request({
                let pref = palette_ref.clone();
                move |_| {
                    *pref.borrow_mut() = None;
                    gtk::glib::Propagation::Proceed
                }
            });

            let scroll = ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vscrollbar_policy(gtk::PolicyType::Automatic)
                .build();

            let main_box = GtkBox::new(Orientation::Vertical, 12);
            main_box.set_margin_start(12);
            main_box.set_margin_end(12);
            main_box.set_margin_top(12);
            main_box.set_margin_bottom(12);

            let categories = [
                ("Basic Shapes", vec![
                    ("╱ Line", Tool::Line), ("□ Rect", Tool::Rectangle),
                    ("○ Circle", Tool::Circle), ("△ Triangle", Tool::Triangle),
                    ("◇ Diamond", Tool::Diamond), ("▭ RRect", Tool::RoundedRect),
                ]),
                ("Diagramming &amp; UML", vec![
                    ("→ Arrow", Tool::Arrow), ("👤 Actor", Tool::Actor),
                    ("🗂 UML", Tool::UmlClass),
                ]),
                ("Icons &amp; Others", vec![
                    ("☁ Cloud", Tool::Cloud), ("🛢 Data", Tool::Database),
                    ("★ Star", Tool::Star), ("♥ Heart", Tool::Heart),
                ]),
            ];

            for (title, items) in categories.iter() {
                let lbl = Label::new(None);
                lbl.set_markup(&format!("<span size='large' weight='bold'>{}</span>", title));
                lbl.set_halign(gtk::Align::Start);
                main_box.append(&lbl);
                
                let grid = gtk::Grid::builder()
                    .row_spacing(6)
                    .column_spacing(6)
                    .column_homogeneous(true)
                    .build();

                for (i, (label, tool)) in items.iter().enumerate() {
                    let btn = ToggleButton::builder().label(*label).build();
                    btn.set_group(Some(&pen_btn_));
                    
                    let (as2, ds2, da2) = (as_.clone(), ds_.clone(), da_.clone());
                    let tc = tool.clone();
                    btn.connect_toggled(move |b| {
                        if b.is_active() {
                            as2.borrow_mut().current_tool = tc.clone();
                            ds2.borrow_mut().selection_rect = None;
                            da2.set_cursor_from_name(Some("crosshair"));
                            da2.queue_draw();
                        }
                    });
                    grid.attach(&btn, (i % 2) as i32, (i / 2) as i32, 1, 1);
                }
                main_box.append(&grid);
                main_box.append(&Separator::new(Orientation::Horizontal));
            }

            scroll.set_child(Some(&main_box));
            win.set_child(Some(&scroll));
            
            *palette_ref.borrow_mut() = Some(win.clone());
            win.present();
        });
    }

    shapes_group.append(&shapes_btn);
    bar.append(&shapes_group);

    // ── Group 4: Actions ──
    let actions_group = GtkBox::new(Orientation::Horizontal, 4);
    actions_group.add_css_class("tool-group");

    let undo_btn = Button::with_label("↩ Undo");
    {
        let (as_, da_) = (app_state.clone(), da.clone());
        undo_btn.connect_clicked(move |_| { as_.borrow_mut().current_note_mut().undo(); da_.queue_draw(); });
    }
    actions_group.append(&undo_btn);

    actions_group.append(&Label::new(Some("Color:")));
    let color_btn = ColorButton::new();
    color_btn.set_rgba(&RGBA::new(0.05, 0.05, 0.05, 1.0));
    {
        let as_ = app_state.clone();
        color_btn.connect_color_set(move |b| {
            let rgba = b.rgba();
            as_.borrow_mut().current_color = Color { r: rgba.red() as f64, g: rgba.green() as f64, b: rgba.blue() as f64, a: rgba.alpha() as f64 };
        });
    }
    actions_group.append(&color_btn);

    actions_group.append(&Label::new(Some("Width:")));
    let scale = Scale::with_range(Orientation::Horizontal, 1.0, 30.0, 0.5);
    scale.set_value(2.0);
    scale.set_size_request(100, -1);
    {
        let as_ = app_state.clone();
        scale.connect_value_changed(move |s| { as_.borrow_mut().line_width = s.value(); });
    }
    actions_group.append(&scale);
    bar.append(&actions_group);

    // ── Group 5: View & Clipboard ──
    let view_group = GtkBox::new(Orientation::Horizontal, 2);
    view_group.add_css_class("tool-group");

    let zoom_in_btn = gtk::Button::with_label("🔍+");
    {
        let (as_, da_) = (app_state.clone(), da.clone());
        zoom_in_btn.connect_clicked(move |_| {
            let mut app = as_.borrow_mut(); app.zoom_level += 0.2;
            da_.set_size_request((app.canvas_width * app.zoom_level) as i32, (app.canvas_height * app.zoom_level) as i32);
            da_.queue_draw();
        });
    }
    view_group.append(&zoom_in_btn);

    let zoom_out_btn = gtk::Button::with_label("🔍-");
    {
        let (as_, da_) = (app_state.clone(), da.clone());
        zoom_out_btn.connect_clicked(move |_| {
            let mut app = as_.borrow_mut(); if app.zoom_level > 0.2 { app.zoom_level -= 0.2; }
            da_.set_size_request((app.canvas_width * app.zoom_level) as i32, (app.canvas_height * app.zoom_level) as i32);
            da_.queue_draw();
        });
    }
    view_group.append(&zoom_out_btn);

    let cut_btn = Button::with_label("✂️ Cut");
    cut_btn.set_tooltip_text(Some("Cut selected items"));
    {
        let (as_, ds_, da_) = (app_state.clone(), draw_state.clone(), da.clone());
        cut_btn.connect_clicked(move |_| {
            let mut d = ds_.borrow_mut();
            if !d.selected_strokes.is_empty() || !d.selected_shapes.is_empty() || !d.selected_images.is_empty() || !d.selected_tables.is_empty() || !d.selected_texts.is_empty() {
                let mut app = as_.borrow_mut();
                let note = app.current_note_mut();
                note.push_undo();
                
                // Clear clipboard
                let mut clip = state::Note::default();

                let mut del_s: Vec<_> = d.selected_strokes.iter().copied().collect(); del_s.sort_unstable_by(|a,b| b.cmp(a));
                let mut del_sh: Vec<_> = d.selected_shapes.iter().copied().collect(); del_sh.sort_unstable_by(|a,b| b.cmp(a));
                let mut del_img: Vec<_> = d.selected_images.iter().copied().collect(); del_img.sort_unstable_by(|a,b| b.cmp(a));
                let mut del_tbl: Vec<_> = d.selected_tables.iter().copied().collect(); del_tbl.sort_unstable_by(|a,b| b.cmp(a));
                let mut del_txt: Vec<_> = d.selected_texts.iter().copied().collect(); del_txt.sort_unstable_by(|a,b| b.cmp(a));
                
                for i in del_s { clip.strokes.push(note.strokes.remove(i)); }
                for i in del_sh { clip.shapes.push(note.shapes.remove(i)); }
                for i in del_img { clip.images.push(note.images.remove(i)); }
                for i in del_tbl { clip.tables.push(note.tables.remove(i)); }
                for i in del_txt { clip.texts.push(note.texts.remove(i)); }
                
                app.clipboard_note = clip;
                
                d.selection_rect = None;
                d.selected_strokes.clear();
                d.selected_shapes.clear();
                d.selected_images.clear();
                d.selected_tables.clear();
                d.selected_texts.clear();
                
                da_.queue_draw();
            }
        });
    }
    view_group.append(&cut_btn);

    let paste_btn = Button::with_label("📋 Paste");
    {
        let (as_, ds_, da_) = (app_state.clone(), draw_state.clone(), da.clone());
        paste_btn.connect_clicked(move |btn| {
            // First check internal vector clipboard
            let mut has_internal = false;
            {
                let mut app = as_.borrow_mut();
                let clip = app.clipboard_note.clone();
                if !clip.strokes.is_empty() || !clip.shapes.is_empty() || !clip.images.is_empty() || !clip.tables.is_empty() || !clip.texts.is_empty() {
                    let note = app.current_note_mut();
                    note.push_undo();
                    // Offset pasted items slightly to distinguish them
                    for mut s in clip.strokes { for p in &mut s.points { p.x += 20.0; p.y += 20.0; } note.strokes.push(s); }
                    for mut sh in clip.shapes { sh.x1 += 20.0; sh.y1 += 20.0; sh.x2 += 20.0; sh.y2 += 20.0; note.shapes.push(sh); }
                    for mut img in clip.images { img.x += 20.0; img.y += 20.0; note.images.push(img); }
                    for mut tb in clip.tables { tb.x += 20.0; tb.y += 20.0; note.tables.push(tb); }
                    for mut tx in clip.texts { tx.x += 20.0; tx.y += 20.0; note.texts.push(tx); }
                    has_internal = true;
                    // Note: pasted items are not automatically selected here for simplicity
                }
            }
            if has_internal {
                let mut d = ds_.borrow_mut();
                d.selection_rect = None;
                d.selected_strokes.clear();
                d.selected_shapes.clear();
                d.selected_images.clear();
                d.selected_tables.clear();
                d.selected_texts.clear();
                da_.queue_draw();
                return;
            }

            // Fallback to system image clipboard
            let clipboard = btn.clipboard();
            let (as_c, da_c) = (as_.clone(), da_.clone());
            clipboard.read_texture_async(gtk::gio::Cancellable::NONE, move |result| {
                if let Ok(Some(texture)) = result {
                    let tmp = "/tmp/sp_paste.png";
                    if texture.save_to_png(tmp).is_ok() {
                        if let Ok(bytes) = std::fs::read(tmp) {
                            std::fs::remove_file(tmp).ok();
                            let mut app = as_c.borrow_mut(); let note = app.current_note_mut(); note.push_undo();
                            note.images.push(state::CanvasImage { x: 20.0, y: 20.0, width: texture.width() as f64, height: texture.height() as f64, png_data: bytes });
                        }
                    }
                    da_c.queue_draw();
                }
            });
        });
    }
    view_group.append(&paste_btn);
    bar.append(&view_group);

    bar
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

fn rebuild_sidebar(
    sidebar: &GtkBox,
    app_state: &SharedApp,
    da: &DrawingArea,
    window: &ApplicationWindow,
) {
    update_window_title(window, app_state);

    while let Some(child) = sidebar.first_child() { sidebar.remove(&child); }

    let n_sessions = app_state.borrow().sessions.len();
    for si in 0..n_sessions {
        // ── Session row ──
        let row = GtkBox::new(Orientation::Horizontal, 2);

        let sess_name = app_state.borrow().sessions[si].name.clone();
        let is_cur = app_state.borrow().current_session_idx == si;
        // Session button — plain Button with .selected CSS class avoids
        // ToggleButton's toggled-signal conflicts during rebuild_sidebar.
        let sess_btn = Button::builder().label(&sess_name).hexpand(true).build();
        if is_cur { sess_btn.add_css_class("selected"); }
        {
            let (as_, sl, da, win) = (app_state.clone(), sidebar.clone(), da.clone(), window.clone());
            sess_btn.connect_clicked(move |_| {
                as_.borrow_mut().current_session_idx = si;
                rebuild_sidebar(&sl, &as_, &da, &win);
                da.queue_draw();
            });
        }
        row.append(&sess_btn);

        // Rename session
        let rename_s = Button::with_label("✎");
        rename_s.set_tooltip_text(Some("Rename session"));
        {
            let (as_, sl, da, win) = (app_state.clone(), sidebar.clone(), da.clone(), window.clone());
            let cur_name = sess_name.clone();
            rename_s.connect_clicked(move |_| {
                let (as2, sl2, da2, win2) = (as_.clone(), sl.clone(), da.clone(), win.clone());
                rename_dialog(&win, "Rename Session", &cur_name, move |new_name| {
                    as2.borrow_mut().sessions[si].name = new_name;
                    rebuild_sidebar(&sl2, &as2, &da2, &win2);
                });
            });
        }
        rename_s.add_css_class("rename-btn");
        row.append(&rename_s);

        // Add note
        let add_note = Button::with_label("+");
        add_note.set_tooltip_text(Some("Add note"));
        {
            let (as_, sl, da, win) = (app_state.clone(), sidebar.clone(), da.clone(), window.clone());
            add_note.connect_clicked(move |_| {
                {
                    let mut s = as_.borrow_mut();
                    let n = s.sessions[si].notes.len() + 1;
                    s.sessions[si].notes.push(state::Note::new(format!("Note {n}")));
                    let last = s.sessions[si].notes.len() - 1;
                    s.sessions[si].current_note_idx = last;
                    s.current_session_idx = si;
                }
                rebuild_sidebar(&sl, &as_, &da, &win);
                da.queue_draw();
            });
        }
        add_note.add_css_class("add-btn");
        row.append(&add_note);

        // Close session — use idle_add_local_once to defer rebuild and
        // avoid GTK re-entry / RefCell borrow panics during event processing.
        let close_s = Button::with_label("✕");
        close_s.set_tooltip_text(Some("Close session"));
        close_s.add_css_class("close-btn");
        {
            let (as_, sl, da, win) = (app_state.clone(), sidebar.clone(), da.clone(), window.clone());
            close_s.connect_clicked(move |_| {
                // Step 1: mutate state immediately (no GTK calls here)
                {
                    let mut s = as_.borrow_mut();
                    if s.sessions.len() == 1 {
                        return;
                    }
                    s.sessions.remove(si);
                    if s.current_session_idx >= s.sessions.len() {
                        s.current_session_idx = s.sessions.len() - 1;
                    }
                }
                // Step 2: defer UI rebuild to next GLib event loop tick
                let (as2, sl2, da2, win2) = (as_.clone(), sl.clone(), da.clone(), win.clone());
                gtk::glib::idle_add_local_once(move || {
                    rebuild_sidebar(&sl2, &as2, &da2, &win2);
                    da2.queue_draw();
                });
            });
        }
        row.append(&close_s);
        sidebar.append(&row);

        // ── Notes for this session ──
        let n_notes = app_state.borrow().sessions[si].notes.len();
        for ni in 0..n_notes {
            let note_row = GtkBox::new(Orientation::Horizontal, 2);
            note_row.set_margin_start(16);

            let note_name = app_state.borrow().sessions[si].notes[ni].name.clone();
            let is_cur_note = is_cur && app_state.borrow().sessions[si].current_note_idx == ni;
            // Note button — same plain-Button approach as sessions.
            let note_btn = Button::builder().label(&note_name).hexpand(true).build();
            if is_cur_note { note_btn.add_css_class("selected"); }
            {
                let (as_, sl, da, win) = (app_state.clone(), sidebar.clone(), da.clone(), window.clone());
                note_btn.connect_clicked(move |_| {
                    let mut s = as_.borrow_mut();
                    s.current_session_idx = si;
                    s.sessions[si].current_note_idx = ni;
                    drop(s);
                    rebuild_sidebar(&sl, &as_, &da, &win);
                    da.queue_draw();
                });
            }
            note_row.append(&note_btn);

            // Rename note
            let rename_n = Button::with_label("✎");
            rename_n.set_tooltip_text(Some("Rename note"));
            {
                let (as_, sl, da, win) = (app_state.clone(), sidebar.clone(), da.clone(), window.clone());
                let cur_name = note_name.clone();
                rename_n.connect_clicked(move |_| {
                    let (as2, sl2, da2, win2) = (as_.clone(), sl.clone(), da.clone(), win.clone());
                    rename_dialog(&win, "Rename Note", &cur_name, move |new_name| {
                        as2.borrow_mut().sessions[si].notes[ni].name = new_name;
                        rebuild_sidebar(&sl2, &as2, &da2, &win2);
                    });
                });
            }
            note_row.append(&rename_n);
            sidebar.append(&note_row);
        }

        sidebar.append(&Separator::new(Orientation::Horizontal));
    }
}

// ── Rename dialog ─────────────────────────────────────────────────────────────

fn rename_dialog(
    window: &ApplicationWindow,
    title: &str,
    current: &str,
    on_ok: impl Fn(String) + 'static,
) {
    let dialog = gtk::Dialog::new();
    dialog.set_title(Some(title));
    dialog.set_transient_for(Some(window));
    dialog.set_modal(true);
    dialog.set_default_size(300, -1);

    let area = dialog.content_area();
    area.set_spacing(8);
    area.set_margin_top(12);
    area.set_margin_bottom(12);
    area.set_margin_start(12);
    area.set_margin_end(12);

    let entry = gtk::Entry::new();
    entry.set_text(current);
    entry.set_activates_default(true);
    area.append(&entry);

    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    let ok = dialog.add_button("Rename", gtk::ResponseType::Ok);
    ok.add_css_class("suggested-action");
    dialog.set_default_response(gtk::ResponseType::Ok);

    dialog.connect_response(move |d, resp| {
        if resp == gtk::ResponseType::Ok {
            let t = entry.text().to_string();
            if !t.is_empty() { on_ok(t); }
        }
        d.destroy();
    });
    dialog.present();
}

// ── Canvas ────────────────────────────────────────────────────────────────────

fn setup_canvas(da: &DrawingArea, app_state: &SharedApp, draw_state: &SharedDraw) {
    {
        let app = app_state.borrow();
        da.set_size_request((app.canvas_width * app.zoom_level) as i32, (app.canvas_height * app.zoom_level) as i32);
    }
    
    da.set_draw_func({
        let (as_, ds) = (app_state.clone(), draw_state.clone());
        move |_, cr, _, _| {
            cr.set_antialias(cairo::Antialias::Best);
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint().unwrap();

            let app = as_.borrow();
            let zoom = app.zoom_level;
            cr.scale(zoom, zoom);
            
            let note = app.current_note();
            if let Some(bg) = &note.bg_color {
                cr.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
                cr.paint().unwrap();
            }
            
            for img in &note.images { draw_canvas_image(cr, img); }
            for txt in &note.texts { draw_canvas_text(cr, txt); }
            for s in &note.strokes { draw_stroke(cr, s); }
            for sp in &note.sprays { draw_spray(cr, sp); }
            for sh in &note.shapes { draw_shape(cr, sh); }
            
            let is_select = app.current_tool == Tool::Select;
            drop(app);
            
            let d = ds.borrow();
            if let Some(ref s) = d.current_stroke { draw_stroke(cr, s); }
            if let Some(ref sp) = d.current_spray { draw_spray(cr, sp); }
            if let Some(ref sh) = d.preview_shape { draw_shape(cr, sh); }
            
            // Draw selection box only if the tool is Select
            if is_select {
                if let Some((rx, ry, rw, rh)) = d.selection_rect.or(d.preview_selection) {
                    cr.set_source_rgba(0.2, 0.5, 1.0, 1.0);
                    cr.set_line_width(1.5);
                    cr.set_dash(&[4.0, 4.0], 0.0);
                    cr.rectangle(rx, ry, rw, rh);
                    cr.stroke().unwrap();
                    cr.set_dash(&[], 0.0);
                    
                    cr.set_source_rgba(0.2, 0.5, 1.0, 0.05);
                    cr.rectangle(rx, ry, rw, rh);
                    cr.fill().unwrap();
                }
            }
        }
    });

    // We removed EventControllerMotion for custom cursor to fix latency

    // Drag
    let drag = GestureDrag::new();

    drag.connect_drag_begin({
        let (as_, ds) = (app_state.clone(), draw_state.clone());
        move |_, sx, sy| {
            let app = as_.borrow();
            let z = app.zoom_level;
            let (sx, sy) = (sx / z, sy / z);
            let (color, width, tool) = (app.current_color.clone(), app.line_width, app.current_tool.clone());
            drop(app);
            let mut d = ds.borrow_mut();
            d.drawing = true;
            d.drag_start = Some((sx, sy));
            if tool == Tool::Pen {
                d.current_stroke = Some(Stroke {
                    points: vec![Point { x: sx, y: sy }],
                    color, width,
                });
            } else if tool == Tool::Spray {
                d.current_spray = Some(Spray {
                    points: vec![],
                    color, radius: width * 3.0,
                });
            } else if tool == Tool::Select {
                let mut clicked_inside = false;
                if let Some((rx, ry, rw, rh)) = d.selection_rect {
                    if sx >= rx && sx <= rx + rw && sy >= ry && sy <= ry + rh {
                        clicked_inside = true;
                    }
                }
                
                if clicked_inside {
                    d.is_moving_selection = true;
                    d.moving_original_strokes.clear();
                    d.moving_original_shapes.clear();
                    d.moving_original_images.clear();
                    d.moving_original_tables.clear();
                    
                    let mut app = as_.borrow_mut();
                    let note = app.current_note_mut();
                    note.push_undo();
                    
                    for idx in d.selected_strokes.clone() { if let Some(s) = note.strokes.get(idx) { d.moving_original_strokes.push((idx, s.clone())); } }
                    for idx in d.selected_shapes.clone() { if let Some(sh) = note.shapes.get(idx) { d.moving_original_shapes.push((idx, sh.clone())); } }
                    for idx in d.selected_images.clone() { if let Some(img) = note.images.get(idx) { d.moving_original_images.push((idx, img.clone())); } }
                    for idx in d.selected_tables.clone() { if let Some(tbl) = note.tables.get(idx) { d.moving_original_tables.push((idx, tbl.clone())); } }
                    for idx in d.selected_texts.clone() { if let Some(txt) = note.texts.get(idx) { d.moving_original_texts.push((idx, txt.clone())); } }
                } else {
                    d.is_moving_selection = false;
                    d.selection_rect = None;
                    d.selected_strokes.clear();
                    d.selected_shapes.clear();
                    d.selected_images.clear();
                    d.selected_tables.clear();
                    d.selected_texts.clear();
                    d.preview_selection = Some((sx, sy, 0.0, 0.0));
                }
            }
        }
    });

    drag.connect_drag_update({
        let (as_, ds, da) = (app_state.clone(), draw_state.clone(), da.clone());
        move |gesture, ox, oy| {
            let (sx, sy) = gesture.start_point().unwrap_or((0.0, 0.0));
            let mut app = as_.borrow_mut();
            let z = app.zoom_level;
            let (sx, sy) = (sx / z, sy / z);
            let (ox, oy) = (ox / z, oy / z);
            let (ex, ey) = (sx + ox, sy + oy);
            
            // Auto expand canvas if we reach 80% of current bounds
            let mut expanded = false;
            if ex > app.canvas_width * 0.8 { app.canvas_width *= 1.2; expanded = true; }
            if ey > app.canvas_height * 0.8 { app.canvas_height *= 1.2; expanded = true; }
            if expanded {
                da.set_size_request((app.canvas_width * app.zoom_level) as i32, (app.canvas_height * app.zoom_level) as i32);
            }
            
            let (tool, color, width) = (app.current_tool.clone(), app.current_color.clone(), app.line_width);
            drop(app);

            // Removed manual cursor tracking for performance
            // let mut d = ds.borrow_mut();
            // d.cursor = Some((ex, ey));

            let mut d = ds.borrow_mut();
            match tool {
                Tool::Pen => {
                    if let Some(ref mut stroke) = d.current_stroke {
                        if let Some(last) = stroke.points.last() {
                            let dx = ex - last.x;
                            let dy = ey - last.y;
                            // Only add if moved ≥ 1px — captures more input for lower latency feel
                            if dx * dx + dy * dy > 1.0 {
                                stroke.points.push(Point { x: ex, y: ey });
                            }
                        }
                    }
                }
                Tool::Spray => {
                    if let Some(ref mut spray) = d.current_spray {
                        let r = spray.radius;
                        // Use time-based pseudo random for burst
                        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos();
                        let mut seed = nanos;
                        let mut next_f64 = || -> f64 {
                            seed ^= seed << 13;
                            seed ^= seed >> 17;
                            seed ^= seed << 5;
                            (seed % 1000) as f64 / 1000.0
                        };
                        for _ in 0..15 {
                            let a = next_f64() * std::f64::consts::TAU;
                            let dist = next_f64().sqrt() * r;
                            spray.points.push(Point { x: ex + a.cos() * dist, y: ey + a.sin() * dist });
                        }
                    }
                }
                Tool::Eraser => {
                    drop(d);
                    let mut app = as_.borrow_mut();
                    let r = 14.0_f64;
                    let note = app.current_note_mut();
                    note.strokes.retain(|s| {
                        !s.points.iter().any(|p| (p.x - ex).hypot(p.y - ey) < r)
                    });
                    note.shapes.retain(|sh| {
                        let mx = (sh.x1 + sh.x2) / 2.0;
                        let my = (sh.y1 + sh.y2) / 2.0;
                        (mx - ex).hypot(my - ey) >= r * 3.0
                    });
                    da.queue_draw();
                    return;
                }
                Tool::Select => {
                    if d.is_moving_selection {
                        let mut app = as_.borrow_mut();
                        let note = app.current_note_mut();
                        for (idx, orig_stroke) in &d.moving_original_strokes {
                            if let Some(s) = note.strokes.get_mut(*idx) {
                                s.points.clear();
                                for p in &orig_stroke.points {
                                    s.points.push(Point { x: p.x + ox, y: p.y + oy });
                                }
                            }
                        }
                        for (idx, orig_shape) in &d.moving_original_shapes {
                            if let Some(sh) = note.shapes.get_mut(*idx) {
                                sh.x1 = orig_shape.x1 + ox; sh.y1 = orig_shape.y1 + oy;
                                sh.x2 = orig_shape.x2 + ox; sh.y2 = orig_shape.y2 + oy;
                            }
                        }
                        for (idx, orig_img) in &d.moving_original_images {
                            if let Some(img) = note.images.get_mut(*idx) {
                                img.x = orig_img.x + ox; img.y = orig_img.y + oy;
                            }
                        }
                        for (idx, orig_tbl) in &d.moving_original_tables {
                            if let Some(tbl) = note.tables.get_mut(*idx) {
                                tbl.x = orig_tbl.x + ox; tbl.y = orig_tbl.y + oy;
                            }
                        }
                        for (idx, orig_txt) in &d.moving_original_texts {
                            if let Some(txt) = note.texts.get_mut(*idx) {
                                txt.x = orig_txt.x + ox; txt.y = orig_txt.y + oy;
                            }
                        }
                        
                        // Update selection rect visual position
                        if let Some((rx, ry, rw, rh)) = d.selection_rect {
                            d.preview_selection = Some((rx + ox, ry + oy, rw, rh));
                        }
                    } else {
                        let (start_x, start_y) = d.drag_start.unwrap_or((sx, sy));
                        let rx = start_x.min(ex);
                        let ry = start_y.min(ey);
                        let rw = (start_x - ex).abs();
                        let rh = (start_y - ey).abs();
                        d.preview_selection = Some((rx, ry, rw, rh));
                    }
                }
                shape_tool => {
                    let (start_x, start_y) = d.drag_start.unwrap_or((sx, sy));
                    let kind = match shape_tool {
                        Tool::Line => ShapeKind::Line,
                        Tool::Rectangle => ShapeKind::Rectangle,
                        Tool::Circle => ShapeKind::Circle,
                        Tool::Arrow => ShapeKind::Arrow,
                        Tool::Star => ShapeKind::Star,
                        Tool::Heart => ShapeKind::Heart,
                        Tool::Triangle => ShapeKind::Triangle,
                        Tool::Diamond => ShapeKind::Diamond,
                        Tool::RoundedRect => ShapeKind::RoundedRect,
                        Tool::UmlClass => ShapeKind::UmlClass,
                        Tool::Actor => ShapeKind::Actor,
                        Tool::Database => ShapeKind::Database,
                        Tool::Cloud => ShapeKind::Cloud,
                        _ => ShapeKind::Line,
                    };
                    d.preview_shape = Some(Shape {
                        kind, x1: start_x, y1: start_y, x2: ex, y2: ey, color, width,
                    });
                }
            }
            da.queue_draw();
        }
    });

    drag.connect_drag_end({
        let (as_, ds, da) = (app_state.clone(), draw_state.clone(), da.clone());
        move |_, _, _| {
            let mut d = ds.borrow_mut();
            d.drawing = false;
            let stroke = d.current_stroke.take();
            let spray = d.current_spray.take();
            let shape  = d.preview_shape.take();

            let mut app = as_.borrow_mut();
            let tool = app.current_tool.clone();
            let note = app.current_note_mut();
            
            if tool == Tool::Select {
                if d.is_moving_selection {
                    // Finalize move
                    if let Some(rect) = d.preview_selection.take() {
                        d.selection_rect = Some(rect);
                    }
                } else if let Some((rx, ry, rw, rh)) = d.preview_selection.take() {
                    // Finalize selection box - compute intersections
                    d.selected_strokes.clear();
                    d.selected_shapes.clear();
                    d.selected_images.clear();
                    d.selected_tables.clear();
                    d.selected_texts.clear();
                    
                    let mut min_x = f64::MAX; let mut min_y = f64::MAX;
                    let mut max_x = f64::MIN; let mut max_y = f64::MIN;
                    let mut found_any = false;
                    
                    let mut add_bounds = |bx1: f64, by1: f64, bx2: f64, by2: f64| {
                        min_x = min_x.min(bx1); min_y = min_y.min(by1);
                        max_x = max_x.max(bx2); max_y = max_y.max(by2);
                        found_any = true;
                    };

                    for (i, s) in note.strokes.iter().enumerate() {
                        let mut intersects = false;
                        let mut b_min_x = f64::MAX; let mut b_min_y = f64::MAX;
                        let mut b_max_x = f64::MIN; let mut b_max_y = f64::MIN;
                        for p in &s.points {
                            b_min_x = b_min_x.min(p.x); b_min_y = b_min_y.min(p.y);
                            b_max_x = b_max_x.max(p.x); b_max_y = b_max_y.max(p.y);
                            if p.x >= rx && p.x <= rx+rw && p.y >= ry && p.y <= ry+rh { intersects = true; }
                        }
                        if intersects { d.selected_strokes.insert(i); add_bounds(b_min_x, b_min_y, b_max_x, b_max_y); }
                    }
                    for (i, sh) in note.shapes.iter().enumerate() {
                        let sh_min_x = sh.x1.min(sh.x2); let sh_min_y = sh.y1.min(sh.y2);
                        let sh_max_x = sh.x1.max(sh.x2); let sh_max_y = sh.y1.max(sh.y2);
                        if !(sh_max_x < rx || sh_min_x > rx+rw || sh_max_y < ry || sh_min_y > ry+rh) {
                            d.selected_shapes.insert(i);
                            add_bounds(sh_min_x, sh_min_y, sh_max_x, sh_max_y);
                        }
                    }
                    for (i, img) in note.images.iter().enumerate() {
                        let i_max_x = img.x + img.width; let i_max_y = img.y + img.height;
                        if !(i_max_x < rx || img.x > rx+rw || i_max_y < ry || img.y > ry+rh) {
                            d.selected_images.insert(i);
                            add_bounds(img.x, img.y, i_max_x, i_max_y);
                        }
                    }
                    for (i, tbl) in note.tables.iter().enumerate() {
                        let t_max_x = tbl.x + (tbl.cols as f64 * tbl.cell_w); let t_max_y = tbl.y + (tbl.rows as f64 * tbl.cell_h);
                        if !(t_max_x < rx || tbl.x > rx+rw || t_max_y < ry || tbl.y > ry+rh) {
                            d.selected_tables.insert(i);
                            add_bounds(tbl.x, tbl.y, t_max_x, t_max_y);
                        }
                    }
                    for (i, txt) in note.texts.iter().enumerate() {
                        let mut w = 100.0; // Approximation if Cairo extents aren't immediately available
                        let h = txt.font_size;
                        if let Ok(surf) = cairo::ImageSurface::create(cairo::Format::ARgb32, 1, 1) {
                            if let Ok(cr) = cairo::Context::new(&surf) {
                                cr.select_font_face(&txt.font_family, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
                                cr.set_font_size(txt.font_size);
                                if let Ok(extents) = cr.text_extents(&txt.text) {
                                    w = extents.width();
                                }
                            }
                        }
                        let t_max_x = txt.x + w; let t_max_y = txt.y + h;
                        if !(t_max_x < rx || txt.x > rx+rw || t_max_y < ry || txt.y > ry+rh) {
                            d.selected_texts.insert(i);
                            add_bounds(txt.x, txt.y, t_max_x, t_max_y);
                        }
                    }

                    if found_any {
                        let pad = 6.0;
                        d.selection_rect = Some((min_x - pad, min_y - pad, max_x - min_x + pad*2.0, max_y - min_y + pad*2.0));
                    } else {
                        d.selection_rect = None;
                    }
                }
            } else {
                d.selection_rect = None;
                d.selected_strokes.clear();
                d.selected_shapes.clear();
                d.selected_images.clear();
                d.selected_tables.clear();
                d.selected_texts.clear();
            }

            if tool != Tool::Select {
                note.push_undo();
                if let Some(s) = stroke { if !s.points.is_empty() { note.strokes.push(s); } }
                if let Some(sp) = spray { if !sp.points.is_empty() { note.sprays.push(sp); } }
                if let Some(sh) = shape { note.shapes.push(sh); }
            }
            drop(app);
            drop(d);
            da.queue_draw();
        }
    });

    // Click for Text Tool
    let click = gtk::GestureClick::new();
    click.set_button(0); // Any button
    click.connect_pressed({
        let (as_, da_) = (app_state.clone(), da.clone());
        move |gesture, _, x, y| {
            let app = as_.borrow();
            let z = app.zoom_level;
            let (wx, wy) = (x / z, y / z);
            let tool = app.current_tool.clone();
            drop(app);
            
            if tool == Tool::Fill {
                let mut app = as_.borrow_mut();
                let color = app.current_color.clone();
                let w = app.canvas_width as i32;
                let h = app.canvas_height as i32;
                let sx = wx as i32;
                let sy = wy as i32;
                
                if sx >= 0 && sx < w && sy >= 0 && sy < h {
                    let note = app.current_note().clone();
                    if let Ok(mut surface) = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h) {
                        if let Ok(cr) = cairo::Context::new(&surface) {
                            // Render everything to temp surface
                            let bg = note.bg_color.clone().unwrap_or(state::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 });
                            cr.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
                            cr.paint().unwrap();
                            for img in &note.images { draw_canvas_image(&cr, img); }
                            for txt in &note.texts { draw_canvas_text(&cr, txt); }
                            for s in &note.strokes { draw_stroke(&cr, s); }
                            for sp in &note.sprays { draw_spray(&cr, sp); }
                            for sh in &note.shapes { draw_shape(&cr, sh); }
                            drop(cr);
                            
                            if let Ok(mut data) = surface.data() {
                                let stride = w as usize * 4;
                                let start_idx = sy as usize * stride + sx as usize * 4;
                                let tb = data[start_idx + 0];
                                let tg = data[start_idx + 1];
                                let tr = data[start_idx + 2];
                                let ta = data[start_idx + 3];
                                
                                let fb = (color.b * 255.0) as u8;
                                let fg = (color.g * 255.0) as u8;
                                let fr = (color.r * 255.0) as u8;
                                let fa = (color.a * 255.0) as u8;
                                
                                if !(tr == fr && tg == fg && tb == fb && ta == fa) {
                                    let mut visited = vec![false; (w * h) as usize];
                                    let mut queue = std::collections::VecDeque::new();
                                    queue.push_back((sx, sy));
                                    visited[(sy * w + sx) as usize] = true;
                                    
                                    let mut fill_data = vec![0u8; (w * h * 4) as usize];
                                    let tolerance = 5i32;
                                    let mut min_x = sx; let mut max_x = sx;
                                    let mut min_y = sy; let mut max_y = sy;
                                    
                                    while let Some((cx, cy)) = queue.pop_front() {
                                        let idx = (cy as usize) * stride + (cx as usize) * 4;
                                        let b = data[idx+0] as i32; let g = data[idx+1] as i32; let r = data[idx+2] as i32; let a = data[idx+3] as i32;
                                        if (b - tb as i32).abs() <= tolerance && (g - tg as i32).abs() <= tolerance && (r - tr as i32).abs() <= tolerance && (a - ta as i32).abs() <= tolerance {
                                            fill_data[idx+0] = fb; fill_data[idx+1] = fg; fill_data[idx+2] = fr; fill_data[idx+3] = fa;
                                            min_x = min_x.min(cx); max_x = max_x.max(cx); min_y = min_y.min(cy); max_y = max_y.max(cy);
                                            
                                            if cx > 0 && !visited[(cy * w + cx - 1) as usize] { visited[(cy * w + cx - 1) as usize] = true; queue.push_back((cx-1, cy)); }
                                            if cx < w-1 && !visited[(cy * w + cx + 1) as usize] { visited[(cy * w + cx + 1) as usize] = true; queue.push_back((cx+1, cy)); }
                                            if cy > 0 && !visited[((cy - 1) * w + cx) as usize] { visited[((cy - 1) * w + cx) as usize] = true; queue.push_back((cx, cy-1)); }
                                            if cy < h-1 && !visited[((cy + 1) * w + cx) as usize] { visited[((cy + 1) * w + cx) as usize] = true; queue.push_back((cx, cy+1)); }
                                        }
                                    }
                                    
                                    if max_x >= min_x && max_y >= min_y {
                                        let crop_w = (max_x - min_x + 1) as usize;
                                        let crop_h = (max_y - min_y + 1) as usize;
                                        if let Ok(mut fill_surf) = cairo::ImageSurface::create(cairo::Format::ARgb32, crop_w as i32, crop_h as i32) {
                                            if let Ok(mut fd) = fill_surf.data() {
                                                for cy in 0..crop_h {
                                                    let src_y = min_y as usize + cy;
                                                    let src_start = src_y * stride + min_x as usize * 4;
                                                    let dst_start = cy * crop_w * 4;
                                                    fd[dst_start .. dst_start + crop_w * 4].copy_from_slice(&fill_data[src_start .. src_start + crop_w * 4]);
                                                }
                                            }
                                            let mut png_bytes = Vec::new();
                                            if fill_surf.write_to_png(&mut png_bytes).is_ok() {
                                                let mut note_mut = app.current_note_mut();
                                                note_mut.push_undo();
                                                note_mut.images.push(state::CanvasImage {
                                                    x: min_x as f64, y: min_y as f64, width: crop_w as f64, height: crop_h as f64, png_data: png_bytes,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                da_.queue_draw();
                return;
            } else if tool == Tool::Text {
                // Spawn a Popover with a text entry
                let pop = gtk::Popover::new();
                pop.set_parent(&da_);
                
                // Point it exactly to where the user clicked (raw coordinates)
                let rect = gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1);
                pop.set_pointing_to(Some(&rect));
                
                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 6);
                vbox.set_margin_top(8); vbox.set_margin_bottom(8);
                vbox.set_margin_start(8); vbox.set_margin_end(8);
                
                let entry = gtk::Entry::builder().placeholder_text("Type text here...").build();
                vbox.append(&entry);
                
                let font_cb = gtk::ComboBoxText::new();
                font_cb.append_text("sans-serif");
                font_cb.append_text("serif");
                font_cb.append_text("monospace");
                font_cb.set_active(Some(0));
                vbox.append(&font_cb);
                
                let size_spin = gtk::SpinButton::with_range(8.0, 72.0, 1.0);
                size_spin.set_value(24.0);
                vbox.append(&size_spin);
                
                let insert_btn = gtk::Button::with_label("Insert Text");
                vbox.append(&insert_btn);
                
                pop.set_child(Some(&vbox));
                pop.popup();
                
                // Handle insertion
                let as2 = as_.clone();
                let da2 = da_.clone();
                let pop_clone = pop.clone();
                insert_btn.connect_clicked(move |_| {
                    let text = entry.text().to_string();
                    if !text.is_empty() {
                        let font_family = font_cb.active_text().unwrap_or("sans-serif".into()).to_string();
                        let font_size = size_spin.value();
                        let mut app = as2.borrow_mut();
                        let color = app.current_color.clone();
                        let note = app.current_note_mut();
                        note.push_undo();
                        note.texts.push(state::CanvasText {
                            text, x: wx, y: wy, font_family, font_size, color
                        });
                        
                        // Check bounds and expand if needed
                        let mut expanded = false;
                        if wx > app.canvas_width * 0.8 { app.canvas_width *= 1.2; expanded = true; }
                        if wy > app.canvas_height * 0.8 { app.canvas_height *= 1.2; expanded = true; }
                        if expanded {
                            da2.set_size_request((app.canvas_width * app.zoom_level) as i32, (app.canvas_height * app.zoom_level) as i32);
                        }
                        
                        da2.queue_draw();
                    }
                    pop_clone.popdown();
                });
                
                // Free the popover when closed to prevent memory leaks
                pop.connect_closed(move |p| {
                    p.unparent();
                });
            } else {
                gesture.set_state(gtk::EventSequenceState::Denied);
            }
        }
    });

    da.add_controller(drag);
    da.add_controller(click);
}

// ── Save / Open ───────────────────────────────────────────────────────────────

fn do_save_session(app_state: &SharedApp, path: &str) {
    let app = app_state.borrow();
    let session = &app.sessions[app.current_session_idx];
    if let Ok(json) = serde_json::to_string_pretty(session) {
        std::fs::write(path, json).ok();
    }
}

fn save_as_dialog(app_state: &SharedApp, window: &ApplicationWindow) {
    let dialog = gtk::FileChooserDialog::new(
        Some("Save Session"),
        Some(window),
        gtk::FileChooserAction::Save,
        &[("Save", gtk::ResponseType::Accept), ("Cancel", gtk::ResponseType::Cancel)],
    );
    {
        let app = app_state.borrow();
        let name = format!("{}.json", app.sessions[app.current_session_idx].name);
        dialog.set_current_name(&name);
    }
    let as_ = app_state.clone();
    let win = window.clone();
    dialog.connect_response(move |d, resp| {
        if resp == gtk::ResponseType::Accept {
            if let Some(path) = d.file().and_then(|f| f.path()) {
                let path_str = path.to_string_lossy().to_string();
                do_save_session(&as_, &path_str);
                let mut app = as_.borrow_mut();
                let si = app.current_session_idx;
                app.sessions[si].save_path = Some(path_str);
                drop(app);
                update_window_title(&win, &as_);
            }
        }
        d.destroy();
    });
    dialog.show();
}

fn open_dialog(
    app_state: &SharedApp,
    sidebar: &GtkBox,
    da: &DrawingArea,
    window: &ApplicationWindow,
) {
    let dialog = gtk::FileChooserDialog::new(
        Some("Open Session"),
        Some(window),
        gtk::FileChooserAction::Open,
        &[("Open", gtk::ResponseType::Accept), ("Cancel", gtk::ResponseType::Cancel)],
    );
    let (as_, sl, da, win) = (app_state.clone(), sidebar.clone(), da.clone(), window.clone());
    dialog.connect_response(move |d, resp| {
        if resp == gtk::ResponseType::Accept {
            if let Some(path) = d.file().and_then(|f| f.path()) {
                let path_str = path.to_string_lossy().to_string();
                if let Ok(data) = std::fs::read_to_string(&path) {
                    if let Ok(mut session) = serde_json::from_str::<state::Session>(&data) {
                        session.save_path = Some(path_str);
                        let idx = {
                            let mut app = as_.borrow_mut();
                            app.sessions.push(session);
                            app.sessions.len() - 1
                        };
                        as_.borrow_mut().current_session_idx = idx;
                        rebuild_sidebar(&sl, &as_, &da, &win);
                        da.queue_draw();
                    }
                }
            }
        }
        d.destroy();
    });
    dialog.show();
}

// ── Export ────────────────────────────────────────────────────────────────────

fn render_note(note: &state::Note, cr: &cairo::Context) {
    cr.set_antialias(cairo::Antialias::Best);
    
    let bg = note.bg_color.clone().unwrap_or(state::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 });
    cr.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
    cr.paint().unwrap();
    
    for img in &note.images { draw_canvas_image(cr, img); }
    for txt in &note.texts { draw_canvas_text(cr, txt); }
    for s in &note.strokes { draw_stroke(cr, s); }
    for sp in &note.sprays { draw_spray(cr, sp); }
    for sh in &note.shapes { draw_shape(cr, sh); }
}

fn export_note_dialog(app_state: &SharedApp, window: &ApplicationWindow, format: &'static str) {
    let title = match format {
        "png" => "Export Note as PNG",
        "jpeg" => "Export Note as JPEG",
        "pdf" => "Export Note as PDF",
        _ => "Export Note",
    };
    let dialog = gtk::FileChooserDialog::new(
        Some(title),
        Some(window),
        gtk::FileChooserAction::Save,
        &[("Export", gtk::ResponseType::Accept), ("Cancel", gtk::ResponseType::Cancel)],
    );
    
    let (app_width, app_height, note_name) = {
        let app = app_state.borrow();
        (app.canvas_width, app.canvas_height, app.current_note().name.clone())
    };
    
    let name = format!("{}.{}", note_name, format);
    dialog.set_current_name(&name);
    
    let as_ = app_state.clone();
    dialog.connect_response(move |d, resp| {
        if resp == gtk::ResponseType::Accept {
            if let Some(path) = d.file().and_then(|f| f.path()) {
                let note = as_.borrow().current_note().clone();
                match format {
                    "png" => {
                        if let Ok(surface) = cairo::ImageSurface::create(cairo::Format::ARgb32, app_width as i32, app_height as i32) {
                            if let Ok(cr) = cairo::Context::new(&surface) {
                                render_note(&note, &cr);
                                if let Ok(mut file) = std::fs::File::create(&path) {
                                    surface.write_to_png(&mut file).ok();
                                }
                            }
                        }
                    }
                    "jpeg" => {
                        if let Ok(surface) = cairo::ImageSurface::create(cairo::Format::ARgb32, app_width as i32, app_height as i32) {
                            if let Ok(cr) = cairo::Context::new(&surface) {
                                render_note(&note, &cr);
                                let mut buf = Vec::new();
                                if surface.write_to_png(&mut buf).is_ok() {
                                    if let Ok(img) = image::load_from_memory(&buf) {
                                        img.into_rgb8().save_with_format(&path, image::ImageFormat::Jpeg).ok();
                                    }
                                }
                            }
                        }
                    }
                    "pdf" => {
                        let path_str = path.to_string_lossy().into_owned();
                        if let Ok(surface) = cairo::PdfSurface::new(app_width, app_height, path_str) {
                            if let Ok(cr) = cairo::Context::new(&surface) {
                                render_note(&note, &cr);
                                cr.show_page();
                                surface.finish();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        d.destroy();
    });
    dialog.show();
}

fn export_session_dialog(app_state: &SharedApp, window: &ApplicationWindow) {
    let dialog = gtk::FileChooserDialog::new(
        Some("Export Session as PDF"),
        Some(window),
        gtk::FileChooserAction::Save,
        &[("Export", gtk::ResponseType::Accept), ("Cancel", gtk::ResponseType::Cancel)],
    );
    
    let (app_width, app_height, sess_name) = {
        let app = app_state.borrow();
        (app.canvas_width, app.canvas_height, app.sessions[app.current_session_idx].name.clone())
    };
    
    let name = format!("{}.pdf", sess_name);
    dialog.set_current_name(&name);
    
    let as_ = app_state.clone();
    dialog.connect_response(move |d, resp| {
        if resp == gtk::ResponseType::Accept {
            if let Some(path) = d.file().and_then(|f| f.path()) {
                let path_str = path.to_string_lossy().into_owned();
                if let Ok(surface) = cairo::PdfSurface::new(app_width, app_height, path_str) {
                    let app = as_.borrow();
                    let session = &app.sessions[app.current_session_idx];
                    for note in &session.notes {
                        if let Ok(cr) = cairo::Context::new(&surface) {
                            render_note(note, &cr);
                            cr.show_page();
                        }
                    }
                    surface.finish();
                }
            }
        }
        d.destroy();
    });
    dialog.show();
}
