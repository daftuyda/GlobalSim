//! C ABI for thread-confined prepared races. See docs/IN-PROCESS.md.
use crate::PreparedRace;

pub struct Handle {
    owner: std::thread::ThreadId,
    race: Option<PreparedRace>,
    metadata: String,
    packet: Vec<u8>,
    error: std::ffi::CString,
}

impl Handle {
    fn failure(&mut self, error: String) -> i32 {
        self.packet.clear();
        self.metadata.clear();
        self.error = std::ffi::CString::new(error.replace('\0', "\\0")).unwrap();
        -1
    }

    fn check_thread(&self) -> bool { self.owner == std::thread::current().id() }
}

#[no_mangle]
pub extern "C" fn globalsim_new() -> *mut Handle {
    Box::into_raw(Box::new(Handle { owner: std::thread::current().id(), race: None,
        metadata: String::new(), packet: Vec::new(), error: std::ffi::CString::default() }))
}

/// # Safety
/// Handle must be live, created on this thread, and not concurrently accessed.
/// Input must designate len readable bytes for this call.
#[no_mangle]
pub unsafe extern "C" fn globalsim_prepare(handle: *mut Handle, input: *const u8, len: usize) -> i32 {
    let Some(handle) = handle.as_mut() else { return -1; };
    if !handle.check_thread() { return -2; }
    handle.race = None;
    handle.packet.clear();
    handle.metadata.clear();
    if input.is_null() { return handle.failure("Null request buffer".into()); }
    let input = match std::str::from_utf8(std::slice::from_raw_parts(input, len)) {
        Ok(input) => input,
        Err(error) => return handle.failure(error.to_string()),
    };
    match PreparedRace::new(input) {
        Ok(race) => { handle.race = Some(race); handle.error = std::ffi::CString::default(); 0 }
        Err(error) => handle.failure(error),
    }
}

/// # Safety
/// Handle must be live, created on this thread, and exclusively accessed.
#[no_mangle]
pub unsafe extern "C" fn globalsim_run(handle: *mut Handle, seed: i64) -> i32 {
    let Some(handle) = handle.as_mut() else { return -1; };
    if !handle.check_thread() { return -2; }
    let Some(race) = handle.race.as_mut() else { return handle.failure("Prepare a race before running it".into()); };
    match race.run(seed) {
        Ok(simulation) => {
            simulation.into_binary(&mut handle.metadata, &mut handle.packet);
            handle.error = std::ffi::CString::default();
            0
        }
        Err(error) => handle.failure(error),
    }
}

/// # Safety
/// Handle must be live and exclusively accessed on its creating thread. The
/// returned view expires at the next prepare, run or free on this handle.
#[no_mangle]
pub unsafe extern "C" fn globalsim_data(handle: *const Handle) -> *const u8 {
    handle.as_ref().filter(|h| h.check_thread()).map_or(std::ptr::null(), |h| h.packet.as_ptr())
}

/// # Safety
/// Same handle requirements as globalsim_data.
#[no_mangle]
pub unsafe extern "C" fn globalsim_size(handle: *const Handle) -> usize {
    handle.as_ref().filter(|h| h.check_thread()).map_or(0, |h| h.packet.len())
}

/// # Safety
/// Same handle requirements as globalsim_data. The NUL-terminated error string
/// expires on the next prepare, run or free on this handle.
#[no_mangle]
pub unsafe extern "C" fn globalsim_error(handle: *const Handle) -> *const std::ffi::c_char {
    handle.as_ref().filter(|h| h.check_thread()).map_or(std::ptr::null(), |h| h.error.as_ptr())
}

/// # Safety
/// Handle must be live, created on this thread, exclusively owned and freed once.
#[no_mangle]
pub unsafe extern "C" fn globalsim_free(handle: *mut Handle) -> i32 {
    if handle.is_null() { return 0; }
    if !(*handle).check_thread() { return -2; }
    drop(Box::from_raw(handle));
    0
}
