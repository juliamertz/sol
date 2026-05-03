use std::ffi::{CStr, CString, c_char, c_void};
use std::io::Write;

use libc::size_t;

#[unsafe(no_mangle)]
pub extern "C" fn alloc(size: size_t) -> *mut c_void {
    unsafe { libc::malloc(size) }
}

#[unsafe(no_mangle)]
pub extern "C" fn dealloc(ptr: *mut c_void) {
    unsafe { libc::free(ptr) }
}

// FIXME: for some reason if i include this function the program segfaults
// #[unsafe(no_mangle)]
// pub extern "C" fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void {
//     unsafe { libc::realloc(ptr, size) }
// }

#[unsafe(no_mangle)]
pub extern "C" fn format_u32(val: u32) -> *mut c_char {
    let formatted = val.to_string();
    // Safety: pretty sure `u32::to_string` won't include any 0 bytes
    let c_string = unsafe { CString::new(formatted).unwrap_unchecked() };
    c_string.into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn println(ptr: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(ptr) };
    let str = c_str.to_string_lossy();

    let mut stdout = std::io::stdout();
    stdout.write_all(str.as_bytes()).unwrap();
    stdout.write_all(&[b'\n']).unwrap();
    stdout.flush().unwrap();
}
