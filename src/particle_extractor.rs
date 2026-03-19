use std::collections::HashMap;

/// Extracts connected particles from a grid.
pub fn extract(
    grid: &[f32],
    id_map: &mut [Vec<usize>],
    range: i16,
    size: usize,
) -> HashMap<usize, Vec<(usize, usize)>> {
    let mut next_id: usize = 1;
    let mut parent: HashMap<usize, usize> = HashMap::new();

    for y in 0..size {
        for x in 0..size {
            if grid[x * size + y] == 0.0 {
                continue;
            }

            let neighbors = check_surroundings(&(x, y), grid, id_map, range, size);

            if neighbors.is_empty() {
                id_map[x][y] = next_id;
                parent.insert(next_id, next_id);
                next_id += 1;
            } else {
                let root = find(neighbors[0], &mut parent);
                id_map[x][y] = root;

                for &other in &neighbors[1..] {
                    union(root, other, &mut parent);
                }
            }
        }
    }

    build_tracks(id_map, &mut parent)
}

/// Builds a map of particle IDs to their coordinates.
fn build_tracks(
    id_map: &[Vec<usize>],
    parent: &mut HashMap<usize, usize>,
) -> HashMap<usize, Vec<(usize, usize)>> {
    let mut tracks: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();

    for (y, id_row) in id_map.iter().enumerate() {
        for (x, _) in id_row.iter().enumerate() {
            let id = id_map[x][y];
            if id == 0 {
                continue;
            }

            let root = find(id, parent);
            tracks.entry(root).or_default().push((x, y));
        }
    }

    tracks
}

/// Finds the root of a particle ID (with path compression)
fn find(x: usize, parent: &mut HashMap<usize, usize>) -> usize {
    let p = parent[&x];
    if p != x {
        let root = find(p, parent);
        parent.insert(x, root);
        root
    } else {
        x
    }
}

/// Unions two particle IDs
fn union(a: usize, b: usize, parent: &mut HashMap<usize, usize>) {
    let ra = find(a, parent);
    let rb = find(b, parent);
    if ra != rb {
        parent.insert(rb, ra);
    }
}

/// Checks all previously uncovered cells in range
pub fn check_surroundings(
    location: &(usize, usize),
    grid: &[f32],
    id_map: &[Vec<usize>],
    range: i16,
    size: usize,
) -> Vec<usize> {
    let mut found_ids: Vec<usize> = Vec::new();
    let size_x = size;
    let size_y = size;

    let (lx, ly) = (location.0 as i16, location.1 as i16);

    // check all cells above and diagonals
    for dx in -range..=range {
        for dy in -range..0 {
            if let Some(id) = check_cell((lx, ly), dx, dy, size_x, size_y, grid, id_map, size)
                && !found_ids.contains(&id)
            {
                found_ids.push(id);
            }
        }
    }

    // check cells left
    for dx in -range..0 {
        let dy = 0;
        if let Some(id) = check_cell((lx, ly), dx, dy, size_x, size_y, grid, id_map, size)
            && !found_ids.contains(&id)
        {
            found_ids.push(id);
        }
    }

    found_ids
}

/// Checks a single cell at offset
#[allow(clippy::too_many_arguments)]
pub fn check_cell(
    loc: (i16, i16),
    dx: i16,
    dy: i16,
    size_x: usize,
    size_y: usize,
    grid: &[f32],
    id_map: &[Vec<usize>],
    size: usize,
) -> Option<usize> {
    let x = loc.0 + dx;
    let y = loc.1 + dy;

    if x < 0 || y < 0 || x >= size_x as i16 || y >= size_y as i16 {
        return None;
    }

    let (x, y) = (x as usize, y as usize);

    if grid[x * size + y] > 0.0 {
        let id = id_map[x][y];
        if id != 0 {
            return Some(id);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_grid(size: usize) -> Vec<f32> {
        let mut grid = vec![0.0f32; size * size];

        // particle 1
        let range: isize = 4;
        for x in -range..=range {
            for y in -range..=range {
                grid[(250 + x) as usize * size + (250 + y) as usize] = 1.0;
            }
        }

        // particle 2
        let range: isize = 3;
        for x in -range..=range {
            for y in -range..=range {
                grid[(5 + x) as usize * size + (5 + y) as usize] = 1.0;
            }
        }

        grid
    }

    #[test]
    fn test_check_cell() {
        let mut id_map = vec![vec![0usize; 256]; 256];
        id_map[2][2] = 1;
        id_map[3][3] = 5;

        let grid = get_grid(256);

        assert_eq!(
            check_cell((3, 3), -1, -1, 256, 256, &grid, &id_map, 256),
            Some(1)
        );
        assert_eq!(
            check_cell((3, 3), -2, -2, 256, 256, &grid, &id_map, 256),
            None
        );
        assert_eq!(
            check_cell((5, 5), -2, -2, 256, 256, &grid, &id_map, 256),
            Some(5)
        );
    }
}
