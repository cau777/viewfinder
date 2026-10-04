import math

import numpy as np
import pytest
from fastapi.testclient import TestClient

from app.main import app, get_tracer
from app.view import OBSERVER_HEIGHT
from viewfinder_core import RayTracer, latlon_to_utm, utm_to_latlon

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
    (directory / "index.csv").write_text(
        "file,width,height,x_start,y_start,x_end,alt_min,alt_max\n"
        f"t.u16,60,60,{X_START},{Y_START},{X_START + 59 * CELL},0,100\n"
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
