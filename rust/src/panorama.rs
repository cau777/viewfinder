use crate::grid::FullGrid;
use crate::ray_collisions::{ray_collisions_around, RayCastResult};
use nalgebra::Vector2;
use std::f64::consts::TAU;

type Rgb = [f64; 3];

const WATER: Rgb = [63.0, 116.0, 163.0];
/// What distant surfaces fade into: atmospheric perspective, so depth reads without shading
const HAZE: Rgb = [201.0, 214.0, 224.0];
/// Distance over which a surface keeps 1/e of its own colour, metres
const HAZE_DISTANCE: f64 = 4000.0;
const SKY_HORIZON: Rgb = [216.0, 230.0, 240.0];
const SKY_ZENITH: Rgb = [127.0, 176.0, 220.0];
/// Elevation (radians) at which the sky reaches its zenith colour
const SKY_GRADIENT: f64 = 0.5;
/// A hit is on a silhouette, and drawn darker, if the pixel above it is sky or this many times farther
const EDGE_RATIO: f64 = 1.3;
const EDGE_DARKEN: f64 = 0.8;

/// Colour of each ASPRS class, sRGB 0-255.
pub fn class_colour(class: Option<u8>) -> Rgb {
    match class {
        Some(1) => [138.0, 122.0, 104.0],            // unassigned: ships, cranes, cars
        Some(2) => [179.0, 163.0, 127.0],            // ground
        Some(3) => [143.0, 176.0, 105.0],            // low vegetation
        Some(4) => [110.0, 154.0, 82.0],             // medium vegetation
        Some(5) => [63.0, 107.0, 58.0],              // high vegetation
        Some(6) | Some(17) => [154.0, 157.0, 163.0], // building, bridge deck
        Some(9) => WATER,
        Some(10) | Some(11) => [109.0, 109.0, 109.0], // rail, road
        _ => [160.0, 160.0, 160.0],                   // other classes, or an export without classes
    }
}

fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// Colour of one ray: the class of what it hit, faded with distance; sky or open water otherwise.
fn ray_colour(result: &RayCastResult, elevation: f64) -> Rgb {
    match *result {
        RayCastResult::Collision { distance, class, .. } => {
            mix(class_colour(class), HAZE, 1.0 - (-distance / HAZE_DISTANCE).exp())
        }
        RayCastResult::NoCollision => mix(SKY_HORIZON, SKY_ZENITH, (elevation / SKY_GRADIENT).clamp(0.0, 1.0)),
    }
}

fn distance(result: &RayCastResult) -> f64 {
    match *result {
        RayCastResult::Collision { distance, .. } => distance,
        _ => f64::INFINITY,
    }
}

/// The full circle around an observer as a `width` x `height` RGB image (3 bytes per pixel, row-major).
/// Each pixel is the ray through its centre: column 0 starts at north and bearings grow clockwise,
/// `width` columns per 360°; row 0 is at `max_elevation`, the last row at `min_elevation` (radians).
pub fn panorama(
    grid: &FullGrid,
    observer_position: Vector2<f64>,
    observer_altitude: f64,
    min_elevation: f64,
    max_elevation: f64,
    width: usize,
    height: usize,
) -> Vec<u8> {
    let row_step = (max_elevation - min_elevation) / height as f64;
    let column_step = TAU / width as f64;
    // Pixel centres: half a step inside the range on each side
    let (lowest, highest) = (min_elevation + row_step / 2.0, max_elevation - row_step / 2.0);
    let results = ray_collisions_around(
        grid, observer_position, observer_altitude, highest, lowest, height,
        column_step / 2.0, TAU + column_step / 2.0, width,
    );
    // Results are per column, lowest elevation first
    let at = |row: usize, column: usize| &results[column * height + (height - 1 - row)];

    let mut image = Vec::with_capacity(width * height * 3);
    for row in 0..height {
        let elevation = highest - row as f64 * row_step;
        for column in 0..width {
            let result = at(row, column);
            let mut colour = ray_colour(result, elevation);
            if row > 0 && distance(result).is_finite() && distance(at(row - 1, column)) > distance(result) * EDGE_RATIO {
                colour = colour.map(|c| c * EDGE_DARKEN);
            }
            image.extend(colour.map(|c| c.round().clamp(0.0, 255.0) as u8));
        }
    }
    image
}

/// Encodes an RGB image (3 bytes per pixel, row-major) as PNG.
pub fn encode_png(rgb: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("writing to a Vec cannot fail");
    writer.write_image_data(rgb).expect("the image has width * height pixels");
    writer.finish().expect("writing to a Vec cannot fail");
    png
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{make_cells, pack_classes, ClassGrid, Parcel, CLASS_MISSING, MISSING};
    use array2d::Array2D;

    /// 60x60 points of 0.5 m, first point centred at (0, 0): ground (class 2) at 10 m, a 30 m building
    /// (class 6) from column 40 to 44, then no data (water) from column 50.
    fn wall_grid() -> FullGrid {
        let n = 60;
        let point = |c: usize| match c {
            40..=44 => (30, 6),
            50.. => (MISSING, CLASS_MISSING),
            _ => (10, 2),
        };
        let values: Vec<(u16, u8)> = (0..n * n).map(|k| point(k % n)).collect();
        let mut parcels = Array2D::filled_by_row_major(|| None, 1, 1);
        parcels[(0, 0)] = Some(Parcel {
            x_start: 0.0,
            y_start: 0.0,
            points: Array2D::from_iter_row_major(values.iter().map(|v| v.0), n, n).unwrap(),
        });
        let mut classes = Array2D::filled_by_row_major(|| None, 1, 1);
        classes[(0, 0)] = Some(pack_classes(&values.iter().map(|v| v.1).collect::<Vec<_>>()));
        let (contents, depth) = make_cells(&parcels, n);
        FullGrid {
            x_start: 0.0, y_start: 0.0, cell_size: 0.5, rows: n, columns: n, depth,
            // alt_min = 0 and alt_max = MISSING - 1 make the stored value the altitude in metres
            alt_min: 0.0, alt_max: (MISSING - 1) as f64,
            contents,
            classes: Some(ClassGrid { resolution: n, parcels: classes }),
        }
    }

    fn pixel(image: &[u8], width: usize, row: usize, column: usize) -> [u8; 3] {
        let k = (row * width + column) * 3;
        [image[k], image[k + 1], image[k + 2]]
    }

    fn rgb(colour: Rgb) -> [u8; 3] {
        colour.map(|c| c.round() as u8)
    }

    #[test]
    fn pixels_are_the_rays_through_their_centres() {
        let grid = wall_grid();
        let observer = Vector2::new(14.0, -15.0);
        let (width, height) = (36, 9);
        let (min, max) = (-0.45, 0.45);
        let image = panorama(&grid, observer, 11.7, min, max, width, height);
        assert_eq!(image.len(), width * height * 3);

        // Column c is centred on bearing (c + 0.5) * 10°, row r on elevation max - (r + 0.5) * 0.1
        let results = ray_collisions_around(&grid, observer, 11.7, max - 0.05, min + 0.05, height, TAU / 72.0, TAU + TAU / 72.0, width);
        for column in 0..width {
            for row in 0..height {
                let result = &results[column * height + height - 1 - row];
                let elevation = max - (row as f64 + 0.5) * 0.1;
                let colour = rgb(ray_colour(result, elevation));
                let darker = rgb(ray_colour(result, elevation).map(|c| c * EDGE_DARKEN));
                let got = pixel(&image, width, row, column);
                assert!(got == colour || got == darker, "pixel ({row}, {column})");
            }
        }
    }

    #[test]
    fn colours_follow_what_the_ray_hits() {
        let grid = wall_grid();
        let (width, height) = (36, 9);
        // Column 8 is centred on 85° (east), column 26 on 265° (west); row r on elevation 0.4 - 0.1 r
        let (east, west) = (8, 26);
        let hazed = |class, distance: f64| rgb(mix(class_colour(class), HAZE, 1.0 - (-distance / HAZE_DISTANCE).exp()));
        // Within a metre of the expected distance, haze changes the colour by less than one step
        let close = |a: [u8; 3], b: [u8; 3]| a.iter().zip(b).all(|(&x, y)| x.abs_diff(y) <= 1);
        let sky = |row: usize| rgb(mix(SKY_HORIZON, SKY_ZENITH, (0.4 - 0.1 * row as f64) / SKY_GRADIENT));

        // 4 m from the building's west face, eyes 1.7 m above the ground
        let image = panorama(&grid, Vector2::new(15.75, -15.0), 11.7, -0.45, 0.45, width, height);
        // East: even the top ray (0.4 rad) hits the 30 m building
        for row in 0..=4 {
            assert!(close(pixel(&image, width, row, east), hazed(Some(6), 4.0)), "east row {row}");
        }
        // West: the lowest ray hits the ground about 4 m away; from the horizon up it leaves the dataset
        assert!(close(pixel(&image, width, 8, west), hazed(Some(2), 4.4)));
        for row in 0..=4 {
            assert_eq!(pixel(&image, width, row, west), sky(row), "west row {row}");
        }

        // From higher up and farther away, the building's top is below the highest rays
        let image = panorama(&grid, Vector2::new(0.25, -15.0), 25.0, -0.45, 0.45, width, height);
        let first_building_row = (0..height).find(|&row| pixel(&image, width, row, east) != sky(row)).unwrap();
        assert!(first_building_row > 0);
        // Building pixels with sky above them are on the silhouette, drawn darker than the facade below
        let edge = pixel(&image, width, first_building_row, east);
        let below = pixel(&image, width, first_building_row + 1, east);
        assert!(close(below, hazed(Some(6), 19.6)));
        assert!(close(edge, hazed(Some(6), 19.6).map(|c| (c as f64 * EDGE_DARKEN).round() as u8)));
    }

    #[test]
    fn no_collision_uses_sky_even_below_the_horizon() {
        for elevation in [-0.5, -0.01, 0.0, 0.25, 0.5] {
            assert_eq!(
                ray_colour(&RayCastResult::NoCollision, elevation),
                mix(SKY_HORIZON, SKY_ZENITH, (elevation / SKY_GRADIENT).clamp(0.0, 1.0)),
            );
        }
    }

    #[test]
    fn encodes_a_png() {
        let png = encode_png(&[255, 0, 0, 0, 0, 255], 2, 1);
        assert_eq!(&png[1..4], b"PNG");
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut buffer).unwrap();
        assert_eq!(buffer, [255, 0, 0, 0, 0, 255]);
    }
}
