//! Browser bindings over `unitarrow-core`.
//!
//! Spec §4: this crate wraps `unitarrow-core` **only** — arrow-rs stays out of
//! the WASM build, and arrow-js owns table handling on the JS side.
//!
//! The ABI is deliberately raw rather than wasm-bindgen. The entire surface is
//! string-in / string-out, so a hand-written ABI keeps the build to a single
//! `cargo build --target wasm32-unknown-unknown`, needs no post-processing tool,
//! and keeps the crate's dependency count at one. If the surface outgrows that,
//! wasm-bindgen is the right next step.
//!
//! ## Calling convention
//!
//! Strings cross as UTF-8 in linear memory. The caller allocates with
//! [`ua_alloc`], writes bytes, calls a function, and receives a packed
//! `(ptr << 32) | len` handle to a JSON reply, which it frees with [`ua_free`].
//!
//! `no_std` throughout: `unitarrow-core` needs only `alloc`, and std's I/O,
//! unwinding, and thread machinery are pure weight in a browser module that
//! roadmap M1 budgets at "tens of KB".

// `no_std` only for the wasm target: on the host this crate is built as an
// rlib for clippy and tests, where std's panic handler and allocator already
// exist and a second set are a hard conflict.
#![cfg_attr(target_arch = "wasm32", no_std)]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::slice;

use unitarrow_core::{canonicalize, conversion, Registry};

/// A bump allocator over a fixed arena.
///
/// wasm32-unknown-unknown ships no allocator, and a general-purpose one costs
/// more bytes than this module can spare. Every call here allocates a few short
/// strings and frees them immediately, and the module is re-instantiated per
/// page load, so never reclaiming is the right trade — with one caveat, made
/// loud below: exhausting the arena returns null, which Rust turns into an
/// allocation failure rather than silent corruption.
#[cfg(target_arch = "wasm32")]
mod bump {
    use core::alloc::{GlobalAlloc, Layout};
    use core::cell::UnsafeCell;

    const ARENA: usize = 8 * 1024 * 1024;

    pub struct Bump {
        arena: UnsafeCell<[u8; ARENA]>,
        next: UnsafeCell<usize>,
    }

    // Sound because wasm32-unknown-unknown is single-threaded.
    unsafe impl Sync for Bump {}

    impl Bump {
        pub const fn new() -> Bump {
            Bump {
                arena: UnsafeCell::new([0; ARENA]),
                next: UnsafeCell::new(0),
            }
        }
    }

    unsafe impl GlobalAlloc for Bump {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            unsafe {
                let next = &mut *self.next.get();
                let start = (*next + layout.align() - 1) & !(layout.align() - 1);
                match start.checked_add(layout.size()) {
                    Some(end) if end <= ARENA => {
                        *next = end;
                        (self.arena.get() as *mut u8).add(start)
                    }
                    _ => core::ptr::null_mut(),
                }
            }
        }
        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
    }
}

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: bump::Bump = bump::Bump::new();

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

/// Module state. Single-threaded by construction.
struct Slot(UnsafeCell<Option<Registry>>);
unsafe impl Sync for Slot {}
static REGISTRY: Slot = Slot(UnsafeCell::new(None));

fn registry() -> Option<&'static Registry> {
    unsafe { (*REGISTRY.0.get()).as_ref() }
}

fn set_registry(r: Registry) {
    unsafe { *REGISTRY.0.get() = Some(r) }
}

/// Allocate `len` bytes for the caller to write a UTF-8 string into.
///
/// # Safety
/// The buffer must be released with [`ua_free`] using the same `len`.
#[no_mangle]
pub unsafe extern "C" fn ua_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    core::mem::forget(buf);
    ptr
}

/// Release a buffer from [`ua_alloc`] or a call reply.
///
/// # Safety
/// `ptr` and `len` must come from this module.
#[no_mangle]
pub unsafe extern "C" fn ua_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(unsafe { Vec::from_raw_parts(ptr, len, len) });
    }
}

/// Pack a reply string into `(ptr << 32) | len`, leaking it for the caller.
fn reply(s: String) -> u64 {
    let bytes = s.into_bytes();
    let len = bytes.len() as u64;
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr() as u64;
    core::mem::forget(boxed);
    (ptr << 32) | len
}

/// # Safety
/// `ptr`/`len` must describe valid UTF-8 owned by this module.
unsafe fn as_str<'a>(ptr: *const u8, len: usize) -> &'a str {
    if ptr.is_null() || len == 0 {
        return "";
    }
    core::str::from_utf8(unsafe { slice::from_raw_parts(ptr, len) }).unwrap_or("")
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

const NO_REGISTRY: &str =
    r#"{"ok":false,"code":"E_REGISTRY_INVALID","message":"no registry loaded"}"#;

/// Load a registry from TOML source.
///
/// # Safety
/// See the module calling convention.
#[no_mangle]
pub unsafe extern "C" fn ua_load_registry(ptr: *const u8, len: usize) -> u64 {
    let src = unsafe { as_str(ptr, len) };
    match Registry::from_toml(src) {
        Ok(r) => {
            let json = format!(
                r#"{{"ok":true,"name":"{}","version":"{}","units":{},"aliases":{}}}"#,
                escape(&r.name),
                escape(&r.version),
                r.unit_count(),
                r.aliases().len()
            );
            set_registry(r);
            reply(json)
        }
        Err(e) => reply(format!(
            r#"{{"ok":false,"code":"{}","message":"{}"}}"#,
            e.code_str(),
            escape(e.message())
        )),
    }
}

/// Canonicalize a unit string against the loaded registry.
///
/// # Safety
/// See the module calling convention.
#[no_mangle]
pub unsafe extern "C" fn ua_canonicalize(ptr: *const u8, len: usize) -> u64 {
    let input = unsafe { as_str(ptr, len) };
    let Some(r) = registry() else {
        return reply(NO_REGISTRY.to_string());
    };
    {
        match canonicalize(input, r, 1) {
            Ok(u) => {
                let dims: Vec<String> = u
                    .dimension
                    .sparse()
                    .iter()
                    .map(|(n, e)| format!(r#""{n}":{e}"#))
                    .collect();
                let describes = match r.describe(&u.canonical) {
                    Some(n) => format!(r#""{}""#, escape(&n)),
                    None => "null".to_string(),
                };
                reply(format!(
                    r#"{{"ok":true,"canonical":"{}","dimension":{{{}}},"dimensionless":{},"describes":{}}}"#,
                    escape(&u.canonical),
                    dims.join(","),
                    u.is_dimensionless(),
                    describes
                ))
            }
            Err(e) => reply(format!(
                r#"{{"ok":false,"code":"{}","message":"{}","offset":{}}}"#,
                e.code_str(),
                escape(e.message()),
                e.offset()
                    .map(|o| o.to_string())
                    .unwrap_or_else(|| "null".into())
            )),
        }
    }
}

/// Compute the conversion between two unit strings, as exact rationals.
/// Input is `from`, a newline, then `to`.
///
/// # Safety
/// See the module calling convention.
#[no_mangle]
pub unsafe extern "C" fn ua_convert(ptr: *const u8, len: usize) -> u64 {
    let both = unsafe { as_str(ptr, len) };
    let Some(r) = registry() else {
        return reply(NO_REGISTRY.to_string());
    };
    {
        let mut parts = both.splitn(2, '\n');
        let (from, to) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        match conversion(from, to, r) {
            Ok(c) => reply(format!(
                r#"{{"ok":true,"from":"{}","to":"{}","num":"{}","den":"{}","pi":{},"offNum":"{}","offDen":"{}","formula":"{}"}}"#,
                escape(&c.from),
                escape(&c.to),
                c.factor.ratio().numerator(),
                c.factor.ratio().denominator(),
                c.factor.pi_exponent(),
                c.offset.numerator(),
                c.offset.denominator(),
                escape(&c.formula())
            )),
            Err(e) => reply(format!(
                r#"{{"ok":false,"code":"{}","message":"{}"}}"#,
                e.code_str(),
                escape(e.message())
            )),
        }
    }
}

/// Suggestions for an unresolved symbol, as a JSON array.
///
/// # Safety
/// See the module calling convention.
#[no_mangle]
pub unsafe extern "C" fn ua_suggest(ptr: *const u8, len: usize) -> u64 {
    let input = unsafe { as_str(ptr, len) };
    let Some(r) = registry() else {
        return reply("[]".to_string());
    };
    {
        let items: Vec<String> = r
            .suggest(input)
            .iter()
            .map(|s| format!(r#""{}""#, escape(s)))
            .collect();
        reply(format!("[{}]", items.join(",")))
    }
}
