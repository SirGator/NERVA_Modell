//! Optional, read-only observations for the local dashboard. No neural state is changed.
use nerva_qemu::capture::Frame;
use nerva_vm::{
    Result,
    model::{Model, Step},
};
use serde_json::{Value, json};
use std::{
    collections::{HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Control {
    pub paused: bool,
    pub stop: bool,
    pub freeze: bool,
}
pub struct Telemetry {
    directory: PathBuf,
    state: Value,
    history: VecDeque<Value>,
    raster: VecDeque<Value>,
}

fn atomic_json(path: &Path, value: &Value) -> Result<()> {
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

impl Telemetry {
    pub fn new(directory: &Path) -> Result<Self> {
        let mut value = Self {
            directory: directory.into(),
            state: json!({"status":"starting"}),
            history: VecDeque::new(),
            raster: VecDeque::new(),
        };
        value.status("starting")?;
        Ok(value)
    }
    pub fn control(&self) -> Result<Control> {
        let path = self.directory.join("dashboard-control.json");
        let v: Value = match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
            Err(e) => return Err(e.into()),
        };
        Ok(Control {
            paused: v["paused"].as_bool().unwrap_or(false),
            stop: v["stop"].as_bool().unwrap_or(false),
            freeze: v["freeze"].as_bool().unwrap_or(false),
        })
    }
    pub fn settings(&mut self, model: &Model) {
        self.state["learning"] = json!(model.plasticity().learning);
        self.state["development"] = json!(model.plasticity().development);
        self.state["homeostasis"] = json!(model.plasticity().homeostasis);
    }
    pub fn status(&mut self, status: &str) -> Result<()> {
        self.state["status"] = json!(status);
        self.state["updated_at"] =
            json!(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64());
        atomic_json(&self.directory.join("dashboard.json"), &self.state)
    }
    pub fn publish(
        &mut self,
        model: &Model,
        step: usize,
        requested: usize,
        result: &Step,
        frame: &Frame,
    ) -> Result<()> {
        self.history.push_back(json!({"step":step+1,"time_s":result.end_us as f64 / 1e6,"core":result.core_events.len(),"sensory":result.sensory_spikes,"motor":result.motor_spikes,"weights":result.weight_changes,"formed":result.formed.len(),"pruned":result.pruned.len(),"synapses":result.core_synapses}));
        while self.history.len() > 300 {
            self.history.pop_front();
        }
        self.raster.extend(
            result
                .core_events
                .iter()
                .map(|(id, time)| json!({"neuron":id,"time_s":*time as f64 / 1e6})),
        );
        while self.raster.len() > 4000 {
            self.raster.pop_front();
        }
        let previous = self.state.get("totals").cloned().unwrap_or(json!({}));
        let total =
            |name: &str, increment: usize| previous[name].as_u64().unwrap_or(0) + increment as u64;
        self.state = json!({
            "status":"running", "step":step+1,"requested_steps":requested,"simulation_us":result.end_us,
            "seed":model.config().seed,"processing":model.processing_ids().len(),"keyboard":model.config().keyboard.name(),
            "learning":model.plasticity().learning,"development":model.plasticity().development,"homeostasis":model.plasticity().homeostasis,
            "core_synapses":result.core_synapses,"neuron_count":model.network().neuron_count(),"synapse_count":model.network().synapses().len(),
            "totals":{"core":total("core",result.core_events.len()),"sensory":total("sensory",result.sensory_spikes),"motor":total("motor",result.motor_spikes),"weights":total("weights",result.weight_changes),"formed":total("formed",result.formed.len()),"pruned":total("pruned",result.pruned.len()),"copies":total("copies",result.action_copy_arrivals),"feedback":total("feedback",result.feedback_arrivals),"homeostasis":total("homeostasis",result.homeostasis_changes)},
            "keys":result.keys.iter().map(|k|k.label()).collect::<Vec<_>>(),
            "motors":model.config().keyboard.keys().iter().map(|k|json!({"label":k.label(),"qcode":k.qcode(),"spikes":result.motor_counts[*k as usize]})).collect::<Vec<_>>(),
            "history":self.history,"raster":self.raster,"core_ids":model.processing_ids().iter().map(|id|id.0).collect::<Vec<_>>()
        });
        let temp = self.directory.join("dashboard-frame.ppm.tmp");
        let mut pixels = format!("P6\n{} {}\n255\n", frame.width(), frame.height()).into_bytes();
        pixels.extend_from_slice(frame.rgb());
        fs::write(&temp, pixels)?;
        fs::rename(temp, self.directory.join("dashboard-frame.ppm"))?;
        if step.is_multiple_of(10) || step + 1 == requested {
            self.network(model, step + 1)?;
        }
        self.status("running")
    }
    fn network(&self, model: &Model, step: usize) -> Result<()> {
        let core: HashSet<_> = model.processing_ids().iter().map(|id| id.0).collect();
        let sensory_count = model.sensor_config().channels() as u64;
        let copies_start = sensory_count
            + model.processing_ids().len() as u64
            + model.config().keyboard.keys().len() as u64;
        let neurons: Vec<_> = model.network().neurons().map(|n| {
            let p=n.position();
            json!({"id":n.id().0,"x":p.x,"y":p.y,"z":p.z,"role":if core.contains(&n.id().0) {"Core"} else if n.id().0 < sensory_count {"Sensorik"} else if n.id().0 >= copies_start {"Aktionskopie"} else {"Motorik"},"polarity":format!("{:?}",n.polarity()),"potential":n.membrane_potential(),"threshold":n.threshold(),"spikes":n.spike_count(),"intrinsic_current":n.intrinsic_current(),"structural_drive":n.structural_drive()})
        }).collect();
        let edges: Vec<_> = model.network().synapses().map(|s|json!({"id":s.id().0,"pre":s.pre().0,"post":s.post().0,"weight":s.weight().get(),"delay_us":s.delay_us(),"utility":s.utility(),"transmissions":s.transmission_count()})).collect();
        atomic_json(
            &self.directory.join("network.json"),
            &json!({"step":step,"neurons":neurons,"edges":edges}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nerva_qemu::vision_encoder::Encoding;
    #[test]
    fn observing_does_not_change_neural_state() {
        let directory =
            std::env::temp_dir().join(format!("nerva-telemetry-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let mut telemetry = Telemetry::new(&directory).unwrap();
        let mut observed = Model::new(7, Encoding::Brightness, true).unwrap();
        let mut control = Model::new(7, Encoding::Brightness, true).unwrap();
        let frame = Frame::new(64, 64, vec![255; 64 * 64 * 3]).unwrap();
        for i in 0..3 {
            let result = observed.step(&frame).unwrap();
            let expected = control.step(&frame).unwrap();
            telemetry.publish(&observed, i, 3, &result, &frame).unwrap();
            assert_eq!(result.core_events, expected.core_events);
            assert_eq!(result.weight_changes, expected.weight_changes);
            assert!(observed.same_replay_state(&control));
        }
        let state: Value =
            serde_json::from_slice(&fs::read(directory.join("dashboard.json")).unwrap()).unwrap();
        assert_eq!(state["step"], 3);
        assert_eq!(state["history"].as_array().unwrap().len(), 3);
        fs::remove_dir_all(directory).unwrap();
    }
}
