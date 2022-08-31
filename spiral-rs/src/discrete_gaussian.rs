use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::thread_rng;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::cell::*;

use crate::client::*;
use crate::params::*;
use crate::poly::*;
use std::f64::consts::PI;

pub const NUM_WIDTHS: usize = 8;

thread_local!(static RNGS: RefCell<Option<ChaCha20Rng>> = RefCell::new(None));

pub struct DiscreteGaussian {
    choices: Vec<i64>,
    dist: WeightedIndex<f64>,
}

impl DiscreteGaussian {
    pub fn init(params: &Params) -> Self {
        Self::init_impl(params, None)
    }

    pub fn init_with_seed(params: &Params, seed: Seed) -> Self {
        Self::init_impl(params, Some(seed))
    }

    fn init_impl(params: &Params, seed: Option<Seed>) -> Self {
        let max_val = (params.noise_width * (NUM_WIDTHS as f64)).ceil() as i64;
        let mut choices = Vec::new();
        let mut table = vec![0f64; 0];
        for i in -max_val..max_val + 1 {
            let p_val = f64::exp(-PI * f64::powi(i as f64, 2) / f64::powi(params.noise_width, 2));
            choices.push(i);
            table.push(p_val);
        }
        let dist = WeightedIndex::new(&table).unwrap();

        if seed.is_some() {
            RNGS.with(|f| *f.borrow_mut() = Some(ChaCha20Rng::from_seed(seed.unwrap())));
        } else {
            RNGS.with(|f| *f.borrow_mut() = Some(ChaCha20Rng::from_seed(thread_rng().gen())));
        }

        Self { choices, dist }
    }

    // FIXME: not constant-time
    pub fn sample(&self) -> i64 {
        RNGS.with(|f| {
            let mut rng = f.borrow_mut();
            let rng_mut = rng.as_mut().unwrap();
            self.choices[self.dist.sample(rng_mut)]
        })
    }

    pub fn sample_matrix(&self, p: &mut PolyMatrixRaw) {
        let modulus = p.get_params().modulus;
        for r in 0..p.rows {
            for c in 0..p.cols {
                let poly = p.get_poly_mut(r, c);
                for z in 0..poly.len() {
                    let mut s = self.sample();
                    s += modulus as i64;
                    s %= modulus as i64; // FIXME: not constant time
                    poly[z] = s as u64;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util::*;

    #[test]
    fn dg_seems_okay() {
        let params = get_test_params();
        let dg = DiscreteGaussian::init_with_seed(&params, get_chacha_seed());
        let mut v = Vec::new();
        let trials = 10000;
        let mut sum = 0;
        for _ in 0..trials {
            let val = dg.sample();
            v.push(val);
            sum += val;
        }
        let mean = sum as f64 / trials as f64;
        let std_dev = params.noise_width / f64::sqrt(2f64 * std::f64::consts::PI);
        let std_dev_of_mean = std_dev / f64::sqrt(trials as f64);
        assert!(f64::abs(mean) < std_dev_of_mean * 5f64);
    }
}
