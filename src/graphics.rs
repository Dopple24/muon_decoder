use crate::{
    Langs,
    decoder::{PartType, Particle},
    file_reader::{Tracks, list_dir},
    particle_extractor::{self},
};
use chrono::Utc;
use eframe::egui::{self, ColorImage};
use egui::CursorIcon;
use rayon::prelude::*;
use rfd::FileDialog;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, io::Write, path::PathBuf};

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
    fn into_readable(self, texts: &crate::Texts) -> String {
        match self {
            Self::North => texts.north.to_string(),
            Self::South => texts.south.to_string(),
            Self::West => texts.west.to_string(),
            Self::East => texts.east.to_string(),
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
    easter_egg_on: bool,
    //config from config.env
    config: crate::Config,

    texts: crate::Texts,
    current_lang: Langs,
    selected_lang: Langs,

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

    show_straight_track: bool,
    show_curly_track: bool,
    show_dot: bool,
    show_heavy_blob: bool,
    show_heavy_track: bool,
    show_int_straight_track: bool,
    show_unknown: bool,
    show_too_short_muon: bool,

    show_dialog: bool,
    input_depth: String,
    input_min_muon_size: String,
    pub pixel_depth: i32,
    pub pixel_width: f32,
    pub min_muon_size: usize,
    pub selected_mode: Orientation,
    renderer_3d: crate::renderer::Renderer3D,

    // Sorting state for muon grid
    muon_sort_column: Option<usize>,
    muon_sort_ascending: bool,

    // Sorting state for sus_muon grid
    sus_muon_sort_column: Option<usize>,
    sus_muon_sort_ascending: bool,

    loading: bool,
}

impl MatrixApp {
    pub fn new(
        tracks: Vec<Particle>,
        scale: usize,
        config: &crate::Config,
        texts: &crate::Texts,
        lang: &Langs,
        easter_egg_on: bool,
    ) -> Self {
        let mut app = Self {
            easter_egg_on,
            config: config.clone(),
            texts: texts.clone(),
            current_lang: lang.clone(),
            selected_lang: lang.clone(),
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
            show_heavy_blob: true,
            show_curly_track: true,
            show_dot: true,
            show_straight_track: true,
            show_heavy_track: true,
            show_unknown: true,
            show_too_short_muon: true,
            show_int_straight_track: true,
            show_dialog: false,
            input_min_muon_size: config.default_min_muon_size.to_string(),
            input_depth: config.default_pixel_depth.to_string(),
            pixel_depth: config.default_pixel_depth as i32,
            pixel_width: config.default_pixel_width,
            selected_mode: Orientation::North,
            min_muon_size: config.default_min_muon_size,
            renderer_3d: crate::renderer::Renderer3D::new(),
            muon_sort_column: None,
            muon_sort_ascending: true,
            sus_muon_sort_column: None,
            sus_muon_sort_ascending: true,
            loading: false,
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
            (self.show_heavy_blob, PartType::HeavyBlob),
            (self.show_curly_track, PartType::CurlyTrack),
            (self.show_dot, PartType::Dot),
            (self.show_straight_track, PartType::StraightTrack),
            (self.show_int_straight_track, PartType::IntStraightTrack),
            (self.show_heavy_track, PartType::HeavyTrack),
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
                        .particle_type(&self.matricees[self.current_file].get_tracks()[matrix_idx].matrix, &self.min_muon_size, &self.config.default_min_muon_size) //has panicked - out of bounds exception self.current_matrix index 95, len 46
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
                let mut buffer = vec![vec![0; self.config.size]; self.config.size];
                let particles =
                    particle_extractor::extract(&p.matrix, &mut buffer, 2, self.config.size)
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
                        &self.min_muon_size,
                        &self.config.default_min_muon_size,
                    );
                    if part_type == PartType::StraightTrack {
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
                    } else if part_type == PartType::IntStraightTrack {
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
            &mut vec![vec![0; self.config.size]; self.config.size],
            2,
            self.config.size,
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

        if self.loading {
            ctx.output_mut(|o| o.cursor_icon = CursorIcon::Wait);
        } else {
            ctx.output_mut(|o| o.cursor_icon = CursorIcon::Default);
        }

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
                ui.heading(&self.texts.title);

                ui.separator();

                if ui.button(&self.texts.arrow_left).clicked() {
                    self.move_data_back();
                }

                if ui.button(&self.texts.arrow_right).clicked() {
                    self.move_data();
                }

                if ui.button(&self.texts.single_mode).clicked() {
                    self.current_mode = Mode::Single;
                    self.current_track =
                        self.current_track.min(self.tracks_to_draw.len().max(1) - 1);
                    self.update_image();
                }

                if ui.button(&self.texts.cubic_view).clicked() {
                    self.renderer_3d.toggle_window();
                }

                if ui.button(&self.texts.combined_mode).clicked() {
                    self.current_mode = Mode::Combined;
                    self.update_image();
                }

                if ui.button(&self.texts.compound_mode).clicked() {
                    self.current_mode = Mode::Compound;
                    self.update_image();
                }

                ui.separator();

                ui.label(match self.current_mode {
                    Mode::Single => format!("{}: {}", &self.texts.mode, &self.texts.single_mode),
                    Mode::Combined => {
                        format!("{}: {}", &self.texts.mode, &self.texts.combined_mode)
                    }
                    Mode::Compound => {
                        format!("{}: {}", &self.texts.mode, &self.texts.compound_mode)
                    }
                });

                egui::ComboBox::from_id_source("lang_selector")
                    .selected_text(self.selected_lang.to_readable())
                    .show_ui(ui, |ui| {
                        Langs::list(self.easter_egg_on).iter().for_each(|lang| {
                            ui.selectable_value(
                                &mut self.selected_lang,
                                lang.clone(),
                                lang.to_readable(),
                            );
                        });
                    });
                if self.selected_lang != self.current_lang {
                    self.texts = crate::Langs::change_lang(&self.selected_lang);
                    self.current_lang = self.selected_lang.clone();
                }
            });
        });

        // ============================
        // LEFT PANEL — STATS
        // ============================
        egui::SidePanel::left("stats")
            .resizable(false)
            .min_width(100.0)
            .show(ctx, |ui| {
                ui.heading(&self.texts.particles);

                let mut count = HashMap::new();
                for p in [
                    PartType::HeavyBlob,
                    PartType::CurlyTrack,
                    PartType::Dot,
                    PartType::StraightTrack,
                    PartType::IntStraightTrack,
                    PartType::HeavyTrack,
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
                                &self.min_muon_size,
                                &self.config.default_min_muon_size,
                            ),
                        )
                        .unwrap() += 1;
                }

                egui::Grid::new("stats_grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        for (label, ty) in [
                            (&self.texts.heavy_blob, PartType::HeavyBlob),
                            (&self.texts.curly_track, PartType::CurlyTrack),
                            (&self.texts.dot, PartType::Dot),
                            (&self.texts.straight_track, PartType::StraightTrack),
                            (&self.texts.int_straight_track, PartType::IntStraightTrack),
                            (&self.texts.heavy_track, PartType::HeavyTrack),
                            (&self.texts.unknown, PartType::Unknown),
                            (&self.texts.too_short_muon, PartType::TooShortMuon),
                        ] {
                            ui.label(label);
                            ui.label(count.get(&ty).unwrap().to_string());
                            ui.end_row();
                        }
                    });

                let response_al = ui.checkbox(&mut self.show_heavy_blob, &self.texts.heavy_blob);
                let response_be = ui.checkbox(&mut self.show_curly_track, &self.texts.curly_track);
                let response_ga = ui.checkbox(&mut self.show_dot, &self.texts.dot);
                let response_mu =
                    ui.checkbox(&mut self.show_straight_track, &self.texts.straight_track);
                let response_is = ui.checkbox(
                    &mut self.show_int_straight_track,
                    &self.texts.int_straight_track,
                );
                let response_sm = ui.checkbox(&mut self.show_heavy_track, &self.texts.heavy_track);
                let response_un = ui.checkbox(&mut self.show_unknown, &self.texts.unknown);
                let response_sh =
                    ui.checkbox(&mut self.show_too_short_muon, &self.texts.too_short_muon);

                if response_al.changed()
                    || response_be.changed()
                    || response_ga.changed()
                    || response_mu.changed()
                    || response_sm.changed()
                    || response_sh.changed()
                    || response_un.changed()
                    || response_is.changed()
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
                if ui.button(&self.texts.export_all).clicked() {
                    let mut all_files_muons = Vec::new();
                    for i in 0..self.matricees.len() {
                        self.current_file = i;
                        self.init_compound_mode();
                        all_files_muons.extend(std::mem::take(&mut self.muons));
                        all_files_muons.extend(std::mem::take(&mut self.sus_muons));
                    }

                    let csv = build_csv(&all_files_muons, &self.texts);
                    if let Err(e) = export_csv(&csv, &self.texts) {
                        self.error = Some(e);
                    }
                }
                egui::ScrollArea::horizontal()
                    .id_source("muons_scroll")
                    .show(ui, |ui| {
                        ui.heading(&self.texts.muon_section_header);
                        if ui.button(&self.texts.export).clicked() {
                            let csv = build_csv(&self.muons, &self.texts);

                            if let Err(e) = export_csv(&csv, &self.texts) {
                                self.error = Some(e);
                            }
                        }

                        show_muon_grid(
                            ui,
                            "muon_grid",
                            &mut self.muons,
                            &mut self.muon_sort_column,
                            &mut self.muon_sort_ascending,
                            &self.texts,
                        );
                        ui.heading(&self.texts.sus_muon_section_header);
                        if ui.button(&self.texts.export).clicked() {
                            let csv = build_csv(&self.sus_muons, &self.texts);

                            if let Err(e) = export_csv(&csv, &self.texts) {
                                self.error = Some(e);
                            }
                        }

                        show_muon_grid(
                            ui,
                            "sus_grid",
                            &mut self.sus_muons,
                            &mut self.sus_muon_sort_column,
                            &mut self.sus_muon_sort_ascending,
                            &self.texts,
                        );
                    });
            });

        // ============================
        // BOTTOM BAR
        // ============================
        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(&self.texts.import).clicked() {
                    self.show_dialog = true;
                }
                if self.show_dialog {
                    egui::Window::new(&self.texts.import_dialog_title)
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading(&self.texts.import_dialog_dimensions);
                            });

                            ui.add_space(10.0);

                            ui.horizontal(|ui| {
                                ui.label(format!("{}:", &self.texts.import_dialog_depth));
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.input_depth)
                                        .hint_text(self.config.default_pixel_depth.to_string())
                                        .desired_width(100.0),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label(format!("{}:", &self.texts.import_dialog_min_muon_size));
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.input_min_muon_size)
                                        .hint_text(self.config.default_min_muon_size.to_string())
                                        .desired_width(100.0),
                                );
                            });

                            ui.add_space(10.0);

                            ui.label(format!("{}:", &self.texts.import_dialog_compass));

                            egui::ComboBox::from_id_source("mode_selector")
                                .selected_text(self.selected_mode.into_readable(&self.texts))
                                .show_ui(ui, |ui| {
                                    Orientation::North
                                        .all_values()
                                        .iter()
                                        .for_each(|direction| {
                                            ui.selectable_value(
                                                &mut self.selected_mode,
                                                *direction,
                                                direction.into_readable(&self.texts),
                                            );
                                        });
                                });

                            ui.add_space(15.0);

                            ui.horizontal(|ui| {
                                if ui.button(&self.texts.import_dialog_confirm).clicked()
                                    && let Ok(depth) = self.input_depth.parse::<i32>()
                                    && let Ok(min_muon_size) =
                                        self.input_min_muon_size.parse::<usize>()
                                {
                                    self.loading = true;
                                    self.pixel_depth = depth;
                                    self.min_muon_size = min_muon_size.max(4);
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
                                                self.config.size,
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
                                    self.loading = false;
                                }

                                if ui.button(&self.texts.import_dialog_cancel).clicked() {
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
                    "{} {}/{}\n {}: {}",
                    self.texts.tracks,
                    self.current_track + 1,
                    self.tracks_to_draw.len(),
                    self.texts.file_location,
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
                        "{}: {:?}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {:?}\n{}: {}",
                        &self.texts.particle_type_label,
                        selected_track.particle_type(&self.matricees[self.current_file].get_tracks()[self.current_matrix].matrix, &self.min_muon_size, &self.config.default_min_muon_size),
                        &self.texts.size_label,
                        selected_track.size(),
                        &self.texts.avg_energy_label,
                        selected_track.avg_energy(&self.matricees[self.current_file].get_tracks()[self.current_matrix].matrix),
                        &self.texts.let_avg_label,
                        selected_track.let_avg(&self.matricees[self.current_file].get_tracks()[self.current_matrix].matrix),
                        &self.texts.total_energy_label,
                        selected_track.total_energy(&self.matricees[self.current_file].get_tracks()[self.current_matrix].matrix),
                        &self.texts.azimuth_label,
                        selected_track.azimuth(),
                        &self.texts.azimuth_offset_label,
                        selected_track.azimuth_offset(),
                        &self.texts.abs_angle_primary_label,
                        selected_track.abs_angle_primary(),
                        &self.texts.zenith_label,
                        selected_track.zenith(),
                        &self.texts.winding_label,
                        selected_track.winding(),
                        &self.texts.get_frame_index_label,
                        selected_track.get_frame_index() + 1,
                        &self.texts.get_timestamp_label,
                        selected_track.get_timestamp(),
                    ));
                }

                else if self.current_mode == Mode::Combined && !self.tracks_to_draw.is_empty(){
                    ui.label(format!(
                        "{}: {}",
                        &self.texts.get_frame_index_label,
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
                    if ui.button(&self.texts.import_dialog_confirm).clicked() {
                        self.error = None;
                    }
                });
        }
    }
}
fn build_csv(muons: &[Muon], texts: &crate::Texts) -> String {
    let mut content = String::new();

    content.push_str(&format!(
        "{},{},{},{},{},{},{},{},{},{}\n",
        &texts.muon_list_zenith,
        &texts.muon_list_abs_angle_primary,
        &texts.muon_list_azimuth,
        &texts.muon_list_azimuth_offset,
        &texts.muon_list_total_energy,
        &texts.muon_list_size,
        &texts.muon_list_let_avg,
        &texts.muon_list_timestamp,
        &texts.muon_list_frame_index,
        &texts.muon_list_file,
    ));

    for muon in muons {
        content.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            muon.zenith,
            muon.abs_angle_primary,
            muon.azimuth,
            muon.azimuth_offset,
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
fn export_csv(content: &str, texts: &crate::Texts) -> Result<(), String> {
    if let Some(path) = FileDialog::new()
        .set_title(&texts.save_csv)
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
    texts: &crate::Texts,
) {
    ui.push_id(id, |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .columns(Column::auto(), 10)
            .header(20.0, |mut header| {
                let headers = [
                    &texts.muon_list_zenith,
                    &texts.muon_list_abs_angle_primary,
                    &texts.muon_list_azimuth,
                    &texts.muon_list_azimuth_offset,
                    &texts.muon_list_total_energy,
                    &texts.muon_list_size,
                    &texts.muon_list_let_avg,
                    &texts.muon_list_timestamp,
                    &texts.muon_list_frame_index,
                    &texts.muon_list_file,
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
