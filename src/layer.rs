use geojson::{Feature, FeatureCollection, GeoJson, Geometry, Value};
use proj::Proj;

pub fn get_nb_class(nb_features: u32) -> i32 {
    (1.0 + 3.3 * (nb_features as f64).log(10.0)).floor() as i32
}

pub fn get_values(geojson: &GeoJson, field_name: &String) -> Vec<f64> {
    let features = match geojson {
        &GeoJson::FeatureCollection(ref collection) => &collection.features,
        _ => panic!("Error: expected a Feature collection of polygons!"),
    };

    let mut res = Vec::new();
    for feature in features {
        if let Some(ref prop) = feature.properties {
            res.push(prop[field_name].as_f64().unwrap());
        } else {
            panic!("Unable to find field {}!", field_name);
        }
    }
    res
}

pub fn reproj(decoded_geojson: &mut GeoJson, input_proj: &Proj, output_proj: &Proj) -> GeoJson {
    let features = match decoded_geojson {
        &mut GeoJson::FeatureCollection(ref mut collection) => &collection.features,
        _ => panic!("Error: expected a Feature collection of polygons!"),
    };
    let mut res = Vec::new();
    for feature in features {
        let geom = feature.to_owned().geometry.unwrap();
        match geom.value {
            Value::Point(ref point) => {
                let p = input_proj
                    .project(&output_proj, (point[0].to_radians(), point[1].to_radians()));
                res.push(Feature {
                             geometry: Some(Geometry::new(Value::Point(vec![p.0, p.1]))),
                             properties: feature.properties.to_owned(),
                             bbox: None,
                             id: feature.id.to_owned(),
                             foreign_members: None,
                         });
            }
            Value::MultiPoint(points) => {
                let mut pts = Vec::new();
                for point in points {
                    let p =
                        input_proj
                            .project(&output_proj, (point[0].to_radians(), point[1].to_radians()));
                    pts.push(vec![p.0, p.1]);
                }
                res.push(Feature {
                             geometry: Some(Geometry::new(Value::MultiPoint(pts))),
                             properties: feature.properties.to_owned(),
                             bbox: None,
                             id: feature.id.to_owned(),
                             foreign_members: None,
                         });
            }
            Value::LineString(positions) => {
                let mut pos = Vec::new();
                for point in positions {
                    let p =
                        input_proj
                            .project(&output_proj, (point[0].to_radians(), point[1].to_radians()));
                    pos.push(vec![p.0, p.1]);
                }
                res.push(Feature {
                             geometry: Some(Geometry::new(Value::LineString(pos))),
                             properties: feature.properties.to_owned(),
                             bbox: None,
                             id: feature.id.to_owned(),
                             foreign_members: None,
                         });
            }
            Value::MultiLineString(lines) => {
                let mut pos = Vec::new();
                for line in lines {
                    let mut v = Vec::new();
                    for point in line {
                        let p =
                            input_proj.project(&output_proj,
                                               (point[0].to_radians(), point[1].to_radians()));
                        v.push(vec![p.0, p.1]);
                    }
                    pos.push(v);
                }
                res.push(Feature {
                             geometry: Some(Geometry::new(Value::MultiLineString(pos))),
                             properties: feature.properties.to_owned(),
                             bbox: None,
                             id: feature.id.to_owned(),
                             foreign_members: None,
                         });
            }
            Value::Polygon(poly) => {
                let mut pos = Vec::new();
                for ring in poly {
                    let mut v = Vec::new();
                    for point in ring {
                        let p =
                            input_proj.project(&output_proj,
                                               (point[0].to_radians(), point[1].to_radians()));
                        v.push(vec![p.0, p.1]);
                    }
                    pos.push(v);
                }
                res.push(Feature {
                             geometry: Some(Geometry::new(Value::Polygon(pos))),
                             properties: feature.properties.to_owned(),
                             bbox: None,
                             id: feature.id.to_owned(),
                             foreign_members: None,
                         });
            }
            Value::MultiPolygon(positions) => {
                let mut pos = Vec::new();
                for poly in positions {
                    let mut v = Vec::new();
                    for ring in poly {
                        let mut _v = Vec::new();
                        for point in ring {
                            let p = input_proj.project(&output_proj,
                                                       (point[0].to_radians(),
                                                        point[1].to_radians()));
                            _v.push(vec![p.0, p.1]);
                        }
                        v.push(_v)
                    }
                    pos.push(v);
                }
                res.push(Feature {
                             geometry: Some(Geometry::new(Value::MultiPolygon(pos))),
                             properties: feature.properties.to_owned(),
                             bbox: None,
                             id: feature.id.to_owned(),
                             foreign_members: None,
                         });
            }
            _ => panic!("I don't know what to do!"),
        }
    }
    GeoJson::from(FeatureCollection {
                      bbox: None,
                      foreign_members: None,
                      features: res,
                  })
}
