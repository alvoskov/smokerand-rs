use smokerand_rs::*;

struct Kiss03State {
    x: u32, // 32-bit LCG state
    y: u32, // xorshift32 state
    z: u32, // MWC state: lower part
    c: u32, // MWC state: higher part (carry)
}

impl Prng for Kiss03State {
    type Output = u32;

    fn new(intf: &CallerAPI) -> Option<Self> {
        // Seed using two 64-bit values converted to 2x32 each
        let seed1 = intf.seed64()?;
        let seed2 = intf.seed64()?;
        
        let x = (seed1 >> 32) as u32;
        let y = match (seed1 & 0xFFFFFFFF) as u32 {
            0 => 0x12345678, // Ensure y is non-zero
            y_raw => y_raw,
        };
        let z = (seed2 >> 32) as u32;
        // Adjust c to be in range 1..2^28
        let c = ((seed2 & 0xFFFFFFF) + 1) as u32;
        Some(Kiss03State { x, y, z, c })
    }
    
    #[inline(always)]
    fn next(&mut self) -> u32 {
        // LCG part: x = 69069 * x + 12345
        self.x = 69069_u32.wrapping_mul(self.x).wrapping_add(12345);
        // Xorshift part
        self.y ^= self.y << 13;
        self.y ^= self.y >> 17;
        self.y ^= self.y << 5;        
        // MWC part: t = 698769069 * z + c
        let t = 698769069_u64.wrapping_mul(self.z as u64).wrapping_add(self.c as u64);
        self.c = (t >> 32) as u32;
        self.z = t as u32;        
        // Combined output
        self.x.wrapping_add(self.y).wrapping_add(self.z)
    }
    
    fn name() -> &'static str {
        "KISS2003"
    }
    
    fn description() -> &'static str {
        "KISS2003 combined PRNG (LCG + Xorshift + MWC)"
    }
    
    fn self_test(intf: &CallerAPI) -> bool {
        use smokerand_rs::PrintfExt;
        
        const X_REF: u32 = 0x8E41D4F8;
        const ITERATIONS: u64 = 10_000_000;
        
        let mut state = Kiss03State {
            x: 123456789,
            y: 987654321,
            z: 43219876,
            c: 6543217,
        };
        
        let mut x: u32 = 0;
        for _ in 0..ITERATIONS {
            x = state.next();
        }
        
        intf.rust_printf(format_args!(
            "Observed: 0x{:08X}; expected: 0x{:08X}\n",
            x, X_REF
        ));
        
        x == X_REF
    }
}

impl_ffi_for_prng! {
    type = Kiss03State,
}
