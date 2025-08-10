use std::collections::HashMap;

pub type Counters = HashMap<String, f32>;

pub fn update_counters(counters: &mut Counters, key: &str, inc: f32) {
    if counters.contains_key(key) {
        *counters.get_mut(key).unwrap() += inc;
    } else {
        counters.insert(key.to_string(), inc);
    }
}

pub fn add_sum_counter_item(counters: &mut Counters, key: &str, subkeys: &[&str]) {
    let sum = subkeys.iter().map(|k| counters.get(*k).unwrap_or(&0.0)).sum::<f32>();
    counters.insert(key.to_string(), sum);
}

pub fn merge_counters(c1: &Counters, c2: &Counters) -> Counters {
    let mut result = c1.clone();
    for (k, v) in c2.iter() {
        if result.contains_key(k) {
            *result.get_mut(k).unwrap() += v;
        } else {
            result.insert(k.clone(), *v);
        }
    }
    result
}
