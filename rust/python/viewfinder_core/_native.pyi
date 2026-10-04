"""
Native core of viewfinder (Rust, via PyO3).
"""

from os import PathLike
from typing import Final, final
__version__: Final[str]

@final
class GridStats:
    """
    Shape and memory use of the loaded tree, returned by `RayTracer.stats()`.
    """
    def __repr__(self, /) -> str: ...
    @property
    def empty_slots_per_level(self, /) -> list[int]:
        """
        Child slots per level that are None because the whole subtree has no points.
        """
    @property
    def nodes_per_level(self, /) -> list[int]:
        """
        Nodes per tree level, root first; the last level holds the 3x3-point leaves.
        """
    @property
    def points_missing_in_leaves(self, /) -> int:
        """
        Points without data inside non-empty leaves.
        """
    @property
    def points_present(self, /) -> int:
        """
        Points with an altitude.
        """
    @property
    def tree_bytes(self, /) -> int:
        """
        Approximate heap bytes used by the tree.
        """

class RayResult:
    """
    Where one ray of `RayTracer.ray_collisions_around()` ends. Horizontal angles are radians
    clockwise from north, elevation angles radians above the horizontal.
    """
    def __repr__(self, /) -> str: ...
    @final
    class Collision(RayResult):
        """
        The ray hit the surface.
        """
        __match_args__: Final = ("distance", "vertical_angle", "horizontal_angle", "latitude", "longitude", "altitude_ray", "altitude_at_collision", "classification")
        def __new__(cls, /, distance: float, vertical_angle: float, horizontal_angle: float, latitude: float, longitude: float, altitude_ray: float, altitude_at_collision: float, classification: int |None) -> RayResult.Collision: ...
        @property
        def altitude_at_collision(self, /) -> float:
            """
            Altitude of the surface at the point hit, metres.
            """
        @property
        def altitude_ray(self, /) -> float:
            """
            Altitude of the ray where it hit, metres.
            """
        @property
        def classification(self, /) -> int |None:
            """
            ASPRS class of the point hit (2 ground, 5 high vegetation, 6 building, 9 water, ...),
            None if the dataset has no classes.
            """
        @property
        def distance(self, /) -> float:
            """
            Distance travelled by the ray (along the slope), metres.
            """
        @property
        def horizontal_angle(self, /) -> float:
            """
            Horizontal angle of the ray, radians clockwise from north.
            """
        @property
        def latitude(self, /) -> float:
            """
            Latitude of the centre of the point hit, degrees.
            """
        @property
        def longitude(self, /) -> float:
            """
            Longitude of the centre of the point hit, degrees.
            """
        @property
        def vertical_angle(self, /) -> float:
            """
            Elevation angle of the ray, radians.
            """
    @final
    class NoCollision(RayResult):
        """
        Left the dataset without hitting a surface.
        """
        __match_args__: Final = ()
        def __new__(cls, /) -> RayResult.NoCollision: ...

@final
class RayTracer:
    def __new__(cls, /, path: str |PathLike[str]) -> RayTracer:
        """
        RayTracer(path)
        
        Loads the u16 LiDAR export (a directory with `index.csv` and `<NAME>.u16` files).
        """
    def __repr__(self, /) -> str: ...
    @property
    def alt_range(self, /) -> tuple[float, float]:
        """
        Altitude range (metres) of the whole dataset.
        """
    def altitude(self, /, row: int, column: int) -> float |None:
        """
        Altitude in metres of the point at (row, column), None if it has no data.
        """
    def altitude_at(self, /, x: float, y: float) -> float |None:
        """
        Altitude in metres of the point nearest to UTM (x, y), None if it has no data.
        """
    def cast_ray(self, /, x: float, y: float, direction_x: float, direction_y: float) -> list[tuple[float, float, float |None]]:
        """
        cast_ray(x, y, direction_x, direction_y)
        
        Points whose cell a horizontal ray from UTM (x, y) crosses, ignoring altitude, nearest first.
        Returns `(x, y, altitude)` per point; altitude is None where a point has no data.
        """
    @property
    def cell_size(self, /) -> float:
        """
        Distance between neighbouring points, in metres.
        """
    @property
    def columns(self, /) -> int:
        """
        Number of point columns (west to east).
        """
    @property
    def depth(self, /) -> int:
        """
        Levels in the tree, including the leaf level.
        """
    def panorama(self, /, x: float, y: float, observer_altitude: float, min_elevation: float, max_elevation: float, width: int, height: int) -> bytes:
        """
        panorama(x, y, observer_altitude, min_elevation, max_elevation, width, height)
        
        PNG of the full circle around UTM (x, y) at `observer_altitude` metres, coloured by what each ray
        hits (its LiDAR class, faded with distance), sky or open water. Column 0 starts at north and
        bearings grow clockwise, `width` columns per 360°; rows go from `max_elevation` at the top to
        `min_elevation` at the bottom (radians).
        """
    def ray_collisions(self, /, x: float, y: float, direction_x: float, direction_y: float, observer_altitude: float, min_elevation: float, max_elevation: float, resolution: int) -> list[float]:
        """
        ray_collisions(x, y, direction_x, direction_y, observer_altitude, min_elevation, max_elevation, resolution)
        
        Casts `resolution` rays from UTM (x, y) at `observer_altitude` metres, in one horizontal direction,
        at elevation angles (radians) evenly spaced from `min_elevation` to `max_elevation`, both included.
        Returns one value per angle, lowest first: the distance in metres the ray travels before hitting
        the surface, or `inf` if it leaves the dataset without a collision.
        """
    def ray_collisions_around(self, /, x: float, y: float, observer_altitude: float, min_elevation: float, max_elevation: float, vertical_resolution: int, min_horizontal_angle: float, max_horizontal_angle: float, horizontal_resolution: int) -> list[list[RayResult]]:
        """
        ray_collisions_around(x, y, observer_altitude, min_elevation, max_elevation, vertical_resolution, min_horizontal_angle, max_horizontal_angle, horizontal_resolution)
        
        `ray_collisions` for `horizontal_resolution` horizontal angles (radians clockwise from north)
        evenly spaced from `min_horizontal_angle` (included) to `max_horizontal_angle` (excluded), so
        0 to 2π is the full circle. Returns one list per horizontal angle, each with
        `vertical_resolution` `RayResult`s from the lowest elevation to the highest.
        """
    @property
    def rows(self, /) -> int:
        """
        Number of point rows (north to south).
        """
    def stats(self, /) -> GridStats:
        """
        Walk the tree and count nodes, points and memory.
        """
    @property
    def x_start(self, /) -> float:
        """
        UTM (EPSG:26910) x of the most NW point.
        """
    @property
    def y_start(self, /) -> float:
        """
        UTM (EPSG:26910) y of the most NW point.
        """

def latlon_to_utm(latitude: float, longitude: float) -> tuple[float, float]:
    """
    latlon_to_utm(latitude, longitude)
    
    UTM zone 10N (EPSG:26910) `(x, y)` in metres of a NAD83 latitude and longitude in degrees.
    """

def utm_to_latlon(x: float, y: float) -> tuple[float, float]:
    """
    utm_to_latlon(x, y)
    
    NAD83 `(latitude, longitude)` in degrees of UTM zone 10N (EPSG:26910) metres.
    """
