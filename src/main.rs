#![windows_subsystem = "windows"]
use dotenvy::from_filename;
use eframe::egui::{self, viewport::IconData};
use serde::Deserialize;
use std::env;
use std::path::Path;
use std::{fmt, fs, str::FromStr};

mod decoder;
mod file_reader;
mod graphics;
mod particle_extractor;
mod renderer;

const DEFAULT_MIN_MUON_SIZE: usize = 20;
const DEFAULT_PIXEL_DEPTH: usize = 30;
const DEFAULT_PIXEL_WIDTH: f32 = 54.6875;
const SIZE: usize = 256;
const LANG: Langs = Langs::En;

const CONFIG_PATH: &str = "./assets/config.env";
const LOCALES_PATH: &str = "./locales";

#[derive(Debug, Clone)]
struct Config {
    pub default_min_muon_size: usize,
    pub default_pixel_depth: usize,
    pub default_pixel_width: f32,
    pub size: usize,
    pub lang: Langs,
}

impl Config {
    pub fn load() -> Self {
        from_filename(CONFIG_PATH).ok();

        Self {
            default_min_muon_size: get_env("DEFAULT_MIN_MUON_SIZE", DEFAULT_MIN_MUON_SIZE),
            default_pixel_depth: get_env("DEFAULT_PIXEL_DEPTH", DEFAULT_PIXEL_DEPTH),
            default_pixel_width: get_env("DEFAULT_PIXEL_WIDTH", DEFAULT_PIXEL_WIDTH),
            size: get_env("SIZE", SIZE),
            lang: get_env("LANGUAGE", LANG),
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
    let configs = Config::load();
    eframe::run_native(
        "Particle decoder",
        options,
        Box::new(move |_cc| {
            Box::new(graphics::MatrixApp::new(
                tracks,
                2,
                &configs,
                &Texts::load(&configs.lang),
                &configs.lang,
            ))
        }),
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

#[derive(Debug, Clone, PartialEq)]
enum Langs {
    Cs,
    En,
}

impl Langs {
    fn list() -> Vec<Self> {
        vec![Self::En, Self::Cs]
    }

    fn change_lang(new_lang: &Langs) -> Texts {
        let content = format!(
            "DEFAULT_MIN_MUON_SIZE=20\n\
        DEFAULT_PIXEL_DEPTH=300\n\
        DEFAULT_PIXEL_WIDTH=54.6875\n\
        SIZE=256\n\
        LANGUAGE=\"{}\"\n",
            new_lang
        );
        if let Err(y) = fs::write(Path::new(CONFIG_PATH), content) {
            eprintln!("error changing lang in config {}", y);
        }

        Texts::load(new_lang)
    }

    #[allow(dead_code)]
    fn save(new_config: Config) {
        let content = format!(
            "DEFAULT_MIN_MUON_SIZE={}\n\
        DEFAULT_PIXEL_DEPTH={}\n\
        DEFAULT_PIXEL_WIDTH={}\n\
        SIZE={}\n\
        LANGUAGE=\"{}\"\n",
            &new_config.default_min_muon_size.to_string(),
            &new_config.default_pixel_depth.to_string(),
            &new_config.default_pixel_width.to_string(),
            &new_config.size.to_string(),
            &new_config.lang.to_string(),
        );
        if let Err(y) = fs::write(Path::new(CONFIG_PATH), content) {
            eprintln!("error changing lang in config {}", y);
        }
    }
}

impl FromStr for Langs {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cs" => Ok(Langs::Cs),
            "en" => Ok(Langs::En),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Langs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Langs::Cs => write!(f, "cs"),
            Langs::En => write!(f, "en"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Texts {
    pub title: String,
    pub arrow_left: String,
    pub arrow_right: String,
    pub single_mode: String,
    pub combined_mode: String,
    pub compound_mode: String,
    pub cubic_view: String,
    pub mode: String,
    pub particles: String,
    pub alpha: String,
    pub beta: String,
    pub gamma: String,
    pub muon: String,
    pub sus_muon: String,
    pub unknown: String,
    pub too_short_muon: String,
    pub muon_section_header: String,
    pub sus_muon_section_header: String,
    pub export: String,
    pub import: String,
    pub import_dialog_title: String,
    pub import_dialog_dimensions: String,
    pub import_dialog_depth: String,
    pub import_dialog_width: String,
    pub import_dialog_min_muon_size: String,
    pub import_dialog_compass: String,
    pub import_dialog_confirm: String,
    pub import_dialog_cancel: String,
    pub north: String,
    pub south: String,
    pub east: String,
    pub west: String,
    pub tracks: String,
    pub file_location: String,
    pub particle_type_label: String,
    pub size_label: String,
    pub avg_energy_label: String,
    pub let_avg_label: String,
    pub total_energy_label: String,
    pub azimuth_label: String,
    pub azimuth_offset_label: String,
    pub abs_angle_primary_label: String,
    pub zenith_label: String,
    pub winding_label: String,
    pub get_frame_index_label: String,
    pub get_timestamp_label: String,
    pub muon_list_zenith: String,
    pub muon_list_abs_angle_primary: String,
    pub muon_list_azimuth: String,
    pub muon_list_total_energy: String,
    pub muon_list_size: String,
    pub muon_list_let_avg: String,
    pub muon_list_timestamp: String,
    pub muon_list_frame_index: String,
    pub muon_list_file: String,
    pub save_csv: String,
}

impl Texts {
    fn load(lang: &Langs) -> Self {
        let text = fs::read_to_string(format!("{}/{}.json", LOCALES_PATH, lang)).unwrap();
        serde_json::from_str(&text).unwrap()
    }
}
