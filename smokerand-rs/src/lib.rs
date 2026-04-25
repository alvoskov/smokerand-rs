use std::ffi::{c_char, CString};
use std::fmt;

mod bindings;
pub use bindings::*;

pub use std::ffi::c_int as CInt;

/// Trait for CallerAPI that adds an idiomatic output
pub trait PrintfExt {
    /// Print string using Rust formatting style
    fn rust_printf(&self, args: fmt::Arguments) -> i32;
    
    fn rust_println(&self, args: fmt::Arguments) -> i32;
}


impl PrintfExt for CallerAPI {
    fn rust_printf(&self, args: fmt::Arguments) -> i32 {
        let printf = match self.printf {
            Some(f) => f,
            None => return -1,
        };

        let formatted = args.to_string();
    
        let format_cstr = match CString::new("%s") {
            Ok(s) => s,
            Err(_) => return -1,
        };
    
        let arg_cstr = match CString::new(formatted) {
            Ok(s) => s,
            Err(_) => return -1,
        };
    
        unsafe { printf(format_cstr.as_ptr(), arg_cstr.as_ptr()) }
    }

    fn rust_println(&self, args: fmt::Arguments) -> i32 {
        let result = self.rust_printf(args);
        if result >= 0 {
            let printf = match self.printf {
                Some(f) => f,
                None => return -1,
            };
            unsafe { printf(b"\n\0".as_ptr() as *const c_char) }
        } else {
            result
        }
    }
}

/// Sealed trait for a limited set of output types
pub trait PrngOutput: Copy + Into<u64> + From<u32> + sealed::Sealed {}

impl PrngOutput for u32 {}
impl PrngOutput for u64 {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
}

/// PRNG interface
pub trait Prng: Sized /*+ 'static*/ {
    type Output: PrngOutput;  /// u32 or u64 only!

    /// Создать новый экземпляр генератора
    /// 
    /// # Аргументы
    /// * `intf` - Интерфейс CallerAPI, предоставляемый фреймворком SmokeRand
    /// 
    /// # Возвращаемое значение
    /// `Some(Self)` при успешном создании, `None` при ошибке
    fn new(intf: &CallerAPI) -> Option<Self>;
    
    /// Generate the next pseudorandom number
    fn next(&mut self) -> Self::Output;
    
    /// Имя генератора (для информационных целей, не используется в FFI)
    fn name() -> &'static str;
    
    /// Описание генератора (для информационных целей, не используется в FFI)
    fn description() -> &'static str { "" }
    
    /// Количество бит в выходе генератора (32 или 64)
    fn bits() -> u32 {
        std::mem::size_of::<Self::Output>() as u32 * 8
    }
    
    /// Самотестирование генератора
    /// 
    /// # Возвращаемое значение
    /// `true` если тест пройден, `false` если нет
    fn self_test(intf: &CallerAPI) -> bool { 
        let _ = intf;
        true 
    }
}


pub mod cwrap {

pub unsafe extern "C" fn get_bits<G: super::Prng>(state: *mut std::ffi::c_void) -> u64 {
    let state = &mut *(state as *mut G);
    <G as super::Prng>::next(state).into()
}


pub unsafe extern "C" fn get_sum<G: super::Prng>(state: *mut std::ffi::c_void, len: usize) -> u64 {
    let state = &mut *(state as *mut G);
    let mut sum = 0u64;
    for _ in 0..len {
        let val = <G as super::Prng>::next(state);
        sum = sum.wrapping_add(val.into());
    }
    sum
}

pub unsafe extern "C" fn create<G: super::Prng>(
    gi: *const super::GeneratorInfo,
    intf: *const super::CallerAPI
) -> *mut std::ffi::c_void {
    let _ = gi;
    if intf.is_null() { 
        return std::ptr::null_mut(); 
    }
    let intf = &*intf;

    // Create the state    
    let state = match <G as super::Prng>::new(intf) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    // We use custom allocator (malloc) from SmokeRand!
    let malloc = match intf.malloc {
        Some(f) => f,
        None => return std::ptr::null_mut(),
    };

    let ptr = malloc(std::mem::size_of::<G>()) as *mut G;
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    // Write the state to the allocated memory    
    ptr.write(state);    
    ptr as *mut std::ffi::c_void
}


pub unsafe extern "C" fn free<G: super::Prng>(
    state: *mut std::ffi::c_void,
    gi: *const super::GeneratorInfo,
    intf: *const super::CallerAPI,
        ) {
    if state.is_null() { 
        return; 
    }
    let _ = gi;
    let intf = &*intf;
    // Manual call of the destructor
    std::ptr::drop_in_place(state as *mut G);
    // Use custom free from SmokeRand!
    if let Some(free) = intf.free {
        free(state as *mut std::ffi::c_void);
    }
}


pub unsafe extern "C" fn self_test<G: super::Prng>(intf: *const super::CallerAPI) -> super::CInt {
    if intf.is_null() {
        return 0;
    }
    let intf = &*intf;
    if <G as super::Prng>::self_test(intf) {
        1 
    } else { 
        0 
    }
}

} // mod cwrap


impl GeneratorInfo {
    pub fn fill_static<G: Prng>(
        gi: &mut GeneratorInfo,
    ) -> i32 {
        // Leaking memory for static strings (acceptable for DLL)
        let name_str = Box::leak(format!("{}\0", G::name()).into_boxed_str());
        let desc_str = Box::leak(format!("{}\0", G::description()).into_boxed_str());

        gi.name = name_str.as_ptr() as *const i8;
        gi.description = desc_str.as_ptr() as *const i8;    
        gi.nbits = G::bits();
        gi.create = Some(cwrap::create::<G>);
        gi.free = Some(cwrap::free::<G>);
        gi.get_bits = Some(cwrap::get_bits::<G>);
        gi.self_test = Some(cwrap::self_test::<G>);
        gi.get_sum = Some(cwrap::get_sum::<G>);
        gi.parent = std::ptr::null();
    
        1
    }

    pub fn fill<G: Prng>(
        &mut self,
    ) -> i32 {
        Self::fill_static::<G>(self)
    }
}

pub type FillGeneratorInfoFn = fn(gi: &mut GeneratorInfo) -> i32;

pub type TaggedGeneratorInfo = (&'static str, crate::FillGeneratorInfoFn);

#[macro_export]
macro_rules! tag_gen_info {
    ($name:literal, $prng_type:ty) => {
        ($name, $crate::GeneratorInfo::fill_static::<$prng_type>)
    }
}



/// Макросы для удобного использования
#[macro_export]
macro_rules! printf {
    ($intf:expr, $($arg:tt)*) => {
        $intf.rust_printf(std::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! printlnf {
    ($intf:expr) => {
        $intf.rust_println(std::format_args!(""))
    };
    ($intf:expr, $($arg:tt)*) => {
        $intf.rust_println(std::format_args!($($arg)*))
    };
}


/// Windows DLL entry point
#[macro_export]
macro_rules! make_dll_entry_point {
    () => {
        #[cfg(all(windows, target_arch = "x86_64"))]
        #[no_mangle]
        pub unsafe extern "system" fn DllMainCRTStartup(
            _hinst: *mut std::ffi::c_void,
            _reason: u32,
            _reserved: *mut std::ffi::c_void,
        ) -> i32 {
            1
        }
    };
}

/// This macro generates all FFI functions requred by SmokeRand framework
#[macro_export]
macro_rules! impl_ffi_for_prng {
    (
        type = $prng_type:ty,
        $(,)?
    ) => {
        $crate::make_dll_entry_point!();

        /// Get information about generators
        #[no_mangle]
        pub unsafe extern "C" fn gen_getinfo(
            gi: *mut $crate::GeneratorInfo,
            intf: *const $crate::CallerAPI,
        ) -> i32 {
            if gi.is_null() {
                return 0;
            }
            let _ = intf;
            let gi_ref: &mut $crate::GeneratorInfo = &mut *gi;
            gi_ref.fill::<$prng_type>()
        }

        #[no_mangle]
        #[export_name = "AddNumbers"]
        pub unsafe extern "C" fn get_bits_exported(state: *mut std::ffi::c_void) -> u64 {
            return $crate::cwrap::get_bits::<$prng_type>(state);
        }
    };
}

/*
#[macro_export]
macro_rules! impl_ffi_for_prng_dispatcher {
    (
        type = $prng_type:ty,
    )
:ident
*/