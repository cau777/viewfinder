/// Latitude and longitude on the globe, in degrees (NAD83, which matches WGS84 to about a metre).
/// A separate type avoids mixing them up with UTM positions and relative vectors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

/// GRS80 ellipsoid (used by NAD83)
const SEMI_MAJOR_AXIS: f64 = 6_378_137.0;
const FLATTENING: f64 = 1.0 / 298.257_222_101;
/// UTM constants
const SCALE_FACTOR: f64 = 0.9996;
const FALSE_EASTING: f64 = 500_000.0;
/// The grid's CRS is EPSG:26910: NAD83 / UTM zone 10N, central meridian 123°W
const CENTRAL_MERIDIAN: f64 = -123.0;

impl Coordinates {
    /// Converts UTM zone 10N (EPSG:26910) metres to latitude and longitude.
    /// Krüger series to n⁴ (Karney 2011): sub-millimetre accuracy within the zone.
    pub fn from_utm(x: f64, y: f64) -> Self {
        let n = FLATTENING / (2.0 - FLATTENING);
        let (n2, n3, n4) = (n * n, n * n * n, n * n * n * n);
        // Meridian radius: 2π·A is the length of a meridian
        let a = SEMI_MAJOR_AXIS / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0);
        let beta = [
            n / 2.0 - 2.0 / 3.0 * n2 + 37.0 / 96.0 * n3 - n4 / 360.0,
            n2 / 48.0 + n3 / 15.0 - 437.0 / 1440.0 * n4,
            17.0 / 480.0 * n3 - 37.0 / 840.0 * n4,
            4397.0 / 161_280.0 * n4,
        ];
        let delta = [
            2.0 * n - 2.0 / 3.0 * n2 - 2.0 * n3 + 116.0 / 45.0 * n4,
            7.0 / 3.0 * n2 - 8.0 / 5.0 * n3 - 227.0 / 45.0 * n4,
            56.0 / 15.0 * n3 - 136.0 / 35.0 * n4,
            4279.0 / 630.0 * n4,
        ];

        // Normalised position on the transverse Mercator projection of the ellipsoid...
        let xi = y / (SCALE_FACTOR * a);
        let eta = (x - FALSE_EASTING) / (SCALE_FACTOR * a);
        // ...then on the projection of a sphere
        let (mut xi_sphere, mut eta_sphere) = (xi, eta);
        for (j, b) in beta.iter().enumerate() {
            let k = 2.0 * (j + 1) as f64;
            xi_sphere -= b * (k * xi).sin() * (k * eta).cosh();
            eta_sphere -= b * (k * xi).cos() * (k * eta).sinh();
        }
        // Conformal latitude on the sphere, then geodetic latitude on the ellipsoid
        let chi = (xi_sphere.sin() / eta_sphere.cosh()).asin();
        let latitude = chi + delta.iter().enumerate().map(|(j, d)| d * (2.0 * (j + 1) as f64 * chi).sin()).sum::<f64>();
        let longitude = eta_sphere.sinh().atan2(xi_sphere.cos());

        Coordinates {
            latitude: latitude.to_degrees(),
            longitude: CENTRAL_MERIDIAN + longitude.to_degrees(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_pyproj() {
        // pyproj: Transformer.from_crs("EPSG:26910", "EPSG:4269", always_xy=True)
        let reference = [
            (491221.95, 5458890.34, 49.28270324100084, -123.12069754209472), // downtown Vancouver
            (481000.0, 5456000.0, 49.25647205460238, -123.26111023690703),
            (500000.0, 5450000.0, 49.20279556763614, -123.00000000000001),
            (510000.0, 5470000.0, 49.38261663560206, -122.86222216051178),
            (300000.0, 4000000.0, 36.12409583310784, -125.22239138545126), // far from the central meridian
        ];
        for (x, y, latitude, longitude) in reference {
            let c = Coordinates::from_utm(x, y);
            // 1e-9° is about 0.1 mm
            assert!((c.latitude - latitude).abs() < 1e-9, "({x}, {y}): latitude {} vs {latitude}", c.latitude);
            assert!((c.longitude - longitude).abs() < 1e-9, "({x}, {y}): longitude {} vs {longitude}", c.longitude);
        }
    }
}
