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
        __match_args__: Final = ("distance", "vertical_angle", "horizontal_angle", "latitude", "longitude", "altitude_ray", "altitude_at_collision")
        def __new__(cls, /, distance: float, vertical_angle: float, horizontal_angle: float, latitude: float, longitude: float, altitude_ray: float, altitude_at_collision: float) -> RayResult.Collision: ...
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
    class Ocean(RayResult):
        """
        Pointing down and left the dataset without hitting anything (no points over open water).
        """
        __match_args__: Final = ()
        def __new__(cls, /) -> RayResult.Ocean: ...
    @final
    class Sky(RayResult):
        """
        Pointing up and left the dataset without hitting anything.
        """
        __match_args__: Final = ()
        def __new__(cls, /) -> RayResult.Sky: ...

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
    def ray_collisions(self, /, x: float, y: float, direction_x: float, direction_y: float, observer_altitude: float, min_elevation: float, max_elevation: float, resolution: int) -> list[float]:
        """
        ray_collisions(x, y, direction_x, direction_y, observer_altitude, min_elevation, max_elevation, resolution)
        
        Casts `resolution` rays from UTM (x, y) at `observer_altitude` metres, in one horizontal direction,
        at elevation angles (radians) evenly spaced from `min_elevation` to `max_elevation`, both included.
        Returns one value per angle, lowest first: the distance in metres the ray travels before hitting
        the surface, `inf` if it probably reaches the sky, or `-inf` if it probably reaches the ocean.
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
