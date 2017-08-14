use jenks;
use std::str::FromStr;

pub enum Classification {
    EqualInterval,
    HeadTail,
    Jenks,
    Quantiles,
}

impl FromStr for Classification {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Jenks" | "jenks" => Ok(Classification::Jenks),
            "Quantiles" | "quantiles" | "Quantile" | "quantile" => Ok(Classification::Quantiles),
            "equal interval" | "equal_interval" | "Equal Interval" | "EqualInverval" => {
                Ok(Classification::EqualInterval)
            }
            "HeadTail" => Ok(Classification::HeadTail),
            _ => Err("Invalid classification name"),
        }
    }
}

pub struct Classif {
    // type_classif: Classification,
    pub values: Vec<f64>,
    // nb_class: u32,
    pub bounds: Vec<f64>,
    pub min: f64,
    pub max: f64,
}

impl Classif {
    pub fn new(nb_class: u32, values: Vec<f64>, type_classif: Classification) -> Self {
        let mut v = values.clone();
        let breaks = match type_classif {
            Classification::Jenks => jenks::get_breaks(&mut v, nb_class),
            Classification::Quantiles => get_quantiles(&mut v, nb_class),
            Classification::EqualInterval => get_equal_interval(&mut v, nb_class),
            Classification::HeadTail => get_head_tail_breaks(&mut v),
        };
        let min = v.first().unwrap();
        let max = v.last().unwrap();
        Classif {
            // type_classif: type_classif,
            values: values,
            // nb_class: nb_class,
            bounds: breaks,
            min: *min,
            max: *max,
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


fn get_equal_interval(values: &mut [f64], nb_class: u32) -> Vec<f64> {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // let nb_elem = values.len();
    let min = values.first().unwrap();
    let max = values.last().unwrap();
    let interval = (max - min) / nb_class as f64;
    let mut breaks = Vec::new();
    let mut val = *min;
    for _ in 0..(nb_class + 1) {
        breaks.push(val);
        val += interval;
    }
    {
        let last = breaks.last_mut().unwrap();
        *last = *max;
    }
    breaks
}

fn get_quantiles(values: &mut [f64], nb_class: u32) -> Vec<f64> {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let nb_elem: usize = values.len();
    let mut breaks = Vec::new();
    breaks.push(values[0]);
    let step = nb_elem as f64 / nb_class as f64;
    for i in 1..nb_class {
        let qidx = (i as f64 * step + 0.49).floor() as usize;
        breaks.push(values[qidx - 1]);
    }
    breaks.push(values[nb_elem - 1]);
    breaks
}

fn get_mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}


fn get_head_tail_breaks(values: &mut [f64]) -> Vec<f64> {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut _mean = get_mean(&values);
    let mut breaks = Vec::new();
    let mut t;
    breaks.push(values[0]);
    loop {
        t = values
            .iter()
            .filter(|&v| *v > _mean)
            .cloned()
            .collect::<Vec<f64>>();
        _mean = get_mean(&t);
        breaks.push(_mean);
        if t.len() < 2 {
            break;
        }
    }
    breaks
}
