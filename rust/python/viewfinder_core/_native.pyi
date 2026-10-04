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
