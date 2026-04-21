use smokerand_rs::*;

struct Lcg128State {
    x: u128,
}

impl Prng for Lcg128State {
    type Output = u64;

    fn new(intf: &CallerAPI) -> Option<Self> {
        let x = intf.seed128()?;
        Some(Lcg128State { x })
    }
    
    #[inline(always)]
    fn next(&mut self) -> u64 {
        // Константы LCG
        const MULTIPLIER: u128 = 18000_69069_69069_69069;
        const INCREMENT: u128 = 1;
        
        // x = (x * MULTIPLIER + INCREMENT) mod 2^128
        self.x = self.x.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
        
        // Возвращаем старшие 64 бита
        (self.x >> 64) as u64
    }
    
    fn name() -> &'static str {
        "LCG128"
    }
    
    fn description() -> &'static str {
        "128-bit LCG, returns high 64 bits (using u128)"
    }


    fn self_test(intf: &CallerAPI) -> bool {
        use smokerand_rs::PrintfExt;
        
        const REFERENCE: u64 = 0x8E878929D96521D7;
        const ITERATIONS: u64 = 1_000_000;
        
        let mut state = Lcg128State{x : 1234567890};        
        let result = (0..ITERATIONS).fold(0, |_, _| state.next());
        
        intf.rust_printf(format_args!(
            "Result: {:016X}; reference value: {:016X}\n",
            result,
            REFERENCE
        ));
        
        result == REFERENCE
    }
}

impl_ffi_for_prng! {
    type = Lcg128State,
    name = "LCG128 (Rust)",
    description = "128-bit LCG, returns high 64 bits",
}
