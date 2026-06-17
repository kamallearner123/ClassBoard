# High-Level Design (HLD)

## Overview
`roughnote` (formerly SimplePaint) is a native, cross-platform vector drawing and note-taking application built using **Rust** and **GTK4**. It utilizes **Cairo** as its core rendering engine for high-performance 2D graphics and provides rich export capabilities (PNG, JPEG, PDF).

## System Architecture

The application strictly follows a Model-View-Controller (MVC) and unidirectional data-flow paradigm to ensure separation of concerns.

```mermaid
flowchart TD
    subgraph UI ["User Interface (GTK4 View)"]
        MainWindow[Application Window]
        Toolbar[Toolbar & Tools Menu]
        Sidebar[Sidebar (Sessions/Notes)]
        DrawingArea[Canvas DrawingArea]
    end

    subgraph Controllers ["Event Controllers"]
        DragHandler[Gesture Drag Controller]
        ClickHandler[Gesture Click Controller]
        KeyHandler[Keyboard Event Controller]
    end

    subgraph CoreState ["Application State (Model)"]
        AppState[Shared AppState]
        Session[Sessions & Notes]
        History[Undo/Redo Stack]
        DrawState[Active Draw State]
    end

    subgraph RenderEngine ["Rendering Engine (Cairo)"]
        CanvasRenderer[Canvas Painter]
        ExportEngine[PDF / PNG / JPEG Exporters]
        ShapeLogic[Chaikin Smoothing & SVG Paths]
    end

    %% Interactions
    MainWindow --> Toolbar
    MainWindow --> Sidebar
    MainWindow --> DrawingArea

    DrawingArea <--> Controllers

    Controllers -- Mutates --> CoreState
    Toolbar -- Modifies Tool/Color/Zoom --> CoreState
    Sidebar -- Switches Notes/Sessions --> CoreState

    CoreState -- Read by --> CanvasRenderer
    CanvasRenderer -- Renders to --> DrawingArea

    Toolbar -- Triggers Export --> ExportEngine
    CoreState -- Read by --> ExportEngine
    ShapeLogic -- Used by --> CanvasRenderer
    ShapeLogic -- Used by --> ExportEngine
```

## Component Breakdown

### 1. User Interface (GTK4 View)
- **Application Window:** The root container utilizing GTK4's `HeaderBar`, `Paned` layouts, and modern styled CSS.
- **Toolbar:** Provides instant access to tools (Pen, Eraser, Spray, Fill, Shapes), actions (Undo, Color, Width), and the **Export Menu**.
- **Sidebar:** Handles document management. Users can create, rename, and manage multiple "Sessions" containing hierarchical "Notes".
- **DrawingArea:** The interactive canvas. Instead of using GTK's NGL/Vulkan hardware rendering directly (which can introduce latency for full-screen scribbling), it forces the `cairo` backend for immediate, low-latency paint operations.

### 2. Event Controllers (Input Handling)
GTK4 delegates user interactions to abstract gesture controllers rather than direct signal handling:
- **GestureDrag:** Captures stroke paths, shape boundary boxes, and selection movements. 
- **GestureClick:** Captures precise coordinate selections (used heavily for the Text tool insertion and the new Raster Flood-Fill tool).
- **Keyboard Events:** Listens globally for `Ctrl+Z` (Undo) and `Delete/Backspace` (Delete selected objects).

### 3. Application State (Model)
Located primarily in `src/state.rs`. The application state is wrapped in an `Rc<RefCell<...>>` pattern (`SharedApp` and `SharedDraw`) to allow mutable access across GTK event closures.
- **AppState:** The ultimate source of truth. Contains all sessions, current tool settings, and zoom levels.
- **Note:** Contains vectors of discrete items: `strokes`, `shapes`, `sprays`, `images`, and `texts`. 
- **DrawState:** An ephemeral state struct tracking active user actions (e.g., previewing a shape before the mouse is released, tracking selected items, or temporary coordinates).

### 4. Rendering Engine (Cairo)
Located primarily in `src/canvas.rs`. 
- **Canvas Painter:** Executes upon `da.queue_draw()`. It clears the screen, applies the zoom scale, and iterates over the data model (`strokes`, `shapes`, etc.) painting them in order.
- **Shape Logic:** Implements mathematical drawing optimizations. It uses Chaikin's corner-cutting algorithm for hyper-smooth pen strokes, and SVG-like bezier curves mapped to bounding boxes for scalable shapes like Hearts and Clouds.
- **Export Engine:** Reuses the exact same canvas painting routines but redirects the `cairo::Context` to a `cairo::PdfSurface` or `cairo::ImageSurface`. It also uses a BFS Flood-Fill algorithm to perform localized raster manipulations over the vector elements.
