use eframe::egui;

mod decoder;
mod file_reader;
mod graphics;
mod particle_extractor;

const SIZE: usize = 256;

fn main() -> eframe::Result<()> {
    let grid: Vec<Vec<Vec<f32>>> = vec![vec![vec![0.0; SIZE]; SIZE]; 1];

    let tracks: Vec<decoder::Particle> = Vec::new();

    // graphics
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([920.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "256x256 Matrix Viewer",
        options,
        Box::new(move |_cc| Box::new(graphics::MatrixApp::new(grid, tracks, 2))),
    )
}
