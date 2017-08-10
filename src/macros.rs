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
