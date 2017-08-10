use jenks;

enum Classification {
    Jenks,
    EqualInterval,
    Quantiles,
}

pub struct Classif {
    type_classif: Classification,
    pub values: Vec<f64>,
    nb_class: u32,
    bounds: Vec<f64>,
}

impl Classif {
    pub fn new(nb_class: u32, values: Vec<f64>) -> Self {
        let mut v = values.clone();
        let breaks = jenks::get_breaks(&mut v, nb_class);
        Classif {
            type_classif: Classification::Jenks,
            values: values,
            nb_class: nb_class,
            bounds: breaks,
        }
    }

    pub fn get_class_index(&self, value: f64) -> Option<u32> {
        for i in 0..self.bounds.len() {
            if value <= self.bounds[i + 1usize] {
                return Some(i as u32);
            }
        }
        None
    }
}
