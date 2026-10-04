import math

import numpy as np
import pytest
from fastapi.testclient import TestClient

from app.main import app, get_tracer
from app.view import OBSERVER_HEIGHT
from viewfinder_core import RayResult, RayTracer, latlon_to_utm, utm_to_latlon

client = TestClient(app)

# 60 x 60 points (0.5 m) in downtown Vancouver: flat ground at 10 m, a 30 m wall on columns 40-44
# (east of the observer) and no data from column 50
X_START, Y_START, CELL = 491200.25, 5458899.75, 0.5
OBSERVER = (X_START + 20 * CELL, Y_START - 30 * CELL)  # column 20, row 30
WALL_WEST_EDGE = X_START + 40 * CELL - CELL / 2


@pytest.fixture(scope="module")
def tracer(tmp_path_factory) -> RayTracer:
    directory = tmp_path_factory.mktemp("export")
    altitudes = np.full((60, 60), 10.0)
    altitudes[:, 40:45] = 30.0
    points = np.round(altitudes / 100 * 65534).astype("<u2")
    points[:, 50:] = 65535  # missing
    points.tofile(directory / "t.u16")
    # ASPRS classes, two per byte (even points in the low nibble): ground, the wall is a building
    classes = np.full((60, 60), 2, np.uint8)
    classes[:, 40:45] = 6
    classes[:, 50:] = 15  # missing
    flat = classes.ravel()
    (flat[0::2] | flat[1::2] << 4).astype(np.uint8).tofile(directory / "t.cls")
    (directory / "index.csv").write_text(
        "file,classes_file,width,height,x_start,y_start,x_end,alt_min,alt_max\n"
        f"t.u16,t.cls,60,60,{X_START},{Y_START},{X_START + 59 * CELL},0,100\n"
    )
    return RayTracer(directory)


@pytest.fixture(autouse=True)
def use_tracer(tracer):
    app.dependency_overrides[get_tracer] = lambda: tracer
    yield
    app.dependency_overrides.clear()


def post_view(x, y, **extra):
    latitude, longitude = utm_to_latlon(x, y)
    return client.post("/api/view", json={"latitude": latitude, "longitude": longitude, **extra})


def test_utm_round_trip():
    # pyproj: EPSG:26910 -> EPSG:4269
    assert utm_to_latlon(491221.95, 5458890.34) == pytest.approx((49.28270324100084, -123.12069754209472), abs=1e-9)
    assert latlon_to_utm(49.28270324100084, -123.12069754209472) == pytest.approx((491221.95, 5458890.34), abs=1e-4)


def test_view_hits_the_wall_and_limits_the_sky_to_the_average():
    r = post_view(*OBSERVER, bearings=8)
    assert r.status_code == 200, r.text
    body = r.json()
    assert body["ground_altitude"] == pytest.approx(10.0, abs=0.01)
    assert body["altitude"] == pytest.approx(10.0 + OBSERVER_HEIGHT, abs=0.01)

    points = body["points"]
    assert [p["bearing"] for p in points] == [0, 45, 90, 135, 180, 225, 270, 315]
    # Only NE, E and SE face the wall; the others leave the grid without hitting the flat ground below
    hits = {p["bearing"]: p["distance"] for p in points if not p["unobstructed"]}
    assert set(hits) == {45, 90, 135}
    wall = WALL_WEST_EDGE - OBSERVER[0]
    assert hits[90] == pytest.approx(wall)
    assert hits[45] == pytest.approx(wall * math.sqrt(2))
    average = sum(hits.values()) / 3
    assert body["average_distance"] == pytest.approx(average)

    for p in points:
        if p["unobstructed"]:
            assert p["distance"] == pytest.approx(average)
        # Every vertex is `distance` away from the observer along its bearing (hits: centre of the point hit)
        x, y = latlon_to_utm(p["latitude"], p["longitude"])
        assert math.hypot(x - OBSERVER[0], y - OBSERVER[1]) == pytest.approx(p["distance"], abs=CELL)


def test_collisions_carry_the_class_hit(tracer):
    # Elevations -0.5 and 0 rad. Looking east (column 2 of 8) the horizontal ray hits the wall, a building;
    # looking west the lower ray hits the ground.
    columns = tracer.ray_collisions_around(*OBSERVER, 10.0 + OBSERVER_HEIGHT, -0.5, 0.0, 2, 0.0, 2 * math.pi, 8)
    assert columns[2][1].classification == 6
    assert columns[6][0].classification == 2


def test_rays_beyond_data_have_no_collision_at_any_elevation(tracer):
    columns = tracer.ray_collisions_around(*OBSERVER, 100.0, -0.001, 0.001, 2, 0.0, 2 * math.pi, 8)
    assert all(isinstance(result, RayResult.NoCollision) for column in columns for result in column)
    distances = tracer.ray_collisions(*OBSERVER, 1.0, 0.0, 100.0, -0.001, 0.001, 2)
    assert distances == [math.inf, math.inf]


def test_panorama_is_a_png_of_the_full_circle():
    latitude, longitude = utm_to_latlon(*OBSERVER)
    r = client.get("/api/panorama.png", params={"latitude": latitude, "longitude": longitude})
    assert r.status_code == 200, r.text
    assert r.headers["content-type"] == "image/png"
    png = r.content
    assert png[:8] == b"\x89PNG\r\n\x1a\n"
    width, height = int.from_bytes(png[16:20], "big"), int.from_bytes(png[20:24], "big")
    assert (width, height) == (360 * 4, 30 * 4)


def test_panorama_without_data_is_404():
    latitude, longitude = utm_to_latlon(X_START - 100, Y_START)
    assert client.get("/api/panorama.png", params={"latitude": latitude, "longitude": longitude}).status_code == 404
    assert client.get("/api/panorama.png", params={"latitude": 100, "longitude": 0}).status_code == 422


def test_no_data_is_404():
    r = post_view(X_START + 55 * CELL, OBSERVER[1])  # no data from column 50
    assert r.status_code == 404


def test_nothing_hit_is_422():
    # On the wall, everything around is lower
    r = post_view(X_START + 42 * CELL, OBSERVER[1])
    assert r.status_code == 422
    assert "No ray hits" in r.json()["detail"]


def test_invalid_parameters_are_422():
    assert client.post("/api/view", json={"latitude": 100, "longitude": 0}).status_code == 422
    assert client.post("/api/view", json={"latitude": 49.28, "longitude": -123.12, "bearings": 2}).status_code == 422


def test_dense_view_is_thinned_without_changing_statistics():
    dense = post_view(*OBSERVER, bearings=1440).json()
    sparse = post_view(*OBSERVER, bearings=360).json()
    assert 3 <= len(dense['points']) < 1440
    assert dense['farthest_distance'] >= max(p['distance'] for p in dense['points'] if not p['unobstructed'])
    assert dense['unobstructed_share'] == pytest.approx(sparse['unobstructed_share'], abs=0.01)
    assert dense['average_distance'] == pytest.approx(sparse['average_distance'], rel=0.02)
    assert [p['bearing'] for p in dense['points']] == sorted(p['bearing'] for p in dense['points'])


def test_thinning_preserves_transitions_seam_and_small_polygons():
    from app.view import ViewPoint, thin_points
    points = [ViewPoint(i * 0.25, 49.28, -123.12, 1, i in (3, 4)) for i in range(12)]
    result = thin_points(points)
    assert points[0] in result and points[-1] in result
    assert all(points[i] in result for i in (2, 3, 4, 5))
    assert len(result) < len(points)
    assert thin_points(points[:3]) == points[:3]


def test_thinning_keeps_sharp_distance_changes_and_open_angular_coverage():
    from app.view import ViewPoint, thin_points
    points = [ViewPoint(i * 0.25, 49.28, -123.12, 10 if i == 5 else 1, False) for i in range(12)]
    result = thin_points(points)
    assert points[5] in result and points[6] in result
    open_points = [ViewPoint(i * 0.25, 49.28, -123.12, 0.1, True) for i in range(1440)]
    thinned = thin_points(open_points)
    assert len(thinned) == 361
    assert all(b.bearing - a.bearing <= 1 for a, b in zip(thinned, thinned[1:]))


def test_view_returns_structured_analysis_without_backend_geometry(monkeypatch):
    import app.view as view_module
    analysis = {
        "date": "2026-10-04", "beauty_score": 74.0,
        "sunrise": {"time": "2026-10-04T07:15:00-07:00", "bearing": 102.0, "open_share": 0.18},
        "sunset": {"time": "2026-10-04T18:45:00-07:00", "bearing": 258.0, "open_share": 0.82},
        "ocean_area": 260000.0, "lake_area": 20000.0, "water_area": 280000.0,
        "openness_area": 1120000.0, "landmarks": ["Test landmark"],
    }
    monkeypatch.setattr(view_module, "analyze", lambda *args: analysis)
    response = post_view(*OBSERVER, bearings=8)
    assert response.status_code == 200
    assert response.json()["analysis"] == analysis
    assert "polygon" not in response.json()


def test_missing_analysis_layers_preserve_core_view(monkeypatch):
    import app.view as view_module

    def unavailable(*args):
        raise FileNotFoundError("missing geographic layer")

    monkeypatch.setattr(view_module, "analyze", unavailable)
    response = post_view(*OBSERVER, bearings=8)
    assert response.status_code == 200
    assert response.json()["analysis"] is None
    assert response.json()["farthest_distance"] > 0
