# roughnote - Linux Drawing & Note-taking Application Specification

## Overview
roughnote is a native Linux application built with Rust designed for drawing, handwriting, and comprehensive note-taking. It aims to provide a robust feature set comparable to Microsoft OneNote, tailored specifically for the Linux ecosystem.

## Core Technology Stack
- **Language:** Rust
- **GUI Framework:** GTK4 / Relm4 or Iced (for native Linux integration and hardware acceleration)
- **Graphics/Rendering:** Cairo / Skia for high-performance canvas rendering
- **Platform Compatibility:** Full support for Wayland and X11

## Feature Requirements

### 1. Inking and Drawing (Core Canvas)
- **Pen Tools:** Pen, Pencil, Highlighter, and Marker with customizable thickness.
- **Pressure Sensitivity:** Full support for drawing tablets (e.g., Wacom, Huion) to adjust stroke width and opacity based on stylus pressure.
- **Eraser Types:**
  - Stroke Eraser (removes the entire continuous stroke).
  - Point/Standard Eraser (erases specific pixels with a customizable radius).
- **Lasso Selection:** Select, move, resize, and rotate hand-drawn strokes or objects.
- **Shape Tools:** Insert perfect shapes (Lines, Arrows, Rectangles, Ovals, Polygons).
- **Shape Recognition:** Option to automatically convert rough, hand-drawn shapes into perfect geometric equivalents ("Ink to Shape").
- **Color Palette:** Quick access to standard colors and a custom color picker (RGB/Hex/HSV).

### 2. Note-taking and Text Editing
- **Infinite Canvas:** Pages that expand dynamically in all directions as you write or draw.
- **Text Boxes:** Click anywhere on the canvas to start typing.
- **Rich Text Formatting:** Support for font styles, sizes, bold, italics, underlines, text colors, and highlighting.
- **Lists:** Bulleted, numbered, and checklist creation.
- **Math Equations:** "Ink to Math" support (recognizing handwritten math) and typing mathematical equations using LaTeX/MathML syntax.

### 3. Organization and Structure
- **Hierarchical Layout:** Organize content into Notebooks > Sections/Tabs > Pages > Subpages.
- **Page Backgrounds:** Customizable canvas backgrounds (Blank, Ruled lines, Grid lines, custom colors).
- **Tags:** Apply tags (e.g., To-Do, Important, Question) to text or drawn elements.
- **Search:** Global search across all notebooks for text, tags, and ideally recognized handwriting (OCR).

### 4. Multimedia and Insertions
- **Images & Files:** Insert pictures, PDFs, and generic file attachments directly onto the canvas.
- **Audio Recording:** Record audio directly within a page, with playback synced to the notes taken during the recording.
- **Tables:** Insert and format basic data tables.
- **Link Insertion:** Hyperlinks to external websites or other pages within the application.

### 5. Linux Specifics & System Integration
- **File Formats & Saving:**
  - Custom open XML/JSON based format for lossless project saves.
  - Export functionality to standard formats: PDF, SVG, PNG, HTML, and Markdown.
  - Auto-save functionality.
- **System Theme Integration:** Automatic switching between Light and Dark mode based on the desktop environment.
- **Keyboard Shortcuts:** Comprehensive and customizable keyboard shortcuts for power users.

### 6. Advanced/Future Considerations
- **Cloud Synchronization:** Sync notes across devices via Nextcloud, WebDAV, or Google Drive.
- **Collaboration:** Real-time multi-user editing over a local network or server.
- **Plugin System:** Allow community-developed Rust or WebAssembly plugins to extend functionality.
