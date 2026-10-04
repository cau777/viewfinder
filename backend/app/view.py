"""What an observer standing at a point can see around them, from the LiDAR grid in the Rust core."""

import math
from dataclasses import dataclass

from viewfinder_core import RayResult, RayTracer, latlon_to_utm, utm_to_latlon


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


def compute_view(tracer: RayTracer, latitude: float, longitude: float, eye_height: float, bearings: int) -> View:
    """Casts `bearings` horizontal rays (elevation 0) around an observer standing `eye_height` metres above
    the surface at (latitude, longitude). Rays that hit nothing are limited to the average sightline."""
    x, y = latlon_to_utm(latitude, longitude)
    ground_altitude = tracer.altitude_at(x, y)
    if ground_altitude is None:
        raise NoDataError("No LiDAR data at this position")
    altitude = ground_altitude + eye_height

    columns = tracer.ray_collisions_around(x, y, altitude, 0.0, 0.0, 1, 0.0, 2 * math.pi, bearings)
    results = [column[0] for column in columns]  # one elevation angle per bearing
    distances = [r.distance for r in results if isinstance(r, RayResult.Collision)]
    if not distances:
        raise NoObstructionError("No ray hits the surface around this position")
    average_distance = sum(distances) / len(distances)

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

    return View(latitude, longitude, ground_altitude, altitude, average_distance, points)
