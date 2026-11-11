use nerva_research::scenario::Scenario;
use nerva_research::{
    Result, error,
    runner::ResearchRunner,
    scenarios::{
        continual::ContinualScenario,
        hippocampal::{
            HippocampalScenario, PARAMETERS as HIPPOCAMPAL_PARAMETERS,
            association::AssociationScenario,
            causal::CausalScenario,
            chain::ChainScenario,
            diagnosis::DiagnosisScenario,
            factorial::FactorialScenario,
            history::{HistoryScenario, PARAMETERS as HISTORY_PARAMETERS},
            multilevel::MultilevelScenario,
            precision::PrecisionScenario,
        },
        m0::{M0Scenario, parameter_names},
        m0_selection::{M0SelectionScenario, selection_parameter_names},
        sensorimotor::{PARAMETERS as SENSORIMOTOR_PARAMETERS, SensorimotorScenario},
    },
    spec::ExperimentSpec,
};
use serde_json::json;
use std::{
    env, fs,
    io::{self, Read},
    path::PathBuf,
};

fn main() {
    if let Err(e) = execute() {
        eprintln!("nerva-research: {e}");
        std::process::exit(1);
    }
}
fn execute() -> Result<()> {
    let mut args = env::args().skip(1);
    let mode = args
        .next()
        .ok_or_else(|| error("mode required: plan/run/report/list"))?;
    if mode == "list" {
        if args.next().is_some() {
            return Err(error("list accepts no arguments"));
        }
        println!(
            "{}",
            json!({"scenarios":[{"name":"m0_sequence","parameters":parameter_names(),
                "metrics":M0Scenario.metric_names(),
                "interventions":M0Scenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"m0_selection","parameters":selection_parameter_names(),"metrics":M0SelectionScenario.metric_names(),
                "interventions":M0SelectionScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"continual_sequences","parameters":["training_repetitions","symbol_interval_us","learning.a_plus","learning.a_minus"],
                "metrics":ContinualScenario.metric_names(),"interventions":ContinualScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"sensorimotor_size","parameters":SENSORIMOTOR_PARAMETERS,"metrics":SensorimotorScenario.metric_names(),
                "interventions":SensorimotorScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"hippocampal_causality","parameters":HIPPOCAMPAL_PARAMETERS,"metrics":CausalScenario.metric_names(),"interventions":CausalScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"hippocampal_precision","parameters":HIPPOCAMPAL_PARAMETERS,"metrics":PrecisionScenario.metric_names(),"interventions":PrecisionScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"hippocampal_direction_path","parameters":HIPPOCAMPAL_PARAMETERS,"metrics":DiagnosisScenario.metric_names(),"interventions":DiagnosisScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"hippocampal_direction_strength","parameters":HIPPOCAMPAL_PARAMETERS,"metrics":FactorialScenario.metric_names(),"interventions":FactorialScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"hippocampal_causal_chain","parameters":HIPPOCAMPAL_PARAMETERS,"metrics":ChainScenario.metric_names(),"interventions":ChainScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"hippocampal_multilevel_diagnosis","parameters":HIPPOCAMPAL_PARAMETERS,"metrics":MultilevelScenario.metric_names(),"interventions":MultilevelScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"hippocampal_learning_history","parameters":HISTORY_PARAMETERS,"metrics":HistoryScenario.metric_names(),"interventions":HistoryScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"hippocampal_association","parameters":HIPPOCAMPAL_PARAMETERS,"metrics":AssociationScenario.metric_names(),"interventions":AssociationScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()},
                {"name":"hippocampal_probe","parameters":HIPPOCAMPAL_PARAMETERS,"metrics":HippocampalScenario.metric_names(),
                "interventions":HippocampalScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()}]})
        );
        return Ok(());
    }
    if !["plan", "run", "report"].contains(&mode.as_str()) {
        return Err(error("unknown mode"));
    }
    let mut spec_path = None;
    let mut output = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--spec" if spec_path.is_none() => {
                spec_path = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| error("--spec requires JSON path"))?,
                ))
            }
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(
                    args.next().ok_or_else(|| error("--output requires path"))?,
                ))
            }
            _ => return Err(error(format!("unexpected argument: {arg}"))),
        }
    }
    let source = if let Some(path) = spec_path {
        fs::read_to_string(path).map_err(|e| error(e.to_string()))?
    } else {
        let mut text = String::new();
        io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| error(e.to_string()))?;
        text
    };
    let spec =
        ExperimentSpec::parse(serde_json::from_str(&source).map_err(|e| error(e.to_string()))?)?;
    // Registration belongs at the application boundary, never in the generic runner.
    match spec.scenario.as_str() {
        "m0_sequence" => dispatch(M0Scenario, &mode, &spec, output),
        "m0_selection" => dispatch(M0SelectionScenario, &mode, &spec, output),
        "continual_sequences" => dispatch(ContinualScenario, &mode, &spec, output),
        "sensorimotor_size" => dispatch(SensorimotorScenario, &mode, &spec, output),
        "hippocampal_probe" => dispatch(HippocampalScenario, &mode, &spec, output),
        "hippocampal_causality" => dispatch(CausalScenario, &mode, &spec, output),
        "hippocampal_multilevel_diagnosis" => dispatch(MultilevelScenario, &mode, &spec, output),
        "hippocampal_learning_history" => dispatch(HistoryScenario, &mode, &spec, output),
        "hippocampal_association" => dispatch(AssociationScenario, &mode, &spec, output),
        "hippocampal_causal_chain" => dispatch(ChainScenario, &mode, &spec, output),
        "hippocampal_precision" => dispatch(PrecisionScenario, &mode, &spec, output),
        "hippocampal_direction_path" => dispatch(DiagnosisScenario, &mode, &spec, output),
        "hippocampal_direction_strength" => dispatch(FactorialScenario, &mode, &spec, output),
        _ => Err(error(format!("unregistered scenario: {}", spec.scenario))),
    }
}
fn dispatch<S: Scenario>(
    scenario: S,
    mode: &str,
    spec: &ExperimentSpec,
    output: Option<PathBuf>,
) -> Result<()> {
    let runner = ResearchRunner { scenario };
    if mode == "plan" {
        let plan = runner.plan(spec)?;
        if let Some(output) = output {
            nerva_research::collector::ResultCollector::open(&output, &plan)?;
        }
        println!("{}", serde_json::to_string_pretty(&plan).unwrap());
    } else {
        let output = output.ok_or_else(|| error("--output required"))?;
        let report = runner.execute(spec, &output, mode == "report")?;
        println!(
            "SUMMARY {} trials in {}",
            report["counts"]["trials"],
            output.display()
        );
    }
    Ok(())
}
