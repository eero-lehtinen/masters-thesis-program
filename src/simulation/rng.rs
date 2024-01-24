use rand::{prelude::*, Error};
use rand_xoshiro::Xoshiro256PlusPlus;

pub struct FastRng(pub Xoshiro256PlusPlus);

impl Default for FastRng {
    fn default() -> Self {
        Self(Xoshiro256PlusPlus::seed_from_u64(98374098))
    }
}

impl RngCore for FastRng {
    #[inline(always)]
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    #[inline(always)]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest);
    }

    #[inline(always)]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.0.try_fill_bytes(dest)
    }
}

impl SeedableRng for FastRng {
    type Seed = <Xoshiro256PlusPlus as SeedableRng>::Seed;

    #[inline(always)]
    fn from_seed(seed: Self::Seed) -> Self {
        Self(Xoshiro256PlusPlus::from_seed(seed))
    }

    #[inline(always)]
    fn from_rng<R: RngCore>(rng: R) -> Result<Self, Error> {
        Xoshiro256PlusPlus::from_rng(rng).map(FastRng)
    }
}
