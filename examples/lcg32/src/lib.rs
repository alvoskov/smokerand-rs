use smokerand_rs::*;

struct Lcg32State {
    x: u32,
}

impl Prng for Lcg32State {
    type Output = u32;
    
    fn new(intf: &CallerAPI) -> Option<Self> {
        let seed = intf.seed32()?;
        Some(Lcg32State { x: seed })
    }
    
    fn next(&mut self) -> u32 {
        self.x = self.x.wrapping_mul(1664525).wrapping_add(1013904223);
        self.x
    }
    
    fn name() -> &'static str {
        "LCG32"
    }

    fn description() -> &'static str {
        "32-bit LCG"
    }
}

impl_ffi_for_prng! {
    type = Lcg32State,
}
