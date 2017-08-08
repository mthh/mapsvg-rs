# mapsvg-rs
## WIP / No real purpose

Draw a bunch of svg path from a set of GeoJSON Feature Collection and a configuration file!

For those who love TOML files and SVG ?!

### Example of TOML configuration file and output:
**config.toml**
```
[map]
width = 500
height = 800
extent = [-8191782.6791878305, -5973576.6304815235, -7371967.8868029471, -2485963.7082710671]
layers = ["Argentine.geojson", "lines.geojson", "points.geojson"]
output = "map.svg"

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
mapsvg -i config.toml
```
**Output:**  

<img src="https://raw.githubusercontent.com/mthh/mapsvg-rs/master/examples/map.svg" width="20%">
