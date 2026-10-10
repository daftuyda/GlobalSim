//! Local passing corridors; kept equivalent to Python lane_selection.py.
//! Rows exclude finished runners and contain distance/lane/speed/target speed.
const LANE: f64 = 1.0 / 18.0;
pub type Row = (f64, f64, f64, f64);

fn visible(pos: f64, lane: f64, vision: f64, p: f64, l: f64) -> bool {
    let gap=p-pos;
    vision>0.0 && gap>=0.0 && gap<=vision && (l-lane).abs()<=(gap/vision*11.5+2.0)*LANE/2.0
}

pub fn collect_targets(pos: f64, lane: f64, speed: f64, target_speed: f64,
                       vision: f64, others: &[Row]) -> Vec<usize> {
    let blocked = |row: Row| {
        others.iter().copied().chain(std::iter::once((pos, lane, speed, target_speed))).any(|(q,m,_,_)| {
            let gap = q - row.0;
            gap > 0.0 && gap < 2.0 && (m-row.1).abs() <= (1.0 - 0.3*gap)*0.75*LANE
        })
    };
    let nearest = others.iter().enumerate().filter_map(|(i,&(p,l,_,_))| {
        let gap = p-pos;
        if gap > 0.0 && gap < 2.0 && (l-lane).abs() <= (1.0 - 0.3*gap)*0.75*LANE {
            Some((gap,i))
        } else { None }
    }).min_by(|a,b| a.partial_cmp(b).unwrap()).map(|x| x.1);
    let targets: Vec<usize> = others.iter().enumerate().filter_map(|(i,&row)| {
        let (p,l,v,wanted) = row;
        let gap = p-pos;
        let closing = speed-v;
        if nearest == Some(i) || (visible(pos,lane,vision,p,l) && crate::overtaking::is_target(gap,closing,target_speed,wanted,v,false,blocked(row))) { Some(i) } else { None }
    }).collect();
    targets
}

pub fn passing_lane(pos: f64, lane: f64, phase: i32, max_lane: f64,
                    vision: f64, others: &[Row], targets: &[usize], lane_preference:u8) -> Option<f64> {
    if targets.is_empty() { return None; }
    let clear = |candidate: f64, end: f64| {
        if candidate < 0.0 || candidate > max_lane { return false; }
        for &(p,l,_,_) in others {
            if visible(pos,lane,vision,p,l) && pos <= p && p <= end && (l-candidate).abs() <= 0.8*LANE { return false; }
            if (p-pos).abs() < 1.05 {
                if l > lane && candidate > lane && candidate > l-2.0*LANE { return false; }
                if l < lane && candidate < lane && candidate < l+2.0*LANE { return false; }
            }
        }
        true
    };
    let mut candidates = Vec::new();
    for &index in targets {
        for direction in [-1.0,1.0] {
            let mut edge=index;
            loop {
                let (p,l,_,_)=others[edge];
                let mut next=edge;
                for (j,&(q,m,_,_)) in others.iter().enumerate() {
                    if q-p>=0.0 && q-p<=3.0 && direction*(m-l)>0.0 && direction*(m-l)<2.0*LANE
                        && direction*m>direction*others[next].1 { next=j; }
                }
                if next==edge {break;}
                edge=next;
            }
            let candidate=others[edge].1+direction*LANE;
            if clear(candidate,others[edge].0+0.5) {candidates.push(candidate);}
        }
    }
    let can_move_in=lane>0.0001 && !others.iter().any(|&(p,l,_,_)|
        (p-pos).abs()<1.05 && lane-l>0.0 && lane-l<2.0*LANE);
    let straight=if phase<2 && can_move_in && lane>LANE {lane-LANE} else {lane};
    let end=targets.iter().map(|&i|others[i].0).fold(f64::INFINITY,f64::min)+3.0;
    if clear(straight,end) {candidates.push(straight);}
    let score = |candidate: f64| {
        let weight = if candidate < lane {
            if phase == 0 { 100.0 } else if phase >= 2 { 1.149999976158142 } else { 1.0 }
        } else { 1.0 };
        let gap=lane-candidate;
        let skill=if (lane_preference==1 && gap>0.) || (lane_preference==2 && gap<0.) {1.5} else {1.};
        ((gap*weight)*skill).abs()
    };
    Some(candidates.into_iter().min_by(|a,b| score(*a).partial_cmp(&score(*b)).unwrap()).unwrap_or(lane))
}
