#![windows_subsystem = "windows"]
use dotenvy::from_filename;
use eframe::egui::{self, viewport::IconData};
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
pub enum Langs {
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

macro_rules! define_locale {
    ($( $field:ident ),* $(,)?) => {
        #[derive(Debug, Clone)]
        pub struct Texts {
            $( pub $field: String, )*
        }

        impl Texts {
            pub fn load(lang: &Langs) -> Self {
                let english: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(EN).expect("English locale is invalid JSON");

                let (raw, is_english) = match lang {
                    Langs::En  => (EN,  true),
                    Langs::Cs  => (CS,  false),
                    Langs::Ocs => (OCS, false),
                    Langs::De  => (DE,  false),
                    Langs::Uwu => (UWU, false),
                };

                let map: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(raw).expect("Locale is invalid JSON");

                let fallback = if is_english { None } else { Some(&english) };
                let mut missing: Vec<&'static str> = Vec::new();

                let result = Self::from_map(&map, fallback, &mut missing);

                if !missing.is_empty() {
                    eprintln!("[i18n] {:?} is missing keys (fell back to English or sentinel): {:?}", lang, missing);
                }

                result
            }

            /// Used by load() at runtime (warns) and by tests (can panic).
            pub fn load_strict(lang: &Langs) -> Result<Self, Vec<&'static str>> {
                let english: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(EN).expect("English locale is invalid JSON");

                let raw = match lang {
                    Langs::En  => EN,
                    Langs::Cs  => CS,
                    Langs::Ocs => OCS,
                    Langs::De  => DE,
                    Langs::Uwu => UWU,
                };

                let map: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(raw).expect("Locale is invalid JSON");

                let is_english = matches!(lang, Langs::En);
                let fallback = if is_english { None } else { Some(&english) };
                let mut missing: Vec<&'static str> = Vec::new();

                let result = Self::from_map(&map, fallback, &mut missing);

                if missing.is_empty() { Ok(result) } else { Err(missing) }
            }

            fn from_map(
                map: &serde_json::Map<String, serde_json::Value>,
                fallback: Option<&serde_json::Map<String, serde_json::Value>>,
                missing: &mut Vec<&'static str>,
            ) -> Self {
                let mut get = |key: &'static str| -> String {
                    if let Some(v) = map.get(key).and_then(|v| v.as_str()) {
                        return v.to_string();
                    }
                    missing.push(key);
                    fallback
                        .and_then(|fb| fb.get(key))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("⚠ MISSING:{key}"))
                };

                Self {
                    $( $field: get(stringify!($field)), )*
                }
            }
        }
    };
}

define_locale! {
    title,
    arrow_left,
    arrow_right,
    single_mode,
    combined_mode,
    compound_mode,
    cubic_view,
    mode,
    particles,
    heavy_blob,
    curly_track,
    dot,
    straight_track,
    int_straight_track,
    heavy_track,
    unknown,
    too_short_muon,
    muon_section_header,
    sus_muon_section_header,
    export,
    export_all,
    import,
    import_dialog_title,
    import_dialog_dimensions,
    import_dialog_depth,
    import_dialog_min_muon_size,
    import_dialog_compass,
    import_dialog_confirm,
    import_dialog_cancel,
    north,
    south,
    east,
    west,
    tracks,
    file_location,
    particle_type_label,
    size_label,
    avg_energy_label,
    let_avg_label,
    total_energy_label,
    azimuth_label,
    azimuth_offset_label,
    abs_angle_primary_label,
    zenith_label,
    winding_label,
    get_frame_index_label,
    get_timestamp_label,
    muon_list_zenith,
    muon_list_abs_angle_primary,
    muon_list_azimuth,
    muon_list_total_energy,
    muon_list_size,
    muon_list_let_avg,
    muon_list_timestamp,
    muon_list_frame_index,
    muon_list_file,
    muon_list_azimuth_offset,
    save_csv,
}

#[cfg(test)]
mod tests {
    use super::*;

    // attempt to load all locales
    #[test]
    fn test_locales() {
        for l in Langs::list(true) {
            // load_strict panics on fail, letting you know if a translation is missing
            // regular load falls back on the english version (or prints a warning) to avoid runtime panics
            Texts::load_strict(&l).inspect_err(|e| {
                panic!("Failed to load locale {}:\n  Missing values: {:?}", l, e);
            }).unwrap();
        }
    }
}
