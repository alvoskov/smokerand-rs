//! Библиотека-помощник для создания плагинов SmokeRand на Rust
//! 
//! Предоставляет трейт `Prng` и макрос `impl_ffi_for_prng!` для
//! автоматической генерации FFI-функций, требуемых фреймворком SmokeRand.

use std::ffi::{c_char, CString};
use std::fmt;

mod bindings;
pub use bindings::*;

pub use std::ffi::c_int as CInt;

/// Трейт-расширение для CallerAPI, добавляющий идиоматичный вывод
pub trait PrintfExt {
    /// Печать строки с форматированием в стиле Rust
    fn rust_printf(&self, args: fmt::Arguments) -> i32;
    
    /// Печать строки с переводом строки
    fn rust_println(&self, args: fmt::Arguments) -> i32;
}


impl PrintfExt for CallerAPI {
    fn rust_printf(&self, args: fmt::Arguments) -> i32 {
        let printf = match self.printf {
            Some(f) => f,
            None => return -1,
        };

        // Преобразуем fmt::Arguments в String
        let formatted = args.to_string();
    
        // Без ? оператора, используем match
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

/// Запечатанный трейт для допустимых выходных типов
pub trait PrngOutput: Copy + Into<u64> + From<u32> + sealed::Sealed {}

impl PrngOutput for u32 {}
impl PrngOutput for u64 {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
}

/// Трейт, который должен реализовать каждый генератор псевдослучайных чисел
pub trait Prng: Sized + 'static {
    type Output: PrngOutput;  // Только u32 или u64!

    /// Создать новый экземпляр генератора
    /// 
    /// # Аргументы
    /// * `intf` - Интерфейс CallerAPI, предоставляемый фреймворком SmokeRand
    /// 
    /// # Возвращаемое значение
    /// `Some(Self)` при успешном создании, `None` при ошибке
    fn new(intf: &CallerAPI) -> Option<Self>;
    
    /// Сгенерировать следующее 64-битное псевдослучайное число
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


/// Макрос для генерации всех FFI-функций, требуемых фреймворком SmokeRand
/// 
/// # Пример использования
/// ```ignore
/// use smokerand_rs::*;
/// 
/// #[repr(C)]
/// struct MyPrngState {
///     state: u64,
/// }
/// 
/// impl Prng for MyPrngState {
///     fn new(intf: &CallerAPI) -> Option<Self> {
///         let get_seed64 = intf.get_seed64?;
///         let seed = unsafe { get_seed64() };
///         Some(MyPrngState { state: seed })
///     }
///     
///     fn next(&mut self) -> u64 {
///         // Ваш алгоритм генерации
///         self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
///         self.state
///     }
///     
///     fn name() -> &'static str { "MyPRNG" }
/// }
/// 
/// impl_ffi_for_prng! {
///     type = MyPrngState,
///     name = "MyPRNG (Rust)",
///     description = "My custom PRNG implemented in Rust",
/// }
/// ```
/// Макрос для генерации всех FFI-функций, требуемых фреймворком SmokeRand
#[macro_export]
macro_rules! impl_ffi_for_prng {
    (
        type = $prng_type:ty,
        name = $name:literal,
        description = $desc:literal
        $(, bits = $bits:literal)?
        $(,)?
    ) => {
        // ========== Точка входа DLL для Windows ==========
        #[cfg(all(windows, target_arch = "x86_64"))]
        #[no_mangle]
        pub unsafe extern "system" fn DllMainCRTStartup(
            _hinst: *mut std::ffi::c_void,
            _reason: u32,
            _reserved: *mut std::ffi::c_void,
        ) -> i32 {
            1
        }

        // ========== Создание и уничтожение генератора ==========
        
        #[no_mangle]
        pub unsafe extern "C" fn create(
                gi: *const $crate::GeneratorInfo,
                intf: *const $crate::CallerAPI
            ) -> *mut std::ffi::c_void {
            let _ = gi;
            if intf.is_null() { 
                return std::ptr::null_mut(); 
            }
            let intf = &*intf;
            
            <$prng_type as $crate::Prng>::new(intf)
                .map(|state| Box::into_raw(Box::new(state)) as *mut _)
                .unwrap_or(std::ptr::null_mut())
        }

        #[no_mangle]
        pub unsafe extern "C" fn free(
            state: *mut std::ffi::c_void,
            gi: *const $crate::GeneratorInfo,
            _intf: *const $crate::CallerAPI,
        ) {
            if state.is_null() { 
                return; 
            }
            let _ = gi;
            let _ = Box::from_raw(state as *mut $prng_type);
        }

        // ========== Генерация случайных чисел ==========
        
        #[no_mangle]
        pub unsafe extern "C" fn get_bits(state: *mut std::ffi::c_void) -> u64 {
            let state = &mut *(state as *mut $prng_type);
            <$prng_type as $crate::Prng>::next(state).into()
        }

        #[no_mangle]
        pub unsafe extern "C" fn get_sum(state: *mut std::ffi::c_void, len: usize) -> u64 {
            let state = &mut *(state as *mut $prng_type);
            let mut sum = 0u64;
            for _ in 0..len {
                let val = <$prng_type as $crate::Prng>::next(state);
                sum = sum.wrapping_add(val.into());
            }
            sum
        }

        // ========== Самотестирование ==========
        
        #[no_mangle]
        pub unsafe extern "C" fn self_test(intf: *const $crate::CallerAPI) -> CInt {
            if intf.is_null() { 
                return 0; 
            }
            let intf = &*intf;
            if <$prng_type as $crate::Prng>::self_test(intf) { 
                1 
            } else { 
                0 
            }
        }

        // ========== Информация о генераторе ==========
        
        #[no_mangle]
        pub unsafe extern "C" fn gen_getinfo(
            gi: *mut $crate::GeneratorInfo,
            _intf: *const $crate::CallerAPI,
        ) -> i32 {
            if gi.is_null() { 
                return 0; 
            }
            
            // Утекаем память для статических строк (приемлемо для DLL)
            let name_str = Box::leak(format!("{}\0", $name).into_boxed_str());
            let desc_str = Box::leak(format!("{}\0", $desc).into_boxed_str());
            
            (*gi).name = name_str.as_ptr() as *const i8;
            (*gi).description = desc_str.as_ptr() as *const i8;
            
            // Используем bits() из трейта, если не указано явно
            let bits: u32 = <$prng_type as $crate::Prng>::bits();
            $( let bits: u32 = $bits; )?
            (*gi).nbits = bits;
            
            (*gi).create = Some(create);
            (*gi).free = Some(free);
            (*gi).get_bits = Some(get_bits);
            (*gi).self_test = Some(self_test);
            (*gi).get_sum = Some(get_sum);
            (*gi).parent = std::ptr::null();
            
            1
        }
    };
}

/// Вспомогательный макрос для конкатенации строковых литералов
/// Используется для добавления нуль-терминатора к статическим строкам
#[doc(hidden)]
#[macro_export]
macro_rules! const_concat {
    ($a:literal, $b:literal) => {
        const {
            // Константное вычисление конкатенации
            // В будущем можно заменить на const_concat! из std
            let a_bytes = $a.as_bytes();
            let b_bytes = $b.as_bytes();
            let mut result = [0u8; a_bytes.len() + b_bytes.len()];
            let mut i = 0;
            while i < a_bytes.len() {
                result[i] = a_bytes[i];
                i += 1;
            }
            while i < result.len() {
                result[i] = b_bytes[i - a_bytes.len()];
                i += 1;
            }
            // Конвертируем в строку на этапе компиляции
            unsafe { std::str::from_utf8_unchecked(&result) }
        }
    };
}

// Альтернатива: простой конкатенатор через макрос
// Используем встроенный concat! для простых случаев
#[doc(hidden)]
#[macro_export]
macro_rules! static_cstr {
    ($s:literal) => {
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($s, "\0").as_bytes()) }
    };
}

/// Вспомогательная функция для безопасного создания C-строки с нуль-терминатором
pub fn to_c_string(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s).expect("String contains null byte")
}

/// Вспомогательная функция для выделения памяти через CallerAPI
/// 
/// # Безопасность
/// Вызывающий код должен гарантировать, что intf указывает на валидный CallerAPI
/*
pub unsafe fn alloc_via_api<T>(intf: &CallerAPI) -> Option<Box<T>> {
    let malloc = intf.malloc?;
    let ptr = malloc(std::mem::size_of::<T>()) as *mut T;
    if ptr.is_null() {
        None
    } else {
        Some(Box::from_raw(ptr))
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_const_concat() {
        let s = const_concat!("Hello", " World");
        assert_eq!(s, "Hello World");
    }
}
