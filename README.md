# mapsvg-rs
## WIP / No real purpose

Toy project about learning rust and also drawing a bunch of svg path from a set of GeoJSON Feature Collection and a configuration file!
For those who love TOML files and SVG ?!

A few dependent crates/repositories where created for this purpose:
- colorbrewer: [GitHub](https://github.com/mthh/colorbrewer-rs) / [crates.io](https://crates.io/crates/colorbrewer)
- classif: [GitHub](https://github.com/mthh/classif) / [crates.io](https://crates.io/crates/classif)
- nightcoords: [GitHub](https://github.com/mthh/nightcoords)

### Features:

- [x] Draw path for points, lines and polygons
- [x] User defined projection
- [x] Single color or "choropleth" coloration
- [x] Night shade (but why?)
- [x] Graticule
- [ ] Osm tiles background
- [ ] Other "mapping" methods (proportional symbols, ... ?)
- [ ] Cool svg filters ?


### Example of TOML configuration file and output:
**config.toml**
```toml
[map]
width = 500
height = 800
projection = "+init=epsg:3857" # This is the projection to use to draw the map
extent = [-8191782.6791878305, -5973576.6304815235, -7371967.8868029471, -2485963.7082710671]
layers = ["Argentine.geojson", "lines.geojson", "points.geojson"]
output = "map.svg"
background = "rgba(45, 45, 244, 0.5)"

[Argentine]
fill = "red"
fill-opacity = "0.1"
stroke = "black"
stroke-width = "1.2"
stroke-opacity = "0.5"

[lines]
stroke = "green"
stroke-width = "2.6"

# Look! It use default values if I don't define any !
# [points]
# radius = "12"
# fill = "rgb(238, 79, 21)"

# OMG! I can set a title!
[title]
content = "OMG! Title!"
font-size = "38"
position = [400, 50]
```

**SVG generation:**
```shell
mapsvg config.toml
```
**Output:**  
See SVG files in the [examples](https://github.com/mthh/mapsvg-rs/tree/master/examples) folder.
