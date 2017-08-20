use geojson::{GeoJson, Feature, FeatureCollection, Geometry, Value};
use std::ops::Add;

fn step_by<T, F>(start: T, end_inclusive: T, step: T, mut body: F)
    where T: Add<Output = T> + PartialOrd + Copy,
          F: FnMut(T)
{
    let mut i = start;
    while i <= end_inclusive {
        body(i);
        i = i + step;
    }
}


pub fn prepare_geojson_graticule() -> GeoJson {
    let mut coordinates = Vec::new();
    step_by(-179.99, 179.99, 9.9, |x| {
        let mut v = Vec::new();
        step_by(-89.99, 89.99, 9.9, |y| { v.push(vec![x, y]); });
        coordinates.push(v);
    });
    step_by(-89.99, 89.99, 9.9, |y| {
        let mut v = Vec::new();
        step_by(-179.99, 179.99, 9.9, |x| { v.push(vec![x, y]); });
        coordinates.push(v);
    });
    let features = vec![Feature {
                            geometry: Some(Geometry::new(Value::MultiLineString(coordinates))),
                            properties: None,
                            bbox: None,
                            id: None,
                            foreign_members: None,
                        }];
    GeoJson::FeatureCollection(FeatureCollection {
                                   bbox: None,
                                   features: features,
                                   foreign_members: None,
                               })
}
