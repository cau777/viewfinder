use nalgebra::Vector2;
use crate::grid::FullGrid;
use crate::ray_collisions::RayCastResult::{Collision, ProbablyOcean, ProbablySky};
use crate::ray_tracing::get_intersection_points;

/// Mean Earth radius in metres.
const EARTH_RADIUS: f64 = 6_371_000.0;
/// Standard atmospheric refraction coefficient: light bends down slightly, so the Earth looks flatter.
const REFRACTION: f64 = 0.13;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RayCastResult {
    Collision {
        /// Distance travelled by the ray (along the slope, not the horizontal), metres
        distance: f64,
    },
    /// Pointing up and left the dataset without hitting anything
    ProbablySky,
    /// Pointing down and left the dataset without hitting anything: the dataset has no points over open water
    ProbablyOcean,
}

/// How far the surface at horizontal distance `distance` sits below the observer's horizontal plane
/// because of the Earth's curvature (reduced by refraction).
fn curvature_drop(distance: f64) -> f64 {
    distance * distance * (1.0 - REFRACTION) / (2.0 * EARTH_RADIUS)
}

/// Casts rays from an observer at `resolution` elevation angles, evenly spaced from
/// `min_elevation_angle` to `max_elevation_angle` (both included), all in one horizontal direction.
/// Returns one result per angle, lowest angle first. A ray hits the first point whose surface is at
/// or above it.
///
/// Angles are in radians and `min_elevation_angle <= max_elevation_angle`. Positions are UTM metres,
/// `observer_direction` is normalized and `observer_altitude` uses the same datum as the grid.
pub fn ray_collisions(grid: &FullGrid,
                      observer_position: Vector2<f64>,
                      observer_direction: Vector2<f64>,
                      observer_altitude: f64,
                      max_elevation_angle: f64,
                      min_elevation_angle: f64,
                      resolution: usize
) -> Vec<RayCastResult> {
    debug_assert!(min_elevation_angle <= max_elevation_angle);
    // Sorted by distance, so the first point above a ray is where it collides
    let points = get_intersection_points(grid, observer_position, observer_direction);
    // A ray passing above a point also passes above it at every higher angle, so each angle
    // continues the search where the previous one stopped: all angles together cost one walk.
    let mut next_point = 0;

    (0..resolution)
        .map(|i| {
            let fraction = if resolution > 1 { i as f64 / (resolution - 1) as f64 } else { 0.0 };
            let angle = min_elevation_angle + (max_elevation_angle - min_elevation_angle) * fraction;
            let slope = angle.tan();

            while let Some(point) = points.get(next_point) {
                if let Some(point_altitude) = point.altitude {
                    let distance = point.top_distance_to_observer;
                    let ray_altitude = observer_altitude + distance * slope;
                    if ray_altitude <= point_altitude - curvature_drop(distance) {
                        return Collision { distance: distance / angle.cos() };
                    }
                }
                next_point += 1;
            }

            if angle < 0.0 { ProbablyOcean } else { ProbablySky }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{make_cells, Parcel, MISSING};
    use array2d::Array2D;

    /// Square grid with altitudes in metres (None = no data), first point centred at (0, 0).
    fn grid_from(altitudes: &[Vec<Option<f64>>], cell_size: f64) -> FullGrid {
        let resolution = altitudes.len();
        let mut points = Array2D::filled_by_row_major(|| MISSING, resolution, resolution);
        for (r, row) in altitudes.iter().enumerate() {
            for (c, altitude) in row.iter().enumerate() {
                // alt_min = 0 and alt_max = MISSING - 1 make the stored value the altitude in metres
                points[(r, c)] = altitude.map_or(MISSING, |a| a as u16);
            }
        }
        let mut parcels = Array2D::filled_by_row_major(|| None, 1, 1);
        parcels[(0, 0)] = Some(Parcel { x_start: 0.0, y_start: 0.0, points });
        let (contents, depth) = make_cells(&parcels, resolution);
        FullGrid {
            x_start: 0.0, y_start: 0.0, cell_size,
            rows: resolution, columns: resolution,
            depth, alt_min: 0.0, alt_max: (MISSING - 1) as f64, contents,
        }
    }

    /// Flat ground at 10 m with a 30 m wall from column 40 to 44, then no data (water) from column 50.
    fn wall_grid() -> FullGrid {
        let n = 60;
        let row: Vec<Option<f64>> = (0..n)
            .map(|c| match c {
                40..=44 => Some(30.0),
                50.. => None,
                _ => Some(10.0),
            })
            .collect();
        grid_from(&vec![row; n], 0.5)
    }

    const EAST: Vector2<f64> = Vector2::new(1.0, 0.0);

    #[test]
    fn one_result_per_angle_including_both_ends() {
        let grid = wall_grid();
        let observer = Vector2::new(0.0, -15.0);
        let results = ray_collisions(&grid, observer, EAST, 11.7, 0.5, -0.5, 7);
        assert_eq!(results.len(), 7);
        assert_eq!(results[0], ray_collisions(&grid, observer, EAST, 11.7, -0.5, -0.5, 1)[0]);
        assert_eq!(results[6], ray_collisions(&grid, observer, EAST, 11.7, 0.5, 0.5, 1)[0]);
    }

    #[test]
    fn wall_ground_and_sky() {
        let grid = wall_grid();
        let observer = Vector2::new(0.0, -15.0);
        let observer_altitude = 11.7;
        // The ray enters the wall at x = 19.75 (column 40's west edge)
        let wall_entry: f64 = 40.0 * 0.5 - 0.25;
        let wall_top_angle = ((30.0 - observer_altitude) / wall_entry).atan();

        let angles = [-0.3, -0.01, 0.0, wall_top_angle - 0.01, wall_top_angle + 0.05];
        for &angle in &angles {
            let result = ray_collisions(&grid, observer, EAST, observer_altitude, angle, angle, 1)[0];
            let expected = if angle < -0.05 {
                // Hits the ground where the ray has dropped 1.7 m: first cell entered at or after that
                let reach: f64 = 1.7 / -angle.tan();
                let entry = ((reach + 0.25) / 0.5).ceil() * 0.5 - 0.25;
                Collision { distance: entry / angle.cos() }
            } else if angle < wall_top_angle {
                Collision { distance: wall_entry / angle.cos() }
            } else {
                ProbablySky
            };
            match (result, expected) {
                (Collision { distance: a }, Collision { distance: b }) => assert!((a - b).abs() < 1e-9, "angle {angle}: {a} vs {b}"),
                _ => assert_eq!(result, expected, "angle {angle}"),
            }
        }
    }

    #[test]
    fn looking_down_over_water_is_ocean() {
        let grid = wall_grid();
        // Standing on the wall, looking just below the horizon: passes over the ground and into the water
        let observer = Vector2::new(21.0, -15.0);
        let result = ray_collisions(&grid, observer, EAST, 31.0, -0.001, -0.001, 1)[0];
        assert_eq!(result, ProbablyOcean);
    }

    #[test]
    fn matches_search_from_scratch_for_every_angle() {
        // Rough terrain with gaps; the resumed search must equal a fresh search per angle
        let n = 45;
        let mut seed = 12345u64;
        let mut next = move || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as f64 / (1u64 << 31) as f64
        };
        let altitudes: Vec<Vec<Option<f64>>> = (0..n)
            .map(|_| (0..n).map(|_| if next() < 0.1 { None } else { Some((next() * 40.0).floor()) }).collect())
            .collect();
        let grid = grid_from(&altitudes, 0.5);

        for k in 0..16 {
            let bearing = k as f64 * std::f64::consts::TAU / 16.0 + 0.1;
            let direction = Vector2::new(bearing.sin(), bearing.cos());
            let observer = Vector2::new(11.1, -10.6);
            let results = ray_collisions(&grid, observer, direction, 25.0, 0.6, -0.6, 50);
            let points = get_intersection_points(&grid, observer, direction);

            for (i, result) in results.iter().enumerate() {
                let angle = -0.6 + 1.2 * i as f64 / 49.0;
                let expected = points
                    .iter()
                    .find(|p| p.altitude.is_some_and(|a| {
                        25.0 + p.top_distance_to_observer * angle.tan() <= a - curvature_drop(p.top_distance_to_observer)
                    }))
                    .map_or(if angle < 0.0 { ProbablyOcean } else { ProbablySky }, |p| Collision {
                        distance: p.top_distance_to_observer / angle.cos(),
                    });
                assert_eq!(*result, expected, "bearing {bearing}, angle {angle}");
            }
        }
    }

    #[test]
    fn earth_curvature_hides_distant_low_targets() {
        // 100 m cells: a 20 m tower 5 km away, observed level from 19 m.
        // Flat-Earth geometry would hit it, but it is ~1.7 m below the horizon at that distance.
        let n = 60;
        let row: Vec<Option<f64>> = (0..n).map(|c| if c == 50 { Some(20.0) } else { Some(0.0) }).collect();
        let grid = grid_from(&vec![row; n], 100.0);
        let observer = Vector2::new(0.0, -1500.0);
        assert_eq!(ray_collisions(&grid, observer, EAST, 19.0, 0.0, 0.0, 1)[0], ProbablySky);
        assert!(matches!(ray_collisions(&grid, observer, EAST, 18.0, 0.0, 0.0, 1)[0], Collision { .. }));
    }
}
