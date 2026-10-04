//! Times `ray_collisions_around` on a real export.
//! cargo run --release --example bench_around -- <export dir>
use nalgebra::Vector2;
use std::time::Instant;
use viewfinder_core::grid::FullGrid;
use viewfinder_core::ray_collisions::ray_collisions_around;
use viewfinder_core::util::Coordinates;

fn main() {
    let dir = std::env::args().nth(1).expect("usage: bench_around <export dir>");
    let start = Instant::now();
    let grid = FullGrid::load(dir.as_ref()).unwrap();
    println!("load: {:.2?}", start.elapsed());

    let observers = [
        ("downtown", 49.2827, -123.1207, 1.7),
        ("queen elizabeth park", 49.2418, -123.1126, 1.7),
        ("tower roof", 49.2827, -123.1207, 120.0),
    ];
    for (name, latitude, longitude, height) in observers {
        let position = Coordinates { latitude, longitude }.to_utm();
        let Some(ground) = grid.altitude_at(position.x, position.y) else {
            println!("{name}: no data");
            continue;
        };
        for (vertical, horizontal) in [(1, 3600), (50, 360)] {
            let start = Instant::now();
            let results = ray_collisions_around(
                &grid, Vector2::new(position.x, position.y), ground + height,
                0.3, -0.3, vertical, 0.0, std::f64::consts::TAU, horizontal,
            );
            let checksum: f64 = results.iter().map(|r| match r {
                viewfinder_core::ray_collisions::RayCastResult::Collision { distance, .. } => *distance,
                _ => 0.0,
            }).sum();
            println!("{name} {vertical}x{horizontal}: {:.2?} (checksum {checksum:.6})", start.elapsed());
        }
    }
}
