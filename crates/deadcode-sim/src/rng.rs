/// Deterministic SplitMix64 PRNG.
///
/// Used to ensure identical simulation results given the same seed.
/// Per-tick RNG is derived as `SimRng::new(base_seed ^ tick_number)`.
#[derive(Debug, Clone)]
pub struct SimRng {
    state: u64,
}

impl SimRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Return the next pseudo-random u64.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    /// Return a random u64 in `[0, bound)`.
    pub fn next_bounded(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        self.next_u64() % bound
    }

    /// Fisher-Yates shuffle — deterministic given the RNG state.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        let len = slice.len();
        if len <= 1 {
            return;
        }
        for i in (1..len).rev() {
            let j = self.next_bounded(i as u64 + 1) as usize;
            slice.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_output() {
        let mut a = SimRng::new(42);
        let mut b = SimRng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = SimRng::new(1);
        let mut b = SimRng::new(2);
        // Very unlikely to produce the same first value.
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn shuffle_deterministic() {
        let mut rng_a = SimRng::new(99);
        let mut rng_b = SimRng::new(99);
        let mut a = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut b = a.clone();
        rng_a.shuffle(&mut a);
        rng_b.shuffle(&mut b);
        assert_eq!(a, b);
    }

    #[test]
    fn shuffle_permutes() {
        let mut rng = SimRng::new(12345);
        let original = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut shuffled = original.clone();
        rng.shuffle(&mut shuffled);
        // With 10 elements and a fixed seed, extremely unlikely to stay sorted.
        assert_ne!(original, shuffled);
    }

    #[test]
    fn bounded_in_range() {
        let mut rng = SimRng::new(7);
        for _ in 0..10_000 {
            let v = rng.next_bounded(10);
            assert!(v < 10);
        }
    }
}
