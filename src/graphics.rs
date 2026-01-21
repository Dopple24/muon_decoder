use crate::{decoder::{PartType, Particle}, file_reader::{list_dir, Tracks}};
use eframe::egui::{self, ColorImage};
use rfd::FileDialog;
use std::{collections::HashMap, path::PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Mode {
    Single,
    Combined,
}

impl Mode {
    fn toggle(&self) -> Self {
        match self {
            Mode::Single => Mode::Combined,
            Mode::Combined => Mode::Single,
        }
    }
}

#[derive(Debug)]
pub struct MatrixApp {
    matricees: Vec<Tracks>,
    current_file: usize,
    current_matrix: usize,
    all_tracks: Vec<Particle>,
    tracks_to_draw: Vec<Particle>,
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
    show_unknown: bool,
}

impl MatrixApp {
    pub fn new(matricees: Vec<Vec<Vec<f32>>>, tracks: Vec<Particle>, scale: usize) -> Self {
        let mut app = Self {
            matricees: vec![Tracks {tracks:matricees, file_path: PathBuf::new()}],
            current_file: 0,
            current_matrix: 0,
            all_tracks: tracks.clone(),
            tracks_to_draw: tracks,
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
            show_unknown: true,
        };
        app.update_image();
        app
    }

    /// Update the image for current track or combined tracks
    fn update_image(&mut self) {

        println!("number of tracks: {}", self.matricees[self.current_file].tracks.len());
        let size_x = self.matricees[self.current_file].tracks[self.current_matrix].len();
        let size_y = self.matricees[self.current_file].tracks[self.current_matrix][0].len();
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
            Mode::Combined => self.tracks_to_draw.iter().map(|p| p.get_track()).collect(),
        };

        for track_cells in tracks_to_draw {
            let color = egui::Color32::WHITE;
            for (x, y) in track_cells {
                for dx in 0..self.scale {
                    for dy in 0..self.scale {
                        let px = x * self.scale + dx;
                        let py = y * self.scale + dy;
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
    }
    fn update_counter(&mut self) {
        let filters = [
            (self.show_alpha, PartType::ALPHA),
            (self.show_beta, PartType::BETA),
            (self.show_gamma, PartType::GAMMA),
            (self.show_muon, PartType::MUON),
            (self.show_unknown, PartType::UNKNOWN),
        ];

        self.tracks_to_draw.clear();

        for track in &self.all_tracks {
            if filters
                .iter()
                .any(|(show, ty)| *show && track.particle_type(&self.matricees[self.current_file].tracks[self.current_matrix]) == *ty)
            {
                self.tracks_to_draw.push(track.clone());
            }
        }
    }
}

impl eframe::App for MatrixApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        use egui::Key;

        // ----------------------------
        // Input handling
        // ----------------------------
        if ctx.input(|i| i.key_pressed(Key::ArrowRight))
            && !self.tracks_to_draw.is_empty()
            && self.current_mode == Mode::Single
        {
            self.current_track = (self.current_track + 1) % self.tracks_to_draw.len();
            self.needs_update = true;
        }

        if ctx.input(|i| i.key_pressed(Key::ArrowLeft))
            && !self.tracks_to_draw.is_empty()
            && self.current_mode == Mode::Single
        {
            self.current_track = if self.current_track == 0 {
                self.tracks_to_draw.len() - 1
            } else {
                self.current_track - 1
            };
            self.needs_update = true;
        }

        if ctx.input(|i| i.key_pressed(Key::M)) {
            if !self.tracks_to_draw.is_empty() {
                self.current_mode = self.current_mode.toggle();
                self.needs_update = true;
            }
        }

        if self.needs_update {
            self.update_image();
            self.needs_update = false;
        }

        // ============================
        // TOP BAR
        // ============================
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Particle Matrix Viewer");

                ui.separator();

                if ui.button("◀ Prev").clicked() && self.current_mode == Mode::Single {
                    self.current_track = if self.current_track == 0 {
                        self.tracks_to_draw.len() - 1
                    } else {
                        self.current_track - 1
                    };
                    self.update_image();
                }

                if ui.button("Next ▶").clicked() && self.current_mode == Mode::Single {
                    self.current_track = (self.current_track + 1) % self.tracks_to_draw.len();
                    self.update_image();
                }

                if ui.button("Toggle Mode").clicked() {
                    if !self.tracks_to_draw.is_empty() {
                        self.current_mode = self.current_mode.toggle();
                        self.update_image();
                    }
                }

                if ui.button("◀ Prev file").clicked() && self.current_mode == Mode::Combined { 
                    todo!(); //not working
                    self.current_matrix = if self.current_matrix == 0 {
                        self.matricees.len() - 1
                    } else {
                        self.current_matrix - 1
                    };
                    println!("{}", self.current_matrix);
                    self.all_tracks =
                            crate::particle_extractor::extract(&self.matricees[self.current_file].tracks[self.current_matrix], &mut vec![vec![0; crate::SIZE]; crate::SIZE], 2)
                                .values()
                                .map(|t| crate::decoder::Particle::new(t.clone()))
                                .collect();
                    self.update_counter();
                    self.update_image();
                }

                if ui.button("Next file ▶").clicked() && self.current_mode == Mode::Combined {
                    if self.current_matrix >= self.matricees[self.current_file].tracks.len() - 1 {
                        self.current_file = (self.current_file + 1) % self.matricees.len();
                        self.current_matrix = 0;
                    }
                    else {
                        self.current_matrix = (self.current_matrix + 1) % self.matricees[self.current_file].tracks.len();
                    }
                    println!("{}", (self.current_file + 1) % self.matricees.len());
                    println!("{}/{},{},{}", self.current_file, self.matricees.len(), self.current_matrix, self.current_track);
                    self.all_tracks =
                            crate::particle_extractor::extract(&self.matricees[self.current_file].tracks[self.current_matrix], &mut vec![vec![0; crate::SIZE]; crate::SIZE], 2)
                                .values()
                                .map(|t| crate::decoder::Particle::new(t.clone()))
                                .collect();
                    self.update_counter();
                    self.update_image();
                }

                ui.separator();

                ui.label(match self.current_mode {
                    Mode::Single => "Mode: Single Track",
                    Mode::Combined => "Mode: Combined",
                });
            });
        });

        // ============================
        // LEFT PANEL — STATS
        // ============================
        egui::SidePanel::left("stats")
            .resizable(false)
            .min_width(180.0)
            .show(ctx, |ui| {
                ui.heading("📊 Particles");

                let mut count = HashMap::new();
                for p in [
                    PartType::ALPHA,
                    PartType::BETA,
                    PartType::GAMMA,
                    PartType::MUON,
                    PartType::UNKNOWN,
                ] {
                    count.insert(p, 0usize);
                }

                for particle in &self.tracks_to_draw {
                    *count
                        .get_mut(&particle.particle_type(&self.matricees[self.current_file].tracks[self.current_matrix]))
                        .unwrap() += 1;
                }

                egui::Grid::new("stats_grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        for (label, ty) in [
                            ("Alpha", PartType::ALPHA),
                            ("Beta", PartType::BETA),
                            ("Gamma", PartType::GAMMA),
                            ("Muon", PartType::MUON),
                            ("Unknown", PartType::UNKNOWN),
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
                let response_un = ui.checkbox(&mut self.show_unknown, "Unknown");

                if response_al.changed()
                    || response_be.changed()
                    || response_ga.changed()
                    || response_mu.changed()
                    || response_un.changed()
                {
                    self.update_counter();
                    self.update_image();
                }
            });

        // ============================
        // CENTER VIEW
        // ============================
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let texture =
                    ui.ctx()
                        .load_texture("track_image", self.image.clone(), Default::default());

                ui.image(&texture);

                ui.add_space(8.0);

                ui.label(format!(
                    "Track {}/{}",
                    self.current_track + 1,
                    self.tracks_to_draw.len()
                ));

                if self.current_mode == Mode::Single {
                    if self.current_track >= self.tracks_to_draw.len() {
                        self.current_track = self.tracks_to_draw.len() - 1;
                    }
                    let selected_track = &self.tracks_to_draw[self.current_track];
                    ui.label(format!(
                        "Particle: {:?}\nmax energy: {}\ntrack length: {}\naverage energy: {}\ntotal energy: {}\nslope: {}\nwinding: {}\nfilename: {:?}",
                        selected_track.particle_type(&self.matricees[self.current_file].tracks[self.current_matrix]),
                        selected_track.max_energy(&self.matricees[self.current_file].tracks[self.current_matrix]),
                        selected_track.size(),
                        selected_track.avg_energy(&self.matricees[self.current_file].tracks[self.current_matrix]),
                        selected_track.total_energy(&self.matricees[self.current_file].tracks[self.current_matrix]),
                        selected_track.slope(),
                        selected_track.winding(),
                        self.matricees[self.current_file].file_path
                    ));
                }
            });
        });

        // ============================
        // BOTTOM BAR
        // ============================
        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 Open File").clicked()
                    && let Some(path) = FileDialog::new().pick_folder()
                {
                    if let Ok(mat) = list_dir(&path) {
                        for m in &mat {
                            println!("{}", m.tracks.len());
                        }
                        self.current_track = 0;
                        self.matricees = mat;
                        let mut id_map = vec![vec![0; crate::SIZE]; crate::SIZE];
                        self.all_tracks =
                            crate::particle_extractor::extract(&self.matricees[self.current_matrix].tracks[self.current_matrix], &mut id_map, 2)
                                .values()
                                .map(|t| crate::decoder::Particle::new(t.clone()))
                                .collect();
                        self.update_counter();
                        self.update_image();
                    } else {
                        self.error = Some("error".to_string());
                    }
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
                    ui.label("File was incorrectly formatted.");
                    ui.add_space(10.0);
                    if ui.button("OK").clicked() {
                        self.error = None;
                    }
                });
        }
    }
}
