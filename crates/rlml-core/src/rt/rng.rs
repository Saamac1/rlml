//! Random streams (M 4.1): xoshiro256** seeded by SplitMix64, with the
//! exact rules every implementation follows, so the same seed gives the
//! same game (STD 1.1).

/// One SplitMix64 step: advances `state` and returns the output.
pub fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// `mix(h, x)`: one SplitMix64 step from `h xor x` (M 4.1).
pub fn mix(h: u64, x: u64) -> u64 {
    let mut s = h ^ x;
    splitmix64(&mut s)
}

/// Mixes a name in byte by byte (UTF-8).
pub fn mix_name(mut h: u64, name: &str) -> u64 {
    for b in name.bytes() {
        h = mix(h, b as u64);
    }
    h
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    /// The state from four SplitMix64 outputs of the seed.
    pub fn from_seed(seed: u64) -> Rng {
        let mut sm = seed;
        Rng { s: [splitmix64(&mut sm), splitmix64(&mut sm), splitmix64(&mut sm), splitmix64(&mut sm)] }
    }

    pub fn from_state(s: [u64; 4]) -> Rng {
        Rng { s }
    }

    pub fn state(&self) -> [u64; 4] {
        self.s
    }

    /// xoshiro256**.
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// `below(n)`, n ≥ 1: Lemire's method without bias.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n >= 1, "below(0)");
        let mut m = (self.next_u64() as u128) * (n as u128);
        if (m as u64) < n {
            let t = n.wrapping_neg() % n;
            while (m as u64) < t {
                m = (self.next_u64() as u128) * (n as u128);
            }
        }
        (m >> 64) as u64
    }

    /// An integer from `a..b`; an empty range draws nothing and gives `None`.
    pub fn range(&mut self, a: i64, b: i64) -> Option<i64> {
        if b < a {
            return None;
        }
        let span = (b as i128 - a as i128 + 1) as u64;
        Some(a + self.below(span) as i64)
    }

    /// `NdM+K`: N separate throws of `1 + below(M)` in order, plus K.
    pub fn dice(&mut self, n: u32, sides: u32, bonus: i64) -> i64 {
        let mut sum = bonus;
        for _ in 0..n {
            sum += 1 + self.below(sides.max(1) as u64) as i64;
        }
        sum
    }

    /// `chance(p)`: always one throw `1 + below(100)`, true when at most p.
    pub fn chance(&mut self, p: i64) -> bool {
        (1 + self.below(100) as i64) <= p
    }

    /// `choose`: an index of a set of `len` members; an empty set draws
    /// nothing.
    pub fn choose(&mut self, len: usize) -> Option<usize> {
        (len > 0).then(|| self.below(len as u64) as usize)
    }

    /// `pick(j, n)`: Fisher–Yates from the front, stopped after `n` places;
    /// `shuffled` and a deck shuffle are the whole sequence.
    pub fn pick<T: Clone>(&mut self, items: &[T], n: usize) -> Vec<T> {
        let mut v = items.to_vec();
        let k = n.min(v.len());
        for i in 0..k {
            let j = i + self.below((v.len() - i) as u64) as usize;
            v.swap(i, j);
        }
        v.truncate(k);
        v
    }

    pub fn shuffle<T: Clone>(&mut self, items: &mut Vec<T>) {
        let n = items.len();
        *items = self.pick(items, n);
    }

    /// The copy a query uses: the caller's state words xor-ed, the caller's
    /// and the query's full names mixed in, a 0 byte between (M 4.1).
    pub fn query_copy(&self, caller: &str, query: &str) -> Rng {
        let h = self.s.iter().fold(0, |h, w| h ^ w);
        let h = mix_name(h, caller);
        let h = mix(h, 0);
        Rng::from_seed(mix_name(h, query))
    }

    /// The presentation stream of a scene start (M 4.1).
    pub fn presentation(seed: u64, scene: &str, ordinal: u64) -> Rng {
        let h = mix(mix_name(seed, scene), 0);
        Rng::from_seed(mix(h, ordinal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference output of splitmix64.c for seed 1234567.
    #[test]
    fn splitmix64_reference() {
        let mut s = 1234567;
        let out: Vec<u64> = (0..5).map(|_| splitmix64(&mut s)).collect();
        assert_eq!(out, [6457827717110365317, 3203168211198807973, 9817491932198370423, 4593380528125082431, 16408922859458223821]);
    }

    /// The reference output of xoshiro256starstar.c from state {1, 2, 3, 4}.
    #[test]
    fn xoshiro256starstar_reference() {
        let mut r = Rng::from_state([1, 2, 3, 4]);
        let out: Vec<u64> = (0..5).map(|_| r.next_u64()).collect();
        assert_eq!(out, [11520, 0, 1509978240, 1215971899390074240, 1216172134540287360]);
    }

    #[test]
    fn same_seed_same_stream() {
        let (mut a, mut b) = (Rng::from_seed(7), Rng::from_seed(7));
        for _ in 0..100 {
            assert_eq!(a.dice(3, 6, 2), b.dice(3, 6, 2));
        }
        assert_ne!(Rng::from_seed(7).next_u64(), Rng::from_seed(8).next_u64());
    }

    #[test]
    fn bounded_values() {
        let mut r = Rng::from_seed(1);
        for _ in 0..1000 {
            assert!(r.below(6) < 6);
            let d = r.dice(2, 6, 1);
            assert!((3..=13).contains(&d));
            assert!((-1..=3).contains(&r.range(-1, 3).unwrap()));
        }
        let before = r.clone();
        assert_eq!(r.range(0, -1), None, "an empty range draws nothing");
        assert_eq!(r.choose(0), None);
        assert_eq!(r, before);
        assert!(!r.chance(0) && r.chance(100), "chance always draws once");
        assert_ne!(r, before);
    }

    #[test]
    fn picks_and_shuffles() {
        let mut r = Rng::from_seed(3);
        let items: Vec<u32> = (0..10).collect();
        let p = r.pick(&items, 4);
        assert_eq!(p.len(), 4);
        let mut sorted = p.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 4, "distinct members");
        let mut all = items.clone();
        Rng::from_seed(3).shuffle(&mut all);
        assert_eq!(&all[..4], &p[..], "pick is the start of the same shuffle");
        assert_eq!(r.pick(&items[..2], 5).len(), 2, "a small set gives all members");
    }

    #[test]
    fn derived_streams() {
        let g = Rng::from_seed(42);
        let a = g.query_copy("STD.HUNTER", "STD.HOSTILES");
        assert_eq!(a, g.query_copy("STD.HUNTER", "STD.HOSTILES"), "a query's copy is fixed by state and names");
        assert_ne!(a, g.query_copy("STD.FOLLOWER", "STD.HOSTILES"));
        assert_ne!(a, g.query_copy("STD.HUNTERSTD", ".HOSTILES"), "the 0 byte separates the names");
        assert_ne!(Rng::presentation(42, "INTRO", 0), Rng::presentation(42, "INTRO", 1));
    }
}
