#![windows_subsystem = "windows"]
use dotenvy::from_filename;
use eframe::egui::{self, viewport::IconData};
use std::env;
use std::path::Path;

mod decoder;
mod file_reader;
mod graphics;
mod particle_extractor;
mod renderer;

const DEFAULT_MIN_MUON_SIZE: usize = 20;
const DEFAULT_PIXEL_DEPTH: usize = 30;
const DEFAULT_PIXEL_WIDTH: f32 = 54.6875;
const SIZE: usize = 256;

#[derive(Debug, Clone)]
struct Config {
    pub default_min_muon_size: usize,
    pub default_pixel_depth: usize,
    pub default_pixel_width: f32,
    pub size: usize,
}

impl Config {
    pub fn load() -> Self {
        from_filename("./assets/config.env").ok();

        Self {
            default_min_muon_size: get_env("DEFAULT_MIN_MUON_SIZE", DEFAULT_MIN_MUON_SIZE),
            default_pixel_depth: get_env("DEFAULT_PIXEL_DEPTH", DEFAULT_PIXEL_DEPTH),
            default_pixel_width: get_env("DEFAULT_PIXEL_WIDTH", DEFAULT_PIXEL_WIDTH),
            size: get_env("SIZE", SIZE),
        }
    }
}

fn get_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() -> eframe::Result<()> {
    let tracks: Vec<decoder::Particle> = Vec::new();

    // graphics
    let options = match load_icon() {
        Some(icon) => eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([920.0, 620.0])
                .with_icon(icon),
            ..Default::default()
        },
        None => eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([920.0, 620.0]),
            ..Default::default()
        },
    };
    eframe::run_native(
        "Particle decoder",
        options,
        Box::new(move |_cc| Box::new(graphics::MatrixApp::new(tracks, 2, &Config::load()))),
    )
}

fn load_icon() -> Option<IconData> {
    // generate blank icon on the fly if one is not found
    let image = match std::fs::read(Path::new(r"assets/image.png")) {
        Ok(icon) => image::load_from_memory(&icon).expect("Failed to convert icon on disk"),
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
