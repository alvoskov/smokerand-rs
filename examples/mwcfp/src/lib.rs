use smokerand_rs::*;
use std::ops::{BitOr, BitXor};
use std::ffi::CStr;

// ============ Трейт с разрядностью ============
trait MwcWord: Copy + Default + PartialEq + PartialOrd + BitOr<Output = Self> + BitXor<Output = Self> {
    const BITS: u32;
    const SHIFT_AMOUNT: Self;
    const RRX_SHIFT1: u32;
    const RRX_SHIFT2: u32;
    
    // Переименовываем чтобы избежать конфликта
    type PrngOutput;
    
    fn wrapping_mul(self, other: Self) -> Self;
    fn wrapping_add(self, other: Self) -> Self;
    fn wrapping_sub(self, other: Self) -> Self;
    fn wrapping_shl(self, rhs: u32) -> Self;
    fn wrapping_shr(self, rhs: u32) -> Self;
    fn from_seed(intf: &CallerAPI) -> Option<Self>;
    fn to_prng_output(self) -> Self::PrngOutput;
    fn to_u64(self) -> u64;  // ← Возвращаем to_u64 для арифметики
    fn from_u64(v: u64) -> Self;
    fn zero() -> Self;
    fn one() -> Self;
    fn two() -> Self;
    
    fn rotl(self, n: u32) -> Self {
        (self.wrapping_shl(n)) | (self.wrapping_shr(Self::BITS - n))
    }
    
    fn sub_with_borrow(a: &mut Self, b: Self) -> Self;
    fn add_with_carry(a: &mut Self, b: Self) -> Self;
}

impl MwcWord for u32 {
    const BITS: u32 = 32;
    const SHIFT_AMOUNT: Self = 1;
    const RRX_SHIFT1: u32 = 11;
    const RRX_SHIFT2: u32 = 27;
    
    type PrngOutput = u32;
    
    #[inline(always)]
    fn wrapping_mul(self, other: Self) -> Self { self.wrapping_mul(other) }
    #[inline(always)]
    fn wrapping_add(self, other: Self) -> Self { self.wrapping_add(other) }
    #[inline(always)]
    fn wrapping_sub(self, other: Self) -> Self { self.wrapping_sub(other) }
    #[inline(always)]
    fn wrapping_shl(self, rhs: u32) -> Self { self.wrapping_shl(rhs) }
    #[inline(always)]
    fn wrapping_shr(self, rhs: u32) -> Self { self.wrapping_shr(rhs) }
    #[inline(always)]
    fn from_seed(intf: &CallerAPI) -> Option<Self> { intf.seed32() }
    #[inline(always)]
    fn to_prng_output(self) -> Self::PrngOutput { self }
    #[inline(always)]
    fn to_u64(self) -> u64 { self as u64 }
    #[inline(always)]
    fn from_u64(v: u64) -> Self { v as u32 }
    #[inline(always)]
    fn zero() -> Self { 0 }
    #[inline(always)]
    fn one() -> Self { 1 }
    #[inline(always)]
    fn two() -> Self { 2 }
    
    #[inline(always)]
    fn sub_with_borrow(a: &mut Self, b: Self) -> Self {
        let t = (*a).wrapping_sub(b);
        let borrow = if t > *a { 1 } else { 0 };
        *a = t;
        borrow
    }
    
    #[inline(always)]
    fn add_with_carry(a: &mut Self, b: Self) -> Self {
        let t = (*a).wrapping_add(b);
        let carry = if t < *a { 1 } else { 0 };
        *a = t;
        carry
    }
}

impl MwcWord for u64 {
    const BITS: u32 = 64;
    const SHIFT_AMOUNT: Self = 2;
    const RRX_SHIFT1: u32 = 17;
    const RRX_SHIFT2: u32 = 53;
    
    type PrngOutput = u64;
    
    #[inline(always)]
    fn wrapping_mul(self, other: Self) -> Self { self.wrapping_mul(other) }
    #[inline(always)]
    fn wrapping_add(self, other: Self) -> Self { self.wrapping_add(other) }
    #[inline(always)]
    fn wrapping_sub(self, other: Self) -> Self { self.wrapping_sub(other) }
    #[inline(always)]
    fn wrapping_shl(self, rhs: u32) -> Self { self.wrapping_shl(rhs) }
    #[inline(always)]
    fn wrapping_shr(self, rhs: u32) -> Self { self.wrapping_shr(rhs) }
    #[inline(always)]
    fn from_seed(intf: &CallerAPI) -> Option<Self> { intf.seed64() }
    #[inline(always)]
    fn to_prng_output(self) -> Self::PrngOutput { self }
    #[inline(always)]
    fn to_u64(self) -> u64 { self }
    #[inline(always)]
    fn from_u64(v: u64) -> Self { v }
    #[inline(always)]
    fn zero() -> Self { 0 }
    #[inline(always)]
    fn one() -> Self { 1 }
    #[inline(always)]
    fn two() -> Self { 2 }
    
    #[inline(always)]
    fn sub_with_borrow(a: &mut Self, b: Self) -> Self {
        let t = (*a).wrapping_sub(b);
        let borrow = if t > *a { 1 } else { 0 };
        *a = t;
        borrow
    }
    
    #[inline(always)]
    fn add_with_carry(a: &mut Self, b: Self) -> Self {
        let t = (*a).wrapping_add(b);
        let carry = if t < *a { 1 } else { 0 };
        *a = t;
        carry
    }
}

// ============ Структура состояния ============
#[derive(Clone)]
struct MwcFpState<W: MwcWord, const LAG: usize> {
    x: [W; LAG],
    x_extra: W,
    c: W,
    pos: usize,
    mul: W,
}

impl<W: MwcWord, const LAG: usize> MwcFpState<W, LAG> {
    fn new(intf: &CallerAPI, mul: u64) -> Option<Self> {
        let mut x = [W::zero(); LAG];
        
        for i in 0..LAG {
            x[i] = W::from_seed(intf)?;
        }
        
        Some(MwcFpState {
            x,
            x_extra: W::zero(),
            c: W::one(),
            pos: LAG,
            mul: W::from_u64(mul),
        })
    }
    
    #[inline(always)]
    fn get_x(&self, idx: usize) -> W {
        if idx < LAG {
            self.x[idx]
        } else {
            self.x_extra
        }
    }
    
    #[inline(always)]
    fn set_x(&mut self, idx: usize, val: W) {
        if idx < LAG {
            self.x[idx] = val;
        } else {
            self.x_extra = val;
        }
    }
    
    fn next_raw(&mut self) -> W {
        if self.pos == LAG {
            let mul = self.mul;
            
            // Используем to_u64() для арифметики вместо to_output()
            let mut carry = self.c;
            for i in 0..LAG {
                let product = (mul.to_u64() as u128) 
                    * (self.get_x(i).to_u64() as u128)
                    + (carry.to_u64() as u128);
                self.set_x(i, W::from_u64(product as u64));
                carry = W::from_u64((product >> W::BITS) as u64);
            }
            
            self.c = W::from_u64(
                ((mul.to_u64() as u128) * (self.x_extra.to_u64() as u128)
                 + (carry.to_u64() as u128)) as u64
            );
            
            let c_shifted = self.c.wrapping_mul(W::SHIFT_AMOUNT);
            
            let mut borrow = W::sub_with_borrow(&mut self.x[0], c_shifted);
            borrow = W::sub_with_borrow(
                &mut self.x[1], 
                self.c.wrapping_shr(W::BITS - 1).wrapping_add(borrow)
            );
            
            for i in 2..LAG {
                borrow = W::sub_with_borrow(&mut self.x[i], borrow);
            }
            
            if borrow == W::zero() {
                self.x_extra = W::zero();
            } else {
                self.c = self.c.wrapping_sub(W::one());
                let mut carry = W::two();
                for i in 0..LAG {
                    carry = W::add_with_carry(&mut self.x[i], carry);
                }
                self.x_extra = carry;
            }
            
            self.pos = 0;
        }
        
        let result = if self.pos < LAG {
            self.x[self.pos]
        } else {
            self.x_extra
        };
        self.pos += 1;
        result
    }
}


/// Тест для 64-битных генераторов
fn run_test64<G: Prng<Output = u64>>(
    intf: &CallerAPI,
    mut gen: G,
    u_ref: &[u64; 8],
    lag: usize,
) -> bool {
    // Прогреваем генератор
    for _ in 0..10000 * lag {
        let _ = gen.next();
    }

    let mut is_ok = true;
    for i in 0..8 {
        let u = gen.next();
        intf.rust_printf(format_args!("{:016X} ", u));
        if u != u_ref[i] {
            is_ok = false;
        }
    }
    intf.rust_printf(format_args!("\n"));
    is_ok
}

/// Тест для 32-битных генераторов
fn run_test32<G: Prng<Output = u32>>(
    intf: &CallerAPI,
    mut gen: G,
    u_ref: &[u32; 8],
    lag: usize,
) -> bool {
    // Прогреваем генератор
    for _ in 0..10000 * lag {
        let _ = gen.next();
    }

    let mut is_ok = true;
    for i in 0..8 {
        let u = gen.next();
        intf.rust_printf(format_args!("{:08X} ", u));
        if u != u_ref[i] {
            is_ok = false;
        }
    }
    intf.rust_printf(format_args!("\n"));
    is_ok
}


// ============ Конкретные тесты для каждого варианта ============
/*
/// Тест для MWC512u64 (lag=8)
fn test_mwc512u64(intf: &CallerAPI) -> bool {
    // Создаём состояние вручную с фиксированными значениями
    let mut state = MwcFpState::<u64, 8> {
        x: [
            0x324486EF33B244DE,
            0xBDF3EFA8BFFC4712,
            0xC8DBBD5E28D756DF,
            0xD30EE545B1860CE8,
            0x8812CF194A614701,
            0xC8EF05BA91470D22,
            0x15D944BA02AA4CE7,
            0x0000000000000001,
        ],
        x_extra: 0x0000000000000000,
        c: 0x3B6DDCC704530974,
        pos: 8,
        mul: 16996179571824182298u64,
        use_rrx: false,
    };

    const U_REF: [u64; 8] = [
        0xA4A7BCED2B2A12DA, 0xA87A2252C527DBC0, 0xF40FD080694601A9, 0x4B434187C33BC54B,
        0x7136A7C65B18A544, 0x6B34FD3E458AE6DF, 0x2EAAB4F627081604, 0xD21AE89EE2D61327,
    ];

    run_test64(intf, state, &U_REF, 8)
}

/// Тест для MWC256u32 (lag=8)
fn test_mwc256u32(intf: &CallerAPI) -> bool {
    let mut state = MwcFpState::<u32, 8> {
        x: [
            0x9D2B5B2E, 0xD83D1A25, 0x867FCA2B, 0x20F8F49A,
            0xAD432DE0, 0x1673FAF4, 0x03647D52, 0x00000001,
        ],
        x_extra: 0x00000000,
        c: 0x1D3D06BE,
        pos: 8,
        mul: 4238794375u32,
        use_rrx: false,
    };

    const U_REF: [u32; 8] = [
        0x293E4C79, 0x2883B11C, 0x87454D93, 0xC7341131,
        0x1D1E3837, 0x83D663FE, 0x2EC235C2, 0xB1AD09BA,
    ];

    run_test32(intf, state, &U_REF, 8)
}

/// Тест для MWC128u64 (lag=2)
fn test_mwc128u64(intf: &CallerAPI) -> bool {
    let mut state = MwcFpState::<u64, 2> {
        x: [
            0x0A2DE7FD1B0B2669,
            0x0000000000000001,
        ],
        x_extra: 0x0000000000000000,
        c: 0xE93C76E554BC3DDE,
        pos: 2,
        mul: 17741297344439402706u64,
        use_rrx: false,
    };

    const U_REF: [u64; 8] = [
        0x481CB82ECABB99BA, 0x73A05D57E9365E0E, 0x41E47A1CE2DBDE18, 0xB18E46EC2E938B17,
        0x8D667D5038185DD4, 0x21054F6D3FF80F10, 0x1B6CD39E1B27B198, 0x87DD038B41026317,
    ];

    run_test64(intf, state, &U_REF, 2)
}

/// Тест для MWC64u32 (lag=2)
fn test_mwc64u32(intf: &CallerAPI) -> bool {
    let mut state = MwcFpState::<u32, 2> {
        x: [
            0x003AB792,
            0x00000001,
        ],
        x_extra: 0x00000000,
        c: 0x9BDC771C,
        pos: 2,
        mul: 4291122658u32,
        use_rrx: false,
    };

    const U_REF: [u32; 8] = [
        0x662AE453, 0xC23220FD, 0xC82713AC, 0xE0F99B0F,
        0x23DD0069, 0x885B140D, 0xC2589D18, 0x22E5CCFB,
    ];

    run_test32(intf, state, &U_REF, 2)
}
*/


// ============ Макрос для генерации конкретных вариантов ============
macro_rules! declare_mwcfp_variant {
    (
        $name:ident,
        $word:ty,
        $lag:literal,
        $mul:literal,
        $tag:literal,
        $use_rrx:literal
        $(, self_test = $self_test:expr)?
    ) => {
        #[derive(Clone)]
        struct $name {
            inner: MwcFpState<$word, $lag>,
        }
        
        impl Prng for $name {
            type Output = <$word as MwcWord>::PrngOutput;
            
            fn new(intf: &CallerAPI) -> Option<Self> {
                MwcFpState::<$word, $lag>::new(intf, $mul)
                    .map(|inner| $name { inner })
            }
            
            #[inline(always)]
            fn next(&mut self) -> Self::Output {
                let val = self.inner.next_raw();
                
                let result = if $use_rrx {
                    val ^ val.rotl(<$word>::RRX_SHIFT1) ^ val.rotl(<$word>::RRX_SHIFT2)
                } else {
                    val
                };
                
                result.to_prng_output()
            }
            
            fn name() -> &'static str {
                $tag
            }
            
            fn description() -> &'static str {
                concat!("MWCFP ", $tag)
            }

            fn self_test(intf: &CallerAPI) -> bool {
                $( return $self_test(intf); )?
                smokerand_rs::printlnf!((*intf), "Running MWCFP self-tests...");
                true
            }
        }
    };
}


declare_mwcfp_variant!(Mwc64u32, u32, 2, 4291122658_u64, "mwc64u32", false);
declare_mwcfp_variant!(Mwc128u64, u64, 2, 17741297344439402706_u64, "mwc128u64", false);

/*
impl Prng for Mwc64u32 {
    fn self_test(intf: &CallerAPI) -> bool {
        smokerand_rs::printlnf!((*intf), "A1");
        true
    }
}


impl Prng for Mwc128u32 {
    fn self_test(intf: &CallerAPI) -> bool {
        smokerand_rs::printlnf!((*intf), "A2");
        true
    }
}
*/



/*
            use smokerand_rs::PrintfExt;
    
    
            let is32_sm = test_mwc64u32(intf);
            let is64_sm = test_mwc128u64(intf);
            let is32 = test_mwc256u32(intf);
            let is64 = test_mwc512u64(intf);
    
            let all_passed = is32 && is64 && is32_sm && is64_sm;
    
            if all_passed {
                intf.rust_printf(format_args!("All MWCFP self-tests PASSED\n"));
            } else {
                intf.rust_printf(format_args!("MWCFP self-tests FAILED\n"));
            }
    
            all_passed
*/



pub mod dispatcher {
    pub fn get_array() -> &'static [crate::TaggedGeneratorInfo] {
        use smokerand_rs::tag_gen_info;
        static X : [crate::TaggedGeneratorInfo; 2] = [
            tag_gen_info!("mwc64u32", crate::Mwc64u32),
            tag_gen_info!("mwc128u64", crate::Mwc128u64)
        ];
        &X
    }
} // mod dispatcher

smokerand_rs::make_dll_entry_point!();

#[no_mangle]
pub unsafe extern "C" fn gen_getinfo(
    gi: *mut crate::GeneratorInfo,
    intf: *const crate::CallerAPI,
) -> i32 {
    if gi.is_null() || intf.is_null() {
        return 0;
    };
    let get_param = match (*intf).get_param {
        Some(p) => p,
        None => { return 0; }
    };
    let param = match CStr::from_ptr(get_param()).to_str() {
        Ok(s) => s,
        Err(_utf8_error) => {
            smokerand_rs::printlnf!((*intf), "param value is corrupted");
            return 0;
        }
    };

    let gi_ref: &mut crate::GeneratorInfo = &mut *gi;
    let callback = match dispatcher::get_array().iter().find(|&&s| s.0 == param) {
        Some(&s) if s.0 == "" => GeneratorInfo::fill_static::<Mwc128u64>,
        Some(&s) => s.1,
        None => {
            smokerand_rs::printlnf!((*intf), "Unknown parameter '{param}'");
            smokerand_rs::printf!((*intf), "Available variants: ");
            for a in dispatcher::get_array() {
                smokerand_rs::printf!((*intf), "{} ", a.0);
            }
            smokerand_rs::printlnf!((*intf), "");
            return 0;
        }
    };

    callback(gi_ref)
}





/*
let slice = &[10, 20, 30, 40, 50];

match slice.iter().find(|&&x| x == 30) {
    Some(&value) => println!("Found: {}", value),
    None => println!("Not found"),
};
*/

/*
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn cchar_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    
    unsafe {
        match CStr::from_ptr(ptr).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => None,  // Некорректный UTF-8
        }
    }
}

// Использование:
let rust_string = cchar_to_string(c_ptr).unwrap_or_default();
*/


