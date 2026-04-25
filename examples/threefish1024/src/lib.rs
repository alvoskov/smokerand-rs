#![feature(portable_simd)]

use smokerand_rs::*;
use std::simd::*;

const TF1024_NWORDS: usize = 16;
const TF1024_NCOPIES: usize = 4;
const TF1024_NROUNDS: usize = 80;
const C240: u64 = 0x1BD11BDAA9FC1A22;

const ROT: [[u32; 8]; 8] = [
    [24, 13, 8, 47, 8, 17, 22, 37],
    [38, 19, 10, 55, 49, 18, 23, 52],
    [33, 4, 51, 13, 34, 41, 59, 17],
    [5, 20, 48, 41, 47, 28, 16, 25],
    [41, 9, 37, 31, 12, 47, 44, 30],
    [16, 34, 56, 51, 4, 53, 42, 41],
    [31, 44, 47, 46, 19, 42, 44, 25],
    [9, 48, 35, 52, 23, 31, 37, 20],
];

#[derive(Clone, Copy)]
struct Tf1024Element {
    u64: [u64; TF1024_NCOPIES],
}

struct Tf1024VecState {
    p: [Tf1024Element; TF1024_NWORDS],
    v: [Tf1024Element; TF1024_NWORDS],
    k: [u64; TF1024_NWORDS + 1],
    t: [u64; 3],
    pos: usize,
}

impl Tf1024VecState {
    fn key_schedule(&self, x: &mut [Simd<u64, TF1024_NCOPIES>; TF1024_NWORDS], s: usize) {
        for i in 0..TF1024_NWORDS {
            let ks = self.k[(s + i) % (TF1024_NWORDS + 1)];
            x[i] += Simd::splat(ks);
        }
        x[TF1024_NWORDS - 3] += Simd::splat(self.t[s % 3]);
        x[TF1024_NWORDS - 2] += Simd::splat(self.t[(s + 1) % 3]);
        x[TF1024_NWORDS - 1] += Simd::splat(s as u64);
    }

    #[inline(always)]
    fn rotl(x: Simd<u64, TF1024_NCOPIES>, n: u32) -> Simd<u64, TF1024_NCOPIES> {
        (x << Simd::splat(n as u64)) | (x >> Simd::splat(64 - n as u64))
    }

    #[inline(always)]
    fn mix_iter(
        x: &mut [Simd<u64, TF1024_NCOPIES>; TF1024_NWORDS],
        i0: usize, i1: usize, di: u32
    ) {
        x[i0] += x[i1];
        x[i1] = Self::rotl(x[i1], di) ^ x[i0];
    }

    #[inline(always)]
    fn mix_round(
        x: &mut [Simd<u64, TF1024_NCOPIES>; TF1024_NWORDS],
        i0: usize, i1: usize, i2: usize, i3: usize,
        i4: usize, i5: usize, i6: usize, i7: usize,
        i8: usize, i9: usize, i10: usize, i11: usize,
        i12: usize, i13: usize, i14: usize, i15: usize,
        rot_id: usize
    ) {
        let r = &ROT[rot_id];
        Self::mix_iter(x, i0, i1, r[0]);
        Self::mix_iter(x, i2, i3, r[1]);
        Self::mix_iter(x, i4, i5, r[2]);
        Self::mix_iter(x, i6, i7, r[3]);
        Self::mix_iter(x, i8, i9, r[4]);
        Self::mix_iter(x, i10, i11, r[5]);
        Self::mix_iter(x, i12, i13, r[6]);
        Self::mix_iter(x, i14, i15, r[7]);
    }

    #[inline(always)]
    fn mix_half0(x: &mut [Simd<u64, TF1024_NCOPIES>; TF1024_NWORDS]) {
        Self::mix_round(x, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15, 0);
        Self::mix_round(x, 0, 9, 2,13, 6,11, 4,15,10, 7,12, 3,14, 5, 8, 1, 1);
        Self::mix_round(x, 0, 7, 2, 5, 4, 3, 6, 1,12,15,14,13, 8,11,10, 9, 2);
        Self::mix_round(x, 0,15, 2,11, 6,13, 4, 9,14, 1, 8, 5,10, 3,12, 7, 3);
    }

    #[inline(always)]
    fn mix_half1(x: &mut [Simd<u64, TF1024_NCOPIES>; TF1024_NWORDS]) {
        Self::mix_round(x, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15, 4);
        Self::mix_round(x, 0, 9, 2,13, 6,11, 4,15,10, 7,12, 3,14, 5, 8, 1, 5);
        Self::mix_round(x, 0, 7, 2, 5, 4, 3, 6, 1,12,15,14,13, 8,11,10, 9, 6);
        Self::mix_round(x, 0,15, 2,11, 6,13, 4, 9,14, 1, 8, 5,10, 3,12, 7, 7);
    }

    #[inline(always)]
    fn rounds8(&self, x: &mut [Simd<u64, TF1024_NCOPIES>; TF1024_NWORDS], s: usize) {
        self.key_schedule(x, s);
        Self::mix_half0(x);
        self.key_schedule(x, s + 1);
        Self::mix_half1(x);
    }

    #[target_feature(enable = "avx2")]
    unsafe fn block_avx2(&mut self) {
        let mut x: [Simd<u64, TF1024_NCOPIES>; TF1024_NWORDS] = std::array::from_fn(|i| {
            Simd::from_array(self.p[i].u64)
        });
        
        self.rounds8(&mut x, 0);  self.rounds8(&mut x, 2);
        self.rounds8(&mut x, 4);  self.rounds8(&mut x, 6);
        self.rounds8(&mut x, 8);  self.rounds8(&mut x, 10);
        self.rounds8(&mut x, 12); self.rounds8(&mut x, 14);
        self.rounds8(&mut x, 16); self.rounds8(&mut x, 18);
        
        self.key_schedule(&mut x, TF1024_NROUNDS / 4);
        
        for i in 0..TF1024_NWORDS {
            self.v[i].u64 = x[i].to_array();
        }
    }

    fn block(&mut self) {
        unsafe{ self.block_avx2() }
    }


    fn init(key: &[u64; TF1024_NWORDS], tweak: &[u64; 2]) -> Self {
        let mut state = Self {
            k: [0; TF1024_NWORDS + 1],
            t: [0; 3],
            p: [Tf1024Element { u64: [0; TF1024_NCOPIES] }; TF1024_NWORDS],
            v: [Tf1024Element { u64: [0; TF1024_NCOPIES] }; TF1024_NWORDS],
            pos: 0,
        };
        
        state.k[TF1024_NWORDS] = C240;
        for i in 0..TF1024_NWORDS {
            state.k[i] = key[i];
            state.k[TF1024_NWORDS] ^= state.k[i];
        }
        
        state.t[0] = tweak[0];
        state.t[1] = tweak[1];
        state.t[2] = tweak[0] ^ tweak[1];
        
        for i in 0..TF1024_NCOPIES {
            state.p[0].u64[i] = i as u64;
        }
        
        state.block();
        state
    }

    fn inc_counter(&mut self) {
        for i in 0..TF1024_NCOPIES {
            self.p[0].u64[i] += TF1024_NCOPIES as u64;
        }
    }

    fn next_raw(&mut self) -> u64 {
        if self.pos >= TF1024_NWORDS * TF1024_NCOPIES {
            self.inc_counter();
            self.block();
            self.pos = 0;
        }
        let i = self.pos & 0xF;
        let j = self.pos >> 4;
        let x = self.v[i].u64[j];
        self.pos += 1;
        x
    }
}

impl Prng for Tf1024VecState {
    type Output = u64;

    fn new(intf: &CallerAPI) -> Option<Self> {
        let mut key = [0u64; TF1024_NWORDS];
        for i in 0..TF1024_NWORDS {
            key[i] = intf.seed64()?;
        }
        Some(Self::init(&key, &[0, 0]))
    }

    #[inline(always)]
    fn next(&mut self) -> u64 {
        self.next_raw()
    }

    fn name() -> &'static str {
        "ThreeFish1024"
    }

    fn description() -> &'static str {
        "Threefish-1024 vectorized PRNG (AVX2 equivalent via std::simd)"
    }

    fn self_test(intf: &CallerAPI) -> bool {
        use smokerand_rs::PrintfExt;
        
        const KEY_1: [u64; TF1024_NWORDS] = [0; TF1024_NWORDS];
        const REF_1: [u64; TF1024_NWORDS] = [
            0x04B3053D0A3D5CF0, 0x0136E0D1C7DD85F7, 0x067B212F6EA78A5C, 0x0DA9C10B4C54E1C6,
            0x0F4EC27394CBACF0, 0x32437F0568EA4FD5, 0xCFF56D1D7654B49C, 0xA2D5FB14369B2E7B,
            0x540306B460472E0B, 0x71C18254BCEA820D, 0xC36B4068BEAF32C8, 0xFA4329597A360095,
            0xC4A36C28434A5B9A, 0xD54331444B1046CF, 0xDF11834830B2A460, 0x1E39E8DFE1F7EE4F
        ];
        
        const KEY_2: [u64; 16] = [
            0x1716151413121110, 0x1F1E1D1C1B1A1918, 0x2726252423222120, 0x2F2E2D2C2B2A2928,
            0x3736353433323130, 0x3F3E3D3C3B3A3938, 0x4746454443424140, 0x4F4E4D4C4B4A4948,
            0x5756555453525150, 0x5F5E5D5C5B5A5958, 0x6766656463626160, 0x6F6E6D6C6B6A6968,
            0x7776757473727170, 0x7F7E7D7C7B7A7978, 0x8786858483828180, 0x8F8E8D8C8B8A8988
        ];
        
        const REF_2: [u64; 16] = [
            0xB0C33CD7DB4D65A6, 0xBC49A85A1077D75D, 0x6855FCAFEA7293E4, 0x1C5385AB1B7754D2,
            0x30E4AAFFE780F794, 0xE1BBEE708CAFD8D5, 0x9CA837B7423B0F76, 0xBD1403670D4963B3,
            0x451F2E3CE61EA48A, 0xB360832F9277D4FB, 0x0AAFC7A65E12D688, 0xC8906E79016D05D7,
            0xB316570A15F41333, 0x74E98A2869F5D50E, 0x57CE6F9247432BCE, 0xDE7CDD77215144DE
        ];
        
        intf.rust_printf(format_args!("Testing Threefish-1024 vectorized implementation...\n"));
        
        let state = Tf1024VecState::init(&KEY_1, &[0, 0]);
        for i in 0..TF1024_NWORDS {
            if state.v[i].u64[0] != REF_1[i] {
                intf.rust_printf(format_args!("Test 1 failed at word {}\n", i));
                return false;
            }
        }
        
        let mut state = Tf1024VecState::init(&KEY_2, &[0x0706050403020100, 0x0F0E0D0C0B0A0908]);
        state.p[0].u64 = [0xF8F9FAFBFCFDFEFF, 0xF8F9FAFBFCFDFEFF + 1, 
                          0xF8F9FAFBFCFDFEFF + 2, 0xF8F9FAFBFCFDFEFF + 3];
        state.p[1].u64 = [0xF0F1F2F3F4F5F6F7; 4];
        state.p[2].u64 = [0xE8E9EAEBECEDEEEF; 4];
        state.p[3].u64 = [0xE0E1E2E3E4E5E6E7; 4];
        state.p[4].u64 = [0xD8D9DADBDCDDDEDF; 4];
        state.p[5].u64 = [0xD0D1D2D3D4D5D6D7; 4];
        state.p[6].u64 = [0xC8C9CACBCCCDCECF; 4];
        state.p[7].u64 = [0xC0C1C2C3C4C5C6C7; 4];
        state.p[8].u64 = [0xB8B9BABBBCBDBEBF; 4];
        state.p[9].u64 = [0xB0B1B2B3B4B5B6B7; 4];
        state.p[10].u64 = [0xA8A9AAABACADAEAF; 4];
        state.p[11].u64 = [0xA0A1A2A3A4A5A6A7; 4];
        state.p[12].u64 = [0x98999A9B9C9D9E9F; 4];
        state.p[13].u64 = [0x9091929394959697; 4];
        state.p[14].u64 = [0x88898A8B8C8D8E8F; 4];
        state.p[15].u64 = [0x8081828384858687; 4];
        
        state.block();
        
        for i in 0..TF1024_NWORDS {
            if state.v[i].u64[0] != REF_2[i] {
                intf.rust_printf(format_args!("Test 2 failed at word {}\n", i));
                return false;
            }
        }
        
        intf.rust_printf(format_args!("All tests passed\n"));
        true
    }
}

impl_ffi_for_prng! {
    type = Tf1024VecState,
}
