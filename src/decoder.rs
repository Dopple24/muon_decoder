use geo::algorithm::line_measures::{Euclidean, Length};
use geo::{Area, ConvexHull};
use geo_types::{Coord, MultiPoint};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PartType {
    Alpha,
    Beta,
    Gamma,
    Muon,
    SusMuon,
    Unknown,
}
use std::cell::RefCell;

#[derive(Clone, Debug)]
pub struct Particle {
    track: Vec<(usize, usize)>,
    frame_index: usize,
    total_energy_cache: RefCell<Option<f32>>,
    roundness_cache: RefCell<Option<f32>>,
    winding_cache: RefCell<Option<f32>>,
    part_type_cache: RefCell<Option<PartType>>,
    let_avg_cache: RefCell<Option<f32>>
}

impl Particle {
    pub fn new(track: Vec<(usize, usize)>, frame_index: usize) -> Self {
        Particle {
            track,
            frame_index,
            total_energy_cache: RefCell::new(None),
            roundness_cache: RefCell::new(None),
            winding_cache: RefCell::new(None),
            part_type_cache: RefCell::new(None),
            let_avg_cache: RefCell::new(None),
        }
    }

    pub fn get_frame_index(&self) -> usize {
        self.frame_index
    }

    pub fn get_track(&self) -> Vec<(usize, usize)> {
        self.track.clone()
    }
    pub fn size(&self) -> usize {
        self.track.len()
    }

    pub fn total_energy(&self, grid: &[Vec<f32>]) -> f32 {
        if let Some(val) = *self.total_energy_cache.borrow() {
            return val;
        }

        let energy: f32 = self.track.iter().map(|&(x, y)| grid[x][y]).sum();

        *self.total_energy_cache.borrow_mut() = Some(energy);
        energy
    }

    pub fn max_energy(&self, grid: &[Vec<f32>]) -> f32 {
        self.track
            .iter()
            .map(|&(x, y)| grid[x][y])
            .fold(0.0, |acc, val| acc.max(val))
    }

    pub fn avg_energy(&self, grid: &[Vec<f32>]) -> f32 {
        self.total_energy(grid) / self.size() as f32
    }

    pub fn let_avg(&self, grid: &[Vec<f32>]) -> f32 {
        if let Some(val) = *self.let_avg_cache.borrow() {
            return val;
        }
        if self.track.is_empty() {
            *self.let_avg_cache.borrow_mut() = Some(0.0);
            return 0.0;
        }

        let (mut min_x, mut max_x) = (usize::MAX, usize::MIN);
        let (mut min_y, mut max_y) = (usize::MAX, usize::MIN);

        for &(x, y) in &self.track {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }

        let x_diff = (max_x - min_x) as f32 + 1.0;
        let y_diff = (max_y - min_y) as f32 + 1.0;

        let diagonal = (x_diff.powi(2) + y_diff.powi(2)).sqrt();


        if diagonal == 0.0 {
            return self.total_energy(grid);
        }

        let let_avg = self.total_energy(grid) / diagonal;

        *self.let_avg_cache.borrow_mut() = Some(let_avg);

        let_avg
        
    }


    pub fn roundness(&self) -> f32 {
        if let Some(val) = *self.roundness_cache.borrow() {
            return val;
        }

        let val = roundness(&self.track);
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
        slope(&linear_regretion(&self.track), &self.track)
            .clamp(-573.0,573.0)
            .atan()
            * 180.0
            / PI as f32 + 90.0
    }

    pub fn abs_slope(&self) -> f32 {
        90.0 - f32::abs(slope(&linear_regretion(&self.track), &self.track)
            .clamp(-573.0,573.0)
            .atan()
            * 180.0
            / PI as f32 )
    }

    pub fn particle_type(&self, grid: &[Vec<f32>]) -> PartType {
        if let Some(pt) = *self.part_type_cache.borrow() {
            return pt;
        }

        let pt = match self.size() {
            0..4 => return PartType::Gamma,
            4..30 => {
                #[allow(clippy::if_same_then_else)]
                if self.max_energy(grid) < 150.0 && self.avg_energy(grid) < 40.0 {
                    #[allow(clippy::if_same_then_else)]
                    if self.winding() < 1.0 {
                        PartType::Beta
                    } else {
                        PartType::Beta
                    }
                } else if self.max_energy(grid) > 100.0 {
                    if self.roundness() > 0.4 {
                        PartType::Unknown //small blob
                    } else {
                        PartType::Unknown
                    }
                } else {
                    PartType::Unknown
                }
            }
            30.. => {
                if self.max_energy(grid) < 200.0 && self.avg_energy(grid) < 40.0 {
                    /*
                    This check was originally only self.winding() > 0.4
                    Second part of the check was added for the purposes of detecting muons which have made an electron excited
                    It assumes, that if winding is relatively small (4.0), only a muon would be able to hold a straight track for 100 or more pixels
                    */
                    //consider removing the second check
                    if self.winding() > 0.4 {
                        if !(self.size() > 100 && self.winding() < 4.0) {
                            PartType::Beta
                        }
                        else {
                            PartType::SusMuon
                        }
                    } else {
                        PartType::Muon
                    }
                } else if self.max_energy(grid) < 200.0 {
                    PartType::Unknown
                } else if self.roundness() > 0.4 {
                    PartType::Alpha
                } else {
                    PartType::Unknown
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
    let perimeter = Euclidean.length(hull.exterior());

    (4.0 * PI * area / (perimeter * perimeter)) as f32
}

fn linear_regretion(track: &[(usize, usize)]) -> (f32, f32) {
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

fn get_totals((avg_x, avg_y): &(f32, f32), track: &[(usize, usize)]) -> (f32, f32) {
    let mut total_off_x = 0.0;
    let mut total_off = 0.0;
    for (x, y) in track {
        total_off_x += (*x as f32 - avg_x).powi(2);
        total_off += (*x as f32 - avg_x) * (*y as f32 - avg_y);
    }
    (total_off, total_off_x)
}

fn get_totals_reverse((avg_y, avg_x): &(f32, f32), track: &[(usize, usize)]) -> (f32, f32) {
    let mut total_off_x = 0.0;
    let mut total_off = 0.0;
    for (y, x) in track {
        total_off_x += (*x as f32 - avg_x).powi(2);
        total_off += (*x as f32 - avg_x) * (*y as f32 - avg_y);
    }
    (total_off, total_off_x)
}

fn slope((avg_x, avg_y): &(f32, f32), track: &[(usize, usize)]) -> f32 {
    let (total_off, total_off_x) = get_totals(&(*avg_x, *avg_y), track);
    total_off / total_off_x //slope
}

fn winding_of_path(track: &[(usize, usize)]) -> f32 {
    let avgs = linear_regretion(track);
    let mut mse = 0.0;

    let (total_off, total_off_x) = get_totals(&avgs, track);
    if (total_off / total_off_x).abs() < 1.0 {
        let slope = total_off / total_off_x;

        let b = avgs.1 - avgs.0 * slope;

        for (x, y) in track {
            let y_pred = slope * (*x as f32) + b;
            let diff = *y as f32 - y_pred;
            mse += diff * diff;
        }
    }
    //swaps axes to prevent failing mechanic near slope = 90 deg
    else {
        mse = 0.0;
        let (total_off_rev, total_off_x_rev) = get_totals_reverse(&avgs, track);
        let slope = total_off_rev / total_off_x_rev;

        let b = avgs.0 - avgs.1 * slope;

        for (y, x) in track {
            let y_pred = slope * (*x as f32) + b;
            let diff = *y as f32 - y_pred;
            mse += diff * diff;
        }
    };

    mse / track.len() as f32
}
