use nerva_research::scenario::Scenario;
use nerva_research::{
    Result, error,
    runner::ResearchRunner,
    scenarios::{
        m0::{M0Scenario, parameter_names},
        m0_selection::{M0SelectionScenario, selection_parameter_names},
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
                "interventions":M0SelectionScenario.interventions().iter().map(|i|i.name.clone()).collect::<Vec<_>>()}]})
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
