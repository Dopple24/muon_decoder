mod decoder;
mod graphics;
mod particle_extractor;

use std::fs::File;
use std::io::{self, BufRead, Error};
use std::path::Path;

use eframe::egui::debug_text::print;
const SIZE: usize = 256;

fn main() -> eframe::Result<()> {
    let grid: Vec<Vec<f32>> = vec![vec![0.0; SIZE]; SIZE];

    let tracks: Vec<decoder::Particle> = Vec::new();

    // graphics
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "256x256 Matrix Viewer",
        options,
        Box::new(move |_cc| Box::new(graphics::MatrixApp::new(grid, tracks, 2))),
    )
}

pub fn read_lines<P>(filename: P) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>>
where
    P: AsRef<Path>,
{
    let file = File::open(&filename)?;
    match matrix_read(&file) {
        Ok(val) => {return Ok(val);},
        Err(_) => (),
    }
    let file = File::open(&filename)?;
    let grids = match ascii_read(&file) {
        Ok(val) => val,
        Err(y) => {
            eprintln!("{}", y);
            return Err(y);
        }
    };
    Ok(grids[0].clone())
}

fn matrix_read(file: &File) -> Result<Vec<Vec<f32>>, std::io::Error>{
    let lines = io::BufReader::new(file).lines();

    let mut grid: Vec<Vec<f32>> = Vec::with_capacity(SIZE);

    for line_result in lines {
        let line = line_result?;
        let row: Vec<f32> = line
            .split_whitespace()
            .map(|val| {
                val.parse::<f32>()
                    .map_err(|e| Error::new(io::ErrorKind::InvalidData, e.to_string()))
            })
            .collect::<Result<Vec<f32>, _>>()?;

        grid.push(row);
    }

    Ok(grid)
}

fn ascii_read(file: &File) -> Result<Vec<Vec<Vec<f32>>>, Box<dyn std::error::Error>> {
    let lines = io::BufReader::new(file).lines();
    let mut grid: Vec<Vec<f32>> = vec![vec![0.0; SIZE]; SIZE];
    let mut grids: Vec<Vec<Vec<f32>>> = Vec::new();
    for line in lines {
        if let Ok(lin) = line {
            if lin.trim() == "#" {
                grids.push(grid);
                grid = vec![vec![0.0; SIZE]; SIZE];
                continue;
            }
            let mut vals = lin.split_whitespace();
            let x: usize = vals.next().ok_or(Error::new(io::ErrorKind::InvalidData, "wrong format"))?.parse()?;
            let y: usize = vals.next().ok_or(Error::new(io::ErrorKind::InvalidData, "wrong format"))?.parse()?;
            let val = vals.next().ok_or(Error::new(io::ErrorKind::InvalidData, "wrong format"))?.parse()?;

            grid[x][y] = val;
        }
    }
    return Ok(grids);
}