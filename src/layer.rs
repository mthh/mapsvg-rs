use std::f64;
use geojson::{Feature, FeatureCollection, GeoJson, Geometry, Value};
use proj::Proj;

use config_params::MapExtent;

pub fn get_nb_class(nb_features: usize) -> u32 {
    (1.0 + 3.3 * (nb_features as f64).log(10.0)).floor() as u32
}

pub fn get_values(features: &[Feature], field_name: &String) -> Vec<f64> {
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

pub fn get_extent(geojson: &GeoJson) -> MapExtent {
    fn verif(point: &Vec<f64>, extent: &mut MapExtent) {
        if point[0] > extent.right {
            extent.right = point[0];
        } else if point[0] < extent.left {
            extent.left = point[0];
        }
        if point[1] > extent.top {
            extent.top = point[1];
        } else if point[1] < extent.bottom {
            extent.bottom = point[1];
        }
    }
    let features = match geojson {
        &GeoJson::FeatureCollection(ref collection) => &collection.features,
        _ => panic!("Error: expected a Feature collection of polygons!"),
    };
    let mut extent = MapExtent {
        left: f64::MAX,
        right: f64::MIN,
        bottom: f64::MAX,
        top: f64::MIN,
    };
    for feature in features {
        if let Some(ref geom) = feature.geometry {
            match geom.value {
                Value::Point(ref point) => {
                    verif(point, &mut extent);
                }
                Value::MultiPoint(ref points) |
                Value::LineString(ref points) => {
                    for point in points {
                        verif(point, &mut extent);
                    }
                }
                Value::MultiLineString(ref rings) |
                Value::Polygon(ref rings) => {
                    for ring in rings {
                        for point in ring {
                            verif(point, &mut extent);
                        }
                    }
                }
                Value::MultiPolygon(ref polygons) => {
                    for polygon in polygons {
                        for ring in polygon {
                            for point in ring {
                                verif(point, &mut extent);
                            }
                        }
                    }
                }
                _ => panic!("GeometryCollection not handled yet!"),
            }
        }
    }
    let a = (extent.right - extent.left) / 10.0;
    let b = (extent.top - extent.bottom) / 10.0;
    extent.right += a;
    extent.left -= a;
    extent.top += b;
    extent.bottom -= b;
    extent
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
                    .project(&output_proj, (point[0].to_radians(), point[1].to_radians()))
                    .unwrap();
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
                    let p = input_proj
                        .project(&output_proj, (point[0].to_radians(), point[1].to_radians()))
                        .unwrap();
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
                    let p = input_proj
                        .project(&output_proj, (point[0].to_radians(), point[1].to_radians()))
                        .unwrap();
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
                        let p = input_proj
                            .project(&output_proj, (point[0].to_radians(), point[1].to_radians()))
                            .unwrap();
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
                        let p = input_proj
                            .project(&output_proj, (point[0].to_radians(), point[1].to_radians()))
                            .unwrap();
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
                            let p = input_proj
                                .project(&output_proj,
                                         (point[0].to_radians(), point[1].to_radians()))
                                .unwrap();
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


pub fn reproj_graticule(decoded_geojson: &mut GeoJson,
                        input_proj: &Proj,
                        output_proj: &Proj)
                        -> GeoJson {
    let features = match decoded_geojson {
        &mut GeoJson::FeatureCollection(ref mut collection) => &collection.features,
        _ => panic!("Error: expected a Feature collection of polygons!"),
    };
    let mut res = Vec::new();
    for feature in features {
        let geom = feature.to_owned().geometry.unwrap();
        match geom.value {
            Value::MultiLineString(lines) => {
                let mut pos = Vec::new();
                for line in lines {
                    let mut v = Vec::new();
                    for point in line {
                        let p =
                            input_proj.project(&output_proj,
                                               (point[0].to_radians(), point[1].to_radians()));
                        match p {
                            Ok(pt) => {
                                v.push(vec![pt.0, pt.1]);
                            }
                            Err(_) => {}
                        }
                    }
                    if v.len() > 1 {
                        pos.push(v);
                    }
                }
                res.push(Feature {
                             geometry: Some(Geometry::new(Value::MultiLineString(pos))),
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
