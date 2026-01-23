use std::fs::File;
use std::io::{self, BufRead, Error};
use std::path::{Path, PathBuf};
use crate::SIZE;

#[derive(Debug)]
pub struct Tracks {
    pub tracks: Vec<Vec<Vec<f32>>>,
    pub file_path: PathBuf,
}

pub fn read_lines<P>(filename: P) -> Result<Vec<Vec<Vec<f32>>>, Box<dyn std::error::Error>>
where
    P: AsRef<Path>,
{
    let file = File::open(&filename)?;
    if let Ok(val) = matrix_read(&file) {
        return Ok(vec![val]);
    }
    let file = File::open(&filename)?;
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
                let resp = val.parse::<f32>()
                    .map_err(|e| Error::new(io::ErrorKind::InvalidData, e.to_string()));
                match resp {
                    Ok(value) => Ok(value),
                    Err(y) => {return Err(y);}
                }
            })
            .collect::<Result<Vec<f32>, _>>()?;

        grid.push(row);
    }

    if grid.len() == 0 {
        return Err(Error::new(io::ErrorKind::InvalidData, "invalid_data"));
    }
    if grid[0].len() != SIZE {
        return Err(Error::new(io::ErrorKind::InvalidData, "invalid_data"));
    }

    Ok(grid)
}

fn ascii_read(file: &File) -> Result<Vec<Vec<Vec<f32>>>, Box<dyn std::error::Error>> {
    let lines = io::BufReader::new(file).lines();
    let mut grid: Vec<Vec<f32>> = vec![vec![0.0; SIZE]; SIZE];
    let mut grids: Vec<Vec<Vec<f32>>> = Vec::new();
    for lin in lines.flatten() {
        if lin.trim() == "#" {
            grids.push(grid);
            grid = vec![vec![0.0; SIZE]; SIZE];
            continue;
        }
        let mut vals = lin.split_whitespace();
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

        grid[x][y] = val;
    }
    if grids.is_empty() {
        return Err(Error::new(io::ErrorKind::InvalidData, "wrong format").into());
    }
    Ok(grids)
}

pub fn list_dir(path: &Path) -> Result<Vec<Tracks>, Box<dyn std::error::Error>>{
    if !path.is_dir() {
        if let Ok(matrix) = read_lines(path){
            let track = Tracks {tracks: matrix, file_path: path.to_path_buf()};
            return Ok(vec![track]);
        }
        else {
            return Err(Error::new(io::ErrorKind::InvalidData, "wrong format").into())
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
        let meta= ok_file.metadata();
        match meta {
            Ok(val) => {
                if val.is_dir() {
                    if let Ok(fils) = &mut list_dir(&ok_file.path()) {
                        files.append(fils);
                    }
                }
                else if val.is_file() {
                    match read_lines(Path::new(&ok_file.path())) {
                        Ok(matrix) => {
                            files.push(Tracks {tracks: matrix, file_path: ok_file.path().to_path_buf()})
                        }
                        Err(_) => (),
                    }
                } 
            }
            Err(y) => {
                eprintln!("meta is wrong: {}", y);
                continue
            }
        }

    }

    return Ok(files);
}