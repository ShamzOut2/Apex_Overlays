//! Minimal reader for the iRacing SDK telemetry format.
//!
//! The same binary layout is used by the live memory-mapped file
//! (`Local\IRSDKMemMapFileName`) and by `.ibt` telemetry files, so all the
//! parsing here works on plain byte slices. The Windows-only [`SharedMemory`]
//! type wraps the live mapping and copies out consistent snapshots.

pub mod yaml;

#[cfg(windows)]
mod shm;
#[cfg(windows)]
pub use shm::{NamedMapping, SharedMemory};
#[cfg(windows)]
mod broadcast;
#[cfg(windows)]
pub use broadcast::{broadcast, BroadcastMsg};

use std::collections::HashMap;

/// `irsdk_header` is 12 int32 fields followed by 4 `irsdk_varBuf` (16 bytes each).
pub const HEADER_LEN: usize = 48 + MAX_BUFS * 16;
pub const VAR_HEADER_LEN: usize = 144;
pub const MAX_BUFS: usize = 4;
pub const STATUS_CONNECTED: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarType {
    Char = 0,
    Bool = 1,
    Int = 2,
    BitField = 3,
    Float = 4,
    Double = 5,
}

impl VarType {
    pub fn from_i32(v: i32) -> Option<Self> {
        Some(match v {
            0 => Self::Char,
            1 => Self::Bool,
            2 => Self::Int,
            3 => Self::BitField,
            4 => Self::Float,
            5 => Self::Double,
            _ => return None,
        })
    }

    pub fn size(self) -> usize {
        match self {
            Self::Char | Self::Bool => 1,
            Self::Int | Self::BitField | Self::Float => 4,
            Self::Double => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VarBuf {
    pub tick_count: i32,
    pub offset: i32,
}

#[derive(Debug, Clone, Default)]
pub struct Header {
    pub ver: i32,
    pub status: i32,
    pub tick_rate: i32,
    pub session_info_update: i32,
    pub session_info_len: i32,
    pub session_info_offset: i32,
    pub num_vars: i32,
    pub var_header_offset: i32,
    pub num_buf: i32,
    pub buf_len: i32,
    pub bufs: [VarBuf; MAX_BUFS],
}

fn rd_i32(b: &[u8], off: usize) -> i32 {
    b.get(off..off + 4).map(|s| i32::from_le_bytes([s[0], s[1], s[2], s[3]])).unwrap_or(0)
}

impl Header {
    pub fn parse(b: &[u8]) -> Option<Self> {
        if b.len() < HEADER_LEN {
            return None;
        }
        let mut h = Header {
            ver: rd_i32(b, 0),
            status: rd_i32(b, 4),
            tick_rate: rd_i32(b, 8),
            session_info_update: rd_i32(b, 12),
            session_info_len: rd_i32(b, 16),
            session_info_offset: rd_i32(b, 20),
            num_vars: rd_i32(b, 24),
            var_header_offset: rd_i32(b, 28),
            num_buf: rd_i32(b, 32),
            buf_len: rd_i32(b, 36),
            bufs: Default::default(),
        };
        for (i, vb) in h.bufs.iter_mut().enumerate() {
            let base = 48 + i * 16;
            vb.tick_count = rd_i32(b, base);
            vb.offset = rd_i32(b, base + 4);
        }
        Some(h)
    }

    pub fn is_connected(&self) -> bool {
        self.status & STATUS_CONNECTED != 0
    }

    /// Index of the most recently written telemetry buffer.
    pub fn latest_buf(&self) -> usize {
        let n = (self.num_buf.clamp(1, MAX_BUFS as i32)) as usize;
        (0..n).max_by_key(|&i| self.bufs[i].tick_count).unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub struct VarHeader {
    pub ty: VarType,
    pub offset: usize,
    pub count: usize,
    pub count_as_time: bool,
    pub name: String,
    pub desc: String,
    pub unit: String,
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    // iRacing strings are Latin-1; every byte maps directly to a code point.
    b[..end].iter().map(|&c| c as char).collect()
}

/// Parse `n` variable headers from a byte slice that starts at the var header table.
pub fn parse_var_headers(b: &[u8], n: usize) -> Vec<VarHeader> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let base = i * VAR_HEADER_LEN;
        let Some(s) = b.get(base..base + VAR_HEADER_LEN) else { break };
        let Some(ty) = VarType::from_i32(rd_i32(s, 0)) else { continue };
        out.push(VarHeader {
            ty,
            offset: rd_i32(s, 4).max(0) as usize,
            count: rd_i32(s, 8).max(1) as usize,
            count_as_time: s[12] != 0,
            name: cstr(&s[16..48]),
            desc: cstr(&s[48..112]),
            unit: cstr(&s[112..144]),
        });
    }
    out
}

/// A resolved telemetry variable. Cheap to copy; resolve once per connection.
#[derive(Debug, Clone, Copy)]
pub struct Var {
    pub ty: VarType,
    pub offset: usize,
    pub count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct VarTable {
    pub headers: Vec<VarHeader>,
    by_name: HashMap<String, usize>,
}

impl VarTable {
    pub fn new(headers: Vec<VarHeader>) -> Self {
        let by_name = headers.iter().enumerate().map(|(i, h)| (h.name.clone(), i)).collect();
        Self { headers, by_name }
    }

    pub fn get(&self, name: &str) -> Option<Var> {
        self.by_name.get(name).map(|&i| {
            let h = &self.headers[i];
            Var { ty: h.ty, offset: h.offset, count: h.count }
        })
    }
}

/// Typed accessors over one telemetry row (a copy of a var buffer).
#[derive(Clone, Copy)]
pub struct Row<'a>(pub &'a [u8]);

impl<'a> Row<'a> {
    fn at(&self, v: Var, idx: usize) -> Option<&'a [u8]> {
        if idx >= v.count {
            return None;
        }
        let sz = v.ty.size();
        let off = v.offset + idx * sz;
        self.0.get(off..off + sz)
    }

    /// Read any numeric/bool variable as f64.
    pub fn f64_at(&self, v: Option<Var>, idx: usize) -> Option<f64> {
        let v = v?;
        let s = self.at(v, idx)?;
        Some(match v.ty {
            VarType::Char | VarType::Bool => s[0] as f64,
            VarType::Int => i32::from_le_bytes(s.try_into().ok()?) as f64,
            VarType::BitField => u32::from_le_bytes(s.try_into().ok()?) as f64,
            VarType::Float => f32::from_le_bytes(s.try_into().ok()?) as f64,
            VarType::Double => f64::from_le_bytes(s.try_into().ok()?),
        })
    }

    pub fn f64(&self, v: Option<Var>) -> f64 {
        self.f64_at(v, 0).unwrap_or(0.0)
    }
    pub fn f32(&self, v: Option<Var>) -> f32 {
        self.f64(v) as f32
    }
    pub fn i32(&self, v: Option<Var>) -> i32 {
        self.i32_at(v, 0)
    }
    pub fn i32_at(&self, v: Option<Var>, idx: usize) -> i32 {
        let Some(var) = v else { return 0 };
        match (var.ty, self.at(var, idx)) {
            (VarType::Int | VarType::BitField, Some(s)) => i32::from_le_bytes([s[0], s[1], s[2], s[3]]),
            _ => self.f64_at(v, idx).unwrap_or(0.0) as i32,
        }
    }
    pub fn u32(&self, v: Option<Var>) -> u32 {
        self.i32(v) as u32
    }
    pub fn bool(&self, v: Option<Var>) -> bool {
        self.f64(v) != 0.0
    }
    pub fn bool_at(&self, v: Option<Var>, idx: usize) -> bool {
        self.f64_at(v, idx).unwrap_or(0.0) != 0.0
    }
    pub fn f32_at(&self, v: Option<Var>, idx: usize) -> f32 {
        self.f64_at(v, idx).unwrap_or(0.0) as f32
    }
}

/// Decode the Latin-1, NUL-terminated session info string.
pub fn decode_session_info(b: &[u8]) -> String {
    cstr(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a fake memory map with the given vars and a single row of data.
    pub fn build_map(vars: &[(&str, VarType, usize)], fill: impl Fn(&str, &mut [u8])) -> Vec<u8> {
        let var_off = HEADER_LEN;
        let mut row_len = 0;
        let mut offsets = Vec::new();
        for (_, ty, count) in vars {
            offsets.push(row_len);
            row_len += ty.size() * count;
        }
        let buf_off = var_off + vars.len() * VAR_HEADER_LEN;
        let mut b = vec![0u8; buf_off + row_len];
        let w = |b: &mut Vec<u8>, off: usize, v: i32| b[off..off + 4].copy_from_slice(&v.to_le_bytes());
        w(&mut b, 0, 2);
        w(&mut b, 4, 1);
        w(&mut b, 8, 60);
        w(&mut b, 24, vars.len() as i32);
        w(&mut b, 28, var_off as i32);
        w(&mut b, 32, 1);
        w(&mut b, 36, row_len as i32);
        w(&mut b, 48, 42);
        w(&mut b, 52, buf_off as i32);
        for (i, (name, ty, count)) in vars.iter().enumerate() {
            let base = var_off + i * VAR_HEADER_LEN;
            w(&mut b, base, *ty as i32);
            w(&mut b, base + 4, offsets[i] as i32);
            w(&mut b, base + 8, *count as i32);
            b[base + 16..base + 16 + name.len()].copy_from_slice(name.as_bytes());
            let (s, e) = (buf_off + offsets[i], buf_off + offsets[i] + ty.size() * count);
            fill(name, &mut b[s..e]);
        }
        b
    }

    #[test]
    fn parses_header_and_vars() {
        let map = build_map(
            &[("Speed", VarType::Float, 1), ("Gear", VarType::Int, 1), ("CarIdxLapDistPct", VarType::Float, 64), ("OnPitRoad", VarType::Bool, 1)],
            |name, s| match name {
                "Speed" => s.copy_from_slice(&42.5f32.to_le_bytes()),
                "Gear" => s.copy_from_slice(&3i32.to_le_bytes()),
                "CarIdxLapDistPct" => s[4 * 5..4 * 6].copy_from_slice(&0.25f32.to_le_bytes()),
                "OnPitRoad" => s[0] = 1,
                _ => {}
            },
        );
        let h = Header::parse(&map).unwrap();
        assert!(h.is_connected());
        assert_eq!(h.tick_rate, 60);
        assert_eq!(h.latest_buf(), 0);
        let vars = VarTable::new(parse_var_headers(&map[h.var_header_offset as usize..], h.num_vars as usize));
        let off = h.bufs[0].offset as usize;
        let row = Row(&map[off..off + h.buf_len as usize]);
        assert_eq!(row.f32(vars.get("Speed")), 42.5);
        assert_eq!(row.i32(vars.get("Gear")), 3);
        assert_eq!(row.f32_at(vars.get("CarIdxLapDistPct"), 5), 0.25);
        assert_eq!(row.f32_at(vars.get("CarIdxLapDistPct"), 64), 0.0);
        assert!(row.bool(vars.get("OnPitRoad")));
        assert_eq!(row.f32(vars.get("Missing")), 0.0);
    }
}
