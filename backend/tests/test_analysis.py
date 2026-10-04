"""Check score scale without requiring geographic datasets."""
import math
import sys
from datetime import datetime
from types import SimpleNamespace
from zoneinfo import ZoneInfo

import pytest

from app.analysis import analyze


@pytest.mark.parametrize("area, expected_score", [(1000, 15), (3500000, 90), (7000000, 90)])
def test_sun_fractions_and_refined_openness_weight(monkeypatch, area, expected_score):
    moment = datetime(2026, 10, 4, 7, 15, tzinfo=ZoneInfo("America/Vancouver"))
    dates = []

    def sun(lat, long, altitude, day):
        dates.append(day)
        return ([1.0, math.pi / 2, 2.0], moment)

    monkeypatch.setitem(sys.modules, "app.features", SimpleNamespace(
        sunrise=sun, sunset=sun, sunrise_score=lambda *args: 0.5,
        sunset_score=lambda *args: 1, ocean_area=lambda *args: 0,
        lake_area=lambda *args: 0, get_landmarks=lambda *args: [],
        openness=lambda *args: area,
    ))
    result = analyze(None, 49, -123, 38, [], [])
    assert result["beauty_score"] == pytest.approx(expected_score)
    assert result["sunrise"]["open_share"] == 0.5
    assert result["sunrise"]["bearing"] == pytest.approx(90)
    assert result["sunrise"]["time"] == moment.isoformat()
    assert all(day.isoformat() == result["date"] for day in dates)
