pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng {
            state: seed.wrapping_add(0x853c_49e6_748f_ea9b),
            inc: 0xda3e_39cb_94b9_5bdb | 1,
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6364136223846793005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn f64(&mut self) -> f64 {
        self.next_u32() as f64 / (u32::MAX as f64 + 1.0)
    }

    /// Internal state pair — for the canonical full-state digest (the RNG
    /// steers the future, so it belongs in there).
    pub fn state_pair(&self) -> (u64, u64) {
        (self.state, self.inc)
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.f64() * (hi - lo)
    }

    /// Uniform integer in the INCLUSIVE range [lo, hi]. Single-value ranges
    /// (lo == hi) consume no entropy and return lo.
    pub fn irange(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(hi >= lo, "irange({lo}, {hi}): empty range");
        if hi <= lo {
            return lo;
        }
        lo + self.next_u32() % (hi - lo + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;

    #[test]
    fn deterministic_sequence() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn irange_is_inclusive_and_bounded() {
        let mut r = Rng::new(11);
        let mut seen_min = false;
        let mut seen_max = false;
        for _ in 0..500 {
            let v = r.irange(3, 6);
            assert!((3..=6).contains(&v));
            seen_min |= v == 3;
            seen_max |= v == 6;
        }
        assert!(seen_min && seen_max, "both endpoints must be reachable");
        assert_eq!(r.irange(5, 5), 5);
    }
}
