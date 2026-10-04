use crate::grid::FullGrid;
use crate::ray_collisions::RayCastResult::{Collision, ProbablyOcean, ProbablySky};
use crate::ray_tracing::IntersectionWalk;
use crate::util::Coordinates;
use nalgebra::Vector2;

/// Mean Earth radius in metres.
const EARTH_RADIUS: f64 = 6_371_000.0;
/// Standard atmospheric refraction coefficient: light bends down slightly, so the Earth looks flatter.
const REFRACTION: f64 = 0.13;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RayCastResult {
    Collision {
        /// Distance travelled by the ray (along the slope, not the horizontal), metres
        distance: f64,
        /// Angle of elevation used to cast the ray, radians
        vertical_angle: f64,
        /// Horizontal direction of the ray, radians clockwise from north
        horizontal_angle: f64,
        /// Latitude and longitude of the centre of the point the ray hit
        coordinates: Coordinates,
        /// Altitude of the ray when it hit the object
        /// Example: if the ray hit the third floor of a skyscraper, this would be the height of the third floor
        altitude_ray: f64,
        /// Max altitude at the point of collision.
        /// Example: if the ray hit the third floor of a skyscraper, this would be the height of the skyscraper
        altitude_at_collision: f64,
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
/// Angles are in radians and `min_elevation_angle <= max_elevation_angle`. `horizontal_angle` is
/// clockwise from north (0 = north, π/2 = east). Positions are UTM metres and `observer_altitude`
/// uses the same datum as the grid.
pub fn ray_collisions(grid: &FullGrid,
                      observer_position: Vector2<f64>,
                      horizontal_angle: f64,
                      observer_altitude: f64,
                      max_elevation_angle: f64,
                      min_elevation_angle: f64,
                      resolution: usize
) -> Vec<RayCastResult> {
    debug_assert!(min_elevation_angle <= max_elevation_angle);
    let observer_direction = Vector2::new(horizontal_angle.sin(), horizontal_angle.cos());

    // Nearest first, so the first point above a ray is where it collides
    let mut walk = IntersectionWalk::new(grid, observer_position, observer_direction);
    // A ray passing above a point also passes above it at every higher angle, so each angle
    // continues the search where the previous one stopped: all angles together cost one walk.
    // The point the previous angle collided with is checked again by the next one.
    let mut current = None;

    (0..resolution)
        .map(|i| {
            let fraction = if resolution > 1 { i as f64 / (resolution - 1) as f64 } else { 0.0 };
            let angle = min_elevation_angle + (max_elevation_angle - min_elevation_angle) * fraction;
            let slope = angle.tan();
            // Subtrees this ray passes entirely above can't hold its collision, nor any higher angle's
            let next_point = |walk: &mut IntersectionWalk| {
                walk.next_unless(|enter, exit, max_altitude| {
                    passes_above(observer_altitude, slope, enter, exit, max_altitude)
                })
            };

            let mut point = current.take().or_else(|| next_point(&mut walk));
            while let Some(p) = point {
                if let Some(point_altitude) = p.altitude {
                    let distance = p.top_distance_to_observer;
                    let ray_altitude = observer_altitude + distance * slope;
                    if ray_altitude <= point_altitude - curvature_drop(distance) {
                        current = Some(p);
                        return Collision {
                            distance: distance / angle.cos(),
                            vertical_angle: angle,
                            horizontal_angle,
                            coordinates: Coordinates::from_utm(p.position.x, p.position.y),
                            altitude_ray: ray_altitude,
                            altitude_at_collision: point_altitude,
                        };
                    }
                }
                point = next_point(&mut walk);
            }

            if angle < 0.0 { ProbablyOcean } else { ProbablySky }
        })
        .collect()
}

/// Whether a ray with `slope` stays above `max_altitude` (minus the curvature drop) for every distance
/// from `enter` to `exit`, so it can't collide with anything there.
fn passes_above(observer_altitude: f64, slope: f64, enter: f64, exit: f64, max_altitude: f64) -> bool {
    // Ray height above the curved surface's reference: observer_altitude + slope·d + c·d², convex in d
    let c = (1.0 - REFRACTION) / (2.0 * EARTH_RADIUS);
    // Widened by a millimetre and compared with a margin, so rounding never skips a collision
    let lowest = (-slope / (2.0 * c)).clamp(enter - 1e-3, exit + 1e-3);
    observer_altitude + slope * lowest + c * lowest * lowest > max_altitude + 1e-6
}

/// `ray_collisions` for `horizontal_resolution` horizontal angles evenly spaced from
/// `min_horizontal_angle` (included) to `max_horizontal_angle` (excluded), so 0 to 2π goes the full
/// circle without casting north twice. Returns `vertical_resolution` results per horizontal angle,
/// horizontal angles first: result `i * vertical_resolution + j` is horizontal angle `i`, elevation `j`.
///
/// Horizontal angles are in radians clockwise from north: 0 is north, π/2 east, π south, 3π/2 west.
pub fn ray_collisions_around(
    grid: &FullGrid,
    observer_position: Vector2<f64>,
    observer_altitude: f64,
    max_elevation_angle: f64,
    min_elevation_angle: f64,
    vertical_resolution: usize,
    min_horizontal_angle: f64,
    max_horizontal_angle: f64,
    horizontal_resolution: usize,
) -> Vec<RayCastResult> {
    debug_assert!(min_elevation_angle <= max_elevation_angle);

    (0..horizontal_resolution)
        .flat_map(|i| {
            let horizontal_angle = min_horizontal_angle
                + (max_horizontal_angle - min_horizontal_angle) * i as f64
                / horizontal_resolution as f64;

            ray_collisions(grid, observer_position, horizontal_angle, observer_altitude, max_elevation_angle, min_elevation_angle, vertical_resolution)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{make_cells, Parcel, MISSING};
    use crate::ray_tracing::get_intersection_points;
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
        parcels[(0, 0)] = Some(Parcel {
            x_start: 0.0,
            y_start: 0.0,
            points,
        });
        let (contents, depth) = make_cells(&parcels, resolution);
        FullGrid {
            x_start: 0.0,
            y_start: 0.0,
            cell_size,
            rows: resolution,
            columns: resolution,
            depth,
            alt_min: 0.0,
            alt_max: (MISSING - 1) as f64,
            contents,
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

    const EAST: f64 = std::f64::consts::FRAC_PI_2;

    /// Distance of a collision, inf for the sky and -inf for the ocean
    fn distance(result: RayCastResult) -> f64 {
        match result {
            Collision { distance, .. } => distance,
            ProbablySky => f64::INFINITY,
            ProbablyOcean => f64::NEG_INFINITY,
        }
    }

    #[test]
    fn one_result_per_angle_including_both_ends() {
        let grid = wall_grid();
        let observer = Vector2::new(0.0, -15.0);
        let results = ray_collisions(&grid, observer, EAST, 11.7, 0.5, -0.5, 7);
        assert_eq!(results.len(), 7);
        assert_eq!(
            results[0],
            ray_collisions(&grid, observer, EAST, 11.7, -0.5, -0.5, 1)[0]
        );
        assert_eq!(
            results[6],
            ray_collisions(&grid, observer, EAST, 11.7, 0.5, 0.5, 1)[0]
        );
    }

    #[test]
    fn wall_ground_and_sky() {
        let grid = wall_grid();
        let observer = Vector2::new(0.0, -15.0);
        let observer_altitude = 11.7;
        // The ray enters the wall at x = 19.75 (column 40's west edge)
        let wall_entry: f64 = 40.0 * 0.5 - 0.25;
        let wall_top_angle = ((30.0 - observer_altitude) / wall_entry).atan();

        let angles = [
            -0.3,
            -0.01,
            0.0,
            wall_top_angle - 0.01,
            wall_top_angle + 0.05,
        ];
        for &angle in &angles {
            let result =
                ray_collisions(&grid, observer, EAST, observer_altitude, angle, angle, 1)[0];
            let (expected, expected_altitude) = if angle < -0.05 {
                // Hits the ground where the ray has dropped 1.7 m: first cell entered at or after that
                let reach: f64 = 1.7 / -angle.tan();
                let entry = ((reach + 0.25) / 0.5).ceil() * 0.5 - 0.25;
                (entry / angle.cos(), 10.0)
            } else if angle < wall_top_angle {
                (wall_entry / angle.cos(), 30.0)
            } else {
                (f64::INFINITY, f64::NAN)
            };
            let got = distance(result);
            if expected.is_finite() {
                assert!((got - expected).abs() < 1e-9, "angle {angle}: {got} vs {expected}");
                let Collision { vertical_angle, horizontal_angle, coordinates, altitude_ray, altitude_at_collision, .. } = result else {
                    unreachable!()
                };
                assert_eq!((vertical_angle, horizontal_angle), (angle, EAST));
                assert_eq!(altitude_at_collision, expected_altitude);
                // The ray is at or below the surface where it hits, and the hit is the cell entered there
                assert!(altitude_ray <= altitude_at_collision);
                // The observer is on the centre line of a row, so the hit cell's centre is half a cell past the entry
                let entry = observer.x + got * angle.cos();
                assert_eq!(coordinates, Coordinates::from_utm(entry + 0.25, observer.y));
            } else {
                assert_eq!(result, ProbablySky, "angle {angle}");
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
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as f64 / (1u64 << 31) as f64
        };
        let altitudes: Vec<Vec<Option<f64>>> = (0..n)
            .map(|_| {
                (0..n)
                    .map(|_| {
                        if next() < 0.1 {
                            None
                        } else {
                            Some((next() * 40.0).floor())
                        }
                    })
                    .collect()
            })
            .collect();
        let grid = grid_from(&altitudes, 0.5);

        for k in 0..16 {
            let bearing = k as f64 * std::f64::consts::TAU / 16.0 + 0.1;
            let direction = Vector2::new(bearing.sin(), bearing.cos());
            let observer = Vector2::new(11.1, -10.6);
            let results = ray_collisions(&grid, observer, bearing, 25.0, 0.6, -0.6, 50);
            let points = get_intersection_points(&grid, observer, direction);

            for (i, result) in results.iter().enumerate() {
                let angle = -0.6 + 1.2 * (i as f64 / 49.0);
                let expected = points
                    .iter()
                    .find(|p| {
                        p.altitude.is_some_and(|a| {
                            25.0 + p.top_distance_to_observer * angle.tan()
                                <= a - curvature_drop(p.top_distance_to_observer)
                        })
                    })
                    .map_or(
                        if angle < 0.0 {
                            ProbablyOcean
                        } else {
                            ProbablySky
                        },
                        |p| Collision {
                            distance: p.top_distance_to_observer / angle.cos(),
                            vertical_angle: angle,
                            horizontal_angle: bearing,
                            coordinates: Coordinates::from_utm(p.position.x, p.position.y),
                            altitude_ray: 25.0 + p.top_distance_to_observer * angle.tan(),
                            altitude_at_collision: p.altitude.unwrap(),
                        },
                    );
                assert_eq!(*result, expected, "bearing {bearing}, angle {angle}");
            }
        }
    }

    #[test]
    fn around_is_ray_collisions_per_horizontal_angle() {
        let grid = wall_grid();
        let observer = Vector2::new(14.0, -15.0);
        let (vertical, horizontal) = (5, 8);
        let results = ray_collisions_around(&grid, observer, 11.7, 1.4, -0.4, vertical, 0.0, std::f64::consts::TAU, horizontal);
        assert_eq!(results.len(), vertical * horizontal);
        for (i, chunk) in results.chunks(vertical).enumerate() {
            // The max angle is excluded: 8 steps of 45° from north, never 360°
            let horizontal_angle = i as f64 * std::f64::consts::TAU / horizontal as f64;
            assert_eq!(chunk, ray_collisions(&grid, observer, horizontal_angle, 11.7, 1.4, -0.4, vertical));
        }
        // Index 2 is east, towards the wall 5.75 m away: the lowest ray hits the ground, the highest (80°) clears it
        let east = &results[2 * vertical..3 * vertical];
        assert!(matches!(east[0], Collision { horizontal_angle, .. } if horizontal_angle == EAST));
        assert_eq!(east[vertical - 1], ProbablySky);
    }

    #[test]
    fn earth_curvature_hides_distant_low_targets() {
        // 100 m cells: a 20 m tower 5 km away, observed level from 19 m.
        // Flat-Earth geometry would hit it, but it is ~1.7 m below the horizon at that distance.
        let n = 60;
        let row: Vec<Option<f64>> = (0..n)
            .map(|c| if c == 50 { Some(20.0) } else { Some(0.0) })
            .collect();
        let grid = grid_from(&vec![row; n], 100.0);
        let observer = Vector2::new(0.0, -1500.0);
        assert_eq!(
            ray_collisions(&grid, observer, EAST, 19.0, 0.0, 0.0, 1)[0],
            ProbablySky
        );
        assert!(matches!(
            ray_collisions(&grid, observer, EAST, 18.0, 0.0, 0.0, 1)[0],
            Collision { .. }
        ));
    }
}
