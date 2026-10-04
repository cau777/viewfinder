pub mod grid;
pub mod ray_tracing;
pub mod util;
pub mod ray_collisions;

use std::path::PathBuf;
use nalgebra::Vector2;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use crate::grid::{CellContents, FullGrid, MISSING};
use crate::ray_collisions::{ray_collisions, RayCastResult};
use crate::ray_tracing::get_intersection_points;

#[pyclass(module = "viewfinder_core._native")]
#[derive(Debug)]
pub struct RayTracer {
    grid: FullGrid
}

/// Shape and memory use of the loaded tree, returned by `RayTracer.stats()`.
#[pyclass(module = "viewfinder_core._native", get_all, frozen)]
#[derive(Debug)]
pub struct GridStats {
    /// Nodes per tree level, root first; the last level holds the 3x3-point leaves.
    nodes_per_level: Vec<usize>,
    /// Child slots per level that are None because the whole subtree has no points.
    empty_slots_per_level: Vec<usize>,
    /// Points with an altitude.
    points_present: usize,
    /// Points without data inside non-empty leaves.
    points_missing_in_leaves: usize,
    /// Approximate heap bytes used by the tree.
    tree_bytes: usize,
}

#[pymethods]
impl GridStats {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[pymethods]
impl RayTracer {
    /// RayTracer(path)
    ///
    /// Loads the u16 LiDAR export (a directory with `index.csv` and `<NAME>.u16` files).
    #[new]
    fn py_new(path: PathBuf) -> PyResult<Self> {
        Ok(Self {
            grid: FullGrid::load(&path)?
        })
    }

    /// Number of point rows (north to south).
    #[getter]
    fn rows(&self) -> usize {
        self.grid.rows
    }

    /// Number of point columns (west to east).
    #[getter]
    fn columns(&self) -> usize {
        self.grid.columns
    }

    /// Levels in the tree, including the leaf level.
    #[getter]
    fn depth(&self) -> u32 {
        self.grid.depth
    }

    /// Distance between neighbouring points, in metres.
    #[getter]
    fn cell_size(&self) -> f64 {
        self.grid.cell_size
    }

    /// UTM (EPSG:26910) x of the most NW point.
    #[getter]
    fn x_start(&self) -> f64 {
        self.grid.x_start
    }

    /// UTM (EPSG:26910) y of the most NW point.
    #[getter]
    fn y_start(&self) -> f64 {
        self.grid.y_start
    }

    /// Altitude range (metres) of the whole dataset.
    #[getter]
    fn alt_range(&self) -> (f64, f64) {
        (self.grid.alt_min, self.grid.alt_max)
    }

    /// Altitude in metres of the point at (row, column), None if it has no data.
    fn altitude(&self, row: usize, column: usize) -> Option<f64> {
        self.grid.altitude(row, column)
    }

    /// Altitude in metres of the point nearest to UTM (x, y), None if it has no data.
    fn altitude_at(&self, x: f64, y: f64) -> Option<f64> {
        self.grid.altitude_at(x, y)
    }

    /// cast_ray(x, y, direction_x, direction_y)
    ///
    /// Points whose cell a horizontal ray from UTM (x, y) crosses, ignoring altitude, nearest first.
    /// Returns `(x, y, altitude)` per point; altitude is None where a point has no data.
    fn cast_ray(&self, x: f64, y: f64, direction_x: f64, direction_y: f64) -> PyResult<Vec<(f64, f64, Option<f64>)>> {
        let direction = Vector2::new(direction_x, direction_y);
        if direction.norm() == 0.0 || !direction.norm().is_finite() {
            return Err(PyValueError::new_err("direction must be a finite, non-zero vector"));
        }
        let hits = get_intersection_points(&self.grid, Vector2::new(x, y), direction.normalize());
        Ok(hits.into_iter().map(|h| (h.position.x, h.position.y, h.altitude)).collect())
    }

    /// ray_collisions(x, y, direction_x, direction_y, observer_altitude, min_elevation, max_elevation, resolution)
    ///
    /// Casts `resolution` rays from UTM (x, y) at `observer_altitude` metres, in one horizontal direction,
    /// at elevation angles (radians) evenly spaced from `min_elevation` to `max_elevation`, both included.
    /// Returns one value per angle, lowest first: the distance in metres the ray travels before hitting
    /// the surface, `inf` if it probably reaches the sky, or `-inf` if it probably reaches the ocean.
    #[allow(clippy::too_many_arguments)]
    fn ray_collisions(
        &self, x: f64, y: f64, direction_x: f64, direction_y: f64,
        observer_altitude: f64, min_elevation: f64, max_elevation: f64, resolution: usize,
    ) -> PyResult<Vec<f64>> {
        let direction = Vector2::new(direction_x, direction_y);
        if direction.norm() == 0.0 || !direction.norm().is_finite() {
            return Err(PyValueError::new_err("direction must be a finite, non-zero vector"));
        }
        if min_elevation.is_nan() || max_elevation.is_nan() || min_elevation > max_elevation {
            return Err(PyValueError::new_err("elevations must be numbers with min_elevation <= max_elevation"));
        }
        let results = ray_collisions(
            &self.grid, Vector2::new(x, y), direction.normalize(), observer_altitude,
            max_elevation, min_elevation, resolution,
        );
        Ok(results
            .into_iter()
            .map(|r| match r {
                RayCastResult::Collision { distance } => distance,
                RayCastResult::ProbablySky => f64::INFINITY,
                RayCastResult::ProbablyOcean => f64::NEG_INFINITY,
            })
            .collect())
    }

    /// Walk the tree and count nodes, points and memory.
    fn stats(&self) -> GridStats {
        let tree = self.grid.tree_stats();
        // Each node is one Arc allocation (two refcounts + the enum); internal nodes also box their child array
        let node_bytes = 2 * size_of::<usize>() + size_of::<CellContents>();
        let internal: usize = tree.nodes_per_level.iter().rev().skip(1).sum();
        let tree_bytes = tree.nodes_per_level.iter().sum::<usize>() * node_bytes
            + internal * size_of::<grid::SubcellGrid>();
        GridStats {
            nodes_per_level: tree.nodes_per_level,
            empty_slots_per_level: tree.empty_slots_per_level,
            points_present: tree.points_present,
            points_missing_in_leaves: tree.points_missing_in_leaves,
            tree_bytes,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "RayTracer(rows={}, columns={}, depth={}, cell_size={}, missing={})",
            self.grid.rows, self.grid.columns, self.grid.depth, self.grid.cell_size, MISSING
        )
    }
}

/// Native core of viewfinder (Rust, via PyO3).
// Declarative (inline) module: required for `experimental-inspect` stub generation.
#[pymodule]
mod _native {
    #[pymodule_export]
    use super::{GridStats, RayTracer};

    #[allow(non_upper_case_globals)]
    #[pymodule_export]
    const __version__: &str = env!("CARGO_PKG_VERSION");
}
