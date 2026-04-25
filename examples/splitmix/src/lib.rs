use smokerand_rs::*;

struct SplitMixState {
    x: u64,
}

impl Prng for SplitMixState {
    type Output = u64; 

    fn new(intf: &CallerAPI) -> Option<Self> {
        let get_seed64 = intf.get_seed64?;
        let seed = unsafe { get_seed64() };
        Some(SplitMixState { x: seed })
    }
    
    #[inline(always)]
    fn next(&mut self) -> u64 {
        const GAMMA: u64 = 0x9E3779B97F4A7C15;
        self.x = self.x.wrapping_add(GAMMA);
        let mut z = self.x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    
    fn name() -> &'static str {
        "SplitMix"
    }
    
    fn description() -> &'static str {
        "SplitMix64 generator"
    }
    
    fn self_test(intf: &CallerAPI) -> bool {
        let state = match Self::new(intf) {
            Some(s) => s,
            None => return false,
        };
        
        // Простой тест: генерируем два числа и проверяем что они разные
        let mut test_state = state;
        let a = test_state.next();
        let b = test_state.next();
        
        a != b
    }
}

// Вызов макроса с обязательными параметрами
impl_ffi_for_prng! {
    type = SplitMixState,
}
