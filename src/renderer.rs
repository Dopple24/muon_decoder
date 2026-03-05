// Copyright pro Jeníka, využití tohoto kódu vyžaduje zmínění Jana Křivského jako spolupracovníka v jakékoliv vědecké či podobné práci

use std::sync::{Mutex, LazyLock};
use crate::decoder::Particle;
use eframe::egui;

static PARTICLES: LazyLock<Mutex<Vec<DimensionalTrack>>> = LazyLock::new(|| Mutex::new(Vec::new()));

#[derive(Default, Clone, Copy)]
struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}

// lets keep a complete implementation
#[allow(unused)]
impl Vector3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    fn normalize(&self) -> Self {
        let len = self.length();
        if len == 0.0 {
            Self::default()
        } else {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        }
    }

    fn dot(&self, other: &Vector3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(&self, other: &Vector3) -> Vector3 {
        Vector3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn add(&self, other: &Vector3) -> Vector3 {
        Vector3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    fn sub(&self, other: &Vector3) -> Vector3 {
        Vector3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn mul(&self, scalar: f32) -> Vector3 {
        Vector3 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

#[derive(Default, Clone, Copy)]
struct DimensionalTrack {
    source: Vector3,
    direction: Vector3,
}

impl DimensionalTrack {
    fn from_particle(particle: Particle) -> Self {
        if particle.get_track().is_empty() {
            return Self::default();
        }

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        for &(x, y) in particle.get_track().iter() {
            sum_x += x as f32;
            sum_y += y as f32;
        }
        let count = particle.get_track().len() as f32;
        let avg_x_pixel = sum_x / count;
        let avg_y_pixel = sum_y / count;

        // Convert pixel coordinates to world coordinates
        // In 2D view: X is horizontal (left-right), Y is vertical (top-bottom)
        // In 3D view at pitch=0, yaw=0: X is horizontal (right/East), Z is horizontal (away/South), Y is vertical (up)
        // So: 2D X -> 3D Z, 2D Y -> 3D X (and keep Y as vertical position)
        //
        // transform: -pi/2
        let source_x = avg_y_pixel - 128.0;
        let source_y = -avg_x_pixel + 128.0;
        let source_z = 0.0;

        let source = Vector3::new(source_x, source_y, source_z);

        // Calculate 3D direction using both 2D track angle and pixel_depth
        // In the 2D view: X goes right, Y goes down
        // In the 3D view: X goes right (East), Y goes up, Z goes away (South when azimuth=0)
        // azimuth: horizontal angle based on detector orientation (0 = North/away)
        // zenith: angle from the 2D track slope in the detector plane
        // secondary_angle: angle from vertical (Y) based on pixel_depth
        let azimuth_rad = particle.azimuth().to_radians();
        let zenith_rad = particle.zenith().to_radians();
        let secondary_angle_rad = particle.azimuth_offset().to_radians();

        // The zenith angle is in the XZ plane (detector plane)
        // When azimuth=0 (North): zenith defines angle between +X (right/East) and +Z (away/South)
        // Apply zenith rotation to get 2D direction in XZ plane
        let dir_x_2d = zenith_rad.sin();
        let dir_z_2d = zenith_rad.cos();

        // Apply azimuth rotation (rotating the XZ plane direction around Y axis)
        let dir_x_rotated = dir_x_2d * azimuth_rad.cos() - dir_z_2d * azimuth_rad.sin();
        let dir_z_rotated = dir_x_2d * azimuth_rad.sin() + dir_z_2d * azimuth_rad.cos();

        // Now apply secondary angle (elevation from detector plane using pixel_depth)
        // This tilts the XZ direction towards Y (up/down)
        let dir_x = dir_x_rotated * secondary_angle_rad.cos();
        let dir_y = secondary_angle_rad.sin();
        let dir_z = f32::abs(dir_z_rotated * secondary_angle_rad.cos());

        let direction = Vector3::new(dir_x, dir_y, dir_z).normalize();

        // norm all directions to positive Z
        Self { source, direction }
    }
}

#[derive(Debug)]
pub struct Renderer3D {
    camera_distance: f32,
    camera_pitch: f32,
    camera_yaw: f32,
    show_window: bool,
    show_backtracks: bool,
    scale: f32,
}

impl Default for Renderer3D {
    fn default() -> Self {
        Self {
            camera_distance: 100.0,
            camera_pitch: 30.0,
            camera_yaw: 0.0,
            show_window: false,
            show_backtracks: true,
            scale: 1.0,
        }
    }
}

impl Renderer3D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        let mut show_window = self.show_window;
        egui::Window::new("3D Particle Track Viewer")
            .default_open(true)
            .open(&mut show_window)
            .resizable(true)
            .default_width(600.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                // Top panel for controls
                Self::render_ui_static(ui, &mut self.camera_distance, &mut self.camera_pitch, &mut self.camera_yaw, &mut self.show_backtracks);

                ui.separator();

                // Bottom panel for 3D rendering - allocate_painter reserves space properly
                let (response, painter) = ui.allocate_painter(
                    egui::Vec2::new(ui.available_width(), ui.available_height()),
                    egui::Sense::hover(),
                );
                self.paint_3d_view_static(&painter, response.rect, self.camera_distance, self.camera_pitch, self.camera_yaw, self.scale);
            });
        self.show_window = show_window;

        self.handle_input(ctx);
    }

    pub fn toggle_window(&mut self) {
        self.show_window = !self.show_window;
    }

    fn render_ui_static(ui: &mut egui::Ui, camera_distance: &mut f32, camera_pitch: &mut f32, camera_yaw: &mut f32, show_backtracks: &mut bool) {
        ui.label("3D Particle Track Visualization");
        ui.separator();

        // Camera controls
        ui.heading("Camera Controls");
        ui.label("Use WASD to orbit around the center");
        ui.label("Q/E to zoom in/out");

        ui.horizontal(|ui| {
            ui.label("Pitch:");
            ui.add(egui::Slider::new(camera_pitch, -89.0..=89.0));
        });

        ui.horizontal(|ui| {
            ui.label("Yaw:");
            ui.add(egui::Slider::new(camera_yaw, -180.0..=180.0));
        });

        ui.horizontal(|ui| {
            ui.label("Zoom:");
            ui.add(egui::Slider::new(camera_distance, 0.1..=5.0));
        });

        ui.checkbox(show_backtracks, "show backtracks");

        ui.separator();

        // Display particle count
        let particle_count = {
            let particles = PARTICLES.lock().unwrap();
            particles.len()
        };
        ui.label(format!("Particle tracks: {}", particle_count));
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::W)) {
            self.camera_pitch += 2.0;
            self.camera_pitch = self.camera_pitch.clamp(-89.0, 89.0);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::S)) {
            self.camera_pitch -= 2.0;
            self.camera_pitch = self.camera_pitch.clamp(-89.0, 89.0);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::A)) {
            self.camera_yaw -= 3.0;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::D)) {
            self.camera_yaw += 3.0;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Q)) {
            self.scale = (self.scale - 0.1).max(0.1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::E)) {
            self.scale = (self.scale + 0.1).min(5.0);
        }
    }

    fn paint_3d_view_static(&mut self, painter: &egui::Painter, rect: egui::Rect, camera_distance: f32, camera_pitch: f32, camera_yaw: f32, scale: f32) {
        // Draw background
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 20, 30));

        let center = egui::Pos2::new(rect.center().x, rect.center().y);
        let particles = PARTICLES.lock().unwrap();

        // Draw detector surface (256x256 square in XY plane)
        Self::draw_detector_surface(painter, center, camera_distance, camera_pitch, camera_yaw, scale * 2.0);

        // Draw coordinate axes
        Self::draw_axes_static(painter, center, camera_distance, camera_pitch, camera_yaw, scale);

        // Draw particle tracks
        for track in particles.iter() {
            self.draw_track_static(painter, track, center, camera_distance, camera_pitch, camera_yaw, scale);
        }

        // Draw info text
        painter.text(
            egui::Pos2::new(rect.min.x + 10.0, rect.min.y + 10.0),
            egui::Align2::LEFT_TOP,
            format!("Camera: Pitch={:.1}°, Yaw={:.1}°, Scale={:.2}x",
                camera_pitch, camera_yaw, scale),
            egui::FontId::monospace(12.0),
            egui::Color32::WHITE,
        );
    }

    fn draw_detector_surface(painter: &egui::Painter, center: egui::Pos2, camera_distance: f32, camera_pitch: f32, camera_yaw: f32, scale: f32) {
        // Draw a 256x256 detector surface as a square in the XY plane
        // Don't apply scale to detector - it's a fixed size reference
        let size = 128.0 * 0.5;
        let corners = [
            Vector3::new(-size, -size, 0.0),
            Vector3::new(size, -size, 0.0),
            Vector3::new(size, size, 0.0),
            Vector3::new(-size, size, 0.0),
        ];

        let projected: Vec<_> = corners.iter()
            .filter_map(|corner| Self::project_3d_to_2d_static(corner, center, camera_distance, camera_pitch, camera_yaw, scale))
            .collect();

        if projected.len() == 4 {
            painter.line_segment([projected[0], projected[1]], egui::Stroke::new(1.5, egui::Color32::GRAY));
            painter.line_segment([projected[1], projected[2]], egui::Stroke::new(1.5, egui::Color32::GRAY));
            painter.line_segment([projected[2], projected[3]], egui::Stroke::new(1.5, egui::Color32::GRAY));
            painter.line_segment([projected[3], projected[0]], egui::Stroke::new(1.5, egui::Color32::GRAY));
        }
    }

    fn project_3d_to_2d_static(point: &Vector3, center: egui::Pos2, _camera_distance: f32, camera_pitch: f32, camera_yaw: f32, scale: f32) -> Option<egui::Pos2> {
        let pitch_rad = camera_pitch.to_radians();
        let yaw_rad = camera_yaw.to_radians();

        let mut p = *point;

        // Yaw rotation (around Y axis)
        let cos_yaw = yaw_rad.cos();
        let sin_yaw = yaw_rad.sin();
        let x = p.x * cos_yaw - p.z * sin_yaw;
        let z = p.x * sin_yaw + p.z * cos_yaw;
        p.x = x;
        p.z = z;

        // Pitch rotation (around X axis)
        let cos_pitch = pitch_rad.cos();
        let sin_pitch = pitch_rad.sin();
        let y = p.y * cos_pitch - p.z * sin_pitch;
        let z = p.y * sin_pitch + p.z * cos_pitch;
        p.y = y;
        p.z = z;

        // Apply scaling
        let screen_x = center.x + p.x * scale * 2.0;
        let screen_y = center.y - p.y * scale * 2.0;

        Some(egui::Pos2::new(screen_x, screen_y))
    }

    fn draw_track_static(&mut self, painter: &egui::Painter, track: &DimensionalTrack, center: egui::Pos2, camera_distance: f32, camera_pitch: f32, camera_yaw: f32, scale: f32) {
        // Draw track line as a ray extending from source in the direction (much longer)
        let end = track.source.add(&track.direction.mul(500.0));
        let start = track.source.sub(&track.direction.mul(500.0));

        if let (Some(hit_pos), Some(start_pos), Some(end_pos)) = (
            Self::project_3d_to_2d_static(&track.source, center, camera_distance, camera_pitch, camera_yaw, scale),
            Self::project_3d_to_2d_static(&start, center, camera_distance, camera_pitch, camera_yaw, scale),
            Self::project_3d_to_2d_static(&end, center, camera_distance, camera_pitch, camera_yaw, scale),
        ) {
            painter.line_segment(
                [hit_pos, end_pos],
                egui::Stroke::new(1.8, egui::Color32::LIGHT_BLUE),
            );

            if self.show_backtracks {
                painter.line_segment(
                    [hit_pos, start_pos],
                    egui::Stroke::new(1.8, egui::Color32::from_rgba_unmultiplied(173, 216, 230, 20)),
                );
            }
            // Draw source point at the start of the line (smaller and less visible)
            painter.circle_filled(hit_pos, 2.0, egui::Color32::from_rgba_unmultiplied(50, 255, 0, 100));
        }
    }

    fn draw_axes_static(painter: &egui::Painter, center: egui::Pos2, camera_distance: f32, camera_pitch: f32, camera_yaw: f32, scale: f32) {
        let origin = Vector3::new(0.0, 0.0, 0.0);
        let x_axis = Vector3::new(50.0, 0.0, 0.0);
        let y_axis = Vector3::new(0.0, 50.0, 0.0);
        let z_axis = Vector3::new(0.0, 0.0, 50.0);

        // X axis (red)
        if let (Some(o), Some(x)) = (
            Self::project_3d_to_2d_static(&origin, center, camera_distance, camera_pitch, camera_yaw, scale),
            Self::project_3d_to_2d_static(&x_axis, center, camera_distance, camera_pitch, camera_yaw, scale),
        ) {
            painter.line_segment(
                [o, x],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 0, 0)),
            );
        }

        // Y axis (green)
        if let (Some(o), Some(y)) = (
            Self::project_3d_to_2d_static(&origin, center, camera_distance, camera_pitch, camera_yaw, scale),
            Self::project_3d_to_2d_static(&y_axis, center, camera_distance, camera_pitch, camera_yaw, scale),
        ) {
            painter.line_segment(
                [o, y],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 0)),
            );
        }

        // Z axis (blue)
        if let (Some(o), Some(z)) = (
            Self::project_3d_to_2d_static(&origin, center, camera_distance, camera_pitch, camera_yaw, scale),
            Self::project_3d_to_2d_static(&z_axis, center, camera_distance, camera_pitch, camera_yaw, scale),
        ) {
            painter.line_segment(
                [o, z],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 0, 255)),
            );
        }
    }
}

pub fn update_data(tracks: Vec<Vec<(usize, usize)>>, app_borrow: &mut crate::graphics::MatrixApp) {
    let particles = tracks.into_iter().map(|t| {
        let particle = Particle::new(
            t,
            0,
            app_borrow.pixel_depth,
            app_borrow.pixel_width,
            app_borrow.selected_mode,
        );

        DimensionalTrack::from_particle(particle)
    }).collect::<Vec<DimensionalTrack>>();

    {
        *PARTICLES.lock().unwrap() = particles;
    }
}

// disclaimer: part of this code was written with the assistance of LLMs
// a big part was still written by hand, but keep it in mind
