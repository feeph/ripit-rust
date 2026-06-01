/*
    YAML-related utilities
*/

#[allow(unused_imports)]
use log::{error, warn, info, debug};

use serde::Serialize;
use serde_yaml::{Mapping, Value};

/// Recursively sorts all YAML mapping keys in numerical order
fn sort_yaml_value(value: &mut Value) {
    match value {
        Value::Mapping(mapping) => {
            let mut entries: Vec<_> = mapping.iter_mut().map(|(k, v)| (k.clone(), v.clone())).collect();
            entries.sort_by(|a, b| {
                let a_num = match &a.0 {
                    Value::Number(n) => n.as_f64(),
                    Value::String(s) => s.parse::<f64>().ok(),
                    _ => None,
                };
                let b_num = match &b.0 {
                    Value::Number(n) => n.as_f64(),
                    Value::String(s) => s.parse::<f64>().ok(),
                    _ => None,
                };
                match (a_num, b_num) {
                    (Some(a_n), Some(b_n)) => a_n.partial_cmp(&b_n).unwrap_or(std::cmp::Ordering::Equal),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => {
                        let a_str = a.0.as_str().unwrap_or("");
                        let b_str = b.0.as_str().unwrap_or("");
                        a_str.cmp(b_str)
                    }
                }
            });

            // Reconstruct mapping with sorted entries and recursively sort nested values
            let mut new_mapping = Mapping::new();
            for (k, mut v) in entries {
                sort_yaml_value(&mut v);
                new_mapping.insert(k, v);
            }
            *mapping = new_mapping;
        }
        Value::Sequence(seq) => {
            for item in seq {
                sort_yaml_value(item);
            }
        }
        _ => {}
    }
}

/// serialize data as YAML-formatted string
pub fn generate_yaml<T: Serialize>(data: T, sort_keys: bool) -> String {
    let mut yaml_value = serde_yaml::to_value(&data).unwrap();
    if sort_keys {
        sort_yaml_value(&mut yaml_value);
    }
    serde_yaml::to_string(&yaml_value).unwrap()
}
