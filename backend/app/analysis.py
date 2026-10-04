from shapely.geometry import Polygon
from features import sunrise, sunset, ocean_area, lake_area, get_landmarks, openness, sunrise_score, sunset_score
from datetime import date

date_ = date.today()

def analyze(polygon, lat, long, alt, border_points_, nonborder_points_):
    print(len(border_points_))
    print(len(nonborder_points_))
    sunrise_azimuth = sunrise(lat, long, alt, date_)
    sunset_azimuth = sunset(lat, long, alt, date_)
    sunrise_score_ = sunrise_score(border_points_, nonborder_points_, sunrise_azimuth[0]) 
    sunset_score_ = sunset_score(border_points_, nonborder_points_, sunset_azimuth[0])
    ocean_area_ = ocean_area(polygon)
    lake_area_ = lake_area(polygon)
    tot_water_area_ = ocean_area_ + lake_area_
    landmarks = get_landmarks(polygon)
    openness_ = openness(polygon)
    #BEAUTY SCORE CALCULATION
    openness_score = (
        (openness_ - 1_000) / (3_500_000 - 1_000)
    )
    openness_score = max(0, min(1, openness_score))
    openness_points = openness_score * 75
    water_score = tot_water_area_ / 1_750_000
    water_score = max(0, min(1, water_score))
    water_points = water_score * 20
    landmark_score = min(len(landmarks) / 5, 1)
    landmark_points = landmark_score * 10
    sunrise_points = (sunrise_score_ * 10)
    sunset_points = (sunset_score_ * 10)
    beauty_score = (
        openness_points
        + water_points
        + landmark_points
        + sunrise_points
        + sunset_points
    )
    beauty_score = min(100, beauty_score)
    print("Sunrise Azimuth:", sunrise_azimuth[0])
    print("Sunset Azimuth:", sunset_azimuth[0])
    print("Sunrise Score:", sunrise_score_)
    print("Sunset Score:", sunset_score_)
    print("Time of Sunrise:", sunrise_azimuth[1])
    print("Time of Sunset:", sunset_azimuth[1])
    print("Area of Ocean Interception:", ocean_area_)
    print("Area of Lake Interception:", lake_area_)
    print("Area of Total Water Interception:", tot_water_area_)
    print("List of Visible Landmarks:", landmarks)
    print("Openness:", openness_)
    print("Beauty Score:", beauty_score)
    print("Openness Points:", openness_points)
    print("Water Points:", water_points)
    print("Landmark Points:", landmark_points)
    print("Sunrise Points:", sunrise_points)
    print("Sunset Points:", sunset_points)
    