use anyhow::{bail, Result};
use geo::{Coord, Intersects, LineString, OffsetCurve, Polygon};
use indicatif::{ProgressBar, ProgressStyle};
use rstar::{RTree, RTreeObject};

use crate::{LCWIP, PCT};

pub fn wgs84_to_osgb36(c: Coord) -> Result<Coord> {
    match lonlat_bng::convert_osgb36(c.x, c.y) {
        Ok((x, y)) => Ok(Coord { x, y }),
        Err(()) => bail!("convert_osgb36 broke on {c:?}"),
    }
}

pub fn osgb36_to_wgs84(c: Coord) -> Result<Coord> {
    match lonlat_bng::convert_osgb36_to_ll(c.x, c.y) {
        Ok((x, y)) => Ok(Coord { x, y }),
        Err(()) => bail!("convert_osgb36_to_ll broke on {c:?}"),
    }
}

pub fn spatial_join<F: FnMut(&LCWIP, &Polygon, Vec<&PCT>) -> Result<()>>(
    lcwip: Vec<LCWIP>,
    pct: Vec<PCT>,
    mut cb: F,
) -> Result<()> {
    println!("Building spatial index");
    let rtree = RTree::bulk_load(pct);

    println!("Joining LCWIP features with PCT, using an index");
    let progress = ProgressBar::new(lcwip.len() as u64).with_style(ProgressStyle::with_template(
                    "[{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({per_sec}, {eta})").unwrap());
    for input in lcwip {
        progress.inc(1);
        let Some(buffered) = buffer_linestring(&input.geometry, 5.0, 5.0) else {
            println!("  Couldn't buffer a linestring, skipping");
            continue;
        };

        // Find all intersecting linestrings
        let mut matching_objects = Vec::new();
        for obj in rtree.locate_in_envelope_intersecting(&buffered.envelope()) {
            if buffered.intersects(&obj.geometry) {
                matching_objects.push(obj);
            }
        }
        cb(&input, &buffered, matching_objects)?;
    }
    progress.finish();
    Ok(())
}

// TODO Eventually this'll be in geo. Uses OffsetCurve from https://github.com/georust/geo/pull/935
pub fn buffer_linestring(
    linestring: &LineString,
    left_meters: f64,
    right_meters: f64,
) -> Option<Polygon> {
    assert!(left_meters >= 0.0);
    assert!(right_meters >= 0.0);
    let left = linestring.offset_curve(-left_meters)?;
    let right = linestring.offset_curve(right_meters)?;
    // Make a polygon by gluing these points together
    let mut pts = left.0;
    pts.reverse();
    pts.extend(right.0);
    Some(Polygon::new(LineString(pts), Vec::new()))
}
