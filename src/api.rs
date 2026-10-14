//! Prepared, thread-confined races with fresh mutable state for every seed.
use crate::*;

fn initialize_tables() -> Result<(), String> {
    static READY: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    READY.get_or_init(|| {
        course_data::init_courses(load_course_table(COURSE_DATA_JSON)?);
        Ok(())
    }).clone()
}

fn conditions() -> &'static BTreeMap<String, activation_conditions::Condition> {
    static TABLE: std::sync::OnceLock<BTreeMap<String, activation_conditions::Condition>> = std::sync::OnceLock::new();
    TABLE.get_or_init(activation_conditions::build_conditions)
}

/// Immutable race preparation plus reusable roster facts. Create one on each
/// calling thread. The handle cannot be sent between threads or shared by them.
pub struct PreparedRace {
    spec: RaceSpec,
    skills: Rc<SkillTable>,
    initialization: Option<retail_initialization::PreparedInitialization>,
    retail_sections: bool,
    retail_runtime: bool,
    world_geometry: Option<Rc<retail_geometry::CourseGeometry>>,
    startup_pass: bool,
    retain_all_frames: bool,
    gate_perm: Option<Vec<usize>>,
    signature: Option<Value>,
    roster: Option<PreparedRoster>,
}

impl PreparedRace {
    /// Prepare a conventional JSON request. Only the seed changes between runs.
    /// Preparation cost is separate from per-seed simulation cost.
    pub fn new(input: &str) -> Result<Self, String> {
        initialize_tables()?;
        let (input, gate_perm) = gate_order(json::parse(input)?)?;
        let raw_skills = match input.get("skillData") {
            Some(data) => data,
            None => bundled_skills()?,
        };
        let skills = Rc::new(load_skill_value(raw_skills)?);
        let spec = parse_race_spec(&input)?;
        let course = bundled_courses()?.get(&spec.course_id.to_string())
            .ok_or("Missing retail course")?;
        let retail_runtime = input.get("experimentalRetailRuntime").and_then(Value::as_bool) == Some(true);
        if spec.trace_conserve_power && !retail_runtime {
            return Err("traceConservePower requires experimentalRetailRuntime".into());
        }
        let startup_pass = match input.get("experimentalRetailStartup") {
            None => false,
            Some(value) => value.as_bool().ok_or("experimentalRetailStartup must be a boolean")?,
        };
        if startup_pass && !retail_runtime {
            return Err("experimentalRetailStartup requires the experimental runtime".into());
        }
        let world_geometry = if input.get("experimentalRetailGeometry").and_then(Value::as_bool) == Some(true) {
            if !retail_runtime || spec.experimental_grid {
                return Err("World geometry requires the experimental runtime and standard starting grid".into());
            }
            Some(bundled_geometry(spec.course_id, course_data::get_course(spec.course_id).distance as f32)?)
        } else { None };
        let retail_sections = input.get("experimentalRetailSections").and_then(Value::as_bool) == Some(true);
        let initialization = if input.get("experimentalRetailInitialization").and_then(Value::as_bool) == Some(true)
            || retail_sections || retail_runtime {
            let horses = spec.horses.iter().map(|horse| {
                Ok(json::obj(vec![
                    ("skills", Value::Array(horse.skills.iter().map(|(sid, level, _)| json::obj(vec![
                        ("skill_id", json::num(sid.parse::<f64>().unwrap_or(0.))),
                        ("level", json::num(*level as f64)),
                    ])).collect())),
                    ("base_wisdom", json::num(retail_initialization::initial_base_wisdom(
                        if horse.wisdom > 1200. { 1200. + (horse.wisdom - 1200.) * 0.5 } else { horse.wisdom },
                        horse.mood) as f64)),
                    ("running_style", json::num(Strategy::parse_str(&horse.strategy_str)?.base_running_style() as f64)),
                ]))
            }).collect::<Result<Vec<_>, String>>()?;
            Some(retail_initialization::PreparedInitialization::new(&horses, course, raw_skills,
                bundled_parameters()?.get("parameters").ok_or("Missing retail parameters")?)?)
        } else { None };
        Ok(Self { spec, skills, initialization, retail_sections, retail_runtime,
            world_geometry, startup_pass, retain_all_frames: input.get("retainAllFrames").and_then(Value::as_bool) == Some(true),
            gate_perm, signature: input.get("dataSignature").cloned(), roster: None })
    }

    /// Run one independent seed with every mutable solver, RNG, timer and
    /// activation observer rebuilt. The result owns its complete selected replay.
    pub fn run(&mut self, seed: i64) -> Result<Simulation, String> {
        self.spec.seed = seed;
        let initialization = self.initialization.as_ref().map(|plan| {
            let seed = i32::try_from(seed).map_err(|_| "Retail seed must be a signed 32-bit integer".to_string())?;
            plan.initialize(seed)
        }).transpose()?;
        let mut value = race_solver_builder::with_skills(self.skills.clone(), || run_race(
            &self.spec, conditions(), initialization.as_ref(), self.retail_sections, self.retail_runtime,
            self.world_geometry.clone(), self.startup_pass, self.retain_all_frames, None, &mut self.roster))?;
        if let Value::Object(map) = &mut value {
            map.insert("experimentalRetailInitialization".into(), Value::Bool(initialization.is_some()));
            if let Some(signature) = &self.signature { map.insert("dataSignature".into(), signature.clone()); }
        }
        if let Some(perm) = &self.gate_perm { restore_request_order(&mut value, perm); }
        Ok(Simulation { value })
    }
}

/// Owned output. Frame views borrow this result and remain valid until it drops.
pub struct Simulation { value: Value }

impl Simulation {
    pub fn frames(&self) -> &crate::ReplayFrames {
        match self.value.get("frames") {
            Some(Value::ReplayFrames(frames)) => frames,
            _ => unreachable!("Prepared runs always retain numeric replay storage"),
        }
    }

    pub fn write_json(&self, destination: &mut String) {
        json::to_string_into(&self.value, destination);
    }

    /// Consume the result, replacing both caller-owned buffers with its binary
    /// replay packet and JSON metadata. Existing capacity can be reused.
    pub fn into_binary(mut self, metadata: &mut String, packet: &mut Vec<u8>) {
        binary_replay::encode(&mut self.value, metadata, packet);
    }
}
