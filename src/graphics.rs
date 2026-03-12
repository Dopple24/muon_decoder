use crate::{
    decoder::{PartType, Particle},
    file_reader::{Tracks, list_dir},
    particle_extractor::{self},
};
use chrono::Utc;
use eframe::egui::{self, ColorImage};
use rayon::prelude::*;
use rfd::FileDialog;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, io::Write, path::PathBuf};

pub const DEFAULT_MIN_MUON_SIZE: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Mode {
    Single,
    Combined,
    Compound,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Orientation {
    North,
    South,
    West,
    East,
}
impl Orientation {
    fn into_readable(self) -> String {
        match self {
            Self::North => "North".to_string(),
            Self::South => "South".to_string(),
            Self::West => "West".to_string(),
            Self::East => "East".to_string(),
        }
    }
    pub fn azimuth(&self) -> f32 {
        match self {
            Self::North => 0.0,
            Self::South => 180.0,
            Self::West => 270.0,
            Self::East => 90.0,
        }
    }
    fn all_values(&self) -> Vec<Orientation> {
        vec![Self::North, Self::South, Self::West, Self::East]
    }
}

#[derive(Debug)]
pub struct Muon {
    file: PathBuf,
    timestamp: chrono::DateTime<Utc>,
    frame_index: usize,
    total_energy: f32,
    azimuth: f32,
    abs_angle_primary: f32,
    azimuth_offset: f32,
    zenith: f32,
    size: usize,
    let_avg: f32,
}

#[derive(Debug)]
pub struct MatrixApp {
    matricees: Vec<Tracks>,
    current_file: usize,
    current_matrix: usize,
    all_tracks: Vec<Particle>,
    tracks_to_draw: Vec<Particle>,
    muons: Vec<Muon>,
    sus_muons: Vec<Muon>,
    scale: usize,
    current_track: usize,
    image: ColorImage,
    needs_update: bool,
    current_mode: Mode,
    error: Option<String>,

    show_alpha: bool,
    show_beta: bool,
    show_gamma: bool,
    show_muon: bool,
    show_sus_muon: bool,
    show_unknown: bool,
    show_too_short_muon: bool,

    show_dialog: bool,
    input_depth: String,
    input_width: String,
    input_min_muon_size: String,
    pub pixel_depth: Option<i32>,
    pub pixel_width: Option<f32>,
    pub min_muon_size: Option<usize>,
    pub selected_mode: Orientation,
    renderer_3d: crate::renderer::Renderer3D,

    // Sorting state for muon grid
    muon_sort_column: Option<usize>,
    muon_sort_ascending: bool,

    // Sorting state for sus_muon grid
    sus_muon_sort_column: Option<usize>,
    sus_muon_sort_ascending: bool,
}

impl MatrixApp {
    pub fn new(tracks: Vec<Particle>, scale: usize) -> Self {
        let mut app = Self {
            matricees: vec![Tracks::default()],
            current_file: 0,
            current_matrix: 0,
            all_tracks: tracks.clone(),
            tracks_to_draw: tracks,
            muons: Vec::new(),
            sus_muons: Vec::new(),
            scale,
            current_track: 0,
            image: ColorImage {
                size: [1, 1],
                pixels: vec![],
            },
            needs_update: true,
            current_mode: Mode::Combined,
            error: None,
            show_alpha: true,
            show_beta: true,
            show_gamma: true,
            show_muon: true,
            show_sus_muon: true,
            show_unknown: true,
            show_too_short_muon: true,
            show_dialog: false,
            input_min_muon_size: DEFAULT_MIN_MUON_SIZE.to_string(),
            input_depth: crate::decoder::DEFAULT_PIXEL_DEPTH.to_string(),
            input_width: crate::decoder::DEFAULT_PIXEL_WIDTH.to_string(),
            pixel_depth: None,
            pixel_width: None,
            selected_mode: Orientation::North,
            min_muon_size: None,
            renderer_3d: crate::renderer::Renderer3D::new(),
            muon_sort_column: None,
            muon_sort_ascending: true,
            sus_muon_sort_column: None,
            sus_muon_sort_ascending: true,
        };
        app.update_image();
        app
    }

    /// Update the image for current track or combined tracks
    fn update_image(&mut self) {
        if self.current_mode == Mode::Compound {
            self.init_compound_mode();
        } else {
            // clear unused memory (we recalculate it again anyway)
            self.muons.clear();
            self.sus_muons.clear();
        }
        if self.current_mode == Mode::Combined {
            self.init_combined();
        }
        self.update_counter();
        let size_x = crate::SIZE;
        let size_y = crate::SIZE;
        let img_x = size_x * self.scale;
        let img_y = size_y * self.scale;
        let mut pixels = vec![egui::Color32::BLACK; img_x * img_y];

        if self.tracks_to_draw.is_empty() {
            self.image = ColorImage {
                size: [img_x, img_y],
                pixels,
            };
            return;
        }

        let tracks_to_draw: Vec<Vec<(usize, usize)>> = match self.current_mode {
            Mode::Single => vec![self.tracks_to_draw[self.current_track].get_track()],
            _ => self.tracks_to_draw.iter().map(|p| p.get_track()).collect(),
        };

        crate::renderer::update_data(tracks_to_draw.clone(), self);

        for track_cells in tracks_to_draw {
            let color = egui::Color32::WHITE;
            for (x, y) in track_cells {
                for dx in 0..self.scale {
                    for dy in 0..self.scale {
                        // Rotate 90 degrees counter-clockwise: (x,y) -> (y, size_x-x)
                        let px = (size_y - 1 - y) * self.scale + dy;
                        let py = x * self.scale + dx;
                        if px < img_x && py < img_y {
                            pixels[px * img_y + py] = color;
                        }
                    }
                }
            }
        }

        self.image = ColorImage {
            size: [img_x, img_y],
            pixels,
        };

        // clear memory
        self.muons.shrink_to_fit();
        self.sus_muons.shrink_to_fit();
        self.all_tracks.shrink_to_fit();
        self.tracks_to_draw.shrink_to_fit();
        self.matricees.shrink_to_fit();
    }

    fn update_counter(&mut self) {
        let filters = [
            (self.show_alpha, PartType::Alpha),
            (self.show_beta, PartType::Beta),
            (self.show_gamma, PartType::Gamma),
            (self.show_muon, PartType::Muon),
            (self.show_sus_muon, PartType::SusMuon),
            (self.show_unknown, PartType::Unknown),
            (self.show_too_short_muon, PartType::TooShortMuon),
        ];

        self.tracks_to_draw.clear();

        for track in &mut self.all_tracks {
            let matrix_idx = if self.current_mode == Mode::Compound {
                track.get_frame_index()
            } else {
                self.current_matrix
            };

            if filters.iter().any(|(show, ty)| {
                *show
                    && track
                        .particle_type(&self.matricees[self.current_file].get_tracks()[matrix_idx].matrix, self.min_muon_size.unwrap_or(DEFAULT_MIN_MUON_SIZE)) //has panicked - out of bounds exception self.current_matrix index 95, len 46
                        == *ty
            }) {
                self.tracks_to_draw.push(track.clone());
            }
        }
        if self.current_track >= self.tracks_to_draw.len() {
            self.current_track = self.tracks_to_draw.len().max(1) - 1;
        }
    }

    fn move_track(&mut self) {
        self.current_track = (self.current_track + 1) % self.tracks_to_draw.len().max(1);
    }

    fn move_track_back(&mut self) {
        self.current_track = if self.current_track == 0 {
            self.tracks_to_draw.len().max(1) - 1
        } else {
            self.current_track - 1
        };
    }

    fn matrix_move(&mut self) {
        self.current_track = 0;
        // check if we are at the end of the current file
        if self.current_matrix >= self.matricees[self.current_file].get_tracks().len().max(1) - 1 {
            self.matricees[self.current_file].clear_cache();

            self.current_file = (self.current_file + 1) % self.matricees.len();
            self.current_matrix = 0;
        } else {
            self.current_matrix = (self.current_matrix + 1)
                % self.matricees[self.current_file].get_tracks().len().max(1);
        }
    }

    fn matrix_move_back(&mut self) {
        self.current_track = 0;
        self.current_matrix = if self.current_matrix == 0 {
            self.matricees[self.current_file].clear_cache();

            self.current_file = if self.current_file == 0 {
                self.matricees.len().max(1) - 1
            } else {
                self.current_file - 1
            };
            self.matricees[self.current_file].get_tracks().len().max(1) - 1
        } else {
            self.current_matrix - 1
        };
    }

    fn move_file(&mut self) {
        self.current_track = 0;
        self.current_matrix = 0;
        let old_file = self.current_file;
        self.current_file = (self.current_file + 1) % self.matricees.len().max(1);
        // Clear the old file's cache after moving to the new file
        self.matricees[old_file].clear_cache();
    }

    fn move_file_back(&mut self) {
        self.current_track = 0;
        self.current_matrix = 0;
        let old_file = self.current_file;
        self.current_file = if self.current_file == 0 {
            self.matricees.len().max(1) - 1
        } else {
            self.current_file - 1
        };
        // Clear the old file's cache after moving to the new file
        self.matricees[old_file].clear_cache();
    }

    fn move_data(&mut self) {
        match self.current_mode {
            Mode::Single => self.move_track(),
            Mode::Combined => self.matrix_move(),
            Mode::Compound => self.move_file(),
        }
        self.update_image();
    }

    fn move_data_back(&mut self) {
        match self.current_mode {
            Mode::Single => self.move_track_back(),
            Mode::Combined => self.matrix_move_back(),
            Mode::Compound => self.move_file_back(),
        }
        self.update_image();
    }

    fn init_compound_mode(&mut self) {
        // Clear old data before loading new data
        self.all_tracks.clear();
        self.sus_muons.clear();
        self.muons.clear();
        self.all_tracks.shrink_to_fit();
        self.muons.shrink_to_fit();
        self.sus_muons.shrink_to_fit();

        let all_buf: Arc<Mutex<Vec<Particle>>> = Arc::new(Mutex::new(Vec::new()));
        let muons_buf: Arc<Mutex<Vec<Muon>>> = Arc::new(Mutex::new(Vec::new()));
        let sus_muons_buf: Arc<Mutex<Vec<Muon>>> = Arc::new(Mutex::new(Vec::new()));

        let file_path = self.matricees[self.current_file].file_path.clone();

        self.matricees[self.current_file]
            .get_tracks()
            .iter_mut()
            .enumerate()
            .par_bridge()
            .into_par_iter()
            .for_each(|(frame_index, p)| {
                let mut buffer = vec![vec![0; crate::SIZE]; crate::SIZE];
                let particles = particle_extractor::extract(&p.matrix, &mut buffer, 2)
                    .into_values()
                    .collect::<Vec<_>>();

                for part in &particles {
                    let mut particle = Particle::new(
                        part.clone(),
                        frame_index,
                        self.pixel_depth,
                        self.pixel_width,
                        self.selected_mode,
                        Some(p.timestamp),
                    );
                    let part_type = particle.particle_type(
                        &p.matrix,
                        self.min_muon_size.unwrap_or(DEFAULT_MIN_MUON_SIZE),
                    );
                    if part_type == PartType::Muon {
                        muons_buf.lock().unwrap().push(Muon {
                            file: file_path.clone(),
                            timestamp: p.timestamp,
                            frame_index,
                            total_energy: particle.total_energy(&p.matrix),
                            azimuth: particle.azimuth(),
                            azimuth_offset: particle.azimuth_offset(),
                            abs_angle_primary: particle.abs_angle_primary(),
                            zenith: particle.zenith(),
                            size: particle.size(),
                            let_avg: particle.let_avg(&p.matrix),
                        })
                    } else if part_type == PartType::SusMuon {
                        sus_muons_buf.lock().unwrap().push(Muon {
                            file: file_path.clone(),
                            timestamp: p.timestamp,
                            frame_index,
                            total_energy: particle.total_energy(&p.matrix),
                            azimuth: particle.azimuth(),
                            azimuth_offset: particle.azimuth_offset(),
                            abs_angle_primary: particle.abs_angle_primary(),
                            zenith: particle.zenith(),
                            size: particle.size(),
                            let_avg: particle.let_avg(&p.matrix),
                        })
                    }
                    all_buf.lock().unwrap().push(particle);
                }
            });

        self.all_tracks = std::mem::take(&mut *all_buf.lock().unwrap());
        self.muons = std::mem::take(&mut *muons_buf.lock().unwrap());
        self.sus_muons = std::mem::take(&mut *sus_muons_buf.lock().unwrap());
        self.all_tracks.shrink_to_fit();
        self.muons.shrink_to_fit();
        self.sus_muons.shrink_to_fit();

        // Reset sorting state when data changes
        self.muon_sort_column = None;
        self.muon_sort_ascending = true;
        self.sus_muon_sort_column = None;
        self.sus_muon_sort_ascending = true;
    }

    fn init_combined(&mut self) {
        let curr_matrix = &self.matricees[self.current_file].get_tracks()[self.current_matrix];
        self.all_tracks = crate::particle_extractor::extract(
            &curr_matrix.matrix,
            &mut vec![vec![0; crate::SIZE]; crate::SIZE],
            2,
        )
        .values()
        .map(|t| {
            crate::decoder::Particle::new(
                t.clone(),
                self.current_matrix,
                self.pixel_depth,
                self.pixel_width,
                self.selected_mode,
                Some(curr_matrix.timestamp),
            )
        })
        .collect();
    }
}

impl eframe::App for MatrixApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        use egui::Key;

        // ----------------------------
        // Input handling
        // ----------------------------
        if ctx.input(|i| i.key_pressed(Key::ArrowRight)) {
            self.move_data();
        }

        if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
            self.move_data_back();
        }

        if ctx.input(|i| i.key_pressed(Key::M)) && !self.tracks_to_draw.is_empty() {
            self.current_mode = Mode::Single;
            self.needs_update = true;
        }

        if self.needs_update {
            self.update_image();
            self.needs_update = false;
        }

        // Display the 3D renderer window
        self.renderer_3d.show(ctx);

        // ============================
        // TOP BAR
        // ============================
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Particle Matrix Viewer");

                ui.separator();

                if ui.button("◀ Prev").clicked() {
                    self.move_data_back();
                }

                if ui.button("Next ▶").clicked() {
                    self.move_data();
                }

                if ui.button("Single").clicked() {
                    self.current_mode = Mode::Single;
                    self.current_track =
                        self.current_track.min(self.tracks_to_draw.len().max(1) - 1);
                    self.update_image();
                }

                if ui.button("3D View").clicked() {
                    self.renderer_3d.toggle_window();
                }

                if ui.button("Combined").clicked() {
                    self.current_mode = Mode::Combined;
                    self.update_image();
                }

                if ui.button("Compound").clicked() {
                    self.current_mode = Mode::Compound;
                    self.update_image();
                }

                ui.separator();

                ui.label(match self.current_mode {
                    Mode::Single => "Mode: Single Track",
                    Mode::Combined => "Mode: Combined",
                    Mode::Compound => "Mode: Compound",
                });
            });
        });

        // ============================
        // LEFT PANEL — STATS
        // ============================
        egui::SidePanel::left("stats")
            .resizable(false)
            .min_width(100.0)
            .show(ctx, |ui| {
                ui.heading("📊 Particles");

                let mut count = HashMap::new();
                for p in [
                    PartType::Alpha,
                    PartType::Beta,
                    PartType::Gamma,
                    PartType::Muon,
                    PartType::SusMuon,
                    PartType::Unknown,
                    PartType::TooShortMuon,
                ] {
                    count.insert(p, 0usize);
                }

                for particle in &mut self.tracks_to_draw {
                    *count
                        .get_mut(
                            &particle.particle_type(
                                &self.matricees[self.current_file].get_tracks()
                                    [self.current_matrix]
                                    .matrix,
                                self.min_muon_size.unwrap_or(DEFAULT_MIN_MUON_SIZE),
                            ),
                        )
                        .unwrap() += 1;
                }

                egui::Grid::new("stats_grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        for (label, ty) in [
                            ("Alpha", PartType::Alpha),
                            ("Beta", PartType::Beta),
                            ("Gamma", PartType::Gamma),
                            ("Muon", PartType::Muon),
                            ("Int muon", PartType::SusMuon),
                            ("Unknown", PartType::Unknown),
                            ("Short muon", PartType::TooShortMuon),
                        ] {
                            ui.label(label);
                            ui.label(count.get(&ty).unwrap().to_string());
                            ui.end_row();
                        }
                    });

                let response_al = ui.checkbox(&mut self.show_alpha, "Alpha");
                let response_be = ui.checkbox(&mut self.show_beta, "Beta");
                let response_ga = ui.checkbox(&mut self.show_gamma, "Gamma");
                let response_mu = ui.checkbox(&mut self.show_muon, "Muon");
                let response_sm = ui.checkbox(&mut self.show_sus_muon, "Sus muon");
                let response_un = ui.checkbox(&mut self.show_unknown, "Unknown");
                let response_sh = ui.checkbox(&mut self.show_too_short_muon, "Short muon");

                if response_al.changed()
                    || response_be.changed()
                    || response_ga.changed()
                    || response_mu.changed()
                    || response_sm.changed()
                    || response_sh.changed()
                    || response_un.changed()
                {
                    self.update_image();
                    // this is redundant, update_image() already calls update_counter itself()
                    // self.update_counter();
                }
            });

        // ============================
        // RIGHT PANEL — STATS
        // ============================
        egui::SidePanel::right("muon_list")
            .resizable(true)
            .min_width(150.0)
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal()
                    .id_source("muons_scroll")
                    .show(ui, |ui| {
                        ui.heading("📊 Muons");
                        if ui.button("export").clicked() {
                            let csv = build_csv(&self.muons);

                            if let Err(e) = export_csv(&csv) {
                                self.error = Some(e);
                            }
                        }

                        show_muon_grid(
                            ui,
                            "muon_grid",
                            &mut self.muons,
                            &mut self.muon_sort_column,
                            &mut self.muon_sort_ascending,
                        );
                        ui.heading("📊 Sus Muons");
                        if ui.button("export").clicked() {
                            let csv = build_csv(&self.sus_muons);

                            if let Err(e) = export_csv(&csv) {
                                self.error = Some(e);
                            }
                        }

                        show_muon_grid(
                            ui,
                            "sus_grid",
                            &mut self.sus_muons,
                            &mut self.sus_muon_sort_column,
                            &mut self.sus_muon_sort_ascending,
                        );
                    });
            });

        // ============================
        // BOTTOM BAR
        // ============================
        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 Open File").clicked() {
                    self.show_dialog = true;
                }
                if self.show_dialog {
                    egui::Window::new("Enter Depth and Width")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading("Dimensions");
                            });

                            ui.add_space(10.0);

                            ui.horizontal(|ui| {
                                ui.label("Depth:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.input_depth)
                                        .hint_text("e.g. 30")
                                        .desired_width(100.0),
                                );
                            });

                            ui.horizontal(|ui| {
                                ui.label("Width:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.input_width)
                                        .hint_text("e.g. 30")
                                        .desired_width(100.0),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Min muon size:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.input_min_muon_size)
                                        .hint_text("20")
                                        .desired_width(100.0),
                                );
                            });

                            ui.add_space(10.0);

                            ui.label("Select mode:");

                            egui::ComboBox::from_id_source("mode_selector")
                                .selected_text(self.selected_mode.into_readable())
                                .show_ui(ui, |ui| {
                                    Orientation::North
                                        .all_values()
                                        .iter()
                                        .for_each(|direction| {
                                            ui.selectable_value(
                                                &mut self.selected_mode,
                                                *direction,
                                                direction.into_readable(),
                                            );
                                        });
                                });

                            ui.add_space(15.0);

                            ui.horizontal(|ui| {
                                if ui.button("OK").clicked()
                                    && let Ok(depth) = self.input_depth.parse::<i32>()
                                    && let Ok(width) = self.input_width.parse::<f32>()
                                    && let Ok(min_muon_size) =
                                        self.input_min_muon_size.parse::<usize>()
                                {
                                    self.pixel_depth = Some(depth);
                                    self.pixel_width = Some(width);
                                    self.min_muon_size = Some(min_muon_size.max(4));
                                    self.show_dialog = false;
                                    if let Some(path) = FileDialog::new().pick_folder() {
                                        if let Ok(mat) = list_dir(&path) {
                                            self.current_track = 0;
                                            self.matricees = mat;
                                            let mut id_map =
                                                vec![vec![0; crate::SIZE]; crate::SIZE];
                                            let curr_matrix = &self.matricees[self.current_matrix]
                                                .get_tracks()[self.current_matrix];
                                            self.all_tracks = crate::particle_extractor::extract(
                                                &curr_matrix.matrix,
                                                &mut id_map,
                                                2,
                                            )
                                            .values()
                                            .map(|t| {
                                                crate::decoder::Particle::new(
                                                    t.clone(),
                                                    self.current_matrix,
                                                    self.pixel_depth,
                                                    self.pixel_width,
                                                    self.selected_mode,
                                                    Some(curr_matrix.timestamp),
                                                )
                                            })
                                            .collect();
                                            self.update_image();
                                        } else {
                                            self.error = Some("error".to_string());
                                        }
                                    }
                                }

                                if ui.button("Cancel").clicked() {
                                    self.show_dialog = false;
                                }
                            });
                        });
                }
            });
        });

        // ============================
        // CENTER VIEW
        // ============================
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_min_width(550.0);
            ui.vertical_centered(|ui| {
                let texture =
                    ui.ctx()
                        .load_texture("track_image", self.image.clone(), Default::default());

                ui.image(&texture);

                ui.add_space(8.0);

                ui.label(format!(
                    "Track {}/{}\n file here: {}",
                    self.current_track + 1,
                    self.tracks_to_draw.len(),
                    self.matricees[self.current_file].file_path.to_string_lossy()
                ));

                if self.current_mode == Mode::Single {
                    if self.current_track >= self.tracks_to_draw.len() {
                        self.current_track = self.tracks_to_draw.len().max(1) - 1;
                    }
                    let selected_track = if self.tracks_to_draw.is_empty() {
                        &mut Particle::new(Vec::new(), 0, self.pixel_depth, self.pixel_width, self.selected_mode, /*timestamp*/Some(self.matricees[self.current_file].get_tracks()[self.current_track].timestamp))
                    }
                    else {
                        &mut self.tracks_to_draw[self.current_track]
                    };
                    ui.label(format!(
                        "Particle: {:?}\nsize: {}\naverage energy: {}\nLET: {}\ntotal energy: {}\nazimuth: {}\nazimuth offset: {}\n abs zenith: {}\n zenith: {}\nwinding: {}\nframe number: {:?}\n timestamp: {}",
                        selected_track.particle_type(&self.matricees[self.current_file].get_tracks()[self.current_matrix].matrix, self.min_muon_size.unwrap_or(DEFAULT_MIN_MUON_SIZE)),
                        selected_track.size(),
                        selected_track.avg_energy(&self.matricees[self.current_file].get_tracks()[self.current_matrix].matrix),
                        selected_track.let_avg(&self.matricees[self.current_file].get_tracks()[self.current_matrix].matrix),
                        selected_track.total_energy(&self.matricees[self.current_file].get_tracks()[self.current_matrix].matrix),
                        selected_track.azimuth(),
                        selected_track.azimuth_offset(),
                        selected_track.abs_angle_primary(),
                        selected_track.zenith(),
                        selected_track.winding(),
                        selected_track.get_frame_index() + 1,
                        selected_track.get_timestamp(),
                    ));
                }

                else if self.current_mode == Mode::Combined && !self.tracks_to_draw.is_empty(){
                    ui.label(format!(
                        "frame number: {}",
                        &self.tracks_to_draw[self.current_track.min(self.tracks_to_draw.len())].get_frame_index() + 1
                    ));
                }
            });
        });

        // ============================
        // ERROR POPUP
        // ============================
        if self.error.is_some() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(
                    egui::Frame::popup(&ctx.style())
                        .rounding(egui::Rounding::same(8.0))
                        .shadow(egui::epaint::Shadow::big_dark()),
                )
                .show(ctx, |ui| {
                    ui.heading("⚠ Error");
                    ui.label(self.error.as_ref().unwrap());
                    ui.add_space(10.0);
                    if ui.button("OK").clicked() {
                        self.error = None;
                    }
                });
        }
    }
}
fn build_csv(muons: &[Muon]) -> String {
    let mut content = String::new();

    content.push_str("zenith,abs_angle,azimuth,total_energy,size,LET,timestamp,frame#,file\n");

    for muon in muons {
        content.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            muon.zenith,
            muon.abs_angle_primary,
            muon.azimuth,
            muon.total_energy,
            muon.size,
            muon.let_avg,
            muon.timestamp,
            muon.frame_index,
            muon.file.to_string_lossy()
        ));
    }

    content
}
fn export_csv(content: &str) -> Result<(), String> {
    if let Some(path) = FileDialog::new()
        .set_title("Save CSV")
        .add_filter("CSV files", &["csv"])
        .set_file_name("data.csv")
        .save_file()
    {
        let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;

        writeln!(file, "{content}").map_err(|e| e.to_string())?;
    }

    Ok(())
}
use egui_extras::{Column, TableBuilder};
fn show_muon_grid(
    ui: &mut egui::Ui,
    id: &str,
    muons: &mut [Muon],
    sort_column: &mut Option<usize>,
    sort_ascending: &mut bool,
) {
    ui.push_id(id, |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .columns(Column::auto(), 10)
            .header(20.0, |mut header| {
                let headers = [
                    "Zenith",
                    "Abs zenith",
                    "Azimuth",
                    "azimuth offset",
                    "total energy",
                    "size",
                    "let",
                    "timestamp",
                    "frame #",
                    "file",
                ];

                for (col_idx, header_text) in headers.iter().enumerate() {
                    header.col(|ui| {
                        // Build header text with sort indicator
                        let header_display = if *sort_column == Some(col_idx) {
                            if *sort_ascending {
                                format!("{} ^", header_text)
                            } else {
                                format!("{} v", header_text)
                            }
                        } else {
                            header_text.to_string()
                        };

                        let response = ui.add(
                            egui::Label::new(header_display)
                                .wrap(false)
                                .sense(egui::Sense::click()),
                        );

                        // Change cursor to pointer on hover
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        if response.clicked() {
                            if *sort_column == Some(col_idx) {
                                // Cycle: descending -> ascending -> off
                                if *sort_ascending {
                                    // Was ascending, now turn off
                                    *sort_column = None;
                                } else {
                                    // Was descending, now ascending
                                    *sort_ascending = true;
                                    sort_muons_in_place(muons, *sort_column, *sort_ascending);
                                }
                            } else {
                                // New column: start with descending
                                *sort_column = Some(col_idx);
                                *sort_ascending = false;
                                sort_muons_in_place(muons, *sort_column, *sort_ascending);
                            }
                        }
                    });
                }
            })
            .body(|body| {
                body.rows(18.0, muons.len(), |mut row| {
                    let muon = &muons[row.index()];

                    row.col(|ui| {
                        ui.add(egui::Label::new(muon.zenith.to_string()).wrap(false));
                    });
                    row.col(|ui| {
                        ui.add(egui::Label::new(muon.abs_angle_primary.to_string()).wrap(false));
                    });
                    row.col(|ui| {
                        ui.add(egui::Label::new(muon.azimuth.to_string()).wrap(false));
                    });
                    row.col(|ui| {
                        ui.add(egui::Label::new(muon.azimuth_offset.to_string()).wrap(false));
                    });
                    row.col(|ui| {
                        ui.add(egui::Label::new(muon.total_energy.to_string()).wrap(false));
                    });
                    row.col(|ui| {
                        ui.add(egui::Label::new(muon.size.to_string()).wrap(false));
                    });
                    row.col(|ui| {
                        ui.add(egui::Label::new(muon.let_avg.to_string()).wrap(false));
                    });
                    row.col(|ui| {
                        ui.add(egui::Label::new(muon.timestamp.to_string()).wrap(false));
                    });
                    row.col(|ui| {
                        ui.add(egui::Label::new(muon.frame_index.to_string()).wrap(false));
                    });
                    row.col(|ui| {
                        ui.add(egui::Label::new(muon.file.to_string_lossy()).wrap(false));
                    });
                });
            });
    });
}

fn sort_muons_in_place(muons: &mut [Muon], column: Option<usize>, ascending: bool) {
    if muons.is_empty() {
        return;
    }

    if let Some(col) = column {
        match col {
            0 => {
                if ascending {
                    muons.sort_by(|a, b| compare_f32(a.zenith, b.zenith));
                } else {
                    muons.sort_by(|a, b| compare_f32_desc(a.zenith, b.zenith));
                }
            }
            1 => {
                if ascending {
                    muons.sort_by(|a, b| compare_f32(a.abs_angle_primary, b.abs_angle_primary));
                } else {
                    muons
                        .sort_by(|a, b| compare_f32_desc(a.abs_angle_primary, b.abs_angle_primary));
                }
            }
            2 => {
                if ascending {
                    muons.sort_by(|a, b| compare_f32(a.azimuth, b.azimuth));
                } else {
                    muons.sort_by(|a, b| compare_f32_desc(a.azimuth, b.azimuth));
                }
            }
            3 => {
                if ascending {
                    muons.sort_by(|a, b| compare_f32(a.azimuth_offset, b.azimuth_offset));
                } else {
                    muons.sort_by(|a, b| compare_f32_desc(a.azimuth_offset, b.azimuth_offset));
                }
            }
            4 => {
                if ascending {
                    muons.sort_by(|a, b| compare_f32(a.total_energy, b.total_energy));
                } else {
                    muons.sort_by(|a, b| compare_f32_desc(a.total_energy, b.total_energy));
                }
            }
            5 => {
                if ascending {
                    muons.sort_by(|a, b| a.size.cmp(&b.size));
                } else {
                    muons.sort_by(|a, b| b.size.cmp(&a.size));
                }
            }
            6 => {
                if ascending {
                    muons.sort_by(|a, b| compare_f32(a.let_avg, b.let_avg));
                } else {
                    muons.sort_by(|a, b| compare_f32_desc(a.let_avg, b.let_avg));
                }
            }
            7 => {
                if ascending {
                    muons.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                } else {
                    muons.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                }
            }
            8 => {
                if ascending {
                    muons.sort_by(|a, b| a.frame_index.cmp(&b.frame_index));
                } else {
                    muons.sort_by(|a, b| b.frame_index.cmp(&a.frame_index));
                }
            }
            9 => {
                if ascending {
                    muons.sort_by(|a, b| a.file.cmp(&b.file));
                } else {
                    muons.sort_by(|a, b| b.file.cmp(&a.file));
                }
            }
            _ => {}
        }
    }
}

fn compare_f32(a: f32, b: f32) -> std::cmp::Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        (false, false) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
    }
}

fn compare_f32_desc(a: f32, b: f32) -> std::cmp::Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        (false, false) => b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal),
    }
}
