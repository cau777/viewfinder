use crate::grid::CellContents::{Points, Subcells};
use array2d::Array2D;
use serde::Deserialize;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;

/// Value the u16 export uses for points without any LiDAR return.
pub const MISSING: u16 = u16::MAX;

#[derive(Debug)]
pub struct FullGrid {
    /// UTM (EPSG:26910) centre of the most NW point of the grid, in metres.
    pub x_start: f64,
    pub y_start: f64,
    /// Distance between neighbouring points, in metres.
    pub cell_size: f64,

    /// Size of the point grid. The tree covers 3^depth x 3^depth points; the rest is empty.
    pub rows: usize,
    pub columns: usize,
    /// Number of levels in the tree, including the leaf (`Points`) level.
    pub depth: u32,

    pub alt_min: f64,
    pub alt_max: f64,

    /// None if the dataset has no points at all.
    pub contents: Option<Arc<CellContents>>,
}

/// The first point is the most NW, the last point is the most SE. Indexed as `[row][column]`.
pub type PointGrid = [[u16; 3]; 3];

pub type SubcellGrid = [[Option<Arc<CellContents>>; 3]; 3];

/// Recursive struct for representing structs in the hierarchy of 9x9 cells.
/// Subtrees without a single point (ocean, area outside the parcels) are None.
#[derive(Clone, Debug)]
pub enum CellContents {
    Points {
        points: PointGrid,
    },
    Subcells {
        // Boxed so leaves don't pay for the 72-byte child array: keeps the enum at 24 bytes
        cells: Box<SubcellGrid>,
    },
}

pub struct Parcel {
    /// UTM centre of the most NW point.
    pub x_start: f64,
    pub y_start: f64,
    pub points: Array2D<u16>,
}

fn indices(rows: usize, columns: usize) -> impl Iterator<Item = (usize, usize)> {
    (0..rows).flat_map(move |r| (0..columns).map(move |c| (r, c)))
}

/// parcels is a 2D. The dataset does not contain parcels for every cell.
/// It omits only ocean tiles for example. Those parcels should be None in the array.
///
/// Builds the tree bottom-up: 3x3 points form a leaf, 3x3 leaves form a cell, 3x3 cells the next
/// level, until a single cell remains. Returns the root and the depth of the tree.
pub fn make_cells(
    parcels: &Array2D<Option<Parcel>>,
    resolution: usize,
) -> (Option<Arc<CellContents>>, u32) {
    let rows = parcels.num_rows() * resolution;
    let columns = parcels.num_columns() * resolution;
    let point = |row: usize, column: usize| -> u16 {
        if row >= rows || column >= columns {
            return MISSING;
        }
        match &parcels[(row / resolution, column / resolution)] {
            Some(parcel) => parcel.points[(row % resolution, column % resolution)],
            None => MISSING,
        }
    };

    // First layer: leaves of 3x3 points
    let (leaf_rows, leaf_columns) = (rows.div_ceil(3), columns.div_ceil(3));
    let leaves = indices(leaf_rows, leaf_columns).map(|(leaf_row, leaf_column)| {
        let mut points: PointGrid = [[MISSING; 3]; 3];
        for (dr, points_row) in points.iter_mut().enumerate() {
            for (dc, p) in points_row.iter_mut().enumerate() {
                *p = point(leaf_row * 3 + dr, leaf_column * 3 + dc);
            }
        }
        let empty = points.iter().flatten().all(|&p| p == MISSING);
        (!empty).then(|| Arc::new(Points { points }))
    });
    let mut cells = Array2D::from_iter_row_major(leaves, leaf_rows, leaf_columns).unwrap();
    let mut depth = 1;

    // Group 3x3 cells into a parent until the final grid cell is a single cell
    while cells.num_rows() > 1 || cells.num_columns() > 1 {
        let (new_rows, new_columns) = (
            cells.num_rows().div_ceil(3),
            cells.num_columns().div_ceil(3),
        );
        let parents = indices(new_rows, new_columns).map(|(cell_row, cell_column)| {
            let mut sub: SubcellGrid = Default::default();
            for (dr, sub_row) in sub.iter_mut().enumerate() {
                for (dc, child) in sub_row.iter_mut().enumerate() {
                    *child = cells
                        .get_mut(cell_row * 3 + dr, cell_column * 3 + dc)
                        .and_then(Option::take);
                }
            }
            let empty = sub.iter().flatten().all(Option::is_none);
            (!empty).then(|| {
                Arc::new(Subcells {
                    cells: Box::new(sub),
                })
            })
        });
        let new_cells = Array2D::from_iter_row_major(parents, new_rows, new_columns).unwrap();
        cells = new_cells;
        depth += 1;
    }

    (cells.get_mut(0, 0).and_then(Option::take), depth)
}

/// Node counts per level of the tree, root first.
#[derive(Debug, Default)]
pub struct TreeStats {
    pub nodes_per_level: Vec<usize>,
    pub empty_slots_per_level: Vec<usize>,
    pub points_present: usize,
    pub points_missing_in_leaves: usize,
}

impl FullGrid {
    /// Loads the u16 export (a directory with index.csv and one `<NAME>.u16` file per parcel).
    pub fn load(dir: &Path) -> io::Result<FullGrid> {
        let rows = read_index(&dir.join("index.csv"))?;
        let first = rows
            .first()
            .ok_or_else(|| invalid("index.csv has no parcels"))?;
        let resolution = first.width;
        if rows
            .iter()
            .any(|r| r.width != resolution || r.height != resolution)
        {
            return Err(invalid(
                "all parcels must be square with the same resolution",
            ));
        }
        if rows
            .iter()
            .any(|r| r.alt_min != first.alt_min || r.alt_max != first.alt_max)
        {
            return Err(invalid("parcels have different altitude ranges; expected the u16 export with a global range"));
        }
        let cell_size = (first.x_end - first.x_start) / (resolution - 1) as f64;
        let parcel_size = cell_size * resolution as f64;

        let x_start = rows.iter().map(|r| r.x_start).fold(f64::INFINITY, f64::min);
        let y_start = rows
            .iter()
            .map(|r| r.y_start)
            .fold(f64::NEG_INFINITY, f64::max);
        let slot = |r: &IndexRow| {
            (
                ((y_start - r.y_start) / parcel_size).round() as usize,
                ((r.x_start - x_start) / parcel_size).round() as usize,
            )
        };
        let parcel_rows = rows.iter().map(|r| slot(r).0).max().unwrap() + 1;
        let parcel_columns = rows.iter().map(|r| slot(r).1).max().unwrap() + 1;

        let mut parcels: Array2D<Option<Parcel>> =
            Array2D::filled_by_row_major(|| None, parcel_rows, parcel_columns);
        for r in &rows {
            let bytes = fs::read(dir.join(&r.file))?;
            if bytes.len() != resolution * resolution * 2 {
                return Err(invalid(&format!(
                    "{}: expected {} bytes, got {}",
                    r.file,
                    resolution * resolution * 2,
                    bytes.len()
                )));
            }
            let values: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&b| u16::from_le_bytes(b))
                .collect();
            let (parcel_row, parcel_column) = slot(r);
            parcels[(parcel_row, parcel_column)] = Some(Parcel {
                x_start: r.x_start,
                y_start: r.y_start,
                points: Array2D::from_row_major(&values, resolution, resolution).unwrap(),
            });
        }

        let (contents, depth) = make_cells(&parcels, resolution);
        Ok(FullGrid {
            x_start,
            y_start,
            cell_size,
            rows: parcel_rows * resolution,
            columns: parcel_columns * resolution,
            depth,
            alt_min: first.alt_min,
            alt_max: first.alt_max,
            contents,
        })
    }

    /// Raw u16 value of a point, None if it has no data or is outside the grid.
    pub fn point(&self, row: usize, column: usize) -> Option<u16> {
        if row >= self.rows || column >= self.columns {
            return None;
        }
        let mut node = self.contents.as_deref()?;
        let mut span = 3usize.pow(self.depth);
        loop {
            span /= 3;
            let (i, j) = ((row / span) % 3, (column / span) % 3);
            match node {
                Subcells { cells } => node = cells[i][j].as_deref()?,
                Points { points } => return Some(points[i][j]).filter(|&p| p != MISSING),
            }
        }
    }

    pub fn decompress_altitude(&self, p: u16) -> Option<f64> {
        if p == MISSING {
            None
        } else {
            Some(self.alt_min + p as f64 / (MISSING - 1) as f64 * (self.alt_max - self.alt_min))
        }
    }

    /// Altitude in metres of a point, None if it has no data or is outside the grid.
    pub fn altitude(&self, row: usize, column: usize) -> Option<f64> {
        self.point(row, column)
            .and_then(|p| self.decompress_altitude(p))
    }

    /// Altitude of the point nearest to UTM coordinates (x, y).
    pub fn altitude_at(&self, x: f64, y: f64) -> Option<f64> {
        let column = ((x - self.x_start) / self.cell_size).round();
        let row = ((self.y_start - y) / self.cell_size).round();
        if row < 0.0 || column < 0.0 {
            return None;
        }
        self.altitude(row as usize, column as usize)
    }

    pub fn tree_stats(&self) -> TreeStats {
        fn visit(node: &CellContents, level: usize, stats: &mut TreeStats) {
            stats.nodes_per_level[level] += 1;
            match node {
                Points { points } => {
                    let present = points.iter().flatten().filter(|&&p| p != MISSING).count();
                    stats.points_present += present;
                    stats.points_missing_in_leaves += 9 - present;
                }
                Subcells { cells } => {
                    for child in cells.iter().flatten() {
                        match child {
                            Some(child) => visit(child, level + 1, stats),
                            None => stats.empty_slots_per_level[level + 1] += 1,
                        }
                    }
                }
            }
        }
        let levels = self.depth as usize;
        let mut stats = TreeStats {
            nodes_per_level: vec![0; levels],
            empty_slots_per_level: vec![0; levels],
            ..Default::default()
        };
        if let Some(root) = &self.contents {
            visit(root, 0, &mut stats);
        }
        stats
    }
}

/// The columns of index.csv this loader needs.
#[derive(Deserialize)]
struct IndexRow {
    file: String,
    width: usize,
    height: usize,
    x_start: f64,
    y_start: f64,
    x_end: f64,
    alt_min: f64,
    alt_max: f64,
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

fn read_index(path: &Path) -> io::Result<Vec<IndexRow>> {
    // Columns not in IndexRow (name, crs, lat/lon, ...) are ignored
    let rows = csv::Reader::from_path(path)?
        .deserialize()
        .collect::<Result<Vec<IndexRow>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2x3 parcels of 6x6 points, middle-bottom parcel absent; value encodes the global position.
    fn synthetic() -> (Array2D<Option<Parcel>>, usize) {
        let resolution = 6;
        let mut parcels = Array2D::filled_by_row_major(|| None, 2, 3);
        for (pr, pc) in indices(2, 3).filter(|&p| p != (1, 1)) {
            let points = Array2D::filled_by_row_major(|| 0, resolution, resolution);
            parcels[(pr, pc)] = Some(Parcel {
                x_start: pc as f64,
                y_start: -(pr as f64),
                points,
            });
            let parcel = parcels[(pr, pc)].as_mut().unwrap();
            for (r, c) in indices(resolution, resolution) {
                let (gr, gc) = (pr * resolution + r, pc * resolution + c);
                // One missing point inside a parcel too
                parcel.points[(r, c)] = if (gr, gc) == (0, 1) {
                    MISSING
                } else {
                    (gr * 100 + gc) as u16
                };
            }
        }
        (parcels, resolution)
    }

    fn grid_from(parcels: &Array2D<Option<Parcel>>, resolution: usize) -> FullGrid {
        let (contents, depth) = make_cells(parcels, resolution);
        FullGrid {
            x_start: 0.0,
            y_start: 0.0,
            cell_size: 1.0,
            rows: parcels.num_rows() * resolution,
            columns: parcels.num_columns() * resolution,
            depth,
            alt_min: 0.0,
            alt_max: (MISSING - 1) as f64,
            contents,
        }
    }

    #[test]
    fn every_point_round_trips() {
        let (parcels, resolution) = synthetic();
        let grid = grid_from(&parcels, resolution);
        // 12x18 points -> 4x6 leaves -> 2x2 -> 1x1
        assert_eq!(grid.depth, 3);
        for (r, c) in indices(grid.rows, grid.columns) {
            let expected = parcels[(r / resolution, c / resolution)]
                .as_ref()
                .map(|p| p.points[(r % resolution, c % resolution)])
                .filter(|&p| p != MISSING);
            assert_eq!(grid.point(r, c), expected, "point ({r}, {c})");
        }
        assert_eq!(grid.point(0, 1), None);
        assert_eq!(grid.point(7, 7), None); // absent parcel
        assert_eq!(grid.point(grid.rows, 0), None);
        assert_eq!(grid.altitude(2, 3), Some(203.0));
    }

    #[test]
    fn empty_subtrees_are_none() {
        let (parcels, resolution) = synthetic();
        let grid = grid_from(&parcels, resolution);
        let stats = grid.tree_stats();
        // The absent parcel covers leaves (2..4, 2..4): those 4 leaves are None
        assert_eq!(stats.nodes_per_level, vec![1, 4, 20]);
        assert_eq!(stats.points_present, 5 * 36 - 1);
        assert_eq!(stats.points_missing_in_leaves, 1);
    }

    #[test]
    fn non_square_grid_reduces_to_one_root() {
        let mut parcels = Array2D::filled_by_row_major(|| None, 1, 5);
        parcels[(0, 4)] = Some(Parcel {
            x_start: 0.0,
            y_start: 0.0,
            points: Array2D::filled_by_row_major(|| 7, 3, 3),
        });
        let grid = grid_from(&parcels, 3);
        assert_eq!(grid.depth, 3); // 3x15 points -> 1x5 leaves -> 1x2 -> 1x1
        assert_eq!(grid.point(2, 14), Some(7));
        assert_eq!(grid.point(0, 0), None);
    }

    #[test]
    fn loads_export_directory() {
        let dir = std::env::temp_dir().join(format!("viewfinder-grid-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        // Two 2x2 parcels side by side, 0.5 m cells
        let header = "name,file,crs,width,height,x_start,y_start,x_end,y_end,alt_min,alt_max\n";
        let rows = "b,b.u16,EPSG:26910,2,2,101.25,200.75,101.75,200.25,-10,10\n\
                    a,a.u16,EPSG:26910,2,2,100.25,200.75,100.75,200.25,-10,10\n";
        fs::write(dir.join("index.csv"), format!("{header}{rows}")).unwrap();
        let encode = |v: [u16; 4]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
        fs::write(dir.join("a.u16"), encode([0, 1, 2, MISSING])).unwrap();
        fs::write(dir.join("b.u16"), encode([MISSING - 1, 5, 6, 7])).unwrap();

        let grid = FullGrid::load(&dir).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!((grid.rows, grid.columns, grid.cell_size), (2, 4, 0.5));
        assert_eq!((grid.x_start, grid.y_start), (100.25, 200.75));
        assert_eq!(grid.point(0, 1), Some(1));
        assert_eq!(grid.point(1, 1), None);
        assert_eq!(grid.altitude(0, 0), Some(-10.0));
        assert_eq!(grid.altitude(0, 2), Some(10.0));
        assert_eq!(grid.altitude_at(101.75, 200.25), grid.altitude(1, 3));
    }
}
