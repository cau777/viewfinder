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

### SUNRISE & SUNSET

# Returns sunrise azimuth in degrees
def sunrise(lat, long, z, date_):
    point = Observer(latitude=lat, longitude=long)
    s = sun(point, date=date_, tzinfo=ZoneInfo("America/Vancouver"))
    return azimuth(point, s["sunrise"])

# Returns sunset azimuth in degrees
def sunset(lat, long, z, date_):
    point = Observer(latitude=lat, longitude=long)
    s = sun(point, date=date_, tzinfo=ZoneInfo("America/Vancouver"))
    return azimuth(point, s["sunset"])

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

print("Landmark: ", get_landmark(49.303200, -123.147750))
print("Landmark: ", get_landmark(49.285956, -123.135573))
print("Landmark: ", get_landmark(49.284402, -123.108909))

### PHOTOS

osv5m_path = pathlib.Path(
    "~/data/raw/photos/vancouver_osv5m.csv"
).expanduser()

osv5m_van = pd.read_csv(osv5m_path)
osv5m_van = osv5m_van[["id", "latitude", "longitude"]]

# print(osv5m_van[["latitude", "longitude"]])