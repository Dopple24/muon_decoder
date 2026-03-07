use crate::SIZE;
use crate::graphics::Orientation;
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

#[derive(Clone, Debug)]
pub struct Particle {
    pixel_depth: i32,
    pub pixel_width: f32,
    track: Vec<(usize, usize)>,
    frame_index: usize,
    total_energy_cache: Option<f32>,
    roundness_cache: Option<f32>,
    winding_cache: Option<f32>,
    part_type_cache: Option<PartType>,
    let_avg_cache: Option<f32>,
    orientation: Orientation,
}

pub const DEFAULT_PIXEL_DEPTH: i32 = 300;
pub const DEFAULT_PIXEL_WIDTH: f32 = 54.6875;

impl Particle {
    pub fn new(
        track: Vec<(usize, usize)>,
        frame_index: usize,
        pixel_depth: Option<i32>,
        pixel_width: Option<f32>,
        orientation: Orientation,
    ) -> Self {
        Particle {
            pixel_depth: pixel_depth.unwrap_or(DEFAULT_PIXEL_DEPTH),
            pixel_width: pixel_width.unwrap_or(DEFAULT_PIXEL_WIDTH),
            track,
            frame_index,
            total_energy_cache: None,
            roundness_cache: None,
            winding_cache: None,
            part_type_cache: None,
            let_avg_cache: None,
            orientation,
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

    pub fn total_energy(&mut self, grid: &[f32]) -> f32 {
        if let Some(val) = self.total_energy_cache {
            return val;
        }

        let energy: f32 = self.track.iter().map(|&(x, y)| grid[x * SIZE + y]).sum();

        self.total_energy_cache = Some(energy);
        energy
    }

    pub fn max_energy(&self, grid: &[f32]) -> f32 {
        self.track
            .iter()
            .map(|&(x, y)| grid[x * SIZE + y])
            .fold(0.0, |acc, val| acc.max(val))
    }

    pub fn avg_energy(&mut self, grid: &[f32]) -> f32 {
        self.total_energy(grid) / self.size() as f32
    }

    fn diag_len(&mut self) -> f32 {
        if self.track.is_empty() {
            self.let_avg_cache = Some(0.0);
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

        (x_diff.powi(2) + y_diff.powi(2)).sqrt()
    }

    pub fn let_avg(&mut self, grid: &[f32]) -> f32 {
        if let Some(val) = self.let_avg_cache {
            return val;
        }
        let diagonal = self.diag_len();

        if diagonal == 0.0 {
            return self.total_energy(grid);
        }

        let let_avg = self.total_energy(grid) / diagonal;

        self.let_avg_cache = Some(let_avg);

        let_avg
    }

    fn secondary_angle(&mut self) -> f32 {
        (self.pixel_depth as f32 / (self.diag_len() * self.pixel_width)).asin() * 180.0 / PI as f32
    }

    pub fn roundness(&mut self) -> f32 {
        if let Some(val) = self.roundness_cache {
            return val;
        }

        let val = roundness(&self.track);
        self.roundness_cache = Some(val);
        val
    }

    pub fn winding(&mut self) -> f32 {
        if let Some(val) = self.winding_cache {
            return val;
        }

        let val = winding_of_path(&self.track).abs(); // CALL YOUR HELPER HERE
        self.winding_cache = Some(val);
        val
    }

    fn angle(&self) -> f32 { // 0 is horizontal, 90 is pointing up
        (slope(&linear_regretion(&self.track), &self.track)
            .clamp(-573.0, 573.0)
            .atan()
            * 180.0
            / PI as f32
            + 180.0) % 180.0
    }

    pub fn abs_angle_primary(&self) -> f32 { // 0 is pointing up
        90.0 - f32::abs(
            slope(&linear_regretion(&self.track), &self.track)
                .clamp(-573.0, 573.0)
                .atan()
                * 180.0
                / PI as f32,
        )
    }

    pub fn azimuth(&self) -> f32 {
        self.orientation.azimuth()
    }

    pub fn zenith(&self) -> f32 {
        self.angle()
    }

    pub fn azimuth_offset(&mut self) -> f32 {
        self.secondary_angle()
    }

    pub fn particle_type(&mut self, grid: &[f32]) -> PartType {
        if let Some(pt) = self.part_type_cache {
            return pt;
        }

        let pt = match self.size() {
            0..4 => return PartType::Gamma,
            4..20 => {
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
            20..30 => {
                #[allow(clippy::if_same_then_else)]
                if self.max_energy(grid) < 150.0 && self.avg_energy(grid) < 40.0 {
                    #[allow(clippy::if_same_then_else)]
                    if self.winding() < 0.25 {
                        //consider 0.2
                        PartType::Muon
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
                        } else {
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

        self.part_type_cache = Some(pt);
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
