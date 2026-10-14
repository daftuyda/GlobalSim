//! Contiguous, exact numeric replay storage. Selection never changes samples.
#[derive(Clone, Debug, Default)]
pub struct ReplayFrames {
    horse_count: usize,
    times: Vec<f64>,
    horses: Vec<[f64; 6]>,
    selected: Option<Vec<usize>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_and_reordering_preserve_samples_and_blockers() {
        let mut frames = ReplayFrames::new(3);
        for tick in 0..5 {
            frames.capture(tick as f64, |i| [tick as f64, i as f64, 100., -0., 0.,
                [-1., 2., 0.][i]]);
        }
        frames.select(vec![0, 2, 4]);
        frames.restore_order(&[2, 0, 1]);
        assert_eq!(frames.raw_len(), 5);
        assert_eq!(frames.len(), 3);
        for i in 0..frames.len() {
            let (t, rows) = frames.frame(i);
            assert_eq!(t, (2 * i) as f64);
            assert_eq!(rows.iter().map(|h| h[1]).collect::<Vec<_>>(), [1., 2., 0.]);
            assert_eq!(rows.iter().map(|h| h[5]).collect::<Vec<_>>(), [1., 2., -1.]);
            assert!(rows.iter().all(|h| h[3].to_bits() == (-0_f64).to_bits()));
        }
    }

    #[test]
    fn empty_roster_can_retain_timestamps_without_rows() {
        let mut frames = ReplayFrames::new(0);
        frames.capture(2., |_| panic!("No runner samples expected"));
        frames.restore_order(&[]);
        assert_eq!(frames.frame(0), (2., &[][..]));
    }
}

impl ReplayFrames {
    pub fn new(horse_count: usize) -> Self {
        Self { horse_count, ..Self::default() }
    }

    pub fn horse_count(&self) -> usize { self.horse_count }
    pub fn raw_len(&self) -> usize { self.times.len() }
    pub fn len(&self) -> usize {
        self.selected.as_ref().map_or(self.times.len(), Vec::len)
    }

    pub fn capture(&mut self, t: f64, sample: impl FnMut(usize) -> [f64; 6]) {
        self.times.push(t);
        self.horses.extend((0..self.horse_count).map(sample));
    }

    pub fn raw_frame(&self, index: usize) -> (f64, &[[f64; 6]]) {
        let start = index * self.horse_count;
        (self.times[index], &self.horses[start..start + self.horse_count])
    }

    pub fn frame(&self, index: usize) -> (f64, &[[f64; 6]]) {
        self.raw_frame(self.selected.as_ref().map_or(index, |s| s[index]))
    }

    pub fn select(&mut self, selected: Vec<usize>) {
        assert!(selected.iter().all(|&i| i < self.raw_len()));
        self.selected = Some(selected);
    }

    pub fn restore_order(&mut self, perm: &[usize]) {
        let mut ordered = vec![[0.; 6]; self.horse_count];
        if self.horse_count == 0 { return; }
        for horses in self.horses.chunks_exact_mut(self.horse_count) {
            for h in horses.iter_mut() {
                let blocker = h[5];
                if blocker >= 0. && blocker.fract() == 0. && (blocker as usize) < perm.len() {
                    h[5] = perm[blocker as usize] as f64;
                }
            }
            if self.horse_count == perm.len() {
                for (internal, &horse) in horses.iter().enumerate() {
                    ordered[perm[internal]] = horse;
                }
                horses.copy_from_slice(&ordered);
            }
        }
    }

    pub fn bitwise_eq(&self, other: &Self) -> bool {
        self.horse_count == other.horse_count && self.len() == other.len()
            && (0..self.len()).all(|i| {
                let (ta, a) = self.frame(i);
                let (tb, b) = other.frame(i);
                ta.to_bits() == tb.to_bits() && a.iter().flatten().zip(b.iter().flatten())
                    .all(|(a, b)| a.to_bits() == b.to_bits())
            })
    }
}
