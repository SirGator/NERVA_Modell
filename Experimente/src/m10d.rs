//! Frozen interventions on recurrent excitation and inhibition in existing M9 checkpoints.
use crate::{Result, m9, m10};
use serde_json::{Value, json};
use std::{path::Path, sync::atomic::AtomicBool};

pub fn without_recurrence(result: &Value) -> Value {
    let mut copy = result.clone();
    for arm in ["before", "aged_frozen", "paired", "scrambled"] {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            copy["arms"][arm][mapping]
                .as_object_mut()
                .unwrap()
                .remove("recurrence_probes");
        }
    }
    copy
}

pub fn run(reference: &Value, output: &Path, flag: &AtomicBool) -> Result<Value> {
    if reference["protocol"] != "NERVA-M9-v2" || reference["case_id"] != "weight4_tau100" {
        return Err("M10-D requires an existing M9-v2 combination result".into());
    }
    let config = m9::parse(reference["requested_config"].clone())?;
    let result = m9::run_with_diagnostics(&config, output, false, flag, false, true)?;
    if !m10::replay_equal(&without_recurrence(&result), reference) {
        return Err("M10-D parent replay differs from original M9".into());
    }
    for arm in ["before", "aged_frozen", "paired", "scrambled"] {
        for mapping in ["normal_mapping", "reversed_mapping"] {
            let model = &result["arms"][arm][mapping];
            if !m10::replay_equal(
                &model["recurrence_probes"]["count_readout"],
                &model["weight_reset_probes"]["count_readout"],
            ) {
                return Err("M10-D decoder differs".into());
            }
            for stage in ["checkpoint_probe", "rest_checkpoint_probe"] {
                let mut sham = model["recurrence_probes"]["stages"][stage]["sham"].clone();
                for trial in sham.as_array_mut().unwrap() {
                    let object = trial.as_object_mut().unwrap();
                    object.remove("trace");
                    object.remove("recurrence_gate");
                }
                if !m10::replay_equal(
                    &sham,
                    &model["weight_reset_probes"]["stages"][stage]["sham"],
                ) {
                    return Err("M10-D sham differs from original frozen probe".into());
                }
            }
        }
    }
    Ok(
        json!({"protocol":"NERVA-M10D-v1","seed":reference["seed"],"case_id":reference["case_id"],
        "reference_reproduced":true,"sham_reproduced":true,"m9_result":result,
        "limits":["Diagnostic interventions on known M9 seeds, not fresh confirmation", "Only Core-to-Core transmission blocked after baseline; all selected in-flight arrivals cancelled", "Frozen weights, same checkpoint and fixed decoder; no retraining", "Motor feedback and non-Core pathways remain active"]}),
    )
}
