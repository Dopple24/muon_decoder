# Particle Matrix Viewer

A Rust application to visualize and analyze particle tracks on a 256×256 grid.  
It extracts connected particles from a grid of energy values, classifies them, and displays them interactively with a GUI.
It expects a .txt file with float values with spaces in between, each row of values is on its seperate row in the file.

---

## Features

- Load a 256×256 grid from a file.
- Detect particles and classify them as **ALPHA**, **BETA**, **GAMMA**, **MUON**, or **UNKNOWN**.
- Interactive GUI to view:
  - Single particle tracks
  - Combined tracks
- Particle statistics and filtering.
- Smooth rendering with scaling support.

---

## Installation

Make sure you have Rust installed (https://rust-lang.org). Then:

```bash
git clone https://github.com/Dopple24/particle-matrix-viewer.git
cd particle-matrix-viewer
cargo build --release
