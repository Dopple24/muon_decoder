use crate::SIZE;
use std::fs::File;
use std::io::{self, BufRead, Error, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Tracks {
    pub tracks_cache: Option<Vec<Vec<f32>>>,
    pub file_path: PathBuf,
}

impl Tracks {
    pub fn get_tracks(&mut self) -> &mut Vec<Vec<f32>> {
        // this is necessary, doing what clippy suggests causes lifetime problems
        #[allow(clippy::unnecessary_unwrap)]
        if self.tracks_cache.is_some() {
            return self.tracks_cache.as_mut().unwrap();
        }
        let tracks = read_lines(&self.file_path).unwrap_or(vec![vec![0.0; SIZE * SIZE]; 1]);
        self.tracks_cache = Some(tracks);
        self.tracks_cache.as_mut().unwrap()
    }

    pub fn clear_cache(&mut self) {
        drop(self.tracks_cache.take());
    }
}

pub fn read_lines<P>(filename: P) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>>
where
    P: AsRef<Path>,
{
    let mut file = File::open(&filename)?;
    if let Ok(val) = matrix_read(&file) {
        return Ok(val);
    }

    // return back to the start of the file to attempt reading again
    file.seek(SeekFrom::Start(0))?;
    let grids = match ascii_read(&file) {
        Ok(val) => val,
        Err(y) => {
            return Err(y);
        }
    };
    Ok(grids)
}

fn matrix_read(file: &File) -> Result<Vec<Vec<f32>>, std::io::Error> {
    let lines = io::BufReader::new(file).lines();

    let mut grid: Vec<Vec<f32>> = Vec::with_capacity(SIZE);

    for line_result in lines {
        let line = line_result?;
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

fn ascii_read(file: &File) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    let lines = io::BufReader::new(file).lines();
    let mut grid: Vec<f32> = vec![0.0; SIZE * SIZE];

    // attempt to avoid reallocations by guessing amount of grids
    let file_size = file.metadata().unwrap().len() as usize;
    let guessed_grids_size = file_size / 1000;

    let mut grids: Vec<Vec<f32>> = Vec::with_capacity(guessed_grids_size);
    for lin in lines.map_while(Result::ok) {
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
        if let Ok(_matrix) = read_lines(path) {
            let track = Tracks {
                tracks_cache: None,
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
                } else if val.is_file()
                    && let Ok(_matrix) = read_lines(Path::new(&ok_file.path()))
                {
                    files.push(Tracks {
                        tracks_cache: None,
                        file_path: ok_file.path().to_path_buf(),
                    })
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
