"""What an observer standing at a point can see around them, from the LiDAR grid in the Rust core."""

import math
from dataclasses import dataclass

from viewfinder_core import RayResult, RayTracer, latlon_to_utm, utm_to_latlon
from shapely.geometry import Polygon
from analysis import analyze
border_points = []
nonborder_points = []

# Extra elevation of the observer above the surface at the point, metres (roughly eye level)
OBSERVER_HEIGHT = 2.5

# The panorama covers every bearing, from a little below the horizon (nearby ground) to well above it
# (skylines and hills), with square pixels so nothing is stretched
PANORAMA_MIN_ELEVATION = -10  # degrees
PANORAMA_MAX_ELEVATION = 20  # degrees
PANORAMA_PIXELS_PER_DEGREE = 4


class NoDataError(ValueError):
    """The LiDAR grid has no surface at the requested position."""


class NoObstructionError(ValueError):
    """No ray hits anything, so there is no average sightline to limit unobstructed rays to."""


@dataclass(frozen=True)
class ViewPoint:
    bearing: float
    """Degrees clockwise from north."""
    latitude: float
    longitude: float
    distance: float
    """Metres from the observer."""
    unobstructed: bool
    """The ray hit nothing; its distance is limited to the average sightline."""


@dataclass(frozen=True)
class View:
    latitude: float
    longitude: float
    ground_altitude: float
    altitude: float
    """Altitude of the observer's eyes, metres."""
    average_distance: float
    """Mean distance of the rays that hit the surface, metres."""
    points: list[ViewPoint]
    """Where each ray ends, by bearing: the vertices of the visible area."""
    farthest_distance: float
    unobstructed_share: float
    polygon: Polygon
    border_points: list[ViewPoint]
    nonborder_points: list[ViewPoint]

def thin_points(points: list[ViewPoint], minimum_distance: float = 0.5) -> list[ViewPoint]:
    """Drop nearby vertices, preserving open/blocked boundaries and at least one ray per degree.

    Open rays are extrapolated by the client, so their angular coverage must be retained.
    Keep both sides of the ring seam and never collapse a polygon below three vertices.
    """
    if len(points) <= 3:
        return points

    def xy(point: ViewPoint) -> tuple[float, float]:
        angle = math.radians(point.bearing)
        return point.distance * math.sin(angle), point.distance * math.cos(angle)

    kept = [points[0]]
    for i, point in enumerate(points[1:-1], 1):
        previous = kept[-1]
        transition = (point.unobstructed != points[i - 1].unobstructed
                      or point.unobstructed != points[i + 1].unobstructed)
        x, y = xy(point)
        last_x, last_y = xy(previous)
        if (transition or point.bearing - previous.bearing >= 1
                or (not point.unobstructed and math.hypot(x - last_x, y - last_y) >= minimum_distance)):
            kept.append(point)
    kept.append(points[-1])
    return kept if len(kept) >= 3 else points

def compute_polygon(results, x, y, bearings, average_distance):
    polygon_points = []  

    for i, result in enumerate(results):
        bearing = 360 * i / bearings

        if isinstance(result, RayResult.Collision):
            polygon_points.append(
                (result.latitude, result.longitude)
            )
            nonborder_points.append((bearing, result.latitude, result.longitude))
        else:
            radians = math.radians(bearing)

            end = utm_to_latlon(
                x + average_distance * math.sin(radians),
                y + average_distance * math.cos(radians)
            )

            polygon_points.append(end)
            border_points.append((bearing, *end))

    coordinates = [(long, lat) for lat, long in polygon_points]

    return Polygon(coordinates)

def observer_position(tracer: RayTracer, latitude: float, longitude: float) -> tuple[float, float, float]:
    """UTM x, y and ground altitude of a position; the observer's eyes are `OBSERVER_HEIGHT` above it."""
    x, y = latlon_to_utm(latitude, longitude)
    ground_altitude = tracer.altitude_at(x, y)
    if ground_altitude is None:
        raise NoDataError("No LiDAR data at this position")
    return x, y, ground_altitude


def compute_view(tracer: RayTracer, latitude: float, longitude: float, bearings: int) -> View:
    """Casts `bearings` horizontal rays (elevation 0) around an observer `OBSERVER_HEIGHT` metres above
    the surface at (latitude, longitude). Rays that hit nothing are limited to the average sightline."""
    x, y, ground_altitude = observer_position(tracer, latitude, longitude)
    altitude = ground_altitude + OBSERVER_HEIGHT

    columns = tracer.ray_collisions_around(x, y, altitude, 0.0, 0.0, 1, 0.0, 2 * math.pi, bearings)
    results = [column[0] for column in columns]  # one elevation angle per bearing
    distances = [r.distance for r in results if isinstance(r, RayResult.Collision)]
    if not distances:
        raise NoObstructionError("No ray hits the surface around this position")
    average_distance = sum(distances) / len(distances)

    polygon = compute_polygon(
    results,
    x,
    y,
    bearings,
    average_distance
)

    points = []
    for i, result in enumerate(results):
        bearing = 360 * i / bearings
        if isinstance(result, RayResult.Collision):
            points.append(ViewPoint(bearing, result.latitude, result.longitude, result.distance, False))
        else:
            # Left the dataset: at elevation 0 it points at the sky (or past the edge of the data)
            radians = math.radians(bearing)
            end = utm_to_latlon(x + average_distance * math.sin(radians), y + average_distance * math.cos(radians))
            points.append(ViewPoint(bearing, *end, average_distance, True))

    analyze(polygon, latitude, longitude, altitude, border_points, nonborder_points)

    return View(latitude, longitude, ground_altitude, altitude, average_distance,
                thin_points(points), max(distances), (len(results) - len(distances)) / len(results), polygon, border_points, nonborder_points)


def panorama_png(tracer: RayTracer, latitude: float, longitude: float) -> bytes:
    """PNG of the full circle seen by the same observer as `compute_view`, coloured by what each ray hits.
    Starts at north and turns clockwise, `PANORAMA_PIXELS_PER_DEGREE` pixels per degree both ways."""
    x, y, ground_altitude = observer_position(tracer, latitude, longitude)
    return tracer.panorama(
        x, y, ground_altitude + OBSERVER_HEIGHT,
        math.radians(PANORAMA_MIN_ELEVATION), math.radians(PANORAMA_MAX_ELEVATION),
        360 * PANORAMA_PIXELS_PER_DEGREE, (PANORAMA_MAX_ELEVATION - PANORAMA_MIN_ELEVATION) * PANORAMA_PIXELS_PER_DEGREE,
    )
