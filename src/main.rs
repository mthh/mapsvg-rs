extern crate svg;
extern crate geojson;
extern crate clap;
extern crate toml;

use std::collections::BTreeMap;
use clap::{Arg, App};
use geojson::{GeoJson, Value};
use std::fs::File;
use std::io::Read;
use svg::Document;
use svg::Node;
use svg::node::element::{Circle, Group, Path, Text};
use svg::node::Text as NodeText;
use svg::node::element::path::Data;


macro_rules! expect_float {
    ($value:expr, $name:expr) => (
        match $value.as_float() {
            Some(v) => v,
            None => panic!("Expected float value on value {}!", $name)
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
            _ => panic!("Error: expected a Feature collection of polygons!"),
        };

        let mut group = Group::new();
        for feature in features {
            let geom = feature.geometry.unwrap();
            match geom.value {
                Value::Point(point) => group.append(self.draw_point(&point, prop)),
                Value::MultiPoint(points) => {
                    for point in &points {
                        group.append(self.draw_point(point, prop));
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
                _ => panic!("Expected a Polygon!"),
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
    // let width: u32 = matches.value_of("width").unwrap().parse::<u32>().unwrap();
    // let height: u32 = matches.value_of("height").unwrap().parse::<u32>().unwrap();

    let mut file = File::open(file_path).unwrap();
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
    let mut document = Document::new()
        .set("x", "0")
        .set("y", "0")
        .set("width", format!("{}", converter.viewport_width))
        .set("height", format!("{}", converter.viewport_height));
    let layers = config_options["map"]["layers"].as_array().unwrap();
    for input_layer in layers {
        let path = input_layer.as_str().unwrap();
        let name = path.split(".geojson").collect::<Vec<&str>>()[0];
        let layer_properties = if (&config_options.as_table().unwrap()).contains_key(name) {
            LayerProperties::from_config(&config_options[name].as_table().unwrap())
        } else {
            LayerProperties::default()
        };
        let mut file = File::open(path).unwrap();
        let mut raw_json = String::new();
        file.read_to_string(&mut raw_json).unwrap();
        let decoded_geojson = raw_json.parse::<GeoJson>().unwrap();
        let group = converter.convert(decoded_geojson, &layer_properties);
        document = document.add(group);
    }
    if let toml::Value::Table(ref title_options) = config_options["title"] {
        let text = Text::new()
            .set("font-size", title_options["font-size"].as_str().unwrap())
            .set("text-anchor", "middle")
            .set("x", title_options["position"][0].as_integer().unwrap())
            .set("y", title_options["position"][1].as_integer().unwrap())
            .add(NodeText::new(title_options["content"].as_str().unwrap()));
        document = document.add(text);
    }
    svg::save(path_output, &document).unwrap();
}
