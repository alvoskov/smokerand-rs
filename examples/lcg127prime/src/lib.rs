use smokerand_rs::*;

struct Lcg128State {
    x: u128
}

impl Lcg128State {
    #[inline(always)]
    fn get_bits_generic_raw(&mut self, a: u64) -> u64 {
        const MASK: u64 = 0x7fffffffffffffff;        
        // a*x using u128 for 128-bit multiplication
        let a_val = a as u128;
        let prod0 = a_val.wrapping_mul((self.x as u64) as u128);
        let prod1 = a_val.wrapping_mul(self.x >> 64).wrapping_add(prod0 >> 64);

        let mut m_low  = prod0 as u64;
        let mut m_mid  = prod1 as u64;        
        let m_high = (prod1 >> 64) as u64;

        // m = h*(2**127 - 1) + l where l <= 2**127 - 1
        let h = (m_high << 1) | (m_mid >> 63);
        m_mid &= MASK;
        
        // l += h (using u128 for carry handling)
        let sum = (m_low as u128).wrapping_add((m_mid as u128) << 64).wrapping_add(h as u128);
        m_low = sum as u64;
        m_mid = (sum >> 64) as u64;
        
        // If highest bit is set, add 1
        if m_mid >> 63 != 0 {
            let sum2 = (m_low as u128).wrapping_add((m_mid as u128) << 64).wrapping_add(1);
            m_low = sum2 as u64;
            m_mid = (sum2 >> 64) as u64;
        }
        
        // Update state
        self.x = (m_low as u128) | (((m_mid & MASK) as u128) << 64);
        // Return lower 64 bits
        m_low
    }

    #[inline(always)]
    fn get_bits_mul1_raw(&mut self) -> u64 {
        self.get_bits_generic_raw(13433445539930070091_u64)
    }
}

impl Prng for Lcg128State {
    type Output = u64;

    fn new(intf: &CallerAPI) -> Option<Self> {
        let x = match intf.seed128()? >> 1 {
            0 => 0x12345678, // Ensure x is non-zero
            x_raw => x_raw,
        };
        Some(Lcg128State { x })
    }

    #[inline(always)]
    fn next(&mut self) -> u64 {
        self.get_bits_mul1_raw()
    }

    fn name() -> &'static str {
        "Lcg127prime:mul1"
    }

    fn description() -> &'static str {
        "The x = ax mod 2**127 - 1 LCG that returns the lower 64 bits.\n\
         Uses multiplier a = 13433445539930070091"
    }

    fn self_test(intf: &CallerAPI) -> bool {
        use smokerand_rs::PrintfExt;

        let mut state = Lcg128State {
            x: 1
        };

        const ITERATIONS: usize = 1_000_000;
        const REF_VALUE: u64 = 0xe490c2a6c3e38bcd;

        let mut u: u64 = 0;
        for _ in 0..ITERATIONS {
            u = state.get_bits_mul1_raw();
        }

        intf.rust_printf(format_args!(
            "Result: {:X}; reference value: {:X}\n",
            u, REF_VALUE
        ));

        u == REF_VALUE
    }
}

impl_ffi_for_prng! {
    type = Lcg128State,
}
