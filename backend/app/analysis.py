"""Structured view analysis, independent of AI panorama descriptions."""

import logging
import math
from datetime import datetime
from zoneinfo import ZoneInfo


def analyze(polygon, lat, long, alt, border_points, nonborder_points):
    # Geographic layers are optional on development machines. Import only when needed.
    from .features import sunrise, sunset, ocean_area, lake_area, get_landmarks, openness, sunrise_score, sunset_score

    today = datetime.now(ZoneInfo("America/Vancouver")).date()
    rise = sunrise(lat, long, alt, today)
    setting = sunset(lat, long, alt, today)
    rise_share = sunrise_score(border_points, nonborder_points, rise[0])
    set_share = sunset_score(border_points, nonborder_points, setting[0])
    ocean = float(ocean_area(polygon))
    lake = float(lake_area(polygon))
    landmarks = get_landmarks(polygon)
    area = float(openness(polygon))
    score = min(100, max(0, min(1, (area - 1_000) / (3_500_000 - 1_000))) * 75
                + max(0, min(1, (ocean + lake) / 1_750_000)) * 20
                + min(len(landmarks) / 5, 1) * 10 + rise_share * 10 + set_share * 10)
    logging.getLogger(__name__).debug(
        "View analysis: open rays=%d, blocked rays=%d, area=%s, water=%s, landmarks=%d, sunrise=%s, sunset=%s, beauty=%s",
        len(border_points), len(nonborder_points), area, ocean + lake, len(landmarks), rise_share, set_share, score,
    )
    return {
        "date": today.isoformat(),
        "beauty_score": score,
        "sunrise": {"time": rise[1].isoformat(), "bearing": math.degrees(rise[0][1]) % 360, "open_share": rise_share},
        "sunset": {"time": setting[1].isoformat(), "bearing": math.degrees(setting[0][1]) % 360, "open_share": set_share},
        "ocean_area": ocean,
        "lake_area": lake,
        "water_area": ocean + lake,
        "openness_area": area,
        "landmarks": landmarks,
    }
