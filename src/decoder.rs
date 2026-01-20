use geo::{Area, ConvexHull, EuclideanLength};
use geo_types::{Coord, MultiPoint};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PartType {
    ALPHA,
    BETA,
    GAMMA,
    MUON,
    UNKNOWN,
}
use std::cell::RefCell;

#[derive(Clone)]
pub struct Particle {
    track: Vec<(usize, usize)>,
    total_energy_cache: RefCell<Option<f32>>,
    roundness_cache: RefCell<Option<f32>>,
    winding_cache: RefCell<Option<f32>>,
    part_type_cache: RefCell<Option<PartType>>,
}

impl Particle {
    pub fn new(track: Vec<(usize, usize)>) -> Self {
        Particle {
            track,
            total_energy_cache: RefCell::new(None),
            roundness_cache: RefCell::new(None),
            winding_cache: RefCell::new(None),
            part_type_cache: RefCell::new(None),
        }
    }

    pub fn get_track(&self) -> Vec<(usize, usize)> {
        self.track.clone()
    }
    pub fn size(&self) -> usize {
        self.track.len()
    }

    pub fn total_energy(&self, grid: &Vec<Vec<f32>>) -> f32 {
        if let Some(val) = *self.total_energy_cache.borrow() {
            return val;
        }

        let energy: f32 = self
            .track
            .iter()
            .map(|&(x, y)| grid[x][y])
            .sum();

        *self.total_energy_cache.borrow_mut() = Some(energy);
        energy
    }

    pub fn max_energy(&self, grid: &Vec<Vec<f32>>) -> f32 {
        self.track
            .iter()
            .map(|&(x, y)| grid[x][y])
            .fold(0.0, |acc, val| acc.max(val))
    }

    pub fn avg_energy(&self, grid: &Vec<Vec<f32>>) -> f32 {
        self.total_energy(grid) / self.size() as f32
    }

    pub fn roundness(&self) -> f32 {
        if let Some(val) = *self.roundness_cache.borrow() {
            return val;
        }

        let val = roundness(&self.track); // CALL YOUR HELPER HERE
        *self.roundness_cache.borrow_mut() = Some(val);
        val
    }

    pub fn winding(&self) -> f32 {
        if let Some(val) = *self.winding_cache.borrow() {
            return val;
        }

        let val = winding_of_path(&self.track).abs(); // CALL YOUR HELPER HERE
        *self.winding_cache.borrow_mut() = Some(val);
        val
    }

    pub fn slope(&self) -> f32 {
        slope(&linear_regretion(&self.track), &self.track).atan() * 180.0 / PI as f32
    }

    pub fn particle_type(&self, grid: &Vec<Vec<f32>>) -> PartType {
        if let Some(pt) = *self.part_type_cache.borrow() {
            return pt;
        }

        let pt = match self.size() {
            0..4 => return PartType::GAMMA,
            4..30 => {
                if self.max_energy(grid) < 150.0 && self.avg_energy(grid) < 40.0 {
                    if self.winding() < 1.0 {
                        PartType::BETA
                    } else {
                        PartType::BETA
                    }
                } else if self.max_energy(grid) > 100.0 {
                    if self.roundness() > 0.4 {
                        PartType::UNKNOWN //small blob
                    } else {
                        PartType::UNKNOWN
                    }
                } else {
                    PartType::UNKNOWN
                }
            }
            30.. => {
                if self.max_energy(grid) < 100.0 && self.avg_energy(grid) < 40.0 {
                    if self.winding() > 1.0 {
                        PartType::BETA
                    } else {
                        PartType::MUON
                    }
                } else if self.max_energy(grid) < 100.0 {
                    PartType::UNKNOWN
                } else if self.roundness() > 0.4 {
                    PartType::ALPHA
                } else {
                    PartType::UNKNOWN
                }
            }
        };

        *self.part_type_cache.borrow_mut() = Some(pt);
        pt
    }
}

fn roundness(points: &[(usize, usize)]) -> f32 {
    let mp: MultiPoint<f64> = points
        .iter()
        .map(|&(x, y)| Coord {
            x: x as f64,
            y: y as f64,
        })
        .collect();

    let hull = mp.convex_hull();

    let area = hull.unsigned_area();
    let perimeter = hull.exterior().euclidean_length();

    (4.0 * PI * area / (perimeter * perimeter)) as f32
}

fn linear_regretion (track: &[(usize, usize)]) -> (f32, f32) {
    let mut total_x = 0.0;
    let mut total_y = 0.0;
    for (x, y) in track {
        total_x += *x as f32;
        total_y += *y as f32;
    }
    let avg_x: f32 = total_x / track.len() as f32;
    let avg_y: f32 = total_y / track.len() as f32;
    (avg_x, avg_y)
}

fn slope((avg_x, avg_y): &(f32, f32), track: &[(usize, usize)]) -> f32 {
    let mut total_off_x = 0.0;
    let mut total_off = 0.0;
    for (x, y) in track {
        total_off_x += (*x as f32 - avg_x).powi(2);
        total_off += (*x as f32 - avg_x) * (*y as f32 - avg_y);
    }

    total_off / total_off_x //slope
}

fn winding_of_path (track: &[(usize, usize)]) -> f32 {
    let avgs = linear_regretion(track);
    let slope = slope(&avgs, track);
    let b = avgs.1 - avgs.0 * slope;

    let mut mse = 0.0;
    for (x, y) in track {
        let y_pred = slope * (*x as f32) + b;
        let diff = *y as f32 - y_pred;
        mse += diff * diff;
    }

    mse / track.len() as f32
}