#![windows_subsystem = "windows"]
use dotenvy::from_filename;
use eframe::egui::{self, viewport::IconData};
use serde::Deserialize;
use std::env;
use std::path::Path;
use std::{fmt, fs, str::FromStr};

use image::load_from_memory;

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

const CONFIG: &str = include_str!("../config.env");
const EN: &str = include_str!("../locales/en.json");
const CS: &str = include_str!("../locales/cs.json");
const DE: &str = include_str!("../locales/de.json");
const OCS: &str = include_str!("../locales/ocs.json");
const UWU: &str = include_str!("../locales/uw.json");

const CONFIG_PATH: &str = "../config.env";

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
        if Path::new(CONFIG_PATH).exists() {
            from_filename(CONFIG_PATH).ok();
        } else {
            fs::write("config.env", CONFIG).unwrap();
            from_filename(CONFIG_PATH).ok();
        }
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
    let args: Vec<String> = env::args().collect();

    let easter_egg_on = if args.len() > 1 {
        args[1] == "ocs"
    } else {
        false
    };

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
                easter_egg_on,
            ))
        }),
    )
}

fn load_icon() -> Option<IconData> {
    let image_bytes = include_bytes!("../assets/image.png");

    let image = load_from_memory(image_bytes).ok()?;
    let image = image.into_rgba8();

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
    Ocs,
    Uwu,
    De,
}

impl Langs {
    fn list(easter_egg_on: bool) -> Vec<Self> {
        if easter_egg_on {
            vec![Self::En, Self::Cs, Self::Ocs, Self::De, Self::Uwu]
        } else {
            vec![Self::En, Self::Cs, Self::De]
        }
    }

    fn to_readable(&self) -> String {
        match self {
            Self::En => "English".to_string(),
            Self::Cs => "Čeština".to_string(),
            Self::De => "Deutsch".to_string(),
            Self::Ocs => "Staročeština".to_string(),
            Self::Uwu => "UwU".to_string(),
        }
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
            "ocs" => Ok(Langs::Ocs),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Langs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Langs::Cs => write!(f, "cs"),
            Langs::En => write!(f, "en"),
            Langs::Ocs => write!(f, "ocs"),
            Langs::De => write!(f, "de"),
            Langs::Uwu => write!(f, "uwu"),
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
    pub heavy_blob: String,
    pub curly_track: String,
    pub dot: String,
    pub straight_track: String,
    pub int_straight_track: String,
    pub heavy_track: String,
    pub unknown: String,
    pub too_short_muon: String,
    pub muon_section_header: String,
    pub sus_muon_section_header: String,
    pub export: String,
    pub export_all: String,
    pub import: String,
    pub import_dialog_title: String,
    pub import_dialog_dimensions: String,
    pub import_dialog_depth: String,
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
    pub muon_list_azimuth_offset: String,
    pub save_csv: String,
}

impl Texts {
    fn load(lang: &Langs) -> Self {
        let text = match lang {
            Langs::En => EN.to_string(),
            Langs::Cs => CS.to_string(),
            Langs::Ocs => OCS.to_string(),
            Langs::De => DE.to_string(),
            Langs::Uwu => UWU.to_string(),
        };
        serde_json::from_str(&text).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // attempt to load all locales
    // panics on fail, letting you know if a translation is missing
    #[test]
    fn test_locales() {
        for l in Langs::list(true) {
            let _ = Texts::load(&l);
        }
    }
}
