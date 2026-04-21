// C-совместимые определения из apidefs.h и coredefs.h
use std::ffi::{c_char, c_int, c_uint, c_void};

/// Информация о RAM
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RamInfo {
    pub total_ram: i64,
    pub available_ram: i64,
}

/// CallerAPI - интерфейс к функциям фреймворка
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CallerAPI {
    pub get_seed32: Option<unsafe extern "C" fn() -> u32>,
    pub get_seed64: Option<unsafe extern "C" fn() -> u64>,
    pub get_param: Option<unsafe extern "C" fn() -> *const c_char>,
    pub malloc: Option<unsafe extern "C" fn(size: usize) -> *mut c_void>,
    pub free: Option<unsafe extern "C" fn(ptr: *mut c_void)>,
    pub printf: Option<unsafe extern "C" fn(format: *const c_char, ...) -> c_int>,
    pub snprintf: Option<unsafe extern "C" fn(buf: *mut c_char, size: usize, format: *const c_char, ...) -> c_int>,
    pub strcmp: Option<unsafe extern "C" fn(lhs: *const c_char, rhs: *const c_char) -> c_int>,
    pub get_ram_info: Option<unsafe extern "C" fn(info: *mut RamInfo) -> c_int>,
}

// Расширяем CallerAPI безопасными методами
impl CallerAPI {
    /// Безопасное получение 128-битного seed
    pub fn seed128(&self) -> Option<u128> {
        let get_seed = self.get_seed64?;
        let seed_hi = unsafe { get_seed() as u128 };
        let seed_lo = unsafe { get_seed() as u128 };
        Some((seed_hi << 64) | seed_lo)
    }

    /// Безопасное получение 64-битного seed
    pub fn seed64(&self) -> Option<u64> {
        let get_seed = self.get_seed64?;
        Some(unsafe { get_seed() })
    }
    
    /// Безопасное получение 32-битного seed
    pub fn seed32(&self) -> Option<u32> {
        let get_seed = self.get_seed32?;
        Some(unsafe { get_seed() })
    }
    
    /// Безопасное выделение памяти
    pub fn allocate<T>(&self) -> Option<*mut T> {
        let malloc = self.malloc?;
        let ptr = unsafe { malloc(std::mem::size_of::<T>()) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr as *mut T)
        }
    }
    
    /// Безопасное освобождение памяти
    pub fn deallocate(&self, ptr: *mut core::ffi::c_void) {
        if let Some(free) = self.free {
            unsafe { free(ptr) };
        }
    }
}

/// GeneratorInfo - информация о генераторе
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GeneratorInfo {
    pub name: *const c_char,
    pub description: *const c_char,
    pub nbits: c_uint,
    pub create: Option<unsafe extern "C" fn(gi: *const GeneratorInfo, intf: *const CallerAPI) -> *mut c_void>,
    pub free: Option<unsafe extern "C" fn(state: *mut c_void, gi: *const GeneratorInfo, intf: *const CallerAPI)>,
    pub get_bits: Option<unsafe extern "C" fn(state: *mut c_void) -> u64>,
    pub self_test: Option<unsafe extern "C" fn(intf: *const CallerAPI) -> c_int>,
    pub get_sum: Option<unsafe extern "C" fn(state: *mut c_void, len: usize) -> u64>,
    pub parent: *const GeneratorInfo,
}
