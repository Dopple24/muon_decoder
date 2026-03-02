use crate::{
    decoder::{PartType, Particle},
    file_reader::{Tracks, list_dir},
    particle_extractor::{self},
};
use eframe::egui::{self, ColorImage};
use rfd::FileDialog;
use std::{collections::HashMap, io::Write, path::PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Mode {
    Single,
    Combined,
    Compound,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Orientation {
    NorthSouth,
    WestEast,
}
impl Orientation {
    fn into_readable(&self) -> String {
        match self {
            Self::NorthSouth => "North - South".to_string(),
            Self::WestEast => "West - East".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct Muon {
    file: PathBuf,
    frame_index: usize,
    total_energy: f32,
    north_south_angle: f32,
    abs_angle_primary: f32,
    west_east_angle: f32,
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

    show_dialog: bool,
    input_depth: String,
    input_width: String,
    pixel_depth: Option<i32>,
    pixel_width: Option<i32>,
    selected_mode: Orientation,
}

impl MatrixApp {
    pub fn new(matricees: Vec<Vec<Vec<f32>>>, tracks: Vec<Particle>, scale: usize) -> Self {
        let mut app = Self {
            matricees: vec![Tracks {
                tracks: matricees,
                file_path: PathBuf::new(),
            }],
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
            show_dialog: false,
            input_depth: "30".to_string(),
            input_width: "30".to_string(),
            pixel_depth: None,
            pixel_width: None,
            selected_mode: Orientation::NorthSouth,
        };
        app.update_image();
        app
    }

    /// Update the image for current track or combined tracks
    fn update_image(&mut self) {
        if self.current_mode == Mode::Compound {
            self.init_compound_mode();
        } else if self.current_mode == Mode::Combined {
            self.init_combined();
        }
        self.update_counter();
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
            }
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
            (self.show_alpha, PartType::Alpha),
            (self.show_beta, PartType::Beta),
            (self.show_gamma, PartType::Gamma),
            (self.show_muon, PartType::Muon),
            (self.show_sus_muon, PartType::SusMuon),
            (self.show_unknown, PartType::Unknown),
        ];

        self.tracks_to_draw.clear();

        for track in &self.all_tracks {
            if filters.iter().any(|(show, ty)| {
                *show
                    && track.particle_type(
                        &self.matricees[self.current_file].tracks[self.current_matrix],
                    ) == *ty
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
        if self.current_matrix >= self.matricees[self.current_file].tracks.len().max(1) - 1 {
            self.current_file = (self.current_file + 1) % self.matricees.len();
            self.current_matrix = 0;
        } else {
            self.current_matrix =
                (self.current_matrix + 1) % self.matricees[self.current_file].tracks.len().max(1);
        }
    }

    fn matrix_move_back(&mut self) {
        self.current_track = 0;
        self.current_matrix = if self.current_matrix == 0 {
            self.current_file = if self.current_file == 0 {
                self.matricees.len().max(1) - 1
            } else {
                self.current_file - 1
            };
            self.matricees[self.current_file].tracks.len().max(1) - 1
        } else {
            self.current_matrix - 1
        };
    }

    fn move_file(&mut self) {
        self.current_track = 0;
        self.current_file = (self.current_file + 1) % self.matricees.len().max(1);
    }

    fn move_file_back(&mut self) {
        self.current_track = 0;
        self.current_file = if self.current_file == 0 {
            self.matricees.len().max(1) - 1
        } else {
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
        self.muons.clear();
        self.sus_muons.clear();
        let _mat: Vec<_> = self.matricees[self.current_file]
            .tracks
            .iter()
            .enumerate()
            .flat_map(|(frame_index, p)| {
                let mut buffer = vec![vec![0; crate::SIZE]; crate::SIZE];
                let particles = particle_extractor::extract(p, &mut buffer, 2)
                    .into_values()
                    .collect::<Vec<_>>();

                for part in &particles {
                    let particle = Particle::new(
                        part.clone(),
                        frame_index,
                        self.pixel_depth,
                        self.pixel_width,
                        self.selected_mode,
                    );
                    let part_type = particle.particle_type(p);
                    if part_type == PartType::Muon {
                        self.muons.push(Muon {
                            file: self.matricees[self.current_file].file_path.clone(),
                            frame_index,
                            total_energy: particle.total_energy(p),
                            north_south_angle: particle.north_south_angle(),
                            abs_angle_primary: particle.abs_angle_primary(),
                            west_east_angle: particle.west_east_angle(),
                            size: particle.size(),
                            let_avg: particle.let_avg(p),
                        })
                    } else if part_type == PartType::SusMuon {
                        self.sus_muons.push(Muon {
                            file: self.matricees[self.current_file].file_path.clone(),
                            frame_index,
                            total_energy: particle.total_energy(p),
                            north_south_angle: particle.north_south_angle(),
                            abs_angle_primary: particle.abs_angle_primary(),
                            west_east_angle: particle.west_east_angle(),
                            size: particle.size(),
                            let_avg: particle.let_avg(p),
                        })
                    }
                    self.all_tracks.push(particle);
                }
                particles
            })
            .collect();
    }

    fn init_combined(&mut self) {
        self.all_tracks = crate::particle_extractor::extract(
            &self.matricees[self.current_file].tracks[self.current_matrix],
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

                if ui.button("Single").clicked() && !self.tracks_to_draw.is_empty() {
                    self.current_mode = Mode::Single;
                    self.current_track = self.current_track.min(self.tracks_to_draw.len() - 1);
                    self.update_image();
                }

                if ui.button("Combined").clicked() && !self.tracks_to_draw.is_empty() {
                    self.current_mode = Mode::Combined;
                    self.update_image();
                }

                if ui.button("Compound").clicked() && !self.tracks_to_draw.is_empty() {
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
                ] {
                    count.insert(p, 0usize);
                }

                for particle in &self.tracks_to_draw {
                    *count
                        .get_mut(&particle.particle_type(
                            &self.matricees[self.current_file].tracks[self.current_matrix],
                        ))
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
                            ("Sus muon", PartType::SusMuon),
                            ("Unknown", PartType::Unknown),
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

                if response_al.changed()
                    || response_be.changed()
                    || response_ga.changed()
                    || response_mu.changed()
                    || response_sm.changed()
                    || response_un.changed()
                {
                    self.update_image();
                    self.update_counter();
                }
            });

        // ============================
        // RIGHT PANEL — STATS
        // ============================
        egui::SidePanel::right("muon_list")
            .resizable(true)
            .min_width(150.0)
            .show(ctx, |ui| {
                egui::ScrollArea::new(true)
                    .id_source("muons_scroll")
                    .show(ui, |ui| {
                        ui.heading("📊 Muons");
                        if ui.button("export").clicked() {
                            let csv = build_csv(&self.muons);

                            if let Err(e) = export_csv(&csv) {
                                self.error = Some(e);
                            }
                        }

                        show_muon_grid(ui, &self.muons);
                        ui.heading("📊 Sus Muons");
                        if ui.button("export").clicked() {
                            let csv = build_csv(&self.sus_muons);

                            if let Err(e) = export_csv(&csv) {
                                self.error = Some(e);
                            }
                        }

                        show_muon_grid(ui, &self.sus_muons);
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

                            ui.add_space(10.0);

                            ui.label("Select mode:");

                            egui::ComboBox::from_id_source("mode_selector")
                                .selected_text(&self.selected_mode.into_readable())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.selected_mode,
                                        Orientation::NorthSouth,
                                        Orientation::NorthSouth.into_readable(),
                                    );
                                    ui.selectable_value(
                                        &mut self.selected_mode,
                                        Orientation::WestEast,
                                        Orientation::WestEast.into_readable(),
                                    );
                                });

                            ui.add_space(15.0);

                            ui.horizontal(|ui| {
                                if ui.button("OK").clicked() {
                                    if let Ok(depth) = self.input_depth.parse::<i32>()
                                        && let Ok(width) = self.input_width.parse::<i32>()
                                    {
                                        self.pixel_depth = Some(depth);
                                        self.pixel_width = Some(width);
                                        self.show_dialog = false;
                                        if let Some(path) = FileDialog::new().pick_folder() {
                                            if let Ok(mat) = list_dir(&path) {
                                                self.current_track = 0;
                                                self.matricees = mat;
                                                let mut id_map =
                                                    vec![vec![0; crate::SIZE]; crate::SIZE];
                                                self.all_tracks =
                                                    crate::particle_extractor::extract(
                                                        &self.matricees[self.current_matrix].tracks
                                                            [self.current_matrix],
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
                                                        )
                                                    })
                                                    .collect();
                                                self.update_image();
                                            } else {
                                                self.error = Some("error".to_string());
                                            }
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
                        &Particle::new(Vec::new(), 0, self.pixel_depth, self.pixel_width, self.selected_mode)
                    }
                    else {
                        &self.tracks_to_draw[self.current_track]
                    };
                    ui.label(format!(
                        "Particle: {:?}\nsize: {}\naverage energy: {}\nLET: {}\ntotal energy: {}\nNorthSouth angle: {}\n abs angle primary: {}\n WestEast angle: {}\nwinding: {}\nframe number: {:?}",
                        selected_track.particle_type(&self.matricees[self.current_file].tracks[self.current_matrix]),
                        selected_track.size(),
                        selected_track.avg_energy(&self.matricees[self.current_file].tracks[self.current_matrix]),
                        selected_track.let_avg(&self.matricees[self.current_file].tracks[self.current_matrix]),
                        selected_track.total_energy(&self.matricees[self.current_file].tracks[self.current_matrix]),
                        selected_track.north_south_angle(),
                        selected_track.abs_angle_primary(),
                        selected_track.west_east_angle(),
                        selected_track.winding(),
                        selected_track.get_frame_index(),
                    ));
                }

                else if self.current_mode == Mode::Combined && !self.tracks_to_draw.is_empty(){
                    ui.label(format!(
                        "frame number: {}",
                        &self.tracks_to_draw[self.current_track.min(self.tracks_to_draw.len())].get_frame_index()
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

    content
        .push_str("NorthSouth_angle,abs_angle,WestEast_angle,total_energy,size,LET,frame#,file\n");

    for muon in muons {
        content.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            muon.north_south_angle,
            muon.abs_angle_primary,
            muon.west_east_angle,
            muon.total_energy,
            muon.size,
            muon.let_avg,
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
fn show_muon_grid(ui: &mut egui::Ui, muons: &[Muon]) {
    egui::Grid::new(ui.next_auto_id())
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            ui.label("NorthSouth angle");
            ui.label("Abs angle - primary");
            ui.label("WestEast angle");
            ui.label("total energy");
            ui.label("size");
            ui.label("let");
            ui.label("frame #");
            ui.label("file");
            ui.end_row();

            for muon in muons {
                ui.label(muon.north_south_angle.to_string());
                ui.label(muon.abs_angle_primary.to_string());
                ui.label(muon.west_east_angle.to_string());
                ui.label(muon.total_energy.to_string());
                ui.label(muon.size.to_string());
                ui.label(muon.let_avg.to_string());
                ui.label(muon.frame_index.to_string());
                ui.label(format!("{}", muon.file.to_string_lossy()));
                ui.end_row();
            }
        });
}
