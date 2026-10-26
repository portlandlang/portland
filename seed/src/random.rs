//! Ruby's random number generator, step for step (ADR 0064): MT19937 as
//! `random.c` seeds and draws it, so a seeded sequence here is the one
//! Ruby would produce — `Random.new(42).rand(10)` is 6 in both, and
//! `shuffle(random: Random.new(1))` lands the same order. The limited
//! draw's retry mask, the 53-bit float, and the shuffle and sample walks
//! are CRuby's, which ruby/spec pins by exact sequence.

/// The Mersenne Twister state, plus the seed it came from and how many
/// 32-bit words it has handed out — two generators are equal when both
/// agree, which is what Ruby's `==` on a Random asks.
#[derive(Clone, Debug)]
pub struct Mt {
    state: [u32; 624],
    index: usize,
    pub seed: i64,
    pub draws: u64,
}

impl PartialEq for Mt {
    fn eq(&self, other: &Self) -> bool {
        self.seed == other.seed && self.draws == other.draws
    }
}

impl Mt {
    /// Seeded as `rand_init` seeds: a seed that fits one 32-bit word goes
    /// through `init_genrand`, a larger one through `init_by_array` over its
    /// words, least significant first; the sign is dropped, as Ruby drops it.
    pub fn new(seed: i64) -> Mt {
        let magnitude = seed.unsigned_abs();
        let mut generator = Mt {
            state: [0; 624],
            index: 624,
            seed,
            draws: 0,
        };
        let low = magnitude as u32;
        let high = (magnitude >> 32) as u32;
        if high == 0 {
            generator.init_genrand(low);
        } else {
            generator.init_by_array(&[low, high]);
        }
        generator
    }

    fn init_genrand(&mut self, seed: u32) {
        self.state[0] = seed;
        for index in 1..624 {
            let previous = self.state[index - 1];
            self.state[index] = 1_812_433_253u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(index as u32);
        }
        self.index = 624;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let mut i = 1usize;
        let mut j = 0usize;
        let rounds = 624.max(key.len());
        for _ in 0..rounds {
            let previous = self.state[i - 1];
            self.state[i] = (self.state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= 624 {
                self.state[0] = self.state[623];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..623 {
            let previous = self.state[i - 1];
            self.state[i] = (self.state[i]
                ^ (previous ^ (previous >> 30)).wrapping_mul(1_566_083_941))
            .wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                self.state[0] = self.state[623];
                i = 1;
            }
        }
        self.state[0] = 0x8000_0000;
        self.index = 624;
    }

    fn twist(&mut self) {
        for index in 0..624 {
            let bits =
                (self.state[index] & 0x8000_0000) | (self.state[(index + 1) % 624] & 0x7fff_ffff);
            let mut twisted = self.state[(index + 397) % 624] ^ (bits >> 1);
            if bits & 1 == 1 {
                twisted ^= 0x9908_b0df;
            }
            self.state[index] = twisted;
        }
        self.index = 0;
    }

    /// `genrand_int32`.
    pub fn int32(&mut self) -> u32 {
        if self.index >= 624 {
            self.twist();
        }
        let mut word = self.state[self.index];
        self.index += 1;
        self.draws += 1;
        word ^= word >> 11;
        word ^= (word << 7) & 0x9d2c_5680;
        word ^= (word << 15) & 0xefc6_0000;
        word ^= word >> 18;
        word
    }

    /// `genrand_res53`: a float in [0, 1) from 53 random bits.
    pub fn real(&mut self) -> f64 {
        let a = f64::from(self.int32() >> 5);
        let b = f64::from(self.int32() >> 6);
        (a * 67_108_864.0 + b) * (1.0 / 9_007_199_254_740_992.0)
    }

    /// `limited_rand`: an integer in 0..=limit, by masking a draw to the
    /// limit's width and drawing again while it lands above — one word for
    /// a limit under 2^32, two for a wider one.
    pub fn limited(&mut self, limit: u64) -> u64 {
        if limit == 0 {
            return 0;
        }
        let mut mask = limit;
        mask |= mask >> 1;
        mask |= mask >> 2;
        mask |= mask >> 4;
        mask |= mask >> 8;
        mask |= mask >> 16;
        mask |= mask >> 32;
        'retry: loop {
            let mut value: u64 = 0;
            for word in (0..2).rev() {
                if (mask >> (word * 32)) & 0xffff_ffff != 0 {
                    value |= u64::from(self.int32()) << (word * 32);
                    value &= mask;
                    if limit < value {
                        continue 'retry;
                    }
                }
            }
            return value;
        }
    }

    /// `RAND_UPTO(max)`: an index below `max`.
    pub fn upto(&mut self, max: usize) -> usize {
        self.limited(max as u64 - 1) as usize
    }
}

/// `rb_ary_shuffle_bang`'s walk on a copy: from the end, each slot swaps
/// with a draw at or below it.
pub fn shuffled<T: Clone>(elements: &[T], generator: &mut Mt) -> Vec<T> {
    let mut copy = elements.to_vec();
    let mut remaining = copy.len();
    while remaining > 0 {
        let drawn = generator.upto(remaining);
        remaining -= 1;
        copy.swap(remaining, drawn);
    }
    copy
}

/// `rb_ary_sample` for a count, step for step: the draws come first, then
/// one, two, and three elements are placed by CRuby's arithmetic, up to ten
/// by its sorted insertion, and more by a partial shuffle of a copy.
pub fn sampled<T: Clone>(elements: &[T], count: usize, generator: &mut Mt) -> Vec<T> {
    let length = elements.len();
    let count = count.min(length);
    if count <= 10 {
        let draws: Vec<usize> = (0..count)
            .map(|offset| generator.upto(length - offset))
            .collect();
        match count {
            0 => return Vec::new(),
            1 => return vec![elements[draws[0]].clone()],
            2 => {
                let first = draws[0];
                let mut second = draws[1];
                if second >= first {
                    second += 1;
                }
                return vec![elements[first].clone(), elements[second].clone()];
            }
            3 => {
                let first = draws[0];
                let mut second = draws[1];
                let mut third = draws[2];
                let (lower, upper) = if second >= first {
                    second += 1;
                    (first, second)
                } else {
                    (second, first)
                };
                if third >= lower {
                    third += 1;
                    if third >= upper {
                        third += 1;
                    }
                }
                return vec![
                    elements[first].clone(),
                    elements[second].clone(),
                    elements[third].clone(),
                ];
            }
            _ => {}
        }
        let mut sorted: Vec<usize> = vec![draws[0]];
        let mut chosen: Vec<usize> = vec![draws[0]];
        for &draw in &draws[1..] {
            let mut position = draw;
            let mut slot = 0;
            while slot < sorted.len() {
                if position < sorted[slot] {
                    break;
                }
                position += 1;
                slot += 1;
            }
            sorted.insert(slot, position);
            chosen.push(position);
        }
        return chosen
            .into_iter()
            .map(|index| elements[index].clone())
            .collect();
    }
    let mut copy = elements.to_vec();
    for offset in 0..count {
        let drawn = generator.upto(length - offset) + offset;
        copy.swap(offset, drawn);
    }
    copy.truncate(count);
    copy
}

/// A seed from the process's entropy, for a generator nobody seeded.
pub fn entropy_seed() -> i64 {
    use std::hash::{BuildHasher, Hasher};
    let hashed = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish();
    (hashed >> 1) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_draws_are_rubys() {
        let mut generator = Mt::new(42);
        let drawn: Vec<u64> = (0..5).map(|_| generator.limited(9)).collect();
        assert_eq!(drawn, vec![6, 3, 7, 4, 6]);
        assert_eq!(Mt::new(42).real(), 0.3745401188473625);
        let mut wide = Mt::new(1 << 40);
        assert_eq!(wide.limited(9), 1);
    }

    #[test]
    fn shuffle_and_sample_are_crubys() {
        assert_eq!(
            shuffled(&["a", "b", "c"], &mut Mt::new(1)),
            vec!["a", "c", "b"]
        );
        let eleven: Vec<i64> = (0..=10).collect();
        assert_eq!(
            shuffled(&eleven, &mut Mt::new(10)),
            vec![2, 6, 8, 5, 7, 10, 3, 1, 0, 4, 9]
        );
        let five = [1, 2, 3, 4, 5];
        assert_eq!(sampled(&five, 1, &mut Mt::new(42)), vec![4]);
        assert_eq!(sampled(&five, 2, &mut Mt::new(42)), vec![4, 1]);
        assert_eq!(sampled(&five, 3, &mut Mt::new(42)), vec![4, 1, 5]);
        assert_eq!(sampled(&five, 5, &mut Mt::new(42)), vec![4, 1, 5, 2, 3]);
        assert_eq!(sampled(&[1, 2, 3, 4], 4, &mut Mt::new(7)), vec![4, 1, 3, 2]);
        let twelve: Vec<i64> = (1..=12).map(|number| number * 10).collect();
        assert_eq!(
            sampled(&twelve, 11, &mut Mt::new(42)),
            vec![70, 50, 100, 80, 110, 10, 90, 30, 20, 40, 120]
        );
    }
}
