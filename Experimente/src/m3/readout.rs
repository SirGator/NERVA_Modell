pub use nerva_vm::readout::{Readout, chronological, core_features, score};
use serde_json::{Value, json};
pub fn paired(values: &[f64]) -> Value {
    let mut random = 0x287bd1e7463afc92;
    let mut boot = Vec::new();
    for _ in 0..10_000 {
        boot.push(
            (0..values.len())
                .map(|_| values[crate::random::next_random(&mut random) as usize % values.len()])
                .sum::<f64>()
                / values.len() as f64,
        );
    }
    boot.sort_by(f64::total_cmp);
    json!({"mean_difference":values.iter().sum::<f64>()/values.len() as f64,
        "seed_differences":values,"bootstrap_95_percent_interval":[boot[250],boot[9749]]})
}
