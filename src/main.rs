extern crate svg;
extern crate geojson;
extern crate clap;
extern crate toml;
extern crate proj;
extern crate colorbrewer;
extern crate classif;

use std::collections::BTreeMap;
use clap::{Arg, App};
use classif::{BoundsInfo, Classification};
use geojson::{GeoJson, Value};
use proj::Proj;
use std::env::set_current_dir;
use std::fs::File;
use std::io::Read;
use std::path::Path as StdPath;
use svg::Document;
use svg::Node;
use svg::node::element::{Circle, Group, Path, Rectangle as Rect, Text};
use svg::node::Text as NodeText;
use svg::node::element::path::Data;

#[macro_use]
mod macros;
mod layer;
mod graticule;
mod config_params;

use config_params::MapExtent;
use layer::{reproj, reproj_graticule, get_nb_class, get_extent, get_values};
use graticule::prepare_geojson_graticule;

struct ChoroplethLayerProperties {
    type_classification: String,
    field_name: String,
    palette_name: String,
    fill_opacity: String,
    stroke: String,
    stroke_opacity: String,
    stroke_width: String,
    radius: String,
}

impl ChoroplethLayerProperties {
    fn from_config(c: &BTreeMap<String, toml::value::Value>) -> Self {
        ChoroplethLayerProperties {
            type_classification: string_or_default!(c.get("classification"), "Quantiles"),
            field_name: string_or_default!(c.get("field"), "aaa"),
            palette_name: string_or_default!(c.get("palette"), "Greens"),
            fill_opacity: string_or_default!(c.get("fill-opacity"), "0.8"),
            stroke: string_or_default!(c.get("stroke"), "black"),
            stroke_opacity: string_or_default!(c.get("stroke-opacity"), "1"),
            stroke_width: string_or_default!(c.get("stroke-width"), "0.7"),
            radius: string_or_default!(c.get("radius"), "4"),
        }
    }
}


struct SingleColorLayerProperties {
    fill: String,
    fill_opacity: String,
    stroke: String,
    stroke_opacity: String,
    stroke_width: String,
    radius: String,
}

impl SingleColorLayerProperties {
    fn from_config(c: &BTreeMap<String, toml::value::Value>) -> Self {
        SingleColorLayerProperties {
            fill: string_or_default!(c.get("fill"), "blue"),
            fill_opacity: string_or_default!(c.get("fill-opacity"), "0.8"),
            stroke: string_or_default!(c.get("stroke"), "black"),
            stroke_opacity: string_or_default!(c.get("stroke-opacity"), "1"),
            stroke_width: string_or_default!(c.get("stroke-width"), "0.7"),
            radius: string_or_default!(c.get("radius"), "4"),
        }
    }
    fn default() -> Self {
        SingleColorLayerProperties {
            fill: String::from("blue"),
            fill_opacity: String::from("0.8"),
            stroke: String::from("black"),
            stroke_opacity: String::from("1"),
            stroke_width: String::from("0.7"),
            radius: String::from("4"),
        }
    }
}

struct Converter<'a> {
    viewport_width: u32,
    viewport_height: u32,
    map_extent: &'a MapExtent,
    resolution: f64,
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

    pub fn draw_point(&self, point: &[f64]) -> Circle {
        Circle::new()
            .set("cx", (point[0] - self.map_extent.left) / self.resolution)
            .set("cy", (self.map_extent.top - point[1]) / self.resolution)
    }

    pub fn draw_path_ring(&self, positions: &[Vec<Vec<f64>>], d: Option<Data>) -> Data {
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

// enum Representation {
//     Unicolor,
//     Random,
//     Choropleth,
// }

struct Renderer {}

impl Renderer {
    fn render_graticule(converter: &Converter, reprojected_graticule: GeoJson) -> Group {
        let features = match reprojected_graticule {
            GeoJson::FeatureCollection(collection) => collection.features,
            _ => {
                println!("Expected a GeoJSON feature collection!");
                std::process::exit(1);
            }
        };

        let mut group = Group::new();
        for feature in features {
            let geom = feature.geometry.unwrap();
            match geom.value {
                Value::MultiLineString(lines) => {
                    let mut data = Data::new();
                    for positions in &lines {
                        data = converter.draw_path_ring(&[positions.to_vec()], Some(data));
                    }
                    group.append(Path::new()
                                     .set("fill", "none")
                                     .set("stroke", "grey")
                                     .set("stroke-dasharray", "5")
                                     .set("d", data));
                }
                _ => {}
            }
        }
        group
    }

    fn render_unicolor(converter: &Converter,
                       decoded_geojson: GeoJson,
                       prop: &SingleColorLayerProperties)
                       -> Group {
        let features = match decoded_geojson {
            GeoJson::FeatureCollection(collection) => collection.features,
            _ => {
                println!("Expected a GeoJSON feature collection!");
                std::process::exit(1)
            }
        };

        let mut group = Group::new();
        for feature in features {
            let geom = feature.geometry.unwrap();
            match geom.value {
                Value::Point(point) => {
                    let circle = converter.draw_point(&point);
                    group.append(circle
                                     .set("fill", prop.fill.clone())
                                     .set("r", prop.radius.clone()))
                }
                Value::MultiPoint(points) => {
                    for point in &points {
                        let circle = converter.draw_point(&point);
                        group.append(circle
                                         .set("fill", prop.fill.clone())
                                         .set("r", prop.radius.clone()))
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
                                          converter.draw_path_ring(&[positions], Some(data))));
                }
                Value::MultiLineString(lines) => {
                    let mut data = Data::new();
                    for positions in &lines {
                        data = converter.draw_path_ring(&[positions.to_vec()], Some(data));
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
                                     .set("d", converter.draw_path_ring(&positions, None)));
                }
                Value::MultiPolygon(polys) => {
                    let mut data = Data::new();
                    for positions in &polys {
                        data = converter.draw_path_ring(positions, Some(data));
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
    fn render_choropleth(converter: &Converter,
                         decoded_geojson: GeoJson,
                         prop: &ChoroplethLayerProperties)
                         -> Group {
        let features = match decoded_geojson {
            GeoJson::FeatureCollection(collection) => collection.features,
            _ => panic!("Error: expected a Feature collection!"),
        };
        let values = get_values(&features, &prop.field_name);
        let nb_class = get_nb_class(values.len());
        let type_classif: Classification = prop.type_classification
            .parse::<Classification>()
            .unwrap_or_else(|_| {
                                println!("Invalid classification name!");
                                std::process::exit(1)
                            });
        let palette_name: colorbrewer::Palette = prop.palette_name
            .parse()
            .unwrap_or_else(|_| {
                                println!("Unexisting palette name!");
                                std::process::exit(1)
                            });
        let classifier = BoundsInfo::new(nb_class, &values, type_classif).unwrap();
        let palette = colorbrewer::get_color_ramp(palette_name, nb_class).unwrap();
        let mut group = Group::new();
        // for (ix, feature) in features.iter().enumerate() {
        features
            .iter()
            .enumerate()
            .map(|(ix, ref feature)| {
                if let Some(ref geom) = feature.geometry {
                    let value = values[ix];
                    let color = palette[classifier.get_class_index(value).unwrap() as usize];
                    match geom.value {
                        Value::Point(ref point) => {
                            let circle = converter.draw_point(&point);
                            group.append(circle.set("fill", color).set("r", prop.radius.clone()))
                        }
                        Value::MultiPoint(ref points) => {
                            for point in points {
                                let circle = converter.draw_point(&point);
                                group
                                    .append(circle.set("fill", color).set("r", prop.radius.clone()))
                            }
                        }
                        Value::LineString(ref positions) => {
                            let data = Data::new();
                            group.append(Path::new()
                                             .set("fill", "none")
                                             .set("stroke", color)
                                             .set("stroke-width", prop.stroke_width.clone())
                                             .set("stroke-opacity",
                                                  prop.stroke_opacity.clone())
                                             .set("d",
                                                  converter
                                                      .draw_path_ring(&[positions.to_vec()],
                                                                      Some(data))));
                        }
                        Value::MultiLineString(ref lines) => {
                            let mut data = Data::new();
                            for positions in lines {
                                data = converter.draw_path_ring(&[positions.to_vec()], Some(data));
                            }
                            group.append(Path::new()
                                             .set("fill", "none")
                                             .set("stroke", color)
                                             .set("stroke-width", prop.stroke_width.clone())
                                             .set("stroke-opacity",
                                                  prop.stroke_opacity.clone())
                                             .set("d", data));
                        }
                        Value::Polygon(ref positions) => {
                            group.append(Path::new()
                                             .set("fill", color)
                                             .set("fill-opacity", prop.fill_opacity.clone())
                                             .set("stroke", prop.stroke.clone())
                                             .set("stroke-width", prop.stroke_width.clone())
                                             .set("stroke-opacity",
                                                  prop.stroke_opacity.clone())
                                             .set("d",
                                                  converter.draw_path_ring(&positions, None)));
                        }
                        Value::MultiPolygon(ref polys) => {
                            let mut data = Data::new();
                            for positions in polys {
                                data = converter.draw_path_ring(positions, Some(data));
                            }
                            data = data.close();
                            group.append(Path::new()
                                             .set("fill", color)
                                             .set("fill-opacity", prop.fill_opacity.clone())
                                             .set("stroke", prop.stroke.clone())
                                             .set("stroke-width", prop.stroke_width.clone())
                                             .set("stroke-opacity",
                                                  prop.stroke_opacity.clone())
                                             .set("d", data));
                        }
                        _ => panic!("I don't handle GeometryCollection yet!!"),
                    }
                }
                // }
            })
            .collect::<Vec<_>>();
        group
    }
}

fn main() {
    let matches = App::new("geojson2svg")
        .version("0.1.0")
        .about("Convert geojson to svg")
        .arg(Arg::with_name("input")
                 .index(1)
                 .required(true)
                 .value_name("CONFIG_FILE")
                 .help("Input configuration file to use (.toml)."))
        .get_matches();
    let file_path = StdPath::new(matches.value_of("input").unwrap());
    if !file_path.exists() || !file_path.is_file() {
        println!("Invalid file path: \"{}\" doesn't exists!",
                 file_path.to_str().unwrap());
        std::process::exit(1)
    }
    match file_path.parent() {
        Some(ref val) => {
            set_current_dir(StdPath::new(val))
                .unwrap_or_else(|_| println!("Unable to use the file path provided!"));
        }
        None => {}
    };
    let mut file = File::open(file_path.file_name().unwrap()).unwrap_or_else(|err| {
        println!("Unable to open configuration file: \"{:?}\"\nError: {}",
                 file_path.to_str().unwrap(),
                 err);
        std::process::exit(1)
    });
    let mut a = String::new();
    file.read_to_string(&mut a).unwrap();
    let config_options = a.parse::<toml::Value>().unwrap();

    let path_output = config_options["map"]
        .get("output")
        .unwrap()
        .as_str()
        .unwrap();

    let config_options_table = config_options.as_table().unwrap();

    // Does the layers need reprojection:
    let projs = match config_options_table.get("map").unwrap().get("projection") {
        Some(&toml::Value::String(ref proj_name)) => {
            let input_proj = Proj::new("+proj=longlat +ellps=WGS84 +datum=WGS84 +no_defs").unwrap();
            let output_proj = Proj::new(proj_name).unwrap();
            Some((input_proj, output_proj))
        }
        Some(_) | None => None,
    };

    // Fetch the list of layers to be rendered:
    let layers = config_options_table["map"]["layers"].as_array().unwrap();
    let geojson_layers = layers
        .iter()
        .map(|input_layer| {
            let path = input_layer.as_str().unwrap();
            let name = path.split(".geojson").collect::<Vec<&str>>()[0];
            let mut file = File::open(path).unwrap_or_else(|err| {
                println!("Unable to open layer at path: \"{}\"\nError: {}", path, err);
                std::process::exit(1)
            });
            let mut raw_json = String::new();
            file.read_to_string(&mut raw_json).unwrap();
            let mut decoded_geojson = raw_json.parse::<GeoJson>().unwrap();
            if let Some((ref input_proj, ref output_proj)) = projs {
                decoded_geojson = reproj(&mut decoded_geojson, &input_proj, &output_proj);
            };
            (name, decoded_geojson)
        })
        .collect::<Vec<(&str, geojson::GeoJson)>>();

    let map_extent = if let toml::Value::String(ref layer_name) = config_options["map"]["extent"] {
        let mut extent: MapExtent = Default::default();
        geojson_layers
            .iter()
            .map(|a| if &a.0 == layer_name {
                     extent = get_extent(&a.1);
                 })
            .collect::<Vec<_>>();
        extent
    } else {
        MapExtent {
            left: expect_float!(config_options["map"]["extent"][0], "extent"),
            right: expect_float!(config_options["map"]["extent"][1], "extent"),
            bottom: expect_float!(config_options["map"]["extent"][2], "extent"),
            top: expect_float!(config_options["map"]["extent"][3], "extent"),
        }
    };

    let width: u32 = config_options["map"]["width"].as_integer().unwrap() as u32;
    let height: u32 = config_options["map"]["height"].as_integer().unwrap() as u32;
    let converter = Converter::new(width, height, &map_extent);

    // Create a new svg document:
    let mut document = Document::new()
        .set("x", "0")
        .set("y", "0")
        .set("width", format!("{}", converter.viewport_width))
        .set("height", format!("{}", converter.viewport_height));

    // Add an underlying rect if the "background" key is provided:
    if let Some(&toml::Value::String(ref bg_color)) =
        config_options_table["map"].get("background") {
        let bg_rect = Rect::new()
            .set("fill", bg_color.as_str())
            .set("width", "100%")
            .set("height", "100%");
        document = document.add(bg_rect);
    };
    // for input_layer in layers {
    //     let path = input_layer.as_str().unwrap();
    //     let name = path.split(".geojson").collect::<Vec<&str>>()[0];
    //     let mut file = File::open(path).unwrap_or_else(|err| {
    //         println!("Unable to open layer at path: \"{}\"\nError: {}", path, err);
    //         std::process::exit(1)
    //     });
    //     let mut raw_json = String::new();
    //     file.read_to_string(&mut raw_json).unwrap();
    //     let mut decoded_geojson = raw_json.parse::<GeoJson>().unwrap();
    //     if let Some((ref input_proj, ref output_proj)) = projs {
    //         decoded_geojson = reproj(&mut decoded_geojson, &input_proj, &output_proj);
    //     };
    // let layer_properties = if config_options_table.contains_key(name) {
    //     SingleColorLayerProperties::from_config(&config_options[name].as_table().unwrap())
    // } else {
    //     SingleColorLayerProperties::default()
    // };

    // Render each layer:
    for (name, decoded_geojson) in geojson_layers {
        let group = if !config_options_table.contains_key(name) {
            let layer_properties = SingleColorLayerProperties::default();
            Renderer::render_unicolor(&converter, decoded_geojson, &layer_properties)

        } else if !config_options_table[name]
                       .as_table()
                       .unwrap()
                       .contains_key("representation") {
            let layer_properties =
                SingleColorLayerProperties::from_config(&config_options[name].as_table().unwrap());
            Renderer::render_unicolor(&converter, decoded_geojson, &layer_properties)
        } else {
            match config_options_table[name].get("representation") {
                Some(&toml::Value::String(ref type_name)) => {
                    if type_name == "choropleth" {
                        let layer_properties =
                            ChoroplethLayerProperties::from_config(config_options_table[name]
                                                                       [type_name]
                                                                           .as_table()
                                                                           .unwrap());
                        Renderer::render_choropleth(&converter, decoded_geojson, &layer_properties)
                    } else {
                        panic!("Invalid representation name");
                    }
                }
                Some(&_) => panic!(""),
                None => panic!(""),
            }
        };
        // let group = Renderer::render_unicolor(&converter, decoded_geojson, &layer_properties);
        document = document.add(group.set("id", name));
    }

    // Add a graticule if requested:
    if let Some(&toml::Value::Table(ref graticule_option)) = config_options.get("graticule") {
        let mut graticule = prepare_geojson_graticule();
        if let Some((ref input_proj, ref output_proj)) = projs {
            graticule = reproj_graticule(&mut graticule, &input_proj, &output_proj);
        }
        let group = Renderer::render_graticule(&converter, graticule);
        document = document.add(group.set("id", "graticule"));
    }

    // Add the source section:
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
