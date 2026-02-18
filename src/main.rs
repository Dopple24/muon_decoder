//#![windows_subsystem = "windows"]
use eframe::egui::{self, viewport::IconData};
use std::path::Path;

mod decoder;
mod file_reader;
mod graphics;
mod particle_extractor;


const SIZE: usize = 256;

fn main() -> eframe::Result<()> {
    let grid: Vec<Vec<Vec<f32>>> = vec![vec![vec![0.0; SIZE]; SIZE]; 1];

    let tracks: Vec<decoder::Particle> = Vec::new();

    // graphics
    let options = match load_icon() {
        Some(icon) => {
            eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default().with_inner_size([920.0, 620.0]).with_icon(icon),
                ..Default::default()
            }
        },
        None => {
            eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default().with_inner_size([920.0, 620.0]),
                ..Default::default()
            }
        }
    };
    eframe::run_native(
        "Muon finder",
        options,
        Box::new(move |_cc| Box::new(graphics::MatrixApp::new(grid, tracks, 2))),
    )
}

fn load_icon() -> Option<IconData> {
    // generate blank icon on the fly if one is not found
    let image = match std::fs::read(Path::new(r"assets/image.png")) {
        Ok(icon) => image::load_from_memory(&icon)
            .expect("Failed to convert icon on disk"),
        _ => {
            eprintln!("Icon file not found, using placeholder");
            image::DynamicImage::new_rgba8(1, 1)
        }
    }
    .into_rgba8();
        

    let (width, height) = image.dimensions();
    let rgba = image.into_raw();

    Some(IconData {
        rgba,
        width,
        height,
    })
}
