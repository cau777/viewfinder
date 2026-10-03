use std::ops::DerefMut;
use std::sync::Arc;
use array2d::Array2D;
use crate::grid::CellContents::Subcells;

pub struct FullGrid {
    first_parcel_lat: f64,
    first_parcel_lon: f64,
    last_parcel_lat: f64,
    last_parcel_lon: f64,

    alt_min: f64,
    alt_max: f64,

    contents: CellContents,
}

/// The first point is the most NW, the last point is the most SE.
type PointGrid = [[u16; 3]; 3];

/// Recursive struct for representing structs in the hierarchy of 9x9 cells
#[derive(Clone)]
pub enum CellContents {
    Points {
        points: PointGrid,
    },
    Subcells {
        cells: [[Option<Arc<CellContents>>; 3]; 3]
    },
}

impl Default for CellContents {
    fn default() -> Self {
        Subcells {
            cells: [[None, None, None], [None, None, None], [None, None, None]]
        }
    }
}

pub struct Parcel {
    lat: f64,
    lon: f64,
    points: Array2D<u16>,
}

/// parcels is a 2D. The dataset does not contain parcels for every cell.
/// It omits only ocean tiles for example. Those parcels should be None in the array.
pub fn make_cells(parcels: Array2D<Option<Parcel>>, resolution: usize, alt_min: f64, alt_max: f64) -> FullGrid {
    let mut all_points: Array2D<PointGrid> = Array2D::filled_with(
        Default::default(),
        usize::div_ceil(parcels.num_rows() * resolution, 3),
        usize::div_ceil(parcels.num_columns() * resolution, 3),
    );

    for parcel_x in 0..parcels.num_rows() {
        for parcel_y in 0..parcels.num_columns() {
            let parcel = parcels.get(parcel_x, parcel_y);
            // TODO: not all point will be filled, some will be full of zeros
            if let Some(Some(parcel)) = parcel {
                for point_x in 0..resolution {
                    for point_y in 0..resolution {
                        let point_x = parcel_x * resolution + point_x;
                        let point_y = parcel_y * resolution + point_y;
                        all_points[(point_x / 3, point_y / 3)][point_x % 3][point_y % 3] = parcel.points[(point_x, point_y)];
                    }
                }
            }
        }
    }

    // Wrap in the first layer
    let mut cells: Array2D<Box<CellContents>> = Array2D::filled_with(
        Box::new(CellContents::default()),
        usize::div_ceil(all_points.num_rows(), 3),
        usize::div_ceil(all_points.num_columns(), 3),
    );

    for point_x in 0..all_points.num_rows() {
        for point_y in 0..all_points.num_columns() {
            let cell = (&mut cells[(point_x / 3, point_y / 3)]).as_mut();
            match cell {
                Subcells { cells } => {
                    cells[point_x % 3][point_y % 3] = Some(Arc::new(CellContents::Points {
                        points: all_points[(point_x, point_y)]
                    }));
                }
                _ => panic!()
            };
        }
    }

    while cells.num_rows() > 1 && cells.num_columns() > 1 {
        let mut new_cells: Array2D<Box<CellContents>> = Array2D::filled_with(
            Box::new(CellContents::default()),
            usize::div_ceil(cells.num_rows(), 3),
            usize::div_ceil(cells.num_columns(), 3),
        );

        for cell_x in 0..cells.num_rows() {
            for cell_y in 0..cells.num_columns() {
                let cell = (&mut new_cells[(cell_x, cell_y)]).as_mut();
                match cell {
                    Subcells { cells } => {
                        cells[cell_x % 3][cell_y % 3] = Some(Arc::new(Subcells {
                            cells: cells.clone()
                        }));
                    }
                    _ => panic!()
                };
            }
        }

        cells = new_cells;
    }

    FullGrid {
        // TODO
    }
}
