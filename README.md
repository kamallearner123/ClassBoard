<div align="center">
  <!-- Ensure you place the logo image at assets/logo.png -->
  <img src="assets/logo.png" alt="RoughNote Logo" width="150"/>
  <h1>RoughNote</h1>
  <p>A native Linux drawing and comprehensive note-taking application built with Rust and GTK4.</p>
</div>

## Overview
RoughNote (formerly SimplePaint) is a native Linux application designed for drawing, handwriting, and comprehensive note-taking. It aims to provide a robust feature set tailored specifically for the Linux ecosystem, serving as a powerful open-source tool for digital note-taking and illustration.

## Features (Planned & Implemented)
- **Inking and Drawing (Core Canvas):** Pen, Pencil, Highlighter, and Marker tools. Pressure sensitivity support for drawing tablets.
- **Erasers & Selection:** Stroke and Standard Erasers. Lasso selection to move, resize, and rotate.
- **Shapes:** Insert standard geometric shapes with planned support for auto-recognizing hand-drawn shapes.
- **Note-taking & Text:** Infinite canvas, rich text formatting, lists, and LaTeX/MathML equation support.
- **Organization:** Notebooks, Sections, Pages with customizable page backgrounds (Grid, Ruled, Blank).
- **Multimedia:** Insert images, PDFs, attachments, audio recordings, and tables.
- **Linux Integration:** Custom lossless save formats, standard exports (PDF, PNG, SVG), automatic Light/Dark mode syncing, and full Wayland/X11 support.

## Tech Stack
- **Language:** Rust (Edition 2024)
- **GUI Framework:** GTK4 
- **Graphics/Rendering:** Cairo for high-performance canvas rendering

## Getting Started

### Prerequisites
Make sure you have Rust and Cargo installed, as well as the GTK4 development libraries for your system.

**On Ubuntu/Debian:**
```bash
sudo apt install libgtk-4-dev
```

**On Fedora:**
```bash
sudo dnf install gtk4-devel
```

### Build & Run
To compile and run the application locally:
```bash
cargo run --release
```

### Cross-Compilation
The project includes a `build.sh` script to build binaries for both Linux and Windows (x86_64 and x86_32).
```bash
./build.sh
```
*Note: Cross-compilation requires the appropriate C toolchains (like `mingw-w64` for Windows) and GTK4 development headers for the target architecture.*

## License
[Add your license here]
