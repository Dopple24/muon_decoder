# Particle Matrix Viewer

A **Rust application for visualization and analysis of particle tracks**
detected on a pixel grid.

The program processes energy values recorded by a pixel detector,
reconstructs **particle tracks**, classifies them into particle types,
and displays them through an interactive GUI.

It supports both **2D and 3D visualization**, particle filtering, and
statistical analysis of detected events.

------------------------------------------------------------------------

# Overview

Particle Matrix Viewer reads detector data stored as text files and
identifies connected clusters of activated pixels corresponding to
particle interactions. The software reconstructs these clusters into
tracks and classifies them based on their shape and size.

The tool is primarily designed for **cosmic ray and radiation detector
experiments using pixel-based sensors**.

------------------------------------------------------------------------

# Features

-   Load detector frames from `.txt` files
-   Optional timestamp support via `.dsc` files
-   Automatic **particle detection and classification**
-   Interactive **GUI viewer**
-   Multiple viewing modes:
    -   **Single track view**
    -   **Combined frame view**
    -   **Compound dataset view**
-   **Interactive 3D particle track visualization**
-   Particle statistics and filtering
-   Scalable rendering
-   Multi-language support

Supported particle classes:

-   **ALPHA**
-   **BETA**
-   **GAMMA**
-   **MUON**
-   **INT MUON**
-   **SHORT MUON**
-   **UNKNOWN**

------------------------------------------------------------------------

# Installation

Download the latest release:

https://github.com/Dopple24/muon_decoder/releases/latest

Or build from source.

## Requirements

-   Rust toolchain\
    https://rust-lang.org

## Build

``` bash
git clone https://github.com/Dopple24/muon_decoder
cd muon_decoder
cargo build --release
```

The compiled binary will appear in:

    target/release/

------------------------------------------------------------------------

# Usage

When opening a dataset, the program prompts for several detector
parameters.

## Detector parameters

You will be asked to enter:

-   **Pixel depth**
-   **Pixel width**
-   **Detector orientation**

The detector must be mounted **vertically**.

The longer side of the detector should point toward one of the cardinal
directions:

-   **North**
-   **South**
-   **East**
-   **West**

Conceptually, the detector should behave like a **compass needle
pointing toward the selected direction**.

These parameters are used to correctly calculate **particle trajectory
angles**.

------------------------------------------------------------------------

# Input Format

The application expects a `.txt` file containing energy values detected
by pixels.

Each line represents a detected pixel hit:

    x y energy

Example:

    12 34 0.84
    13 34 1.02
    14 34 0.95

Where:

-   **x** → pixel x-coordinate
-   **y** → pixel y-coordinate
-   **energy** → measured energy value

## Timestamp data (optional)

A `.dsc` file may be provided alongside the `.txt` file.

This file contains timestamps corresponding to each captured frame,
allowing the program to associate particle tracks with **capture
times**.

------------------------------------------------------------------------

# Viewing Modes

## Single Mode

Displays a **single particle track**.

Information shown includes:

-   energy values
-   track length
-   particle classification
-   calculated angles

This mode is useful for **detailed inspection of individual events**.

------------------------------------------------------------------------

## Combined Mode

Displays **all tracks detected within a single frame** of the input
file.

The frame number corresponds to the index of the frame in the source
file (starting from **1**).

This mode allows quick analysis of **event activity within a single
capture**.

------------------------------------------------------------------------

## Compound Mode

Displays **all particle tracks from a directory of files**.

Useful for:

-   analyzing large datasets
-   collecting statistics
-   exporting **muon track data**

------------------------------------------------------------------------

## 3D View

Interactive 3D visualization of reconstructed particle tracks.

This feature allows spatial exploration of particle trajectories and
helps better understand track geometry.

3D viewer implementation by **@Jenyyk**.

------------------------------------------------------------------------

# Particle Classification

Particles are classified based on the **shape and size of their pixel
clusters**.

### Alpha

-   Thick, round clusters
-   High energy density
-   Usually short and blob-like

### Beta

-   Short tracks
-   Often curved or irregular

### Gamma

-   Single or very small clusters
-   Typically appear as **dots**

### Muon

-   Straight tracks
-   Minimum length: **20 pixels** (default)
-   Threshold can be changed when opening the file

### Int Muon

"Interesting muon"

-   Long tracks
-   Not perfectly straight
-   May indicate:
    -   delta electron generation
    -   scattering
    -   other unusual events

### Short Muon

Tracks that are:

-   longer than **20 pixels**
-   shorter than the user-defined **minimum muon length**

### Unknown

Tracks that cannot be reliably classified.

------------------------------------------------------------------------

# Calculated Values

The software computes several physical quantities for each detected
particle.

### Zenith

Angle from which the particle arrived.

Range:

    -90° to 90°

Where:

-   **0°** = particle coming directly from above.

This value is most reliable for **muon tracks**.

### Absolute Angle

Absolute value of the zenith angle.

### Azimuth

Orientation of the detector.

Reference directions:

    0°   → North
    90°  → East
    180° → South
    270° → West

### Azimuth Offset

Muons rarely travel perfectly perpendicular to the detector.

This value estimates the **true azimuth angle of the particle
trajectory**.

Only the **absolute value** can be determined, because the detector
cannot distinguish whether the particle came from the left or right
side.

### Total Energy

Sum of all pixel energies belonging to a track in keV.

    total energy = sum(pixel energies)

### Size

Number of pixels belonging to a particle track.

### LET (Linear Energy Transfer)

Energy deposited per unit track length in keV/cm.

    LET = total energy / particle length

------------------------------------------------------------------------

# Language Support

Currently supported languages:

-   **English** (`en`)
-   **Czech** (`cs`)
-   **German** (`de`)

------------------------------------------------------------------------

# Credits

3D visualization implemented by:

**@Jenyyk**\
https://github.com/jenyyk

------------------------------------------------------------------------

# License

License: GPL-3.0
