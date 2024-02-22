mod geo_utils;

use anyhow::{bail, Result};
use geo::{LineString, MapCoords, MapCoordsInPlace, Point};
use geojson::{
    de::deserialize_geometry, feature::Id, Feature, FeatureCollection, GeoJson, Geometry,
};
use rstar::{RTreeObject, AABB};
use serde::{Deserialize, Serialize};

use self::geo_utils::*;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        panic!("Call with path to lcwips.geojson and pct.geojson");
    }

    let lcwip = load_lcwip(&args[1])?;
    let pct = load_pct(&args[2])?;

    if false {
        // Actually produce the results
        let mut results = Vec::new();
        spatial_join(lcwip, pct, |input, _buffered, matching_pct| {
            results.push(JoinResult {
                feature_id: input.id,
                matching_baseline_values: matching_pct.iter().map(|x| x.baseline).collect(),
            });
            Ok(())
        })?;
        fs_err::write("joined.json", &serde_json::to_string_pretty(&results)?)?;
    } else {
        // Debug individual matches
        let mut count = 5;
        spatial_join(lcwip, pct, |input, buffered, matching_pct| {
            if count > 0 {
                count -= 1;
                let mut features = Vec::new();

                // The original linestring
                features.push(Feature::from(Geometry::from(
                    &input.geometry.try_map_coords(osgb36_to_wgs84)?,
                )));
                // Buffered
                features.push(Feature::from(Geometry::from(
                    &buffered.try_map_coords(osgb36_to_wgs84)?,
                )));
                // And any matching PCT segments
                for x in matching_pct {
                    let mut f =
                        Feature::from(Geometry::from(&x.geometry.try_map_coords(osgb36_to_wgs84)?));
                    f.set_property("baseline", x.baseline);
                    features.push(f);
                }

                let gj = GeoJson::from(features);
                fs_err::write(
                    format!("debug{count}.geojson"),
                    &serde_json::to_string_pretty(&gj)?,
                )?;
            }
            Ok(())
        })?;
    }

    Ok(())
}

fn load_lcwip(path: &str) -> Result<Vec<LCWIP>> {
    // TODO deserialize_feature_collection_str_to_vec doesn't support reading the feature ID, so do
    // this more manually
    println!("Loading {path}");
    let string = fs_err::read_to_string(path)?;
    let fc: FeatureCollection = string.parse()?;

    println!(
        "  Got {} features. Transforming from WGS84 to OSGB36...",
        fc.features.len()
    );
    let mut list = Vec::new();
    for f in fc {
        let Some(Id::Number(ref number_id)) = f.id else {
            bail!("Some feature doesn't have a numeric ID");
        };
        let id = number_id.as_u64().unwrap();
        // TODO Feed in cleaned input with only LineStrings
        let mut linestring: LineString = match f.try_into() {
            Ok(ls) => ls,
            Err(_) => {
                continue;
            }
        };
        // TODO Isles of Scilly is breaking
        match linestring.try_map_coords_in_place(wgs84_to_osgb36) {
            Ok(()) => {
                list.push(LCWIP {
                    geometry: linestring,
                    id,
                });
            }
            Err(err) => {
                println!("  {err}");
            }
        }
    }
    Ok(list)
}

pub struct LCWIP {
    // In OSGB36
    geometry: LineString,
    // Feature ID from the input
    id: u64,
}

fn load_pct(path: &str) -> Result<Vec<PCT>> {
    println!("Loading {path}");
    let string = fs_err::read_to_string(path)?;
    let mut features: Vec<PCT> = geojson::de::deserialize_feature_collection_str_to_vec(&string)?;
    println!(
        "  Got {} features. Transforming from WGS84 to OSGB36...",
        features.len()
    );

    for f in &mut features {
        f.geometry.try_map_coords_in_place(wgs84_to_osgb36)?;
    }

    Ok(features)
}

#[derive(Deserialize)]
pub struct PCT {
    // In OSGB36
    #[serde(deserialize_with = "deserialize_geometry")]
    geometry: LineString,
    baseline: usize,
}

#[derive(Serialize)]
struct JoinResult {
    feature_id: u64,
    matching_baseline_values: Vec<usize>,
}

impl RTreeObject for PCT {
    type Envelope = AABB<Point<f64>>;

    fn envelope(&self) -> Self::Envelope {
        self.geometry.envelope()
    }
}
