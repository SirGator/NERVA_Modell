use super::KeyboardLayout;
use crate::Result;
use nerva_qemu::vision_encoder::Encoding;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelKind {
    Recurrent,
    Direct,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HomeostasisScope {
    #[default]
    All,
    Core,
    CoreSensor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelConfig {
    pub seed: u64,
    pub encoding: Encoding,
    pub keyboard: KeyboardLayout,
    pub kind: ModelKind,
    pub processing_neurons: usize,
    pub learning: bool,
    pub development: bool,
    pub homeostasis: bool,
    pub homeostasis_scope: HomeostasisScope,
    pub action_copy_weight: f32,
    pub core_membrane_tau_us: f32,
    pub core_threshold: f32,
    pub core_weight_scale: f32,
    pub core_fanout: usize,
    pub core_delay_us: Option<u64>,
    pub stdp_tau_us: f32,
    pub sensory_weight_scale: f32,
    pub feedback: bool,
    pub initial_structural_drive: f32,
    pub pruning_min_below_us: u64,
    pub motor_competition: bool,
    pub motor_inhibition_weight: f32,
    pub motor_weight_scale: f32,
    pub motor_refractory_us: u64,
    pub motor_timing: bool,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            encoding: Encoding::Brightness,
            keyboard: KeyboardLayout::Full,
            kind: ModelKind::Recurrent,
            processing_neurons: 256,
            learning: true,
            development: true,
            homeostasis: true,
            homeostasis_scope: HomeostasisScope::All,
            action_copy_weight: 0.9,
            core_membrane_tau_us: 20_000.0,
            core_threshold: 1.0,
            core_weight_scale: 1.0,
            core_fanout: 4,
            core_delay_us: None,
            stdp_tau_us: 20_000.0,
            sensory_weight_scale: 1.0,
            feedback: true,
            initial_structural_drive: 2.5,
            pruning_min_below_us: 300_000,
            motor_competition: false,
            motor_inhibition_weight: 2.0,
            motor_weight_scale: 1.0,
            motor_refractory_us: 10_000,
            motor_timing: true,
        }
    }
}

impl ModelConfig {
    pub fn validate(&self) -> Result<()> {
        if !self.core_threshold.is_finite()
            || !(0.2..=2.0).contains(&self.core_threshold)
            || !self.core_weight_scale.is_finite()
            || !(0.0..=6.0).contains(&self.core_weight_scale)
            || !(4..=16).contains(&self.core_fanout)
            || self
                .core_delay_us
                .is_some_and(|x| !(1..=20_000).contains(&x))
            || !self.stdp_tau_us.is_finite()
            || !(1_000.0..=100_000.0).contains(&self.stdp_tau_us)
            || !self.sensory_weight_scale.is_finite()
            || !(0.1..=2.0).contains(&self.sensory_weight_scale)
        {
            return Err("invalid experimental core threshold/weight/fanout/delay, STDP tau or visual weight scale".into());
        }
        if !self.action_copy_weight.is_finite()
            || !(0.0..=10.0).contains(&self.action_copy_weight)
            || !self.core_membrane_tau_us.is_finite()
            || !(1_000.0..=100_000.0).contains(&self.core_membrane_tau_us)
        {
            return Err("action-copy weight must be finite in 0..=10; Core membrane tau in 1000..=100000 us".into());
        }
        if !self.motor_weight_scale.is_finite()
            || !(0.0..=4.0).contains(&self.motor_weight_scale)
            || !(1_000..=50_000).contains(&self.motor_refractory_us)
        {
            return Err(
                "motor scale must be finite in 0..=4 and refractory time in 1000..=50000 us".into(),
            );
        }
        if self.motor_competition
            && (self.kind != ModelKind::Recurrent || self.keyboard != super::KeyboardLayout::Ad)
        {
            return Err("motor competition is an explicit recurrent A/D experiment".into());
        }
        if !self.motor_inhibition_weight.is_finite()
            || !(0.0..=10.0).contains(&self.motor_inhibition_weight)
        {
            return Err("motor inhibition weight must be finite and in 0..=10".into());
        }
        if !(16..=1024).contains(&self.processing_neurons) {
            return Err("processing neuron count must be in 16..=1024".into());
        }
        if !self.initial_structural_drive.is_finite()
            || !(0.0..=3.0).contains(&self.initial_structural_drive)
        {
            return Err("initial structural drive must be finite and in 0..=3".into());
        }
        if self.pruning_min_below_us == 0 {
            return Err("pruning minimum duration must be positive".into());
        }
        Ok(())
    }

    pub fn frozen(mut self) -> Self {
        self.learning = false;
        self.development = false;
        self.homeostasis = false;
        self
    }
}
