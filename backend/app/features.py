from astral import Observer
from astral.sun import sun, azimuth
from datetime import date
from zoneinfo import ZoneInfo
import geopandas as gpd
from shapely.geometry import box
from shapely.geometry import Point
from huggingface_hub import hf_hub_download
import pandas as pd
import pathlib
import matplotlib.pyplot as plt
import contextily as ctx
import math

### SUNRISE & SUNSET

# Returns sunrise azimuth in degrees
def sunrise(lat, long, z, date_):
    point = Observer(latitude=lat, longitude=long)
    s = sun(point, date=date_, tzinfo=ZoneInfo("America/Vancouver"))
    center = math.radians(azimuth(point, s["sunrise"]))
    interval = 0.1
    lower = center - interval
    upper = center + interval
    return [lower, center, upper], s["sunrise"]

# print(sunrise(
#     lat=49.2827,
#     long=-123.1207,
#     z=0,
#     date_=date(2026, 10, 4)
# ))

# Returns sunset azimuth in degrees
def sunset(lat, long, z, date_):
    point = Observer(latitude=lat, longitude=long)
    s = sun(point, date=date_, tzinfo=ZoneInfo("America/Vancouver"))
    center = math.radians(azimuth(point, s["sunset"]))
    interval = 0.1
    lower = center - interval
    upper = center + interval
    return [lower, center, upper], s["sunset"]

# print(sunset(
#     lat=49.2827,
#     long=-123.1207,
#     z=0,
#     date_=date(2026, 10, 4)
# ))

def sunrise_score(border_points, points, sunrise_azimuth):
    lower, _, upper = sunrise_azimuth

    border_within_interval = sum(
        lower <= math.radians(border_point[0]) <= upper
        for border_point in border_points
    )

    nonborder_within_interval = sum(
        lower <= math.radians(point[0]) <= upper
        for point in points
    )

    total_points_within_interval = border_within_interval + nonborder_within_interval

    print("tot_points:", total_points_within_interval)
    print("border_within:", border_within_interval)
    print("nonborder_within", nonborder_within_interval)

    return (border_within_interval) / total_points_within_interval if total_points_within_interval > 0 else 0

def sunset_score(border_points, points, sunset_azimuth):
    lower, _, upper = sunset_azimuth

    border_within_interval = sum(
        lower <= math.radians(border_point[0]) <= upper
        for border_point in border_points
    )

    nonborder_within_interval = sum(
        lower <= math.radians(point[0]) <= upper
        for point in points
    )

    total_points_within_interval = border_within_interval + nonborder_within_interval

    return (border_within_interval) / total_points_within_interval if total_points_within_interval > 0 else 0

### OCEAN

filename = "~/data/raw/world-internal-waters/eez_internal_waters_v4.gpkg"
gdf = gpd.read_file(filename)
canada_gdf = gdf[gdf["SOVEREIGN1"] == "Canada"]
canada_geo = canada_gdf.geometry
van_box = box(-124, 49, -122, 49.5)
van_ocean = gpd.clip(canada_geo, van_box)

# Returns true if water, false if not
def is_ocean(lat, long):
    point = Point(long, lat)
    bool = canada_geo.geometry.contains(point).any()
    return bool

def ocean_area(polygon):
    intersections = canada_geo.intersection(polygon)
    return intersections.to_crs("EPSG:32610").area.sum()

# print("Ocean: ", is_ocean(49.303570, -123.205256))
# print("Ocean: ", is_ocean(49.266451, -123.209655))

### LAKES

van_lakes = gpd.read_file("~/data/raw/hydro/vancouver_lakes.gpkg")
lake_index = van_lakes.sindex

# Returns true if lake, false if not
def is_lake(lat, long):
    point = Point(long, lat)

    candidates = lake_index.query(point, predicate="within")

    return len(candidates) > 0

def lake_area(polygon):
    intersections = van_lakes.geometry.intersection(polygon)

    return intersections.to_crs("EPSG:32610").area.sum()

# print("Lake: ", is_lake(49.236201, -122.972080))
# print("Lake: ", is_lake(49.242227, -122.972166))

### LANDMARKS

van_landmarks = gpd.read_file("~/data/raw/landmarks/van_landmarks.gpkg")
landmarks_index = van_landmarks.sindex

# Returns true if lake, false if not
def get_landmark(lat, long):
    point = Point(long, lat)

    candidates = landmarks_index.query(point, predicate="within")

    if len(candidates) == 0:
        return False

    return van_landmarks.iloc[candidates[0]]["name"]

def get_landmarks(polygon):
    intersections = van_landmarks.geometry.intersects(polygon)

    return van_landmarks.loc[intersections, "name"].tolist()

# print("Landmark: ", get_landmark(49.303200, -123.147750))
# print("Landmark: ", get_landmark(49.285956, -123.135573))
# print("Landmark: ", get_landmark(49.284402, -123.108909))

### OPENNESS

def openness(polygon):
    polygon = gpd.GeoSeries([polygon], crs="EPSG:4326").to_crs("EPSG:32610")
    return polygon.area.iloc[0]

### PHOTOS

osv5m_path = pathlib.Path(
    "~/data/raw/photos/vancouver_osv5m.csv"
).expanduser()

osv5m_van = pd.read_csv(osv5m_path)
osv5m_van = osv5m_van[["id", "latitude", "longitude"]]

# print(osv5m_van[["latitude", "longitude"]])