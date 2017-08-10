extern crate svg;
extern crate geojson;
extern crate clap;
extern crate toml;
extern crate proj;
extern crate colorbrewer;

use std::collections::BTreeMap;
use clap::{Arg, App};
use geojson::{GeoJson, Value};
use proj::Proj;
use std::fs::File;
use std::io::Read;
use svg::Document;
use svg::Node;
use svg::node::element::{Circle, Group, Path, Rectangle as Rect, Text};
use svg::node::Text as NodeText;
use svg::node::element::path::Data;


macro_rules! expect_float {
    ($value:expr, $name:expr) => (
        match $value.as_float() {
            Some(v) => v,
            None => panic!("Expected float value on property \"{}\"!", $name)
        }
    )
}

macro_rules! string_or_default {
    ($value:expr, $default:expr) => (
        if $value.is_none() {
            $default.to_string()
        } else {
            match $value.unwrap().as_str() {
                Some(v) => v.to_string(),
                None => $default.to_string()
            }
        }
    )
}

struct MapExtent {
    left: f64,
    right: f64,
    bottom: f64,
    top: f64,
}

struct Converter<'a> {
    viewport_width: u32,
    viewport_height: u32,
    map_extent: &'a MapExtent,
    resolution: f64,
}

struct LayerProperties {
    fill: String,
    fill_opacity: String,
    stroke: String,
    stroke_opacity: String,
    stroke_width: String,
    radius: String,
}

fn get_nb_class(nb_features: f64) -> i32 {
    (1.0 + 3.3 * nb_features.log(10.0)).floor() as i32
}

fn get_values(geojson: &GeoJson, field_name: &String) -> Vec<f64> {
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

fn reproj(decoded_geojson: &mut GeoJson, input_proj: &Proj, output_proj: &Proj) -> GeoJson {
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
                res.push(geojson::Feature {
                             geometry: Some(geojson::Geometry::new(Value::Point(vec![p.0, p.1]))),
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
                res.push(geojson::Feature {
                             geometry: Some(geojson::Geometry::new(Value::MultiPoint(pts))),
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
                res.push(geojson::Feature {
                             geometry: Some(geojson::Geometry::new(Value::LineString(pos))),
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
                res.push(geojson::Feature {
                             geometry: Some(geojson::Geometry::new(Value::MultiLineString(pos))),
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
                res.push(geojson::Feature {
                             geometry: Some(geojson::Geometry::new(Value::Polygon(pos))),
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
                res.push(geojson::Feature {
                             geometry: Some(geojson::Geometry::new(Value::MultiPolygon(pos))),
                             properties: feature.properties.to_owned(),
                             bbox: None,
                             id: feature.id.to_owned(),
                             foreign_members: None,
                         });
            }
            _ => panic!("I don't know what to do!"),
        }
    }
    geojson::GeoJson::from(geojson::FeatureCollection {
                               bbox: None,
                               foreign_members: None,
                               features: res,
                           })
}

impl LayerProperties {
    fn from_config(c: &BTreeMap<String, toml::value::Value>) -> Self {
        LayerProperties {
            fill: string_or_default!(c.get("fill"), "blue"),
            fill_opacity: string_or_default!(c.get("fill-opacity"), "0.8"),
            stroke: string_or_default!(c.get("stroke"), "black"),
            stroke_opacity: string_or_default!(c.get("stroke-opacity"), "1"),
            stroke_width: string_or_default!(c.get("stroke-width"), "0.7"),
            radius: string_or_default!(c.get("radius"), "4"),
        }
    }
    fn default() -> Self {
        LayerProperties {
            fill: String::from("blue"),
            fill_opacity: String::from("0.8"),
            stroke: String::from("black"),
            stroke_opacity: String::from("1"),
            stroke_width: String::from("0.7"),
            radius: String::from("4"),
        }
    }
}

impl<'a> Converter<'a> {
    pub fn new(viewport_width: u32, viewport_height: u32, map_extent: &'a MapExtent) -> Self {
        let xres = (map_extent.right - map_extent.left) / viewport_width as f64;
        let yres = (map_extent.top - map_extent.bottom) / viewport_height as f64;
        let res = xres.max(yres);
        Converter {
            viewport_width: viewport_width,
            viewport_height: viewport_height,
            map_extent: map_extent,
            resolution: res,
        }
    }

    pub fn convert(&self, decoded_geojson: GeoJson, prop: &LayerProperties) -> Group {
        let features = match decoded_geojson {
            GeoJson::FeatureCollection(collection) => collection.features,
            _ => panic!("Error: expected a Feature collection!"),
        };

        let mut group = Group::new();
        for feature in features {
            let geom = feature.geometry.unwrap();
            match geom.value {
                Value::Point(point) => group.append(self.draw_point(&point, prop)),
                Value::MultiPoint(points) => {
                    for point in &points {
                        group.append(self.draw_point(&point, prop));
                    }
                }
                Value::LineString(positions) => {
                    let data = Data::new();
                    group.append(Path::new()
                                     .set("fill", "none")
                                     .set("stroke", prop.stroke.clone())
                                     .set("stroke-width", prop.stroke_width.clone())
                                     .set("stroke-opacity", prop.stroke_opacity.clone())
                                     .set("d",
                                          self.draw_path_polygon(&[positions], Some(data))));
                }
                Value::MultiLineString(lines) => {
                    let mut data = Data::new();
                    for positions in &lines {
                        data = self.draw_path_polygon(&[positions.to_vec()], Some(data));
                    }
                    group.append(Path::new()
                                     .set("fill", "none")
                                     .set("stroke", prop.stroke.clone())
                                     .set("stroke-width", prop.stroke_width.clone())
                                     .set("stroke-opacity", prop.stroke_opacity.clone())
                                     .set("d", data));
                }
                Value::Polygon(positions) => {
                    group.append(Path::new()
                                     .set("fill", prop.fill.clone())
                                     .set("fill-opacity", prop.fill_opacity.clone())
                                     .set("stroke", prop.stroke.clone())
                                     .set("stroke-width", prop.stroke_width.clone())
                                     .set("stroke-opacity", prop.stroke_opacity.clone())
                                     .set("d", self.draw_path_polygon(&positions, None)));
                }
                Value::MultiPolygon(polys) => {
                    let mut data = Data::new();
                    for positions in &polys {
                        data = self.draw_path_polygon(positions, Some(data));
                    }
                    data = data.close();
                    group.append(Path::new()
                                     .set("fill", prop.fill.clone())
                                     .set("fill-opacity", prop.fill_opacity.clone())
                                     .set("stroke", prop.stroke.clone())
                                     .set("stroke-width", prop.stroke_width.clone())
                                     .set("stroke-opacity", prop.stroke_opacity.clone())
                                     .set("d", data));
                }
                _ => panic!("I don't handle GeometryCollection yet!!"),
            }
        }
        group
    }

    fn draw_point(&self, point: &[f64], prop: &LayerProperties) -> Circle {
        Circle::new()
            .set("cx", (point[0] - self.map_extent.left) / self.resolution)
            .set("cy", (self.map_extent.top - point[1]) / self.resolution)
            .set("r", prop.radius.clone())
            .set("fill", prop.fill.clone())
    }

    fn draw_path_polygon(&self, positions: &[Vec<Vec<f64>>], d: Option<Data>) -> Data {
        let (mut data, close) = if d.is_some() {
            (d.unwrap(), false)
        } else {
            (Data::new(), true)
        };
        for ring in positions {
            let mut iter = ring.iter();
            let first = iter.next().unwrap();
            data = data.move_to(((first[0] - self.map_extent.left) / self.resolution,
                                 (self.map_extent.top - first[1]) / self.resolution));
            for point in iter {
                data = data.line_to(((point[0] - self.map_extent.left) / self.resolution,
                                     (self.map_extent.top - point[1]) / self.resolution));
            }
        }
        if close { data.close() } else { data }
    }
}

fn main() {
    let matches = App::new("geojson2svg")
        .version("0.1.0")
        .about("Convert geojson to svg")
        .arg(Arg::with_name("input")
                 .short("i")
                 .long("input")
                 .required(true)
                 .takes_value(true)
                 .value_name("FILE")
                 .help("Input configuration file to use (.toml)."))
        .get_matches();
    let file_path = matches.value_of("input").unwrap();

    let mut file = File::open(file_path).unwrap_or_else(|err| {
        println!("Unable to open configuration file: {}\nError: {}",
                 file_path,
                 err);
        std::process::exit(1)
    });
    let mut a = String::new();
    file.read_to_string(&mut a).unwrap();
    let config_options = a.parse::<toml::Value>().unwrap();
    let width: u32 = config_options["map"]["width"].as_integer().unwrap() as u32;
    let height: u32 = config_options["map"]["height"].as_integer().unwrap() as u32;
    let map_extent = MapExtent {
        left: expect_float!(config_options["map"]["extent"][0], "extent"),
        right: expect_float!(config_options["map"]["extent"][1], "extent"),
        bottom: expect_float!(config_options["map"]["extent"][2], "extent"),
        top: expect_float!(config_options["map"]["extent"][3], "extent"),
    };
    let path_output = config_options["map"]
        .get("output")
        .unwrap()
        .as_str()
        .unwrap();
    let converter = Converter::new(width, height, &map_extent);

    // Create a new svg document:
    let mut document = Document::new()
        .set("x", "0")
        .set("y", "0")
        .set("width", format!("{}", converter.viewport_width))
        .set("height", format!("{}", converter.viewport_height));

    let config_options_table = config_options.as_table().unwrap();

    // Add an underlying rect if the "background" key is provided:
    if let Some(&toml::Value::String(ref bg_color)) =
        config_options_table["map"].get("background") {
        let bg_rect = Rect::new()
            .set("fill", bg_color.as_str())
            .set("width", "100%")
            .set("height", "100%");
        document = document.add(bg_rect);
    };

    let layers = config_options_table["map"]["layers"].as_array().unwrap();

    for input_layer in layers {
        let path = input_layer.as_str().unwrap();
        let name = path.split(".geojson").collect::<Vec<&str>>()[0];
        let layer_properties = if config_options_table.contains_key(name) {
            // match config_options_table[name].get("representation") {
            //     Some(&toml::Value::String(ref name)) => {
            //         LayerProperties::from_config(&config_options[name].as_table().unwrap())
            //     }
            //     Some(&_) => panic!(""),
            //     None => LayerProperties::from_config(&config_options[name].as_table().unwrap()),
            // }
            LayerProperties::from_config(&config_options[name].as_table().unwrap())
        } else {
            LayerProperties::default()
        };
        let mut file = File::open(path).unwrap_or_else(|err| {
            println!("Unable to open layer at path: {}\nError: {}", path, err);
            std::process::exit(1)
        });
        let mut raw_json = String::new();
        file.read_to_string(&mut raw_json).unwrap();
        let mut decoded_geojson = raw_json.parse::<GeoJson>().unwrap();
        if let Some(&toml::Value::String(ref proj_name)) =
            config_options_table.get("map").unwrap().get("projection") {
            let input_proj = Proj::new("+proj=longlat +ellps=WGS84 +datum=WGS84 +no_defs").unwrap();
            let output_proj = Proj::new(proj_name).unwrap();
            decoded_geojson = reproj(&mut decoded_geojson, &input_proj, &output_proj);
        };
        let group = converter.convert(decoded_geojson, &layer_properties);
        document = document.add(group.set("id", name));
    }

    // Add the source section :
    if let Some(&toml::Value::Table(ref source_options)) = config_options_table.get("source") {
        if !source_options.contains_key("content") {
            println!("\"Source\" section need to have a content!");
            std::process::exit(1);
        }
        // Fetch the x, y and text-anchor values:
        let position: (i32, i32, &'static str) = match source_options.get("position") {
            Some(&toml::Value::Array(ref pos)) => {
                (pos[0].as_integer().unwrap() as i32, pos[1].as_integer().unwrap() as i32, "middle")
            }
            Some(&toml::Value::String(ref horiz_pos)) => {
                let v = if horiz_pos == "right" {
                    (converter.viewport_width - converter.viewport_width / 25,
                     converter.viewport_height - converter.viewport_height / 25,
                     "end")
                } else if horiz_pos == "center" {
                    (converter.viewport_width / 2,
                     converter.viewport_height - converter.viewport_height / 25,
                     "middle")
                } else {
                    (converter.viewport_width / 25,
                     converter.viewport_height - converter.viewport_height / 25,
                     "start")
                };
                (v.0 as i32, v.1 as i32, v.2)
            }
            Some(&_) | None => {
                ((converter.viewport_width - converter.viewport_width / 15) as i32,
                 (converter.viewport_height - converter.viewport_height / 15) as i32,
                 "end")
            }
        };
        let font_size = if let Some(&toml::Value::String(ref val)) =
            source_options.get("font-size") {
            val
        } else {
            "14"
        };
        let text = Text::new()
            .set("id", "source")
            .set("font-size", font_size)
            .set("x", position.0)
            .set("y", position.1)
            .set("text-anchor", position.2)
            .add(NodeText::new(source_options["content"].as_str().unwrap()));
        document = document.add(text);
    }

    // Add the title :
    if let Some(&toml::Value::Table(ref title_options)) = config_options_table.get("title") {
        if !title_options.contains_key("content") || !title_options["content"].is_str() {
            println!("\"Title\" section need to have a content!");
            std::process::exit(1);
        }
        let font_size = if let Some(&toml::Value::String(ref val)) =
            title_options.get("font-size") {
            val
        } else {
            "22"
        };
        let text = Text::new()
            .set("id", "title")
            .set("font-size", font_size)
            .set("text-anchor", "middle")
            .set("x", title_options["position"][0].as_integer().unwrap())
            .set("y", title_options["position"][1].as_integer().unwrap())
            .add(NodeText::new(title_options["content"].as_str().unwrap()));
        document = document.add(text);
    }
    svg::save(path_output, &document).unwrap();
}
