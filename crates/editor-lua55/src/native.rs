use crate::protocol::{Conversion, Primitive, Profile, MAX_SOURCE};
use std::{ffi::CString, ptr::NonNull};
#[repr(C)]
#[derive(Default)]
struct Converted {
    flags: u32,
    integer: i64,
    number: f64,
    length: usize,
    coerced_integer: i64,
}
unsafe extern "C" {
    fn hye_lua55_release() -> u32;
    fn hye_lua55_runtime_matches_header() -> libc::c_int;
    fn hye_lua55_identity(output: *mut u8, capacity: usize) -> usize;
    fn hye_lua55_numeric_locale_c() -> libc::c_int;
    fn hye_lua55_seed_failed() -> libc::c_int;
    fn hye_lua55_compile(
        source: *const u8,
        length: usize,
        name: *const libc::c_char,
        error: *mut u8,
        capacity: usize,
        written: *mut usize,
    ) -> libc::c_int;
    fn hye_lua55_coercions_new() -> *mut libc::c_void;
    fn hye_lua55_coercions_close(pointer: *mut libc::c_void);
    fn hye_lua55_convert(
        pointer: *mut libc::c_void,
        kind: libc::c_int,
        integer: i64,
        number: f64,
        bytes: *const u8,
        length: usize,
        result: *mut Converted,
        text: *mut u8,
        capacity: usize,
    ) -> libc::c_int;
}
pub fn compile(source: &[u8], load_name: &[u8]) -> Result<(bool, Vec<u8>), String> {
    if source.len() > MAX_SOURCE || load_name.len() > 4096 {
        return Err("Compiler source/name exceeds its bounds".into());
    }
    let mut name = Vec::with_capacity(load_name.len() + 1);
    name.push(b'@');
    name.extend_from_slice(load_name);
    let name = CString::new(name).map_err(|_| "Compiler load name contains NUL")?;
    let mut diagnostic = vec![0u8; 4096];
    let mut length = 0;
    // SAFETY: bounded live buffers; adapter retains no pointers and compiles
    // text only. It never opens libraries or invokes the compiled chunk.
    let status = unsafe {
        hye_lua55_compile(
            source.as_ptr(),
            source.len(),
            name.as_ptr(),
            diagnostic.as_mut_ptr(),
            diagnostic.len(),
            &mut length,
        )
    };
    if length > diagnostic.len() {
        return Err("Compiler returned an invalid diagnostic length".into());
    }
    match status {
        0 | 1 => {}
        2 => {
            // SAFETY: read-only status of this thread's last state constructor.
            return Err(if unsafe { hye_lua55_seed_failed() } == 1 {
                "Private compiler seed is unavailable"
            } else {
                "Compiler allocation budget exceeded"
            }
            .into());
        }
        3 => return Err("Compiler diagnostic budget exceeded".into()),
        _ => return Err("Compiler returned an unknown status".into()),
    }
    diagnostic.truncate(length);
    Ok((status == 0, diagnostic))
}
pub fn profile() -> Result<Profile, String> {
    // SAFETY: read-only header/runtime identity queries with bounded output.
    let code = unsafe { hye_lua55_release() };
    if !matches!(code, 50500 | 50501) || unsafe { hye_lua55_runtime_matches_header() } != 1 {
        return Err("Lua 5.5 compiler release/runtime is not reviewed".into());
    }
    let release = format!("{}.{}.{}", code / 10000, code / 100 % 100, code % 100);
    let mut identity = [0u8; 512];
    let length = unsafe { hye_lua55_identity(identity.as_mut_ptr(), identity.len()) };
    if length == 0 || length > identity.len() {
        return Err("Lua compiler identity exceeds its bounds".into());
    }
    let identity =
        std::str::from_utf8(&identity[..length]).map_err(|_| "Invalid Lua compiler identity")?;
    if !identity.contains(&format!("Lua {release} "))
        || unsafe { hye_lua55_numeric_locale_c() } != 1
    {
        return Err("Lua compiler identity or numeric locale differs".into());
    }
    // Observe syntax capability, rather than inferring a library build flag
    // from a client header or accepting the whole language by release number.
    let declaration = compile(b"global x; x=1", b"profile")?.0;
    let identifier = compile(b"local global=1;return global", b"profile")?.0;
    Ok(Profile {
        release,
        library_identity: identity.into(),
        family: 505,
        numeric_locale: "C",
        private_seed: "kernel_random",
        global_declaration_compiles: declaration,
        global_identifier_compiles: identifier,
        imported_code_executed: false,
        complete_language_verified: false,
        selected_compositor_verified: false,
    })
}
struct Coercions(NonNull<libc::c_void>);
impl Drop for Coercions {
    fn drop(&mut self) {
        // SAFETY: uniquely owned matching native constructor; no cloned handle.
        unsafe { hye_lua55_coercions_close(self.0.as_ptr()) };
    }
}
pub fn convert(values: &[Primitive]) -> Result<Vec<Conversion>, String> {
    if values.len() > 256 {
        return Err("Primitive batch exceeds 256 values".into());
    }
    // SAFETY: fresh allocator-bounded state; no user functions/metatables enter it.
    let pointer = NonNull::new(unsafe { hye_lua55_coercions_new() });
    let Some(pointer) = pointer else {
        // SAFETY: read-only status of the just-failed state constructor.
        return Err(if unsafe { hye_lua55_seed_failed() } == 1 {
            "Private compiler seed is unavailable"
        } else {
            "Primitive allocation failed"
        }
        .into());
    };
    let state = Coercions(pointer);
    let mut text = vec![0u8; 65536];
    let mut results = Vec::with_capacity(values.len());
    let mut retained = 0usize;
    for value in values {
        let (kind, integer, number, bytes): (_, _, _, &[u8]) = match value {
            Primitive::Nil => (0, 0, 0.0, &[]),
            Primitive::Bool(v) => (1, i64::from(*v), 0.0, &[]),
            Primitive::Int(v) => (2, *v, 0.0, &[]),
            Primitive::FloatBits(bits) => (3, 0, f64::from_bits(*bits), &[]),
            Primitive::Bytes(v) => (4, 0, 0.0, v),
        };
        if bytes.len() > text.len() {
            return Err("Primitive text exceeds 64 KiB".into());
        }
        let mut out = Converted::default();
        // SAFETY: bounded live input/output buffers; allocating conversions
        // remain inside lua_pcall in the adapter and its stack is reset.
        let status = unsafe {
            hye_lua55_convert(
                state.0.as_ptr(),
                kind,
                integer,
                number,
                bytes.as_ptr(),
                bytes.len(),
                &mut out,
                text.as_mut_ptr(),
                text.len(),
            )
        };
        if status != 0 || out.length > text.len() {
            return Err("Primitive conversion exceeds its native bounds".into());
        }
        retained = retained.saturating_add(out.length + 64);
        if retained > 512 * 1024 {
            return Err("Primitive batch output exceeds 512 KiB".into());
        }
        results.push(Conversion {
            flags: out.flags,
            integer: out.integer,
            number_bits: out.number.to_bits(),
            coerced_integer: out.coerced_integer,
            text: text[..out.length].into(),
        });
    }
    Ok(results)
}
