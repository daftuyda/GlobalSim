//! Port of hp_policy.py.
//!
//! Python's `RaceSolver.hp` is a duck-typed field holding either policy and
//! `self.hp.tick(self, dt)` passes the *whole solver* in. That aliases
//! `self` (immutable read) with `self.hp` (mutable) at the same time, which
//! Rust's borrow checker forbids when `hp` is itself a field of the solver.
//! Every method here that needs live solver state takes a small `Copy`
//! context struct of just the fields it reads instead of a solver
//! reference, which sidesteps the aliasing entirely and needs no unsafe.

use crate::course_data::{phase_start, CourseData};
use crate::horse_types::HorseParameters;
use crate::random_gen::Rule30CARng;
use std::cell::RefCell;
use std::rc::Rc;
use crate::retail_physics::{Parameters as RetailParameters, HpState as RetailHpFlags};
use crate::retail_random::RetailRandom;
use crate::retail_spurt::{self, Plan as RetailSpurtPlan};

const HP_STRATEGY_COEFFICIENT: [f64; 6] = [0.0, 0.95, 0.89, 1.0, 0.995, 0.86];
// [surface][ground]; index 0 unused (surfaces are 1-indexed), mirroring the
// Python port's ragged nested list.
const HP_CONSUMPTION_GROUND_MODIFIER: [[f64; 5]; 3] = [
    [0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 1.0, 1.02, 1.02],
    [0.0, 1.0, 1.0, 1.01, 1.02],
];

#[derive(Clone, Copy)]
pub struct HpTickCtx {
    pub is_pace_down: bool,
    pub is_downhill_mode: bool,
    pub is_kakari: bool,
    pub phase: i32,
    pub current_speed: f64,
    pub is_compete_top: bool,
    pub is_runaway: bool,
}

pub struct GameHpState {
    distance: f64,
    base_speed: f64,
    ground_modifier: f64,
    rng: Rule30CARng,
    pub max_hp: f64,
    pub hp: f64,
    guts_modifier: f64,
    subpar_accept_chance: i64,
    retail: Option<RetailHpConfig>,
    pub last_retail_spurt_plan: Option<RetailSpurtPlan>,
}

#[cfg(test)]
mod retail_tests {
    use super::*;
    use crate::horse_types::{Aptitude,Strategy};
    fn parameters()->Rc<RetailParameters> {
        let value=crate::json::parse(include_str!("../data/global_parameters.json")).unwrap();
        Rc::new(RetailParameters::from_value(value.get("parameters").unwrap()).unwrap())
    }
    fn course(distance:f64,surface:i32)->CourseData {
        CourseData {race_track_id:1,distance,distance_type:3,surface,turn:1,course_set_status:vec![],corners:vec![],
            straights:vec![],slopes:vec![],max_lane:1.2,first_move_lane_point:375.,empty_in_move:true}
    }
    fn horse(style:i32,guts:f64,wisdom:f64)->HorseParameters {
        HorseParameters{speed:1200.,stamina:900.,power:1000.,guts,wisdom,strategy:Strategy::from_i32(style).unwrap(),
            distance_aptitude:Aptitude::A,surface_aptitude:Aptitude::A,strategy_aptitude:Aptitude::A,raw_stamina:900.,raw_power:1000.}
    }
    #[test]
    fn integrated_hp_drain_and_heals_match_python_bits() {
        let f=crate::json::parse(include_str!("../tests/fixtures/retail_hp_port.json")).unwrap();
        let parameters=parameters();
        for trace in f.get_array("traces").unwrap() {
            let style=trace.get_i64("strategy").unwrap() as i32;
            let course=course(trace.get_f64("distance").unwrap(),trace.get_i64("surface").unwrap() as i32);
            let mut hp=GameHpState::with_retail(&course,trace.get_i64("ground").unwrap() as i32,parameters.clone(),
                Rc::new(RefCell::new(RetailRandom::new(17))));
            hp.init(&horse(style,600.,1200.));
            assert_eq!((hp.max_hp as f32).to_bits(),trace.get_i64("max_bits").unwrap() as u32);
            for (tick,row) in trace.get_array("rows").unwrap().iter().enumerate() {
                let flags=row.get_i64("flags").unwrap();
                let ctx=HpTickCtx {is_pace_down:flags&1!=0,is_kakari:flags&2!=0,is_downhill_mode:flags&4!=0,is_compete_top:flags&8!=0,
                    is_runaway:style==5,phase:row.get_i64("phase").unwrap() as i32,current_speed:row.get_f64("velocity").unwrap()};
                hp.recover(row.get_f64("heal").unwrap());
                assert_eq!((hp.hp_per_second(ctx,ctx.current_speed) as f32).to_bits(),row.get_i64("rate_bits").unwrap() as u32);
                hp.tick(ctx,f.get_f64("dt").unwrap());
                assert_eq!((hp.hp as f32).to_bits(),row.get_i64("hp_bits").unwrap() as u32,"style {style} tick {tick}");
                assert_eq!(hp.hp,(hp.hp as f32) as f64);
            }
        }
    }
    #[test]
    fn hp_guts_factor_refreshes_on_reinitialization_and_preserves_phase_boundary() {
        let parameters=parameters();
        let course=course(2400.,2);
        let mut hp=GameHpState::with_retail(&course,4,parameters.clone(),
            Rc::new(RefCell::new(RetailRandom::new(1))));
        for guts in [1.,600.,2000.,800.] {
            hp.init(&horse(1,guts,1200.));
            for phase in [-1,0,1,2,3] {
                for flags in 0..32 {
                    let ctx=HpTickCtx {is_pace_down:flags&1!=0,is_kakari:flags&2!=0,
                        is_downhill_mode:flags&4!=0,is_compete_top:flags&8!=0,
                        is_runaway:flags&16!=0,phase,current_speed:22.123_f32 as f64};
                    let rate=parameters.hp_state_rate(RetailHpFlags {rushing:ctx.is_kakari,
                        competing:ctx.is_compete_top,runaway:ctx.is_runaway,
                        pace_down:ctx.is_pace_down,downhill:ctx.is_downhill_mode});
                    let expected=parameters.hp_per_second(ctx.current_speed as f32,
                        hp.base_speed as f32,hp.ground_modifier as f32,guts as f32,
                        phase.max(0) as usize,rate);
                    assert_eq!((hp.hp_per_second(ctx,ctx.current_speed) as f32).to_bits(),expected.to_bits());
                }
            }
        }
    }

    #[test]
    fn spurt_policy_uses_shared_stream_and_current_hp() {
        let f=crate::json::parse(include_str!("../tests/fixtures/retail_physics_port.json")).unwrap();
        let parameters=parameters();
        for row in f.get_array("spurt_vectors").unwrap() {
            let ground=row.get_f64("ground").unwrap();
            let ground=if ground==1. {1} else if ground<1.015 {3} else {4};
            let course=course(row.get_f64("distance").unwrap(),2);
            let random=Rc::new(RefCell::new(RetailRandom::new(row.get_i64("seed").unwrap() as i32)));
            let mut hp=GameHpState::with_retail(&course,ground,parameters.clone(),random.clone());
            hp.init(&horse(1,row.get_f64("guts").unwrap(),row.get_f64("wisdom").unwrap()));
            hp.hp=row.get_f64("hp").unwrap();
            let (start,speed)=hp.get_last_spurt_pair(row.get_f64("position").unwrap(),row.get_f64("maximum").unwrap(),row.get_f64("base").unwrap());
            assert_eq!((start as f32).to_bits(),row.get_i64("start_bits").unwrap() as u32);
            assert_eq!((speed as f32).to_bits(),row.get_i64("speed_bits").unwrap() as u32);
            assert_eq!(hp.last_retail_spurt_plan.unwrap().result,row.get_i64("result").unwrap() as i32);
            assert_eq!(random.borrow().draw_count,row.get_i64("draws").unwrap() as usize);
        }
    }
    #[test]
    fn exhausted_hp_has_no_debt_against_later_recovery() {
        let mut noop=HpPolicy::Noop;
        noop.spend(5000.);
        assert!(noop.has_remaining_hp());
        let mut normal=GameHpState::new(&course(2000.,1),1,Rule30CARng::new(0,0));
        normal.init(&horse(1,600.,1200.));
        let before=normal.hp;
        normal.spend(1./3.);
        assert_eq!(normal.hp,before-1./3.);
        normal.spend(5000.);
        assert_eq!(normal.hp,0.);
        normal.recover(0.055);
        assert_eq!(normal.hp,normal.max_hp*0.055);
        normal.recover(-1.);
        assert_eq!(normal.hp,0.);
        let mut retail=GameHpState::with_retail(&course(2000.,1),1,parameters(),Rc::new(RefCell::new(RetailRandom::new(0))));
        retail.init(&horse(1,600.,1200.));
        retail.spend(5000.);
        assert_eq!(retail.hp,0.);
        retail.recover(0.055);
        assert_eq!(retail.hp,((retail.max_hp as f32)*0.055_f32) as f64);
        retail.recover(-1.);
        assert_eq!(retail.hp,0.);
    }
}

struct RetailHpConfig {
    parameters: Rc<RetailParameters>,
    random: Rc<RefCell<RetailRandom>>,
    guts_factor: f32,
    wisdom: f32,
}

impl GameHpState {
    pub fn new(course: &CourseData, ground: i32, rng: Rule30CARng) -> Self {
        GameHpState {
            distance: course.distance,
            base_speed: 20.0 - (course.distance - 2000.0) / 1000.0,
            ground_modifier: HP_CONSUMPTION_GROUND_MODIFIER[course.surface as usize][ground as usize],
            rng,
            // Placeholder until init(): the first round of skill activations
            // happens before init() is called, but some conditions access HP
            // methods during that round (e.g. is_hp_empty_onetime), so this
            // must be "initialized enough" not to blow up.
            max_hp: 1.0,
            hp: 1.0,
            guts_modifier: 1.0,
            subpar_accept_chance: 0,
            retail: None,
            last_retail_spurt_plan: None,
        }
    }

    /// Select before init/gate skills. The shared stream belongs to the race;
    /// creating this policy must not draw from or clone its random state.
    pub fn with_retail(course: &CourseData, ground: i32, parameters: Rc<RetailParameters>,
                       random: Rc<RefCell<RetailRandom>>) -> Self {
        let mut state=Self::new(course,ground,Rule30CARng::new(0,0));
        state.base_speed=parameters.course_speed(course.distance as f32) as f64;
        let guts_factor=parameters.hp_guts_factor(1.);
        state.retail=Some(RetailHpConfig{parameters,random,guts_factor,wisdom:1.});
        state
    }

    pub fn init(&mut self, horse: &HorseParameters) {
        if let Some(retail)=&mut self.retail {
            self.max_hp=retail.parameters.initial_hp(horse.stamina as f32,horse.strategy as usize,self.distance as f32) as f64;
            self.hp=self.max_hp;
            // Retail HP captures guts at initialization; later stat effects
            // do not change the policy's existing consumption parameters.
            retail.guts_factor=retail.parameters.hp_guts_factor(horse.guts as f32);
            retail.wisdom=horse.wisdom as f32;
            self.last_retail_spurt_plan=None;
            return;
        }
        self.max_hp = 0.8 * HP_STRATEGY_COEFFICIENT[horse.strategy as usize] * horse.stamina + self.distance;
        self.hp = self.max_hp;
        self.guts_modifier = 1.0 + 200.0 / (600.0 * horse.guts).sqrt();
        self.subpar_accept_chance = ((15.0 + 0.05 * horse.wisdom) * 1000.0).round() as i64;
    }

    fn get_status_modifier(ctx: HpTickCtx) -> f64 {
        // Client selects a base state rate, then multiplies downhill.
        let mut modifier = if ctx.is_compete_top {
            if ctx.is_kakari {if ctx.is_runaway {7.7} else {3.6}}
            else if ctx.is_runaway {3.5} else {1.4}
        } else if ctx.is_kakari {1.6}
        else if ctx.is_pace_down {0.6} else {1.0};
        if ctx.is_downhill_mode {
            modifier *= 0.4;
        }
        modifier
    }

    fn hp_per_second(&self, ctx: HpTickCtx, velocity: f64) -> f64 {
        if let Some(retail)=&self.retail {
            let rate=retail.parameters.hp_state_rate(RetailHpFlags{rushing:ctx.is_kakari,competing:ctx.is_compete_top,
                runaway:ctx.is_runaway,pace_down:ctx.is_pace_down,downhill:ctx.is_downhill_mode});
            return retail.parameters.hp_per_second_with_guts_factor(velocity as f32,self.base_speed as f32,self.ground_modifier as f32,
                (ctx.phase >= 2).then_some(retail.guts_factor),rate) as f64;
        }
        let guts_modifier = if ctx.phase >= 2 { self.guts_modifier } else { 1.0 };
        20.0 * (velocity - self.base_speed + 12.0).powi(2) / 144.0
            * Self::get_status_modifier(ctx)
            * self.ground_modifier
            * guts_modifier
    }

    pub fn tick(&mut self, ctx: HpTickCtx, dt: f64) {
        if self.hp <= 0. { return; }
        if self.retail.is_some() {
            let cost=(self.hp_per_second(ctx,ctx.current_speed) as f32)*(dt as f32);
            self.hp=((self.hp as f32-cost).max(0.)) as f64;
            return;
        }
        // _ConsumeHp (0x3421fb0) clamps at zero, without accumulating debt.
        self.spend(self.hp_per_second(ctx, ctx.current_speed) * dt);
    }

    pub fn has_remaining_hp(&self) -> bool {
        self.hp > 0.0
    }

    pub fn hp_ratio_remaining(&self) -> f64 {
        (self.hp / self.max_hp).max(0.0)
    }

    pub fn recover(&mut self, modifier: f64) {
        if self.retail.is_some() {
            let recovered=(self.max_hp as f32)*(modifier as f32);
            self.hp=(self.max_hp as f32).min(self.hp as f32+recovered).max(0.) as f64;
            return;
        }
        self.hp = self.max_hp.min(self.hp + self.max_hp * modifier).max(0.);
    }

    pub fn spend(&mut self, amount: f64) {
        self.hp=if self.retail.is_some() {
            (self.hp as f32-amount as f32).max(0.).min(self.max_hp as f32) as f64
        } else {(self.hp-amount).max(0.).min(self.max_hp)};
    }

    pub fn tick_start_dash(&mut self, ctx: HpTickCtx, dt:f32) {
        let retail=self.retail.as_ref().expect("Retail gate drain requires retail HP policy");
        let rate=retail.parameters.hp_state_rate(RetailHpFlags{rushing:ctx.is_kakari,competing:ctx.is_compete_top,
            runaway:ctx.is_runaway,pace_down:ctx.is_pace_down,downhill:ctx.is_downhill_mode});
        let drain=(20.*rate as f64*self.ground_modifier) as f32;
        self.hp=(self.hp as f32-drain*dt).max(0.) as f64;
    }

    /// Returns (transition_pos, speed). `pos` is the live solver's current
    /// position; everything else needed is captured in `self` or the fixed
    /// synthetic "last leg" context below (phase 2, no pace-down/downhill/
    /// kakari), matching the Python port's `_LastLegState` literal exactly.
    pub fn get_last_spurt_pair(&mut self, pos: f64, max_speed: f64, base_target_speed2: f64) -> (f64, f64) {
        let lastleg = HpTickCtx { is_pace_down: false, is_downhill_mode: false, is_kakari: false, phase: 2, current_speed: 0.0, is_compete_top:false, is_runaway:false };
        if let Some(retail)=&self.retail {
            let input=retail_spurt::Input {distance:self.distance as f32,position:pos as f32,hp:self.hp as f32,
                max_speed:max_speed as f32,base_speed:base_target_speed2 as f32,wisdom:retail.wisdom};
            let plan=retail_spurt::choose(input,retail_spurt::SearchParameters::default(),
                |speed| self.hp_per_second(lastleg,speed as f64) as f32,&mut *retail.random.borrow_mut());
            self.last_retail_spurt_plan=Some(plan);
            return (plan.start as f64,plan.speed as f64);
        }
        let max_dist = self.distance - phase_start(self.distance, 2);
        let s = (max_dist - 60.0) / max_speed;
        if self.hp >= self.hp_per_second(lastleg, max_speed) * s {
            return (-1.0, max_speed);
        }

        let mut candidates: Vec<(f64, f64)> = Vec::new();
        let remain_distance = self.distance - 60.0 - pos;
        let mut speed = max_speed - 0.1;
        while speed >= base_target_speed2 {
            let hp_at_base = self.hp_per_second(lastleg, base_target_speed2);
            let hp_at_speed = self.hp_per_second(lastleg, speed);
            let denom = base_target_speed2 * hp_at_speed - hp_at_base * speed;
            let numer = base_target_speed2 * self.hp - hp_at_base * remain_distance;
            let spurt_duration = (remain_distance / speed).min((numer / denom).max(0.0));
            let spurt_distance = spurt_duration * speed + 60.0;
            candidates.push((self.distance - spurt_distance, speed));
            speed -= 0.1;
        }

        if candidates.is_empty() {
            // maxSpeed - 0.1 < baseTargetSpeed2 (can happen with very low
            // speed, e.g. 1). Not clear what's "correct"; opt for never
            // starting spurt, matching the TS engine.
            return (self.distance, max_speed);
        }

        let finish_time = |cand: &(f64, f64)| (cand.0 - pos) / base_target_speed2 + (self.distance - cand.0) / cand.1;
        candidates.sort_by(|a, b| finish_time(a).partial_cmp(&finish_time(b)).unwrap());
        for &cand in &candidates {
            if self.rng.uniform(100_000.0) as i64 <= self.subpar_accept_chance {
                return cand;
            }
        }
        candidates[candidates.len() - 1]
    }
}

pub enum HpPolicy {
    Noop,
    Game(GameHpState),
}

impl HpPolicy {
    pub fn init(&mut self, horse: &HorseParameters) {
        if let HpPolicy::Game(g) = self {
            g.init(horse);
        }
    }

    pub fn tick(&mut self, ctx: HpTickCtx, dt: f64) {
        if let HpPolicy::Game(g) = self {
            g.tick(ctx, dt);
        }
    }

    pub fn has_remaining_hp(&self) -> bool {
        match self {
            HpPolicy::Noop => true,
            HpPolicy::Game(g) => g.has_remaining_hp(),
        }
    }

    pub fn hp_ratio_remaining(&self) -> f64 {
        match self {
            HpPolicy::Noop => 1.0,
            HpPolicy::Game(g) => g.hp_ratio_remaining(),
        }
    }

    pub fn recover(&mut self, modifier: f64) {
        if let HpPolicy::Game(g) = self {
            g.recover(modifier);
        }
    }

    pub fn get_last_spurt_pair(&mut self, pos: f64, max_speed: f64, base_target_speed2: f64) -> (f64, f64) {
        match self {
            HpPolicy::Noop => (-1.0, max_speed),
            HpPolicy::Game(g) => g.get_last_spurt_pair(pos, max_speed, base_target_speed2),
        }
    }

    /// Keep mutations behind the policy so experimental HP stays float32.
    pub fn spend(&mut self, amount:f64) {
        if let HpPolicy::Game(g)=self {g.spend(amount);}
    }

    /// Current HP for frame recording (`max(0.0,
    /// s.hp.hp) if hasattr(s.hp, "hp") else 0.0` in Python).
    pub fn hp_value(&self) -> Option<f64> {
        match self {
            HpPolicy::Noop => None,
            HpPolicy::Game(g) => Some(g.hp),
        }
    }
}
