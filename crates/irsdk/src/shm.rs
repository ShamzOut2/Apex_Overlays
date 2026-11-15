//! Live connection to the iRacing memory-mapped telemetry file (Windows only).

use crate::{parse_var_headers, Header, VarTable, HEADER_LEN, VAR_HEADER_LEN};
use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, VirtualQuery, FILE_MAP_READ, MEMORY_BASIC_INFORMATION, MEMORY_MAPPED_VIEW_ADDRESS,
};
use windows_sys::Win32::System::Threading::{OpenEventW, WaitForSingleObject};

/// `SYNCHRONIZE` standard access right (winnt.h).
const SYNCHRONIZE: u32 = 0x0010_0000;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub struct SharedMemory {
    mapping: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
    event: HANDLE,
    len: usize,
    vars: VarTable,
    vars_key: (i32, i32),
}

// The handles are process-wide kernel objects; moving them across threads is fine.
unsafe impl Send for SharedMemory {}

impl SharedMemory {
    /// Opens the mapping if the simulator is running. Returns `None` otherwise.
    pub fn open() -> Option<Self> {
        unsafe {
            let mapping = OpenFileMappingW(FILE_MAP_READ, 0, wide("Local\\IRSDKMemMapFileName").as_ptr());
            if mapping.is_null() {
                return None;
            }
            let view = MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 0);
            if view.Value.is_null() {
                CloseHandle(mapping);
                return None;
            }
            let mut info: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
            VirtualQuery(view.Value, &mut info, std::mem::size_of::<MEMORY_BASIC_INFORMATION>());
            let event = OpenEventW(SYNCHRONIZE, 0, wide("Local\\IRSDKDataValidEvent").as_ptr());
            Some(Self { mapping, view, event, len: info.RegionSize, vars: VarTable::default(), vars_key: (-1, -1) })
        }
    }

    fn bytes(&self) -> &[u8] {
        // SAFETY: the view stays mapped for the lifetime of `self` and `len` comes from VirtualQuery.
        // The simulator writes concurrently; callers copy what they need and re-validate tick counts.
        unsafe { std::slice::from_raw_parts(self.view.Value as *const u8, self.len) }
    }

    /// Blocks until the simulator signals a new telemetry row, or `timeout_ms` elapses.
    pub fn wait(&self, timeout_ms: u32) -> bool {
        if self.event.is_null() {
            std::thread::sleep(std::time::Duration::from_millis(timeout_ms.min(16) as u64));
            return true;
        }
        unsafe { WaitForSingleObject(self.event, timeout_ms) == WAIT_OBJECT_0 }
    }

    pub fn header(&self) -> Header {
        let mut raw = [0u8; HEADER_LEN];
        // SAFETY: len >= HEADER_LEN for any live mapping; volatile-ish copy of a small region.
        unsafe { ptr::copy_nonoverlapping(self.view.Value as *const u8, raw.as_mut_ptr(), HEADER_LEN.min(self.len)) };
        Header::parse(&raw).unwrap_or_default()
    }

    /// Variable table for the current connection (re-parsed only when the layout changes).
    pub fn vars(&mut self, h: &Header) -> &VarTable {
        let key = (h.num_vars, h.var_header_offset);
        if key != self.vars_key {
            let start = h.var_header_offset.max(0) as usize;
            let end = (start + h.num_vars.max(0) as usize * VAR_HEADER_LEN).min(self.len);
            self.vars = VarTable::new(parse_var_headers(&self.bytes()[start.min(end)..end], h.num_vars.max(0) as usize));
            self.vars_key = key;
        }
        &self.vars
    }

    /// Copies the newest consistent telemetry row into `out`. Returns its tick count.
    pub fn read_row(&self, out: &mut Vec<u8>) -> Option<i32> {
        for _ in 0..3 {
            let h = self.header();
            if !h.is_connected() {
                return None;
            }
            let i = h.latest_buf();
            let (tick, off, len) = (h.bufs[i].tick_count, h.bufs[i].offset.max(0) as usize, h.buf_len.max(0) as usize);
            if off + len > self.len {
                return None;
            }
            out.clear();
            out.extend_from_slice(&self.bytes()[off..off + len]);
            // If the sim rotated into this buffer while we copied, try again.
            if self.header().bufs[i].tick_count == tick {
                return Some(tick);
            }
        }
        None
    }

    /// Session info YAML string; call when `Header::session_info_update` changes.
    pub fn session_info(&self, h: &Header) -> String {
        let start = h.session_info_offset.max(0) as usize;
        let end = (start + h.session_info_len.max(0) as usize).min(self.len);
        crate::decode_session_info(&self.bytes()[start.min(end)..end])
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(self.view);
            CloseHandle(self.mapping);
            if !self.event.is_null() {
                CloseHandle(self.event);
            }
        }
    }
}

/// A read-only view of another program's named memory-mapped file (e.g. Le Mans Ultimate's
/// "LMU_Data"). The mapping is closed when this is dropped.
pub struct NamedMapping {
    mapping: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
    len: usize,
}

// The handles are process-wide kernel objects; moving them across threads is fine.
unsafe impl Send for NamedMapping {}

impl NamedMapping {
    /// Opens the mapping if it exists (the program publishing it is running). `None` otherwise.
    pub fn open(name: &str) -> Option<Self> {
        unsafe {
            let mapping = OpenFileMappingW(FILE_MAP_READ, 0, wide(name).as_ptr());
            if mapping.is_null() {
                return None;
            }
            let view = MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 0);
            if view.Value.is_null() {
                CloseHandle(mapping);
                return None;
            }
            let mut info: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
            VirtualQuery(view.Value, &mut info, std::mem::size_of::<MEMORY_BASIC_INFORMATION>());
            Some(Self { mapping, view, len: info.RegionSize })
        }
    }

    /// Size of the mapped view in bytes (at least the mapping's size, rounded up to a page).
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Copies a `T` from `offset`, if it lies inside the view. The other program writes
    /// concurrently, so callers check what they read for consistency.
    ///
    /// # Safety
    /// `T` must be plain old data (any bit pattern valid), such as `#[repr(C)]` structs of
    /// numbers and byte arrays.
    pub unsafe fn read<T: Copy>(&self, offset: usize) -> Option<T> {
        let end = offset.checked_add(std::mem::size_of::<T>())?;
        if end > self.len {
            return None;
        }
        Some(ptr::read_unaligned((self.view.Value as *const u8).add(offset) as *const T))
    }
}

impl Drop for NamedMapping {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(self.view);
            CloseHandle(self.mapping);
        }
    }
}
