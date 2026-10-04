"""Check score scale without requiring geographic datasets."""
import math
import sys
from datetime import datetime
from types import SimpleNamespace
from zoneinfo import ZoneInfo

import pytest

from app.analysis import analyze


def test_sun_percentages_do_not_saturate_beauty_score(monkeypatch):
    moment = datetime(2026, 10, 4, 7, 15, tzinfo=ZoneInfo("America/Vancouver"))
    dates = []

    def sun(lat, long, altitude, day):
        dates.append(day)
        return ([1.0, math.pi / 2, 2.0], moment)

    monkeypatch.setitem(sys.modules, "app.features", SimpleNamespace(
        sunrise=sun, sunset=sun, sunrise_score=lambda *args: 50,
        sunset_score=lambda *args: 100, ocean_area=lambda *args: 0,
        lake_area=lambda *args: 0, get_landmarks=lambda *args: [],
        openness=lambda *args: 1000,
    ))
    result = analyze(None, 49, -123, 38, [], [])
    assert result["beauty_score"] == pytest.approx(15)
    assert result["sunrise"]["open_share"] == 0.5
    assert result["sunrise"]["bearing"] == pytest.approx(90)
    assert result["sunrise"]["time"] == moment.isoformat()
    assert all(day.isoformat() == result["date"] for day in dates)
