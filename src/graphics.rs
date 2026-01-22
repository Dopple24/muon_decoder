use crate::{decoder::{PartType, Particle}, file_reader::{Tracks, list_dir}, particle_extractor};
use eframe::egui::{self, ColorImage};
use rfd::FileDialog;
use std::{collections::HashMap, path::PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Mode {
    Single,
    Combined,
    Compound,
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
            matricees: vec![Tracks {tracks: matricees, file_path: PathBuf::new()}],
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
        if self.current_mode == Mode::Compound {
            self.init_compound_mode();
        }
        else if self.current_mode == Mode::Combined {
            self.init_combined();
        }
        self.update_counter();
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
            Mode::Compound => {
                    self.init_compound_mode();
                    self.tracks_to_draw.iter().map(|p| p.get_track()).collect()
                },
        };

        println!("all_tracks length: {}", self.all_tracks.len());
        println!("priv tracks_to_draw length: {}", tracks_to_draw.len());
        println!("current_mode: {:?}", self.current_mode);

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
        println!("tracks_to-draw_length{}", self.tracks_to_draw.len());
    }

    fn move_track(&mut self) {
        self.current_track = (self.current_track + 1) % self.tracks_to_draw.len();
    }

    fn move_track_back(&mut self) {
        self.current_track = if self.current_track == 0 {
            self.tracks_to_draw.len() - 1
        } else {
            self.current_track - 1
        };
    }

    fn matrix_move(&mut self) {
        if self.current_matrix >= self.matricees[self.current_file].tracks.len() - 1 {
            self.current_file = (self.current_file + 1) % self.matricees.len();
            self.current_matrix = 0;
        }
        else {
            self.current_matrix = (self.current_matrix + 1) % self.matricees[self.current_file].tracks.len();
        }
    }

    fn matrix_move_back(&mut self) {
        self.current_matrix = if self.current_matrix == 0 {
            self.current_file = if self.current_file == 0 {
                self.matricees.len() - 1
            }
            else {
                self.current_file - 1
            };
            self.matricees[self.current_file].tracks.len() - 1
        } else {
            self.current_matrix - 1
        };
    }

    fn move_file(&mut self) {
        self.current_file = (self.current_file + 1) % self.matricees.len();
    }

    fn move_file_back(&mut self) {
        self.current_file = if self.current_file == 0 {
                self.matricees.len() - 1
            }
            else {
                self.current_file - 1
            };
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
        self.all_tracks = Vec::new();
        let _mat: Vec<_> = self.matricees[self.current_file]
            .tracks
            .iter()
            .map(|p| {
                let mut buffer = vec![vec![0; crate::SIZE]; crate::SIZE];
                let particles = particle_extractor::extract(p, &mut buffer, 2)
                    .into_values()
                    .collect::<Vec<_>>();

                for (index, part) in particles.iter().enumerate() {
                    let particle = Particle::new(part.clone());
                    let part_type = particle.particle_type(p);
                    println!("particle type {:?} was assigned, index: {}", part_type, index);
                    self.all_tracks.push(particle);                           
                }
                particles
            })
            .flatten()
            .collect();
    }

    fn init_combined(&mut self) {
        self.all_tracks =
            crate::particle_extractor::extract(&self.matricees[self.current_file].tracks[self.current_matrix], &mut vec![vec![0; crate::SIZE]; crate::SIZE], 2)
                .values()
                .map(|t| crate::decoder::Particle::new(t.clone()))
                .collect();
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
        {
            self.move_data();            
        }

        if ctx.input(|i| i.key_pressed(Key::ArrowLeft))
            && !self.tracks_to_draw.is_empty()
        {
            self.move_data_back();
        }

        if ctx.input(|i| i.key_pressed(Key::M)) {
            if !self.tracks_to_draw.is_empty() {
                self.current_mode = Mode::Single;
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

                if ui.button("◀ Prev").clicked(){
                    self.move_data_back();
                }

                if ui.button("Next ▶").clicked(){
                    self.move_data();
                }

                if ui.button("Single").clicked() {
                    if !self.tracks_to_draw.is_empty() {
                        self.current_mode = Mode::Single;
                        self.current_track = self.current_track.max(self.tracks_to_draw.len() - 1);
                        self.update_image();
                    }
                }

                if ui.button("Combined").clicked() {
                    if !self.tracks_to_draw.is_empty() {
                        self.current_mode = Mode::Combined;
                        self.update_image();
                    }
                }


                if ui.button("Compound").clicked() {
                    if !self.tracks_to_draw.is_empty() {
                        self.current_mode = Mode::Compound;
                        self.update_image();
                    }
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
                    self.update_image();
                    self.update_counter();
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
                    "Track {}/{}\n file: {:?}",
                    self.current_track + 1,
                    self.tracks_to_draw.len(),
                    self.matricees[self.current_file].file_path
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
                        self.current_track = 0;
                        self.matricees = mat;
                        let mut id_map = vec![vec![0; crate::SIZE]; crate::SIZE];
                        self.all_tracks =
                            crate::particle_extractor::extract(&self.matricees[self.current_matrix].tracks[self.current_matrix], &mut id_map, 2)
                                .values()
                                .map(|t| crate::decoder::Particle::new(t.clone()))
                                .collect();
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
