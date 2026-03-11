use crate::SIZE;
use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Error};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Tracks {
    tracks_cache: Option<Vec<Frame>>,
    file_content: Vec<String>,
    pub file_path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub matrix: Vec<f32>,
    pub timestamp: chrono::DateTime<Utc>,
}
impl Frame {
    pub fn new(matrix: Vec<f32>, timestamp: chrono::DateTime<Utc>) -> Self {
        Frame { matrix, timestamp }
    }
}

impl Tracks {
    pub fn get_tracks(&mut self) -> &mut Vec<Frame> {
        // this is necessary, doing what clippy suggests causes lifetime problems
        #[allow(clippy::unnecessary_unwrap)]
        if self.tracks_cache.is_some() {
            return self.tracks_cache.as_mut().unwrap();
        }

        let timestamps = get_timestamps(&self.file_path);

        let frames = read_lines(&self.file_content)
            .unwrap_or(vec![vec![0.0; SIZE * SIZE]; 1])
            .into_iter()
            .enumerate()
            .map(|(time, matrix)| {
                Frame::new(matrix, {
                    if time >= timestamps.len() {
                        chrono::DateTime::default()
                    } else {
                        timestamps[time]
                    }
                })
            })
            .collect();
        self.tracks_cache = Some(frames);
        self.tracks_cache.as_mut().unwrap()
    }

    pub fn clear_cache(&mut self) {
        drop(self.tracks_cache.take());
    }
}

pub fn read_lines(lines: &[String]) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    if let Ok(val) = matrix_read(lines.iter()) {
        return Ok(val);
    }

    let grids = match ascii_read(lines.iter()) {
        Ok(val) => val,
        Err(y) => {
            return Err(y);
        }
    };
    Ok(grids)
}

fn matrix_read<'a, I>(lines: I) -> Result<Vec<Vec<f32>>, std::io::Error>
where
    I: Iterator<Item = &'a String>,
{
    let mut grid: Vec<Vec<f32>> = Vec::with_capacity(SIZE);

    for line in lines {
        let row: Vec<f32> = line
            .split_whitespace()
            .map(|val| {
                let resp = val
                    .parse::<f32>()
                    .map_err(|e| Error::new(io::ErrorKind::InvalidData, e.to_string()));
                match resp {
                    Ok(value) => Ok(value),
                    Err(y) => Err(y),
                }
            })
            .collect::<Result<Vec<f32>, _>>()?;

        grid.push(row);
    }

    if grid.is_empty() {
        return Err(Error::new(io::ErrorKind::InvalidData, "invalid_data"));
    }
    if grid[0].len() != SIZE {
        return Err(Error::new(io::ErrorKind::InvalidData, "invalid_data"));
    }

    Ok(grid)
}

fn ascii_read<'a, I>(lines: I) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>>
where
    I: Iterator<Item = &'a String>,
{
    let mut grid: Vec<f32> = vec![0.0; SIZE * SIZE];

    let mut grids: Vec<Vec<f32>> = Vec::new();
    for lin in lines {
        if lin.trim() == "#" {
            grids.push(grid);
            grid = vec![0.0; SIZE * SIZE];
            continue;
        }
        let mut vals = lin.split_ascii_whitespace();
        let x: usize = vals
            .next()
            .ok_or(Error::new(io::ErrorKind::InvalidData, "wrong format"))?
            .parse()?;
        let y: usize = vals
            .next()
            .ok_or(Error::new(io::ErrorKind::InvalidData, "wrong format"))?
            .parse()?;
        let val = vals
            .next()
            .ok_or(Error::new(io::ErrorKind::InvalidData, "wrong format"))?
            .parse()?;

        grid[x * SIZE + y] = val;
    }
    if grids.is_empty() {
        return Err(Error::new(io::ErrorKind::InvalidData, "wrong format").into());
    }

    // save memory from first pre-allocation
    grids.shrink_to_fit();

    Ok(grids)
}

pub fn list_dir(path: &Path) -> Result<Vec<Tracks>, Box<dyn std::error::Error>> {
    if !path.is_dir() {
        let file = File::open(path)?;
        let reader = std::io::BufReader::new(&file);
        let lines = reader.lines();
        let lines: Vec<String> = lines.map(|l| l.unwrap()).collect();
        if read_lines(&lines).is_ok() {
            let track = Tracks {
                tracks_cache: None,
                file_content: lines,
                file_path: path.to_path_buf(),
            };
            return Ok(vec![track]);
        } else {
            return Err(Error::new(io::ErrorKind::InvalidData, "wrong format").into());
        }
    }
    let paths = std::fs::read_dir(path).unwrap();
    let mut files: Vec<Tracks> = Vec::new();
    for file in paths {
        let ok_file = match file {
            Ok(val) => val,
            Err(_) => {
                continue;
            }
        };
        let meta = ok_file.metadata();
        match meta {
            Ok(val) => {
                if val.is_dir() {
                    if let Ok(fils) = &mut list_dir(&ok_file.path()) {
                        files.append(fils);
                    }
                } else if val.is_file() {
                    let file_desc = File::open(ok_file.path())?;
                    let reader = std::io::BufReader::new(&file_desc);
                    let lines_r = reader.lines();
                    let mut lines = Vec::new();
                    for l in lines_r {
                        match l {
                            Ok(l) => lines.push(l),
                            Err(_) => continue,
                        }
                    }
                    if read_lines(&lines).is_ok() {
                        files.push(Tracks {
                            tracks_cache: None,
                            file_content: lines,
                            file_path: ok_file.path().to_path_buf(),
                        })
                    }
                }
            }
            Err(y) => {
                eprintln!("meta is wrong: {}", y);
                continue;
            }
        }
    }

    Ok(files)
}

fn get_timestamps(path: &Path) -> Vec<chrono::DateTime<Utc>> {
    let dsc = match File::open(path.with_added_extension("dsc")) {
        Ok(val) => val,
        Err(_) => {
            return Vec::new();
        }
    };
    let mut timestamps: Vec<DateTime<Utc>> = Vec::new();
    let mut dsc_lines = BufReader::new(dsc).lines();
    while let Some(line) = dsc_lines.next() {
        if line.unwrap() == "\"Start time (string)\" (\"Acquisition start time (string)\"):" {
            dsc_lines.next();
            timestamps.push(DateTime::from_naive_utc_and_offset(
                chrono::NaiveDateTime::parse_from_str(
                    &dsc_lines
                        .next()
                        .unwrap_or(Ok(chrono::NaiveDateTime::default().to_string()))
                        .unwrap_or(chrono::NaiveDateTime::default().to_string()),
                    "%a %b %d %H:%M:%S%.f %Y",
                )
                .unwrap_or_default(),
                Utc,
            ));
        }
    }
    timestamps
}
