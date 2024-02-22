import geopandas as gpd
import pandas as pd
import json
import time

def addBuffer(geodataframe, buffer_radius):
    original_crs = geodataframe.crs
    buffered_geodataframe = geodataframe.to_crs(3857).copy()
    buffered_geodataframe.geometry = buffered_geodataframe['geometry'].buffer(buffer_radius)
    return buffered_geodataframe.to_crs(original_crs)

def printTimeAndMessage(message):
    t = time.localtime()
    current_time = time.strftime("%H:%M:%S", t)
    text_to_print = "{}: {}"
    print(text_to_print.format(current_time, message))

def joinAndSave(leftGDF, rightGDF, name):
    printTimeAndMessage("Starting to Join: " + name)
    joinedGDF = leftGDF.sjoin(rightGDF, how="left", lsuffix='_left', rsuffix='_right')
    printTimeAndMessage("Finished joining: " + name)
    joinedGDF.to_file('./output/' + name + '.geojson', driver='GeoJSON')
    printTimeAndMessage("Saved: output/" + name + ".geojson")

printTimeAndMessage("job's started")
all_schemes = gpd.read_file('./input/all_lcwips_output.geojson')
    

with open('./input/all_lcwips_output.geojson') as json_file:
    json_loaded = json.load(json_file)

    ids = pd.json_normalize(json_loaded["features"])["id"]
    all_schemes["feature_id"] = ids


    routes_only = all_schemes.loc[all_schemes.geometry.geometry.type=='LineString']
    print(len(routes_only))
    print(5/len(routes_only))

    routes_only = routes_only.head()

    printTimeAndMessage("about to add buffers")
    buffered_lcwips = addBuffer(routes_only, 12.5) 
    printTimeAndMessage("buffers added")

    buffered_lcwips.to_file('./output/buffered_lcwips_sample.geojson', driver='GeoJSON')


    commute = gpd.read_file('./input/commute-rnet_all.geojson')
    school = gpd.read_file('./input/school-rnet_all.geojson')

    joinAndSave(buffered_lcwips, commute, "lcwip-commutes")
    joinAndSave(buffered_lcwips, school, "lcwip-school-trips")

    printTimeAndMessage("job's done, writing other outputs next")
    buffered_lcwips.to_file('./output/buffered_lcwips.geojson', driver='GeoJSON')
    routes_only.to_file('./output/routes_only.geojson', driver='GeoJSON')
    printTimeAndMessage("outputs written")


