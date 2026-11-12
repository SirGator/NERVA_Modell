//! Preregistered causal atlas: local information, support, sign, strength and maintenance.
use super::{Config, HippocampalScenario, association::AssociationScenario};
use crate::{
    Result, error,
    scenario::{Intervention, Scenario, TrialMetrics},
};
use nerva_vm::model::LearningRule;
use serde_json::{Value, json};
pub(super) mod atlas;
pub(super) mod images;
pub(super) mod interaction;
pub struct Mld02Scenario;
pub const CASES: [&str; 8] = ["F", "B", "G", "C", "P", "L_plus", "L_minus", "D"];
pub(super) const METRICS: [&str; 8] = [
    "reference_verified",
    "recurrent_reference_verified",
    "novel_core_memory_candidate",
    "native_training_passes",
    "matched_observations",
    "interaction_a",
    "interaction_b",
    "offline_verified",
];
pub(super) fn rule(case: &str) -> LearningRule {
    match case {
        "G" => LearningRule::ThreeFactorGain64,
        "C" => LearningRule::ThreeFactorConstantPositive,
        "P" => LearningRule::PairStdp,
        "L_plus" => LearningRule::ThreeFactorPotentiatingOnly,
        "L_minus" => LearningRule::ThreeFactorDepressingOnly,
        _ => LearningRule::ThreeFactorAccumulated,
    }
}
impl Scenario for Mld02Scenario {
    type Config = Config;
    fn name(&self) -> &'static str {
        "hippocampal_mld02"
    }
    fn default_config(&self) -> Config {
        AssociationScenario.default_config()
    }
    fn set_parameter(&self, c: &mut Config, n: &str, v: &Value) -> Result<()> {
        HippocampalScenario.set_parameter(c, n, v)
    }
    fn validate(&self, c: &Config) -> Result<()> {
        AssociationScenario.validate(c)
    }
    fn interventions(&self) -> Vec<Intervention> {
        ["atlas", "interaction"]
            .into_iter()
            .map(|name| Intervention { name: name.into() })
            .collect()
    }
    fn metric_names(&self) -> Vec<&'static str> {
        METRICS.to_vec()
    }
    fn snapshot(&self, c: &Config, seed: u64, i: &Intervention) -> Value {
        json!({"protocol":"NERVA-MLD-02/1","seed":seed,"section":i.name,"processing_neurons":c.processing_neurons,"training_repetitions":c.training_repetitions,"rest_us":c.rest_us,
        "cases":CASES,"gain":64.,"contribution_filters":"individual requested deltas before carry; signed eligibility retained","probes":images::PROBES,
        "novel_cues":"one original cue pixel at a time; excluded from reference construction","distractor":"disjoint equal-density shared6/unique2 image pair",
        "reference":{"method":"sensory_and_recurrent","dose":1.,"recurrent_method":"recurrent_two_hop","recurrent_dose":1.,"max_single_edge_ablations":8},
        "norm_tolerance":0.01,"cosine_tolerance":0.999,"warmup_frames":20,"warmup":"distractor fullA/fullB/black/black; development and homeostasis active, then topology frozen",
        "training_passes_per_execution":if i.name=="atlas"{80}else{18},"training_budget_is_upper_bound":true,"decision_epsilon":1e-9})
    }
    fn run(&self, c: &Config, seed: u64, i: &Intervention) -> Result<TrialMetrics> {
        self.validate(c)?;
        match i.name.as_str() {
            "atlas" => atlas::run(c, seed),
            "interaction" => interaction::run(c, seed),
            _ => Err(error("unknown MLD02 section")),
        }
    }
    fn evaluate(&self, r: &[Value], replay: bool) -> Result<Value> {
        Ok(
            json!({"status":"mld02_preregistered_diagnosis","passed":null,"records":r.len(),"replay_checked":replay,"memory_claim_requires_novel_and_distractor_controls":true}),
        )
    }
}
