//! Input-only shared-stream initialization. Unknown random expressions fail.
//! The normal native race backend is not switched until runtime ordering agrees.
use std::collections::BTreeMap;
use crate::json::Value;
use crate::retail_random::RetailRandom;

// RaceParameter ctor/GetMotivationCoef (3431530/3431830): integer master
// rate scaled by float32 0.0001, then raw Wit multiplied and clamped.
pub fn initial_base_wisdom(raw: f64, mood: i32) -> f32 {
    let coefficient = (10000 + 200 * mood.clamp(-2, 2)) as f32 * 0.0001_f32;
    ((raw as f32) * coefficient).clamp(1.0, 2000.0)
}

#[cfg(test)]
mod initial_wisdom_tests {
    use super::initial_base_wisdom;

    #[test]
    fn official_export_values_and_constructor_bounds() {
        assert_eq!(initial_base_wisdom(1000., -1) as f64, 979.9999389648438);
        assert_eq!(initial_base_wisdom(1014., -1) as f64, 993.719970703125);
        let coefficients = [0.9599999785423279, 0.9799999594688416, 1.0,
            1.0199999809265137, 1.0399999618530273];
        for (i, coefficient) in coefficients.iter().enumerate() {
            assert_eq!(initial_base_wisdom(1024., i as i32 - 2) as f64, coefficient * 1024.);
        }
        assert_eq!(initial_base_wisdom(1600., 0), 1600.);
        assert_eq!(initial_base_wisdom(2000., 2), 2000.);
        assert_eq!(initial_base_wisdom(0., 0), 1.);
    }
}

#[derive(Debug, Clone)]
pub enum Windows { Distances(Vec<(f32,f32)>), Lottery(bool) }
#[derive(Debug, Clone)]
pub struct Trigger {
    pub skill_id: String,
    pub alternative: usize,
    pub field: String,
    pub name: String,
    pub argument: i32,
    pub windows: Windows,
    pub draw_start: usize,
    pub draw_end: usize,
}
#[derive(Default,Clone)]
pub struct HorseInitialization {
    pub runtime_phase: bool,
    pub triggers: Vec<Trigger>,
    pub skill_enabled: BTreeMap<String,bool>,
    pub force_in_move_coefficient: f32,
    pub start_delay: f32,
    pub section_modifiers: Vec<f32>,
    pub rushing_chance: f32,
    pub rushing_section: Option<i32>,
}
#[derive(Clone)]
pub struct Initialization {
    pub horses: Vec<HorseInitialization>,
    pub stages: BTreeMap<String,usize>,
    pub rng: RetailRandom,
}

fn number(value:&Value,key:&str)->Result<f32,String> { Ok(value.get_f64(key)? as f32) }
fn skill_id(entry:&Value)->Result<String,String> { Ok(entry.get_i64("skill_id")?.to_string()) }
fn field<'a>(value:&'a Value,key:&str)->Result<&'a Value,String> {
    value.get(key).ok_or_else(||format!("Missing retail initialization field {key}"))
}

fn terms(text:&str)->Vec<(String,String,i32)> {
    let bytes=text.as_bytes();let mut at=0;let mut found=Vec::new();
    while at<bytes.len() {
        if !(bytes[at].is_ascii_lowercase() || bytes[at]==b'_') {at+=1;continue;}
        let start=at;
        while at<bytes.len() && (bytes[at].is_ascii_lowercase() || bytes[at]==b'_') {at+=1;}
        let name=&text[start..at];let op=at;
        while at<bytes.len() && b"<>=!".contains(&bytes[at]) {at+=1;}
        let operator=&text[op..at];let arg=at;
        while at<bytes.len() && bytes[at].is_ascii_digit() {at+=1;}
        if !operator.is_empty() && arg<at {
            if let Ok(n)=text[arg..at].parse() {found.push((name.to_string(),operator.to_string(),n));}
        }
    }
    found
}

fn distance_window(rng:&mut RetailRandom,start:f32,end:f32)->(f32,f32) {
    let value=rng.real_distance(start,start.max(end-10.0));(value,value+10.0)
}

fn sample_trigger(name:&str,arg:i32,rng:&mut RetailRandom,course:&Value)->Result<Windows,String> {
    let distance=number(course,"distance")?;let section=distance/6.0;
    let phases=[0.0,section,section*4.0,section*5.0,distance,f32::MAX];
    let mut windows=Vec::new();
    match name {
        "all_corner_random" => {
            if arg!=1 {return Err("Unsupported all_corner_random argument".into());}
            let mut candidates=Vec::new();
            for c in course.get_array("corners")? {
                candidates.push((number(c,"start")?,(c.get_f64("start")?+c.get_f64("length")?) as f32));
            }
            for _ in 0..4 {
                if candidates.is_empty() {break;}
                let index=rng.integer(0,candidates.len() as i32) as usize;
                let (mut start,end)=candidates[index];
                let window=distance_window(rng,start,end);windows.push(window);
                if end-window.1<10.0 {candidates.remove(index);}
                else {start=window.1;candidates[index].0=start;}
                candidates.retain(|c|c.0>=start);
            }
            windows.sort_by(|a,b|a.0.total_cmp(&b.0));
        }
        "straight_random" => {
            let rows=course.get_array("straights")?;
            if !rows.is_empty() {
                let row=&rows[rng.integer(0,rows.len() as i32) as usize];
                windows.push(distance_window(rng,number(row,"start")?,number(row,"end")?));
            }
        }
        "up_slope_random" | "down_slope_random" => {
            if arg!=1 {return Err("Unsupported random slope argument".into());}
            let uphill=name=="up_slope_random";
            let mut slopes=Vec::new();
            for row in course.get_array("slopes")? {
                let slope=number(row,"slope")?;
                if slope!=0.0 && (slope>0.0)==uphill {slopes.push(row);}
            }
            if !slopes.is_empty() {
                let row=slopes[rng.integer(0,slopes.len() as i32) as usize];
                windows.push(distance_window(rng,number(row,"start")?,(row.get_f64("start")?+row.get_f64("length")?) as f32));
            }
        }
        "corner_random" | "is_finalcorner_random" => {
            let rows=course.get_array("corners")?;
            // Global SkillTriggerCornerRandom._InitDistance (0x35c3200)
            // reverse-scans explicit corner IDs; absent matches draw nothing.
            let row = if arg <= 0 { None }
            else if name == "corner_random" && rows.iter().any(|r| r.get("index").is_some()) {
                for r in rows { r.get_i64("index")?; }
                rows.iter().rev().find(|r| r.get_i64("index").ok() == Some(arg as i64))
            } else {
                // Final-corner selection is separate. Retain legacy metadata support.
                let index=if name=="is_finalcorner_random" {rows.len() as i32-1} else {rows.len() as i32+arg-5};
                if index >= 0 { rows.get(index as usize) } else { None }
            };
            if let Some(row) = row {
                windows.push(distance_window(rng,number(row,"start")?,(row.get_f64("start")?+row.get_f64("length")?) as f32));
            }
        }
        "last_straight_random" => {
            if arg!=1 {return Err("Unsupported last straight argument".into());}
            if let Some(corner)=course.get_array("corners")?.last() {
                let start=number(corner,"start")?;
                if distance-start<=1000.0 {
                    for row in course.get_array("straights")?.iter().rev() {
                        if number(row,"start")?>=start {
                            windows.push(distance_window(rng,number(row,"start")?,number(row,"end")?));
                            break;
                        }
                    }
                    // Client 0x35c2dc5 completes initialization without a draw
                    // if no qualifying following straight exists.
                }
            }
        }
        "phase_random" | "phase_firsthalf_random" | "phase_laterhalf_random" => {
            if !(0..=4).contains(&arg) {return Err("Unsupported random phase".into());}
            let (mut start,mut end)=(phases[arg as usize],phases[arg as usize+1]);
            let middle=(end-start)*0.5+start;
            if name=="phase_firsthalf_random" {end=middle;}
            if name=="phase_laterhalf_random" {start=middle;}
            windows.push(distance_window(rng,start,end));
        }
        "distance_rate_after_random" => windows.push(distance_window(rng,distance*arg as f32/100.0,distance)),
        "random_lot" => return Ok(Windows::Lottery(rng.integer(0,100)<arg)),
        _ => return Err(format!("Untraced retail random trigger: {name}")),
    }
    Ok(Windows::Distances(windows))
}

pub fn initialize(seed:i32,horses:&[Value],course:&Value,skills:&Value,parameters:&Value)->Result<Initialization,String> {
    let mut rng=RetailRandom::new(seed);
    let mut rows:Vec<HorseInitialization>=(0..horses.len()).map(|_|HorseInitialization::default()).collect();
    for (horse,row) in horses.iter().zip(rows.iter_mut()) {
        for entry in horse.get_array("skills")? {
            let sid=skill_id(entry)?;
            for (alternative,alt) in field(skills,&sid)?.get_array("alternatives")?.iter().enumerate() {
                for column in ["precondition","condition"] {
                    for (name,operator,argument) in terms(alt.get(column).and_then(Value::as_str).unwrap_or("")) {
                        if !name.contains("random") {continue;}
                        if operator!="==" {return Err(format!("Untraced retail random operator: {name}{operator}"));}
                        let draw_start=rng.draw_count;
                        let windows=sample_trigger(&name,argument,&mut rng,course)?;
                        row.triggers.push(Trigger {skill_id:sid.clone(),alternative,field:column.into(),name,argument,
                            windows,draw_start,draw_end:rng.draw_count});
                    }
                }
            }
        }
    }
    let mut stages=BTreeMap::from([("after_triggers".into(),rng.draw_count)]);
    let styles=["","RunningOut","Proceding","Insertion","Chasing"];
    for (horse,row) in horses.iter().zip(rows.iter_mut()) {
        let style=horse.get_i64("running_style")?;
        let coefficient=if (1..=4).contains(&style) {number(parameters,&format!("forceInMoveTargetSpeedAdd{}",styles[style as usize]))?} else {0.0};
        let span=number(parameters,"forceInMoveTargetSpeedAddRandomRange")?;
        let offset=if coefficient>0.0 {rng.real(0.0,span)} else if coefficient<0.0 {rng.real(-span,0.0)} else {0.0};
        row.force_in_move_coefficient=coefficient+offset;
    }
    stages.insert("after_ai".into(),rng.draw_count);
    for (horse,row) in horses.iter().zip(rows.iter_mut()) {
        let chance=(100.0-9000.0/number(horse,"base_wisdom")?).max(20.0);
        for entry in horse.get_array("skills")? {
            let sid=skill_id(entry)?;
            let enabled=field(skills,&sid)?.get_i64("wisdomCheck")?!=1 || chance>rng.real(0.0,100.0);
            row.skill_enabled.insert(sid,enabled);
        }
    }
    stages.insert("after_wisdom".into(),rng.draw_count);
    for (horse,row) in horses.iter().zip(rows.iter_mut()) {
        let mut multiplier=1.0f32;
        for entry in horse.get_array("skills")? {
            let sid=skill_id(entry)?;
            if !row.skill_enabled[&sid] {continue;}
            for alt in field(skills,&sid)?.get_array("alternatives")? {
                if alt.get_str("condition")?!="always==1" || !alt.get("precondition").and_then(Value::as_str).unwrap_or("").is_empty() {continue;}
                for effect in alt.get_array("effects")? {
                    if effect.get_i64("type")?==10 && effect.get_i64("target")?==1 {
                        let removal=(10000.0-effect.get_f64("modifier")?) as f32/10000.0;
                        multiplier*=1.0-removal;
                    }
                }
            }
        }
        row.start_delay=rng.real(0.0,number(parameters,"startDelayMax")?)*multiplier;
    }
    stages.insert("after_delays".into(),rng.draw_count);
    Ok(Initialization {horses:rows,stages,rng})
}

/// Continue the manager's shared stream: all section values, then rushing.
/// Wisdom is the adjusted value after starting green effects; the bonus is
/// in percentage points. These are predicted inputs, not recorded outcomes.
pub fn initialize_sections_and_rushing(init:&mut Initialization, horses:&[(f32,f32)],
                                      course:&Value,parameters:&Value)->Result<(),String> {
    if horses.len()!=init.horses.len() {return Err("Roster changed during initialization".into());}
    let base=20.0-(number(course,"distance")?-2000.0)/1000.0;
    let speeds=field(parameters,"BasetTargetSpeed")?;
    for ((wisdom,_),row) in horses.iter().zip(init.horses.iter_mut()) {
        let logarithm=((*wisdom*0.1) as f64).log10() as f32;
        // Isolated section-only expression-store hypothesis. The logarithm is
        // still the baseline rounded result. Range results are float32 returns.
        let lower=((*wisdom as f64/number(speeds,"BaseTargetSpeedRandomMinusVal2")? as f64)*logarithm as f64
            +number(speeds,"BaseTargetSpeedRandomMinusVal1")? as f64) as f32;
        let upper=((*wisdom as f64/number(speeds,"BaseTargetSpeedRandomPlusVal1")? as f64)*logarithm as f64) as f32;
        row.section_modifiers=(0..24).map(|_| {
            let sample=init.rng.next_double() as f32;
            let width=(upper-lower).abs();
            let random=(sample as f64*width as f64+lower as f64) as f32;
            let random=if random<lower {lower} else if random>upper {upper} else {random};
            ((random as f64 * 0.01_f32 as f64) * base as f64) as f32
        }).collect();
    }
    init.stages.insert("after_sections".into(),init.rng.draw_count);
    let temptation=field(parameters,"Temptation")?;
    for ((wisdom,bonus),row) in horses.iter().zip(init.horses.iter_mut()) {
        let term=*wisdom*0.1+1.0;
        let logarithm=((term*term) as f64).log10() as f32;
        let fraction=number(temptation,"TemptationStartPerVal1")?/logarithm;
        row.rushing_chance=fraction*fraction+bonus;
        row.rushing_section=if row.rushing_chance>init.rng.integer(0,100) as f32 {
            Some(init.rng.integer(temptation.get_i64("TemptationLotSectionMin")? as i32,
                                   temptation.get_i64("TemptationLotSectionMax")? as i32+1))
        } else {None};
    }
    init.stages.insert("after_rushing".into(),init.rng.draw_count);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_following_straight_keeps_empty_trigger_and_rng_state() {
        let course=crate::json::parse(r#"{"distance":2400,"corners":[{"start":2348,"length":500}],"straights":[{"start":1900,"end":2400}]}"#).unwrap();
        let mut rng=RetailRandom::new(123);
        assert!(matches!(sample_trigger("last_straight_random",1,&mut rng,&course).unwrap(),Windows::Distances(windows) if windows.is_empty()));
        assert_eq!(rng.draw_count,0);
    }
    #[test]
    fn all_ninety_official_delays_are_bit_exact_from_inputs() {
        let data=crate::json::parse(include_str!("../tests/fixtures/retail_initialization_official.json")).unwrap();
        let mut count=0;
        for case in data.get_array("cases").unwrap() {
            let course=field(field(&data,"courses").unwrap(),case.get_str("course_id").unwrap()).unwrap();
            let result=initialize(case.get_i64("seed").unwrap() as i32,case.get_array("horses").unwrap(),course,
                field(&data,"skills").unwrap(),field(&data,"parameters").unwrap()).unwrap();
            for (row,expected) in result.horses.iter().zip(case.get_array("expected_start_delays").unwrap()) {
                assert_eq!(row.start_delay as f64,expected.as_f64().unwrap());count+=1;
            }
        }
        assert_eq!(count,90);
    }

    #[test]
    fn all_official_rushing_eligibility_and_sections_follow_shared_stream() {
        let data=crate::json::parse(include_str!("../tests/fixtures/retail_initialization_official.json")).unwrap();
        let mut runners=0;let mut enabled=0;
        for case in data.get_array("cases").unwrap() {
            let course=field(field(&data,"courses").unwrap(),case.get_str("course_id").unwrap()).unwrap();
            let horses=case.get_array("horses").unwrap();
            let params=field(&data,"parameters").unwrap();
            let mut result=initialize(case.get_i64("seed").unwrap() as i32,horses,course,
                field(&data,"skills").unwrap(),params).unwrap();
            let wisdom:Vec<_>=horses.iter().map(|h|(number(h,"base_wisdom").unwrap(),0.0)).collect();
            initialize_sections_and_rushing(&mut result,&wisdom,course,params).unwrap();
            for (row,expected) in result.horses.iter().zip(case.get_array("expected_rushing_sections").unwrap()) {
                assert_eq!(row.rushing_section,expected.as_f64().map(|v|v as i32));
                assert_eq!(row.section_modifiers.len(),24);
                runners+=1;enabled+=usize::from(row.rushing_section.is_some());
            }
            assert_eq!(result.stages["after_sections"]-result.stages["after_delays"],24*horses.len());
        }
        assert_eq!((runners,enabled),(90,5));
    }
}
