use crate::grid::{CellContents, FullGrid};
use crate::util::Coordinates;
use nalgebra::Vector2;
use smallvec::SmallVec;
use std::collections::VecDeque;
use std::f64::consts::SQRT_2;
use std::sync::Arc;

#[derive(Copy, Clone)]
struct PossibleIntersection<'a> {
    cell: &'a CellContents,
    x_start: f64,
    y_start: f64,
    scale: f64,
    approach_to_observer: f64,
}

pub struct Intersection {
    pub position: Vector2<f64>,
    pub altitude: Option<f64>,
}

/// All positions are in UTM (meters)
/// observer_direction must be normalized
pub fn get_intersection_points(
    grid: &FullGrid,
    observer_position: Vector2<f64>,
    observer_direction: Vector2<f64>,
) -> Vec<Intersection> {
    let mut queue: VecDeque<PossibleIntersection> = VecDeque::new();
    let mut result = Vec::new();

    queue.push_back(PossibleIntersection {
        cell: &grid.contents.as_ref().unwrap(),
        x_start: grid.x_start,
        y_start: grid.y_start,
        scale: grid.cell_size,
        approach_to_observer: 0.0,
    });

    while !queue.is_empty() {
        let PossibleIntersection {
            cell,
            x_start,
            y_start,
            scale,
            ..
        } = queue.pop_front().unwrap();

        let possible_intersection_radius = scale * (SQRT_2 / 2.0);
        let mut possible_collisions = SmallVec::<[PossibleIntersection; 9]>::new();

        for i in 0..3 {
            let point_x = x_start + (i as f64 + 0.5) * scale;
            for j in 0..3 {
                let point_y = y_start + (j as f64 + 0.5) * scale;
                let point_pos = Vector2::new(point_x, point_y);
                let cosine = Vector2::dot(&observer_direction, &point_pos);
                if cosine <= 0.0 {
                    continue; // The ray is pointing in the other direction
                }

                let point_dist = (point_pos - observer_position).magnitude();
                let sine = f64::sqrt(1.0 - cosine * cosine);
                let approach_to_point = point_dist * sine;

                if approach_to_point < possible_intersection_radius {
                    match cell {
                        CellContents::Subcells { cells } => {
                            let subcell = match &cells[i][j] {
                                Some(v) => v,
                                None => continue,
                            };

                            possible_collisions.push(PossibleIntersection {
                                cell: subcell,
                                x_start: x_start + (i as f64) * scale,
                                y_start: y_start + (j as f64) * scale,
                                scale: scale / 3.0,
                                approach_to_observer: point_dist * cosine,
                            });
                        }
                        CellContents::Points { points } => result.push(Intersection {
                            position: point_pos,
                            altitude: grid.decompress_altitude(points[i][j]),
                        }),
                    };
                }
            }
        }

        // Points closer to the observer are more interesting
        possible_collisions.sort_by(|a, b| {
            a.approach_to_observer
                .partial_cmp(&b.approach_to_observer)
                .unwrap()
                .reverse()
        });

        for possible_collision in possible_collisions {
            queue.push_front(possible_collision)
        }
    }

    result // Consider this semi-sorted, closer points will be to the front
}
