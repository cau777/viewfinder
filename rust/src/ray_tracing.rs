use crate::grid::{CellContents, FullGrid};
use nalgebra::Vector2;
use smallvec::SmallVec;

#[derive(Copy, Clone)]
struct PossibleIntersection<'a> {
    cell: &'a CellContents,
    /// NW corner of the cell (UTM, metres)
    x_start: f64,
    y_start: f64,
    /// Side of each of the 3x3 children (metres)
    scale: f64,
    /// Distance along the ray at which it enters the cell
    approach_to_observer: f64,
    /// Distance along the ray at which it leaves the cell
    exit_from_observer: f64,
}

#[derive(Copy, Clone, Debug)]
pub struct Intersection {
    pub position: Vector2<f64>,
    pub altitude: Option<f64>,
    pub top_distance_to_observer: f64,
}

/// Distances along the ray at which it enters and leaves the axis-aligned square [min, max], or None
/// if it misses. Entry is 0 if the observer is inside. Slab method: intersect the ray's ranges for x and y.
fn ray_crosses_square(origin: Vector2<f64>, direction: Vector2<f64>, min: Vector2<f64>, max: Vector2<f64>) -> Option<(f64, f64)> {
    let mut t_enter = 0.0f64;
    let mut t_exit = f64::INFINITY;
    for axis in 0..2 {
        if direction[axis] == 0.0 {
            // Parallel to this slab: either always inside it or never
            if origin[axis] < min[axis] || origin[axis] > max[axis] {
                return None;
            }
            continue;
        }
        let t1 = (min[axis] - origin[axis]) / direction[axis];
        let t2 = (max[axis] - origin[axis]) / direction[axis];
        t_enter = t_enter.max(t1.min(t2));
        t_exit = t_exit.min(t1.max(t2));
    }
    (t_enter <= t_exit).then_some((t_enter, t_exit))
}

/// Walks the points whose 0.5 m cell a ray crosses (ignoring altitude), nearest first.
/// Points without data inside the dataset are included with altitude None; empty subtrees are skipped.
/// Lazy, so callers can stop early or skip subtrees with `next_unless`.
pub struct IntersectionWalk<'a> {
    grid: &'a FullGrid,
    origin: Vector2<f64>,
    direction: Vector2<f64>,
    /// Centre of the last column/row: the leaves past them are padding up to 3^depth points
    x_last: f64,
    y_last: f64,
    /// Depth-first, nearest child first: pops from the end
    stack: Vec<PossibleIntersection<'a>>,
    /// Points of the last leaf expanded, farthest first
    pending: SmallVec<[Intersection; 9]>,
}

impl<'a> IntersectionWalk<'a> {
    /// All positions are in UTM (meters). `direction` must be normalized.
    pub fn new(grid: &'a FullGrid, origin: Vector2<f64>, direction: Vector2<f64>) -> Self {
        let mut walk = IntersectionWalk {
            grid,
            origin,
            direction,
            x_last: grid.x_start + grid.columns.saturating_sub(1) as f64 * grid.cell_size,
            y_last: grid.y_start - grid.rows.saturating_sub(1) as f64 * grid.cell_size,
            stack: Vec::new(),
            pending: SmallVec::new(),
        };
        if let Some(root) = grid.contents.as_deref() {
            // grid.x_start/y_start is the centre of the NW point; the tree starts at its corner
            let (x_start, y_start) = (grid.x_start - grid.cell_size / 2.0, grid.y_start + grid.cell_size / 2.0);
            let scale = grid.cell_size * 3f64.powi(grid.depth as i32 - 1);
            let min = Vector2::new(x_start, y_start - 3.0 * scale);
            let max = Vector2::new(x_start + 3.0 * scale, y_start);
            if let Some((approach_to_observer, exit_from_observer)) = ray_crosses_square(origin, direction, min, max) {
                walk.stack.push(PossibleIntersection { cell: root, x_start, y_start, scale, approach_to_observer, exit_from_observer });
            }
        }
        walk
    }

    /// Next point along the ray, skipping every subtree for which `skip(enter, exit, max_altitude)` is
    /// true: the ray crosses it between distances `enter` and `exit`, and its highest point is `max_altitude`.
    pub fn next_unless(&mut self, mut skip: impl FnMut(f64, f64, f64) -> bool) -> Option<Intersection> {
        loop {
            if let Some(hit) = self.pending.pop() {
                return Some(hit);
            }
            let cell = self.stack.pop()?;
            // Never MISSING: empty subtrees are not in the tree
            let max_altitude = self.grid.decompress_altitude(cell.cell.max()).unwrap_or(f64::INFINITY);
            if !skip(cell.approach_to_observer, cell.exit_from_observer, max_altitude) {
                self.expand(cell);
            }
        }
    }

    fn expand(&mut self, PossibleIntersection { cell, x_start, y_start, scale, .. }: PossibleIntersection<'a>) {
        let mut possible_collisions = SmallVec::<[PossibleIntersection; 9]>::new();

        // i is the row (going south, y decreasing), j the column (going east)
        for i in 0..3 {
            for j in 0..3 {
                let min = Vector2::new(x_start + j as f64 * scale, y_start - (i + 1) as f64 * scale);
                let max = min + Vector2::new(scale, scale);
                let Some((approach_to_observer, exit_from_observer)) = ray_crosses_square(self.origin, self.direction, min, max) else {
                    continue;
                };

                match cell {
                    CellContents::Subcells { cells, .. } => {
                        if let Some(subcell) = &cells[i][j] {
                            possible_collisions.push(PossibleIntersection {
                                cell: subcell,
                                x_start: min.x,
                                y_start: max.y,
                                scale: scale / 3.0,
                                approach_to_observer,
                                exit_from_observer,
                            });
                        }
                    }
                    CellContents::Points { points, .. } => {
                        let position = (min + max) / 2.0;
                        if position.x <= self.x_last && position.y >= self.y_last {
                            self.pending.push(Intersection {
                                position,
                                altitude: self.grid.decompress_altitude(points[i][j]),
                                top_distance_to_observer: approach_to_observer,
                            });
                        }
                    }
                };
            }
        }

        // Cells along a ray don't overlap, so the order they are entered in is the order of everything inside them.
        // Both are popped from the end, so farthest first.
        self.pending.sort_by(|a, b| b.top_distance_to_observer.total_cmp(&a.top_distance_to_observer));
        possible_collisions.sort_by(|a, b| b.approach_to_observer.total_cmp(&a.approach_to_observer));
        self.stack.extend(possible_collisions);
    }
}

impl Iterator for IntersectionWalk<'_> {
    type Item = Intersection;

    fn next(&mut self) -> Option<Intersection> {
        self.next_unless(|_, _, _| false)
    }
}

/// All points whose 0.5 m cell the ray crosses (ignoring altitude), sorted by distance from the observer.
/// Points without data inside the dataset are included with altitude None; empty subtrees are skipped.
/// All positions are in UTM (meters)
/// observer_direction must be normalized
pub fn get_intersection_points(
    grid: &FullGrid,
    observer_position: Vector2<f64>,
    observer_direction: Vector2<f64>,
) -> Vec<Intersection> {
    IntersectionWalk::new(grid, observer_position, observer_direction).collect() // Closer points at the front
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{make_cells, Parcel, MISSING};
    use array2d::Array2D;

    /// 3x4 parcels of 7x7 points (0.5 m) with one parcel absent and some points missing.
    fn synthetic_grid() -> FullGrid {
        let resolution = 7;
        let mut parcels = Array2D::filled_by_row_major(|| None, 3, 4);
        for r in 0..3 {
            for c in 0..4 {
                if (r, c) == (1, 2) {
                    continue;
                }
                let points = Array2D::filled_by_row_major(|| 0u16, resolution, resolution);
                parcels[(r, c)] = Some(Parcel { x_start: 0.0, y_start: 0.0, points });
                let parcel = parcels[(r, c)].as_mut().unwrap();
                for pr in 0..resolution {
                    for pc in 0..resolution {
                        let (gr, gc) = (r * resolution + pr, c * resolution + pc);
                        parcel.points[(pr, pc)] = if (gr + 2 * gc) % 11 == 0 { MISSING } else { (gr * 100 + gc) as u16 };
                    }
                }
            }
        }
        let (contents, depth) = make_cells(&parcels, resolution);
        FullGrid {
            x_start: 1000.25, y_start: 2000.25, cell_size: 0.5,
            rows: 3 * resolution, columns: 4 * resolution,
            depth, alt_min: 0.0, alt_max: (MISSING - 1) as f64, contents, classes: None,
        }
    }

    fn row_column(grid: &FullGrid, position: Vector2<f64>) -> (usize, usize) {
        (
            ((grid.y_start - position.y) / grid.cell_size).round() as usize,
            ((position.x - grid.x_start) / grid.cell_size).round() as usize,
        )
    }

    /// Every point of the grid tested on its own, without the tree. The tree drops only whole empty
    /// subtrees, so points of an absent parcel are still reported (altitude None) if their 3x3 leaf has data.
    fn brute_force(grid: &FullGrid, origin: Vector2<f64>, direction: Vector2<f64>) -> Vec<(usize, usize)> {
        let half = Vector2::new(grid.cell_size / 2.0, grid.cell_size / 2.0);
        let mut hits = Vec::new();
        for row in 0..grid.rows {
            for column in 0..grid.columns {
                let centre = Vector2::new(grid.x_start + column as f64 * grid.cell_size, grid.y_start - row as f64 * grid.cell_size);
                let (leaf_row, leaf_column) = (row / 3 * 3, column / 3 * 3);
                let leaf_has_data = (leaf_row..leaf_row + 3)
                    .any(|r| (leaf_column..leaf_column + 3).any(|c| grid.point(r, c).is_some()));
                if !leaf_has_data {
                    continue;
                }
                if ray_crosses_square(origin, direction, centre - half, centre + half).is_some() {
                    hits.push((row, column));
                }
            }
        }
        hits
    }

    fn grid_parcel_absent(row: usize, column: usize) -> bool {
        (row / 7, column / 7) == (1, 2)
    }

    fn rays() -> Vec<(Vector2<f64>, Vector2<f64>)> {
        let mut rays = Vec::new();
        let origins = [
            Vector2::new(1003.1, 1996.7),  // inside the grid
            Vector2::new(1007.0, 1995.0),  // on a cell corner
            Vector2::new(990.0, 2010.0),   // outside, NW
            Vector2::new(1020.0, 1985.0),  // outside, SE
            Vector2::new(1008.9, 1993.2),  // inside the absent parcel
        ];
        for origin in origins {
            for k in 0..24 {
                let angle = k as f64 * std::f64::consts::TAU / 24.0 + 0.0123 * (k % 3) as f64;
                rays.push((origin, Vector2::new(angle.cos(), angle.sin())));
            }
            // Exactly axis-aligned
            for direction in [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)] {
                rays.push((origin, Vector2::new(direction.0, direction.1)));
            }
        }
        rays
    }

    #[test]
    fn matches_brute_force() {
        let grid = synthetic_grid();
        for (origin, direction) in rays() {
            let hits = get_intersection_points(&grid, origin, direction);
            let mut got: Vec<_> = hits.iter().map(|h| row_column(&grid, h.position)).collect();
            got.sort();
            let mut expected = brute_force(&grid, origin, direction);
            expected.sort();
            assert_eq!(got, expected, "ray from {origin:?} towards {direction:?}");
        }
    }

    #[test]
    fn sorted_by_distance_and_altitudes_match() {
        let grid = synthetic_grid();
        for (origin, direction) in rays() {
            let hits = get_intersection_points(&grid, origin, direction);
            let along: Vec<f64> = hits.iter().map(|h| (h.position - origin).dot(&direction)).collect();
            // Sorted by entry distance; cell centres are within a cell diagonal of it (several cells are
            // entered at distance 0 when the observer is on a corner)
            let tolerance = grid.cell_size * std::f64::consts::SQRT_2;
            assert!(along.windows(2).all(|w| w[1] >= w[0] - tolerance), "unsorted for ray from {origin:?}");
            for hit in &hits {
                let (row, column) = row_column(&grid, hit.position);
                assert_eq!(hit.altitude, grid.altitude(row, column));
            }
        }
    }

    #[test]
    fn dense_sampling_is_covered() {
        // Points sampled along the ray must all be in cells the ray was reported to cross
        let grid = synthetic_grid();
        for (origin, direction) in rays() {
            let hits: std::collections::HashSet<_> =
                get_intersection_points(&grid, origin, direction).iter().map(|h| row_column(&grid, h.position)).collect();
            for step in 0..4000 {
                let p = origin + direction * (step as f64 * 0.01);
                let (row, column) = ((grid.y_start + 0.25 - p.y) / 0.5, (p.x - grid.x_start + 0.25) / 0.5);
                if row < 0.0 || column < 0.0 || row >= grid.rows as f64 || column >= grid.columns as f64 {
                    continue;
                }
                // On a cell edge, which side a sample falls on is down to rounding
                let on_edge = |v: f64| (v - v.round()).abs() < 1e-6;
                if on_edge(row) || on_edge(column) {
                    continue;
                }
                let (row, column) = (row.floor() as usize, column.floor() as usize);
                if grid_parcel_absent(row, column) {
                    continue;
                }
                assert!(hits.contains(&(row, column)), "missed ({row}, {column}) on ray from {origin:?}");
            }
        }
    }

    #[test]
    fn ray_missing_the_grid_is_empty() {
        let grid = synthetic_grid();
        let away = get_intersection_points(&grid, Vector2::new(990.0, 2010.0), Vector2::new(-1.0, 0.0));
        assert!(away.is_empty());
    }
}
