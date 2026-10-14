//! Port of condition_parser.py.
//!
//! The original is a Pratt (TDOP) parser purely because the upstream
//! TypeScript engine was written that way; the grammar it implements has no
//! parentheses and only three precedence levels, which a plain recursive
//! descent parser expresses directly and equivalently:
//!
//!   Or  ::= And '@' Or | And
//!   And ::= Cmp '&' And | Cmp
//!   Cmp ::= condition Op integer
//!
//! Every Parser this codebase ever constructs uses the stock And/Or/
//! comparison operators (only the *condition table* is ever swapped, for
//! `.withActivateCountsAsRandom()`), so unlike the Python port this file
//! does not reproduce the swappable-operator-class machinery -- there is
//! nothing here that would ever exercise it.

use crate::activation_conditions::{make_and, make_or, CmpKind, CondResult, Condition, Op};
use std::collections::BTreeMap;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type ParseContext = (usize, bool, bool);

struct CachedExpression {
    root: Rc<Op>,
    sampled_nodes: Vec<(String, i32)>,
}

#[derive(Default)]
struct ParseCache {
    contexts: HashMap<ParseContext, HashMap<String, Rc<CachedExpression>>>,
    entries: usize,
}

thread_local! {
    // Tables have static lifetimes; their addresses are stable identities. Only
    // immutable syntax is shared, and eviction cannot affect an existing Op.
    static PARSE_CACHE: RefCell<ParseCache> = RefCell::new(ParseCache::default());
}

#[derive(Clone)]
enum Token {
    Int(i64),
    Ident(String),
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
}

fn is_id_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'
}

fn tokenize(s: &str) -> CondResult<Vec<Token>> {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut i = 0usize;
    let mut out = Vec::new();
    while i < n {
        let ch = chars[i];
        if ch.is_ascii_digit() {
            let start = i;
            while i < n && chars[i].is_ascii_digit() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            out.push(Token::Int(text.parse().map_err(|_| "bad integer literal".to_string())?));
        } else if is_id_char(ch) {
            let start = i;
            while i < n && is_id_char(chars[i]) {
                i += 1;
            }
            out.push(Token::Ident(chars[start..i].iter().collect()));
        } else if ch == '=' {
            if i + 1 >= n || chars[i + 1] != '=' {
                return Err("expected =".to_string());
            }
            i += 2;
            out.push(Token::Eq);
        } else if ch == '!' {
            if i + 1 >= n || chars[i + 1] != '=' {
                return Err("expected =".to_string());
            }
            i += 2;
            out.push(Token::Neq);
        } else if ch == '<' {
            if i + 1 < n && chars[i + 1] == '=' {
                i += 2;
                out.push(Token::Lte);
            } else {
                i += 1;
                out.push(Token::Lt);
            }
        } else if ch == '>' {
            if i + 1 < n && chars[i + 1] == '=' {
                i += 2;
                out.push(Token::Gte);
            } else {
                i += 1;
                out.push(Token::Gt);
            }
        } else if ch == '@' {
            i += 1;
            out.push(Token::Or);
        } else if ch == '&' {
            i += 1;
            out.push(Token::And);
        } else {
            return Err(format!("invalid character {ch:?}"));
        }
    }
    Ok(out)
}

struct TokStream {
    toks: Vec<Token>,
    pos: usize,
    retail: Option<std::collections::VecDeque<crate::retail_initialization::Trigger>>,
    runtime_phase: bool,
}

impl TokStream {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
}

fn parse_or(ts: &mut TokStream, conditions: &'static BTreeMap<String, Condition>) -> CondResult<Op> {
    let left = parse_and(ts, conditions)?;
    if matches!(ts.peek(), Some(Token::Or)) {
        ts.advance();
        let right = parse_or(ts, conditions)?;
        make_or(left, right)
    } else {
        Ok(left)
    }
}

fn parse_and(ts: &mut TokStream, conditions: &'static BTreeMap<String, Condition>) -> CondResult<Op> {
    let left = parse_cmp(ts, conditions)?;
    if matches!(ts.peek(), Some(Token::And)) {
        ts.advance();
        let right = parse_and(ts, conditions)?;
        make_and(left, right)
    } else {
        Ok(left)
    }
}

fn parse_cmp(ts: &mut TokStream, conditions: &'static BTreeMap<String, Condition>) -> CondResult<Op> {
    let name = match ts.advance() {
        Some(Token::Ident(s)) => s,
        _ => return Err("expected condition identifier".to_string()),
    };
    let cond: &'static Condition = if ts.runtime_phase && name=="phase" {
        crate::activation_conditions::retail_phase_condition()
    } else if ts.runtime_phase && name=="distance_rate" {
        crate::activation_conditions::retail_distance_rate_condition()
    } else if ts.runtime_phase && name=="remain_distance" {
        crate::activation_conditions::retail_remaining_distance_condition()
    } else if ts.runtime_phase && name=="furlong" {
        crate::activation_conditions::retail_furlong_condition()
    } else if ts.runtime_phase && (name=="corner" || name=="is_finalcorner") {
        crate::activation_conditions::retail_corner_condition(name=="is_finalcorner")
    } else if ts.runtime_phase && name=="is_finalcorner_laterhalf" {
        crate::activation_conditions::retail_final_corner_half_condition()
    } else if ts.runtime_phase && name=="slope" {
        crate::activation_conditions::retail_slope_condition()
    } else {crate::activation_conditions::lookup_condition(conditions, &name)
        .ok_or_else(|| format!("unknown condition {name}"))?};
    let kind = match ts.advance() {
        Some(Token::Eq) => CmpKind::Eq,
        Some(Token::Neq) => CmpKind::Neq,
        Some(Token::Lt) => CmpKind::Lt,
        Some(Token::Lte) => CmpKind::Lte,
        Some(Token::Gt) => CmpKind::Gt,
        Some(Token::Gte) => CmpKind::Gte,
        _ => return Err("expected comparison operator".to_string()),
    };
    let arg = match ts.advance() {
        Some(Token::Int(n)) => n as f64,
        _ => return Err("expected integer".to_string()),
    };
    if name.contains("random") {
        if let Some(triggers)=ts.retail.as_mut() {
            let trigger=triggers.pop_front().ok_or_else(||format!("Missing sampled retail trigger: {name}"))?;
            if !matches!(kind,CmpKind::Eq) || trigger.name!=name || trigger.argument as f64!=arg {
                return Err("Sampled retail trigger differs from expression node".into());
            }
            return Ok(Op::Fixed(trigger.windows));
        }
    }
    Ok(Op::Cmp { cond, kind, arg })
}

/// Parses one condition-expression string (e.g. `"phase==2&distance_rate>=50"`)
/// against the given condition table (`Conditions` or
/// `ConditionsWithActivateCountsAsRandom`).
pub fn parse(conditions: &'static BTreeMap<String, Condition>, s: &str) -> CondResult<Op> {
    parse_with_retail(conditions,s,None)
}

pub fn parse_with_retail(conditions: &'static BTreeMap<String, Condition>, s: &str,
    retail: Option<Vec<crate::retail_initialization::Trigger>>) -> CondResult<Op> {
    parse_with_retail_runtime(conditions,s,retail,false)
}

pub fn parse_with_retail_runtime(conditions: &'static BTreeMap<String, Condition>, s: &str,
    retail: Option<Vec<crate::retail_initialization::Trigger>>, runtime_phase:bool) -> CondResult<Op> {
    let context = (conditions as *const _ as usize, runtime_phase, retail.is_some());
    let cached = PARSE_CACHE.with(|cache| cache.borrow().contexts.get(&context)
        .and_then(|entries| entries.get(s)).cloned());
    if let Some(cached) = cached {
        let mut triggers = retail.unwrap_or_default().into_iter();
        let mut windows = Vec::with_capacity(cached.sampled_nodes.len());
        for (name, argument) in &cached.sampled_nodes {
            let trigger = triggers.next().ok_or_else(|| format!("Missing sampled retail trigger: {name}"))?;
            if trigger.name != *name || trigger.argument != *argument {
                return Err("Sampled retail trigger differs from expression node".into());
            }
            windows.push(trigger.windows);
        }
        if triggers.next().is_some() {
            return Err("Unused sampled retail expression nodes".into());
        }
        return Ok(Op::Shared(cached.root.clone(), windows));
    }
    // The first parse uses the original error ordering. Cache only successful
    // syntax, then remove all runner-specific samples from the stored tree.
    let sampled_nodes = retail.as_ref().map(|triggers| triggers.iter()
        .map(|trigger| (trigger.name.clone(), trigger.argument)).collect()).unwrap_or_default();
    let mut root = parse_uncached(conditions, s, retail, runtime_phase)?;
    let mut windows = Vec::new();
    root.separate_samples(&mut windows);
    let root = Rc::new(root);
    let cached = Rc::new(CachedExpression { root: root.clone(), sampled_nodes });
    PARSE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.entries >= 4096 {
            cache.contexts.clear();
            cache.entries = 0;
        }
        cache.contexts.entry(context).or_default().insert(s.to_string(), cached);
        cache.entries += 1;
    });
    Ok(Op::Shared(root, windows))
}

fn parse_uncached(conditions: &'static BTreeMap<String, Condition>, s: &str,
    retail: Option<Vec<crate::retail_initialization::Trigger>>, runtime_phase:bool) -> CondResult<Op> {
    let toks = tokenize(s)?;
    if toks.is_empty() {
        return Err("empty condition expression".to_string());
    }
    let mut ts = TokStream { toks, pos: 0, retail: retail.map(Into::into), runtime_phase };
    let op = parse_or(&mut ts, conditions)?;
    if ts.pos != ts.toks.len() {
        return Err("trailing tokens after expression".to_string());
    }
    if ts.retail.as_ref().is_some_and(|queue| !queue.is_empty()) {
        return Err("Unused sampled retail expression nodes".into());
    }
    Ok(op)
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use crate::activation_conditions::{build_conditions, take_anyskill_nodes};
    use crate::course_data::CourseData;
    use crate::horse_types::{Aptitude, HorseParameters, Strategy};
    use crate::race_parameters::{RaceParameters, RaceParametersWithSkillId};
    use crate::region::Region;
    use crate::retail_initialization::{Trigger, Windows};

    fn inputs() -> (CourseData, HorseParameters) {
        (CourseData {
            race_track_id: 1, distance: 2400.0, distance_type: 3, surface: 1, turn: 1,
            course_set_status: vec![], corners: vec![], straights: vec![], slopes: vec![],
            max_lane: 1.3, first_move_lane_point: 0.0, empty_in_move: true,
        }, HorseParameters {
            speed: 1000.0, stamina: 1000.0, power: 1000.0, guts: 1000.0, wisdom: 1000.0,
            strategy: Strategy::Nige, distance_aptitude: Aptitude::A,
            surface_aptitude: Aptitude::A, strategy_aptitude: Aptitude::A,
            raw_stamina: 1000.0, raw_power: 1000.0,
        })
    }

    fn trigger(start: f32) -> Trigger {
        Trigger { skill_id: "1".into(), alternative: 0, field: "condition".into(),
            name: "phase_random".into(), argument: 0,
            windows: Windows::Distances(vec![(start, start + 10.0)]), draw_start: 0, draw_end: 1 }
    }

    #[test]
    fn cached_syntax_keeps_samples_and_observer_state_private() {
        let table = Box::leak(Box::new(build_conditions()));
        let (course, horse) = inputs();
        let params = RaceParameters::default();
        let extra = RaceParametersWithSkillId { params: &params, skill_id: "1".into() };
        let regions = [Region::new(0.0, course.distance)];
        let expression = "phase_random==0&is_activate_any_skill==1";
        let first = parse_with_retail_runtime(table, expression, Some(vec![trigger(10.0)]), true).unwrap();
        let second = parse_with_retail_runtime(table, expression, Some(vec![trigger(100.0)]), true).unwrap();
        take_anyskill_nodes();
        assert_eq!(first.apply(&regions, &course, &horse, &extra).unwrap().0, vec![Region::new(10.0, 20.0)]);
        let first_nodes = take_anyskill_nodes();
        assert_eq!(second.apply(&regions, &course, &horse, &extra).unwrap().0, vec![Region::new(100.0, 110.0)]);
        let second_nodes = take_anyskill_nodes();
        assert_eq!(first_nodes.len(), 1);
        assert_eq!(second_nodes.len(), 1);
        assert_ne!(first_nodes, second_nodes);
        // Re-applying a parsed Op also constructs fresh observation state.
        assert_eq!(first.apply(&regions, &course, &horse, &extra).unwrap().0, vec![Region::new(10.0, 20.0)]);
        assert_ne!(first_nodes, take_anyskill_nodes());
    }

    #[test]
    fn warm_cache_preserves_sample_validation_errors() {
        let table = Box::leak(Box::new(build_conditions()));
        let expression = "phase_random==0&always==1";
        parse_with_retail_runtime(table, expression, Some(vec![trigger(10.0)]), true).unwrap();
        let mut wrong_name = trigger(10.0);
        wrong_name.name = "corner_random".into();
        let mut wrong_argument = trigger(10.0);
        wrong_argument.argument = 1;
        for triggers in [vec![], vec![wrong_name], vec![wrong_argument], vec![trigger(10.0), trigger(20.0)]] {
            let expected = parse_uncached(table, expression, Some(triggers.clone()), true).err();
            assert!(expected.is_some());
            assert_eq!(parse_with_retail_runtime(table, expression, Some(triggers), true).err(), expected);
        }
    }

    #[test]
    fn parse_context_separates_runtime_and_condition_tables() {
        let ordinary = Box::leak(Box::new(build_conditions()));
        let randomized = Box::leak(Box::new(crate::activation_conditions::build_conditions_with_activate_counts_as_random()));
        let (course, horse) = inputs();
        let params = RaceParameters::default();
        let extra = RaceParametersWithSkillId { params: &params, skill_id: "1".into() };
        let regions = [Region::new(0.0, course.distance)];
        for _ in 0..2 {
            let static_phase = parse_with_retail_runtime(ordinary, "phase>=2", Some(vec![]), false).unwrap();
            let dynamic_phase = parse_with_retail_runtime(ordinary, "phase>=2", Some(vec![]), true).unwrap();
            let static_result = static_phase.apply(&regions, &course, &horse, &extra).unwrap();
            let dynamic_result = dynamic_phase.apply(&regions, &course, &horse, &extra).unwrap();
            assert_eq!(static_result.0, vec![Region::new(1600.0, 2400.0)]);
            assert!(static_result.1.is_none());
            assert_eq!(dynamic_result.0, regions);
            assert!(dynamic_result.1.is_some());
            assert!(matches!(parse(ordinary, "activate_count_all>=3").unwrap().sample_policy(), crate::activation_sample_policy::SamplePolicy::Immediate));
            assert!(matches!(parse(randomized, "activate_count_all>=3").unwrap().sample_policy(), crate::activation_sample_policy::SamplePolicy::Random));
        }
    }
}
