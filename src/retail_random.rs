//! Legacy System.Random plus the extracted RaceRandom float wrapper.
//! Not a claim that all runtime consumers follow the retail draw order yet.

#[derive(Clone)]
pub struct RetailRandom {
    state: [i32;56],
    a: usize,
    b: usize,
    pub draw_count: usize,
}

/// Runtime state machines receive the shared stream instead of cloning it.
/// The small interface also permits exact lottery-boundary tests.
pub trait RandomSource {
    fn integer(&mut self, lower: i32, upper: i32) -> i32;
    fn real(&mut self, lower: f32, upper: f32) -> f32;
}

impl RandomSource for RetailRandom {
    fn integer(&mut self, lower: i32, upper: i32) -> i32 { RetailRandom::integer(self, lower, upper) }
    fn real(&mut self, lower: f32, upper: f32) -> f32 { RetailRandom::real(self, lower, upper) }
}

impl RetailRandom {
    pub fn new(seed: i32) -> Self {
        let mut state=[0;56];
        let magnitude=if seed==i32::MIN {i32::MAX} else {seed.abs()};
        let mut previous=161803398i32.wrapping_sub(magnitude);
        state[55]=previous;
        let mut current=1i32;
        for n in 1..55 {
            let slot=21*n%55;
            state[slot]=current;
            current=previous.wrapping_sub(current);
            if current<0 {current=current.wrapping_add(i32::MAX);}
            previous=state[slot];
        }
        for _ in 0..4 {
            for slot in 1..56 {
                let value=state[slot].wrapping_sub(state[1+(slot+30)%55]);
                state[slot]=if value<0 {value.wrapping_add(i32::MAX)} else {value};
            }
        }
        Self {state,a:0,b:21,draw_count:0}
    }

    pub fn next_int(&mut self) -> i32 {
        self.a=if self.a<55 {self.a+1} else {1};
        self.b=if self.b<55 {self.b+1} else {1};
        let mut value=self.state[self.a].wrapping_sub(self.state[self.b]);
        if value==i32::MAX {value-=1;}
        if value<0 {value=value.wrapping_add(i32::MAX);}
        self.state[self.a]=value;
        self.draw_count+=1;
        value
    }

    pub fn next_double(&mut self) -> f64 {
        self.next_int() as f64*(1.0/i32::MAX as f64)
    }

    pub fn integer(&mut self, lower:i32, upper:i32) -> i32 {
        assert!(lower<=upper,"Random bounds must be ordered");
        let width=upper as i64-lower as i64;
        if width<=i32::MAX as i64 {
            return ((self.next_double()*width as f64) as i64+lower as i64) as i32;
        }
        let mut value=self.next_int() as i64;
        if self.next_int()%2==0 {value=-value;}
        let sample=(value+(i32::MAX as i64-1)) as f64/(2*i32::MAX as i64-1) as f64;
        ((sample*width as f64) as i64+lower as i64) as i32
    }

    // Isolated hypothesis: preserve the float sample and interval width,
    // but store the distance only after multiply-and-add. No extra draw.
    pub fn real_distance(&mut self, lower:f32, upper:f32) -> f32 {
        let sample=self.next_double() as f32;
        let width=(upper-lower).abs();
        let value=(sample as f64*width as f64+lower as f64) as f32;
        if value<lower {lower} else if value>upper {upper} else {value}
    }

    pub fn real(&mut self, lower:f32, upper:f32) -> f32 {
        let sample=self.next_double() as f32;
        let value=sample*(upper-lower).abs()+lower;
        if value<lower {lower} else if value>upper {upper} else {value}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_dotnet_reference_all_signed_seed_boundaries() {
        let data=crate::json::parse(include_str!("../tests/fixtures/retail_rng_dotnet.json")
            .trim_start_matches('\u{feff}')).unwrap();
        for row in data.as_array().unwrap() {
            let seed=row.get_i64("seed").unwrap() as i32;
            let mut rng=RetailRandom::new(seed);
            for value in row.get_array("integers").unwrap() {
                assert_eq!(rng.next_int() as f64,value.as_f64().unwrap());
            }
            let mut rng=RetailRandom::new(seed);
            for value in row.get_array("doubles").unwrap() {
                assert_eq!(rng.next_double(),value.as_f64().unwrap());
            }
            let mut rng=RetailRandom::new(seed);
            for value in row.get_array("wide").unwrap() {
                assert_eq!(rng.integer(i32::MIN,i32::MAX) as f64,value.as_f64().unwrap());
            }
        }
    }

    #[test]
    fn zero_width_still_consumes_a_draw() {
        let mut rng=RetailRandom::new(953795537);
        assert_eq!(rng.integer(5,5),5);
        assert_eq!(rng.real(12.0,12.0),12.0);
        assert_eq!(rng.draw_count,2);
    }
}
