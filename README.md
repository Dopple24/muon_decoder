# Particle Matrix Viewer

A Rust application to visualize and analyze particle tracks on a 256×256 grid.  
It extracts connected particles from a grid of energy values, classifies them, and displays them interactively with a GUI.
On file open a dialog popup appears, please enter pixel depth, pixel width and the orientation - orientation is the way the detector was installed - the pixel detector must be vertically placed and the longer side should point either to north (south) or to west (east). It should look like a needle of a compass when pointing north.
It expects a .txt file with float values with spaces in between, each row of values is on its separate row in the file.

---

## Features

- Load a 256×256 grid from a file.
- Detect particles and classify them as **ALPHA**, **BETA**, **GAMMA**, **MUON**, **SUS MUON** or **UNKNOWN**.
- Interactive GUI to view:
  - Single particle tracks
  - Combined tracks
- Interactive 3D viewer to visualize particle paths (courtesy of [@Jenyyk](https://github.com/jenyyk))
- Particle statistics and filtering.
- Smooth rendering with scaling support.

---

## Calculated values

**North - South angle**: The angle from which the particle came (accurate only for muons and sus muons), if oriented North - South, than has a value 0° to 180°, if it has a West - East orientation, it is impossible to distinguish if the particle came from left hand side or the right hand site and hence it only shows from which angle it came.  
**Abs angle**: The primary angle (if north south orientation, than the North - South angle...) is changed to only have values 0° to 90° (e.g. 163° > 17°)  
**West - East angle**: The angle from which the particle came (accurate only for muons and sus muons), if oriented West - East, than has a value 0° to 180°, if it has a North - South orientation, it is impossible to distinguish if the particle came from left hand side or the right hand site and hence it only shows from which angle it came.  
**total energy**: sum of all energies gathered by the pixels  
**size**: number of pixels hit  
**LET**: total energy / the length of the particle NOT the same as size  

---

## Installation

Make sure you have Rust installed (https://rust-lang.org). Then:

```bash
git clone https://github.com/Dopple24/muon_decoder
cd muon_decoder
cargo build --release
