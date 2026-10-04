from shapely.geometry import Polygon
from features import sunrise, sunset, ocean_area, lake_area, get_landmarks, openness, sunrise_score, sunset_score
from datetime import date

date_ = date.today()

def analyze(polygon, lat, long, alt, border_points_, nonborder_points_):
    sunrise_azimuth = sunrise(lat, long, alt, date_)
    sunset_azimuth = sunset(lat, long, alt, date_)
    sunrise_score_ = sunrise_score(border_points_, nonborder_points_, sunrise_azimuth[0]) 
    sunset_score_ = sunset_score(border_points_, nonborder_points_, sunset_azimuth[0])
    ocean_area_ = ocean_area(polygon)
    lake_area_ = lake_area(polygon)
    tot_water_area_ = ocean_area_ + lake_area_
    landmarks = get_landmarks(polygon)
    openness_ = openness(polygon)
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
    
    
    
    