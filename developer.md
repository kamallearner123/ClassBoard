# RoughNote Developer Guide

This document outlines the high-level architecture and implementation details for the RoughNote application.

## 1. Core Architecture
RoughNote is built using **Rust** and **GTK4**, with **Cairo** serving as the primary graphics rendering engine.

### Why Cairo?
By default, GTK4 attempts to use hardware acceleration (NGL or Vulkan) for its rendering pipeline. However, for a full-screen `DrawingArea` that requires continuous, high-frequency updates (like drawing smooth ink strokes), uploading large textures to the GPU every frame introduces massive latency. Therefore, the application explicitly sets `GSK_RENDERER=cairo` to utilize fast, CPU-based native Cairo rendering, which drastically improves drawing latency and responsiveness.

## 2. State Management

The application state is split into two primary shared structs, wrapped in `Rc<RefCell<T>>` to allow safe, shared mutability across GTK event callbacks.

### `AppState`
`AppState` manages the persistent and global state of the application.
* **Sessions & Notes**: The app is structured hierarchically into `Session`s, which contain multiple `Note`s. 
* **Tool & Color**: Tracks the currently selected drawing tool (`current_tool`) and color (`current_color`).
* **Note Primitive Vectors**: A `Note` struct (`src/state.rs`) contains vectors of drawing primitives:
  * `strokes: Vec<Stroke>`
  * `shapes: Vec<Shape>`
  * `images: Vec<CanvasImage>`
  * `tables: Vec<CanvasTable>`
  * `texts: Vec<CanvasText>`
  * `sprays: Vec<Spray>`
* **Undo/Redo**: The `Note` struct maintains an `undo_stack` and `redo_stack`. Whenever a significant action is completed (e.g., finishing a stroke), `note.push_undo()` takes a snapshot of the current state arrays.

### `DrawState`
`DrawState` manages transient, high-frequency state during active interactions.
* **Active Drawing**: Tracks the stroke or shape currently being drawn (`current_stroke`, `preview_shape`) before it is finalized and pushed to the `Note`.
* **Lasso Selection**: Tracks the current selection bounding box (`selection_rect`) and the indices of all selected primitives (`selected_strokes`, `selected_shapes`, etc.).
* **Cursor Tracking**: Tracks the real-time mouse position (`cursor_pos`) for rendering dynamic previews, such as the Eraser or Spray tool radius indicator.

## 3. Rendering Pipeline (`src/canvas.rs`)

All rendering goes through the GTK `DrawingArea::set_draw_func`. 
The rendering sequence is carefully ordered:
1. **Background**: Paint the solid background color.
2. **Ruled Lines/Grids**: Draw horizontal/vertical rules if configured (`rule_gap`).
3. **Solid Primitives**: Draw all saved images, texts, normal strokes, sprays, and shapes in sequence.
4. **Eraser Strokes**: The Cairo operator is temporarily switched to `cairo::Operator::DestOut`. Eraser strokes are drawn using this operator, which non-destructively masks out the underlying ink. The operator is then reset to `Over`.
5. **Transient State**: Draw the active stroke or shape preview from `DrawState`.
6. **Selection UI**: Draw the bounding box of the Lasso selection tool.
7. **Cursor Preview**: Draw the tool radius preview (if applicable).

### Chaikin Smoothing
To ensure hand-drawn strokes look completely smooth without introducing latency, `draw_stroke` utilizes the **Chaikin curve-smoothing algorithm** (`chaikin(pts, iters)`). It iteratively cuts the corners of the raw input points. It is guaranteed never to overshoot, unlike Catmull-Rom splines, making it perfect for rapid, dense input points from a mouse or tablet.

## 4. Input Handling
Input is captured using GTK4 event controllers attached to the `DrawingArea`:
* **`GestureDrag`**: Handles the start, update, and end of drawing actions.
  * *Drag Start*: Initializes a transient primitive in `DrawState`.
  * *Drag Update*: Appends points to the active stroke or updates the preview shape coordinates. Queues a redraw.
  * *Drag End*: Finalizes the primitive, pushes an undo state, moves the primitive into `AppState::Note`, and clears `DrawState`.
* **`EventControllerMotion`**: Tracks the mouse position strictly for rendering tool previews (like the eraser circle).

## 5. Exporting

Exporting to PNG or PDF leverages the exact same rendering logic.
1. A temporary `cairo::ImageSurface` or `cairo::PdfSurface` is created based on the calculated canvas bounds.
2. A new Cairo `Context` is bound to the surface.
3. The `render_note` helper function (which skips drawing selection boxes and cursors) is called to paint the primitives onto the surface.
4. The surface is saved to the disk.

## 6. Build and Distribution
* The project uses standard `cargo build`.
* A helper script (`build_deb.sh`) is provided to bundle the compiled binary, a `.desktop` file, and an application icon into a native Debian (`.deb`) package for seamless installation on Ubuntu/Debian-based systems.
