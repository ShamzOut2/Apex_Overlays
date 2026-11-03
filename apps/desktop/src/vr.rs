//! VR: the overlays shown in the headset as SteamVR overlays.
//!
//! Each overlay window is captured with Windows.Graphics.Capture (so it looks and updates
//! exactly as on the monitor) and its pixels are handed to SteamVR as an overlay texture.
//! An overlay is either fixed in the car (placed from SteamVR's seated position) or follows
//! the head, at the place its VR settings give: by default where it sits on the monitor,
//! mapped onto an imaginary screen in front of the driver.
//!
//! OpenVR is used through its C API (openvr_capi.h) from `openvr_api.dll`, which is bundled
//! (BSD licence, see vr/OPENVR_LICENSE.txt) and written next to the app's data on first use.

use std::sync::atomic::AtomicBool;

/// Set by the recenter hotkey; the VR thread resets SteamVR's seated position.
pub static RECENTER: AtomicBool = AtomicBool::new(false);

#[cfg(not(windows))]
pub fn spawn(_app: tauri::AppHandle) {}

#[cfg(windows)]
pub use imp::spawn;

#[cfg(windows)]
mod imp {
    use super::RECENTER;
    use crate::Core;
    use apex_core::vr::{placement, transform, Placement};
    use std::collections::HashMap;
    use std::ffi::{c_char, c_void, CString};
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    use tauri::{AppHandle, Manager};
    use windows::core::{Interface, HSTRING, PCSTR};
    use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
    use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
    use windows::Graphics::DirectX::DirectXPixelFormat;
    use windows::Graphics::SizeInt32;
    use windows::Win32::Foundation::{HMODULE, HWND};
    use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
    use windows::Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        D3D11_RESOURCE_MISC_SHARED, D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX, D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
        D3D11_USAGE_DEFAULT,
    };
    use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
    use windows::Win32::Graphics::Dxgi::{IDXGIDevice, IDXGIKeyedMutex, IDXGIResource1, DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::System::WinRT::Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess};
    use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    const DLL: &[u8] = include_bytes!("../vr/openvr_api.dll");

    // OpenVR C API. A function table is an array of pointers; these positions were generated
    // from openvr_capi.h for the interface versions named here.
    const OVERLAY_VERSION: &str = "FnTable:IVROverlay_028";
    const SYSTEM_VERSION: &str = "FnTable:IVRSystem_026";
    const CHAPERONE_VERSION: &str = "FnTable:IVRChaperone_004";
    const OV_CREATE: usize = 1;
    const OV_DESTROY: usize = 3;
    const OV_SET_FLAG: usize = 11;
    const OV_SET_ALPHA: usize = 16;
    const OV_SET_WIDTH: usize = 22;
    const OV_SET_TRANSFORM_ABSOLUTE: usize = 33;
    const OV_SET_TRANSFORM_DEVICE: usize = 35;
    const OV_SHOW: usize = 43;
    const OV_HIDE: usize = 44;
    const OV_SET_TEXTURE: usize = 60;
    const SYS_POLL_NEXT_EVENT: usize = 30;
    const SYS_ACK_QUIT: usize = 47;
    const CHAP_RESET_ZERO_POSE: usize = 8;
    const APP_OVERLAY: i32 = 2;
    const APP_BACKGROUND: i32 = 3;
    const SEATED: i32 = 0;
    const HMD: u32 = 0;
    const FLAG_PREMULTIPLIED: i32 = 2_097_152;
    const EVENT_QUIT: u32 = 700;
    /// sizeof(VREvent_t) on 64-bit Windows.
    const EVENT_SIZE: usize = 64;

    #[repr(C)]
    struct Texture {
        handle: *mut c_void,
        kind: i32,
        color_space: i32,
    }

    type InitFn = unsafe extern "C" fn(*mut i32, i32, *const c_char) -> isize;
    type InterfaceFn = unsafe extern "C" fn(*const c_char, *mut i32) -> isize;
    type ShutdownFn = unsafe extern "C" fn();

    struct Api {
        init: InitFn,
        interface: InterfaceFn,
        shutdown: ShutdownFn,
    }

    /// A running OpenVR session (the three function tables).
    struct Vr {
        overlay: *const usize,
        system: *const usize,
        chaperone: *const usize,
    }

    impl Vr {
        /// Function `i` of a table, as type `F`.
        unsafe fn f<F: Copy>(table: *const usize, i: usize) -> F {
            unsafe { std::mem::transmute_copy(&*table.add(i)) }
        }
        fn create(&self, key: &str, name: &str) -> Option<u64> {
            let (k, n) = (CString::new(key).ok()?, CString::new(name).ok()?);
            let mut h = 0u64;
            let f: unsafe extern "system" fn(*const c_char, *const c_char, *mut u64) -> i32 = unsafe { Self::f(self.overlay, OV_CREATE) };
            (unsafe { f(k.as_ptr(), n.as_ptr(), &mut h) } == 0).then_some(h)
        }
        fn destroy(&self, h: u64) {
            let f: unsafe extern "system" fn(u64) -> i32 = unsafe { Self::f(self.overlay, OV_DESTROY) };
            unsafe { f(h) };
        }
        fn premultiplied(&self, h: u64) {
            let f: unsafe extern "system" fn(u64, i32, bool) -> i32 = unsafe { Self::f(self.overlay, OV_SET_FLAG) };
            unsafe { f(h, FLAG_PREMULTIPLIED, true) };
        }
        fn place(&self, h: u64, p: &Placement) {
            let m = transform(p);
            unsafe {
                let width: unsafe extern "system" fn(u64, f32) -> i32 = Self::f(self.overlay, OV_SET_WIDTH);
                let alpha: unsafe extern "system" fn(u64, f32) -> i32 = Self::f(self.overlay, OV_SET_ALPHA);
                width(h, p.width);
                alpha(h, p.alpha);
                if p.head {
                    let f: unsafe extern "system" fn(u64, u32, *const [[f32; 4]; 3]) -> i32 = Self::f(self.overlay, OV_SET_TRANSFORM_DEVICE);
                    f(h, HMD, &m);
                } else {
                    let f: unsafe extern "system" fn(u64, i32, *const [[f32; 4]; 3]) -> i32 = Self::f(self.overlay, OV_SET_TRANSFORM_ABSOLUTE);
                    f(h, SEATED, &m);
                }
            }
        }
        fn visible(&self, h: u64, on: bool) {
            let f: unsafe extern "system" fn(u64) -> i32 = unsafe { Self::f(self.overlay, if on { OV_SHOW } else { OV_HIDE }) };
            unsafe { f(h) };
        }
        fn texture(&self, h: u64, tex: &ID3D11Texture2D) {
            let t = Texture { handle: tex.as_raw(), kind: 0, color_space: 0 };
            let f: unsafe extern "system" fn(u64, *const Texture) -> i32 = unsafe { Self::f(self.overlay, OV_SET_TEXTURE) };
            unsafe { f(h, &t) };
        }
        /// SteamVR is closing: say so and stop.
        fn quitting(&self) -> bool {
            let mut ev = [0u8; EVENT_SIZE];
            let poll: unsafe extern "system" fn(*mut u8, u32) -> bool = unsafe { Self::f(self.system, SYS_POLL_NEXT_EVENT) };
            while unsafe { poll(ev.as_mut_ptr(), EVENT_SIZE as u32) } {
                if u32::from_ne_bytes([ev[0], ev[1], ev[2], ev[3]]) == EVENT_QUIT {
                    let ack: unsafe extern "system" fn() = unsafe { Self::f(self.system, SYS_ACK_QUIT) };
                    unsafe { ack() };
                    return true;
                }
            }
            false
        }
        fn recenter(&self) {
            let f: unsafe extern "system" fn(i32) = unsafe { Self::f(self.chaperone, CHAP_RESET_ZERO_POSE) };
            unsafe { f(SEATED) };
        }
    }

    fn load(dir: &std::path::Path) -> Result<Api, String> {
        let path = dir.join("openvr").join("openvr_api.dll");
        if std::fs::metadata(&path).map(|m| m.len() as usize).ok() != Some(DLL.len()) {
            std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(&path, DLL).map_err(|e| format!("could not write openvr_api.dll: {e}"))?;
        }
        unsafe {
            let lib = LoadLibraryW(&HSTRING::from(path.as_os_str())).map_err(|e| format!("could not load openvr_api.dll: {e}"))?;
            let sym = |name: &str| {
                let n = CString::new(name).unwrap();
                GetProcAddress(lib, PCSTR(n.as_ptr() as *const u8)).ok_or(format!("openvr_api.dll has no {name}"))
            };
            Ok(Api {
                init: std::mem::transmute::<unsafe extern "system" fn() -> isize, InitFn>(sym("VR_InitInternal2")?),
                interface: std::mem::transmute::<unsafe extern "system" fn() -> isize, InterfaceFn>(sym("VR_GetGenericInterface")?),
                shutdown: std::mem::transmute::<unsafe extern "system" fn() -> isize, ShutdownFn>(sym("VR_ShutdownInternal")?),
            })
        }
    }

    /// Start OpenVR as an overlay app, only if SteamVR is already running (a background-app
    /// probe first, so enabling VR never starts SteamVR by itself).
    fn start(api: &Api) -> Result<Vr, String> {
        let mut err = 0i32;
        unsafe { (api.init)(&mut err, APP_BACKGROUND, std::ptr::null()) };
        if err != 0 {
            return Err(if err == 121 { "SteamVR is not running".into() } else { format!("SteamVR is not available (error {err})") });
        }
        unsafe { (api.shutdown)() };
        unsafe { (api.init)(&mut err, APP_OVERLAY, std::ptr::null()) };
        if err != 0 {
            return Err(format!("SteamVR did not accept the overlays (error {err})"));
        }
        let table = |name: &str| -> Result<*const usize, String> {
            let n = CString::new(name).unwrap();
            let mut e = 0i32;
            let p = unsafe { (api.interface)(n.as_ptr(), &mut e) };
            if e != 0 || p == 0 {
                Err(format!("SteamVR has no {name} (error {e}); update SteamVR"))
            } else {
                Ok(p as *const usize)
            }
        };
        match (table(OVERLAY_VERSION), table(SYSTEM_VERSION), table(CHAPERONE_VERSION)) {
            (Ok(overlay), Ok(system), Ok(chaperone)) => Ok(Vr { overlay, system, chaperone }),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                unsafe { (api.shutdown)() };
                Err(e)
            }
        }
    }

    /// The Direct3D 11 device captures are copied with, and its WinRT face for the capture API.
    struct Gfx {
        device: ID3D11Device,
        context: ID3D11DeviceContext,
        winrt: IDirect3DDevice,
    }

    fn gfx() -> windows::core::Result<Gfx> {
        let mut device = None;
        let mut context = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?
        };
        let device: ID3D11Device = device.ok_or_else(windows::core::Error::empty)?;
        let dxgi: IDXGIDevice = device.cast()?;
        let winrt: IDirect3DDevice = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? }.cast()?;
        Ok(Gfx { device, context: context.ok_or_else(windows::core::Error::empty)?, winrt })
    }

    /// One overlay window being captured into a texture SteamVR reads.
    struct Capture {
        hwnd: isize,
        pool: Direct3D11CaptureFramePool,
        session: GraphicsCaptureSession,
        size: SizeInt32,
        tex: Option<(ID3D11Texture2D, i32, i32)>,
        /// For the OpenXR layer: the texture is shared by name (see `apex_xr_shared::texture_name`)
        /// with a keyed mutex (key 0). `handle` is its id (0 = none), `nt` keeps the name alive.
        keyed: bool,
        handle: u64,
        nt: Option<windows::Win32::Foundation::HANDLE>,
        /// Frames copied so far.
        frame: u32,
    }

    impl Capture {
        fn new(g: &Gfx, hwnd: isize, keyed: bool) -> windows::core::Result<Self> {
            let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
            let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(HWND(hwnd as *mut c_void))? };
            let size = item.Size()?;
            let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&g.winrt, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, size)?;
            let session = pool.CreateCaptureSession(&item)?;
            let _ = session.SetIsCursorCaptureEnabled(false);
            // Windows 11: no yellow "being captured" border around the overlay.
            let _ = session.SetIsBorderRequired(false);
            session.StartCapture()?;
            Ok(Self { hwnd, pool, session, size, tex: None, keyed, handle: 0, nt: None, frame: 0 })
        }

        /// The newest frame, copied into this capture's own texture (None if nothing new).
        fn next(&mut self, g: &Gfx) -> windows::core::Result<Option<&ID3D11Texture2D>> {
            let Ok(frame) = self.pool.TryGetNextFrame() else { return Ok(None) };
            let content = frame.ContentSize()?;
            if content.Width <= 0 || content.Height <= 0 {
                return Ok(None);
            }
            let surface = frame.Surface()?;
            let src: ID3D11Texture2D = unsafe { surface.cast::<IDirect3DDxgiInterfaceAccess>()?.GetInterface()? };
            // Read no more than the frame buffer holds (it is resized for the next frame).
            let (w, h) = (content.Width.min(self.size.Width), content.Height.min(self.size.Height));
            if self.tex.as_ref().is_none_or(|t| t.1 != w || t.2 != h) {
                let desc = D3D11_TEXTURE2D_DESC {
                    Width: w as u32,
                    Height: h as u32,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                    CPUAccessFlags: 0,
                    MiscFlags: if self.keyed {
                        D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX.0 | D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0
                    } else {
                        D3D11_RESOURCE_MISC_SHARED.0
                    } as u32,
                };
                let mut t = None;
                unsafe { g.device.CreateTexture2D(&desc, None, Some(&mut t))? };
                if let Some(old) = self.nt.take() {
                    unsafe {
                        let _ = windows::Win32::Foundation::CloseHandle(old);
                    }
                }
                self.handle = 0;
                if let (Some(t), true) = (&t, self.keyed) {
                    // A new name for every texture, so the layer opens the new one.
                    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                    let id = ((std::process::id() as u64) << 32) | NEXT.fetch_add(1, Ordering::Relaxed);
                    let name = HSTRING::from(apex_xr_shared::texture_name(id));
                    let access = DXGI_SHARED_RESOURCE_READ.0 | DXGI_SHARED_RESOURCE_WRITE.0;
                    self.nt = Some(unsafe { t.cast::<IDXGIResource1>()?.CreateSharedHandle(None, access, &name)? });
                    self.handle = id;
                }
                self.tex = t.map(|t| (t, w, h));
            }
            let Some((tex, _, _)) = self.tex.as_ref() else { return Ok(None) };
            let region = D3D11_BOX { left: 0, top: 0, front: 0, right: w as u32, bottom: h as u32, back: 1 };
            // Shared with the OpenXR layer: write only while holding the texture's lock (a
            // frame is skipped if the game is copying it right now).
            let mutex: Option<IDXGIKeyedMutex> = if self.keyed { tex.cast().ok() } else { None };
            let locked = mutex.as_ref().map(|m| unsafe { (Interface::vtable(m).AcquireSync)(Interface::as_raw(m), 0, 8) }.0 == 0);
            if locked == Some(false) {
                return Ok(None);
            }
            unsafe {
                g.context.CopySubresourceRegion(tex, 0, 0, 0, 0, &src, 0, Some(&region));
                g.context.Flush();
                if let Some(m) = &mutex {
                    let _ = m.ReleaseSync(0);
                }
            }
            self.frame = self.frame.wrapping_add(1);
            if content != self.size {
                // The window was resized: capture at the new size from the next frame on.
                self.size = content;
                self.pool.Recreate(&g.winrt, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, content)?;
            }
            Ok(Some(tex))
        }
    }

    impl Drop for Capture {
        fn drop(&mut self) {
            if let Some(h) = self.nt.take() {
                unsafe {
                    let _ = windows::Win32::Foundation::CloseHandle(h);
                }
            }
            let _ = self.session.Close();
            let _ = self.pool.Close();
        }
    }

    // ---- OpenXR: panels for the app's OpenXR layer inside the game -----------------------

    use apex_xr_shared::{self as xs, Panel, Shared};
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows::Win32::System::Memory::{CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegGetValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_SET_VALUE, REG_DWORD,
        REG_OPTION_NON_VOLATILE, RRF_RT_REG_DWORD,
    };
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    /// The layer DLL (apps/xr-layer), embedded at build time (empty if it was not built).
    const LAYER_DLL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/apex_xr_layer.dll"));
    const IMPLICIT_LAYERS: &str = "SOFTWARE\\Khronos\\OpenXR\\1\\ApiLayers\\Implicit";

    /// The shared memory the layer reads (see `apex_xr_shared`).
    struct ShmWriter {
        handle: HANDLE,
        view: MEMORY_MAPPED_VIEW_ADDRESS,
    }

    impl ShmWriter {
        fn create() -> Result<Self, String> {
            let size = std::mem::size_of::<Shared>();
            let handle = unsafe { CreateFileMappingW(INVALID_HANDLE_VALUE, None, PAGE_READWRITE, 0, size as u32, &HSTRING::from(xs::NAME)) }
                .map_err(|e| format!("shared memory: {e}"))?;
            let view = unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, size) };
            if view.Value.is_null() {
                unsafe {
                    let _ = CloseHandle(handle);
                }
                return Err("shared memory: could not map it".into());
            }
            unsafe { std::ptr::write_volatile(view.Value as *mut Shared, Shared::default()) };
            Ok(Self { handle, view })
        }

        /// Publish the panels (seqlock: odd `seq` while writing).
        fn write(&self, panels: &[Panel], recenter: u32) {
            use std::sync::atomic::{fence, Ordering::Release};
            let p = self.view.Value as *mut Shared;
            let mut s = Shared::default();
            let n = panels.len().min(xs::MAX_PANELS);
            s.panels[..n].copy_from_slice(&panels[..n]);
            s.count = n as u32;
            s.recenter = recenter;
            s.written_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
            unsafe {
                let seq = std::ptr::read_volatile(std::ptr::addr_of!((*p).seq));
                let start = (seq | 1).wrapping_add(if seq % 2 == 1 { 2 } else { 0 });
                std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).seq), start);
                fence(Release);
                s.seq = start;
                std::ptr::write_volatile(p, s);
                fence(Release);
                std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).seq), start.wrapping_add(1));
            }
        }
    }

    impl Drop for ShmWriter {
        fn drop(&mut self) {
            // Nothing to draw any more (the layer stops at once rather than when it goes stale).
            self.write(&[], 0);
            unsafe {
                let _ = UnmapViewOfFile(self.view);
                let _ = CloseHandle(self.handle);
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// The layer is registered as an implicit OpenXR layer (value 0 = enabled) under `root`.
    fn registered(root: HKEY, manifest: &str) -> bool {
        let (key, name) = (wide(IMPLICIT_LAYERS), wide(manifest));
        let mut v = 0u32;
        let mut size = 4u32;
        let r = unsafe {
            RegGetValueW(root, PCWSTR(key.as_ptr()), PCWSTR(name.as_ptr()), RRF_RT_REG_DWORD, None, Some(&mut v as *mut u32 as *mut c_void), Some(&mut size))
        };
        r.is_ok() && v == 0
    }

    fn register_user(manifest: &str) -> bool {
        let (key, name) = (wide(IMPLICIT_LAYERS), wide(manifest));
        let mut h = HKEY::default();
        unsafe {
            if RegCreateKeyExW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()), None, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_SET_VALUE, None, &mut h, None)
                .is_err()
            {
                return false;
            }
            let ok = RegSetValueExW(h, PCWSTR(name.as_ptr()), None, REG_DWORD, Some(&0u32.to_le_bytes())).is_ok();
            let _ = RegCloseKey(h);
            ok
        }
    }

    /// Ask Windows (one admin prompt) to register the layer for every user: the OpenXR
    /// loader in every game reads that list.
    fn register_machine(manifest: &str) {
        let args = format!("add \"HKLM\\{IMPLICIT_LAYERS}\" /v \"{manifest}\" /t REG_DWORD /d 0 /f /reg:64");
        unsafe { ShellExecuteW(None, w!("runas"), w!("reg.exe"), &HSTRING::from(args), PCWSTR::null(), SW_HIDE) };
    }

    /// Put the layer DLL and its manifest in the data folder and register it. Returns the
    /// manifest path.
    fn install_layer(dir: &std::path::Path) -> Result<String, String> {
        if LAYER_DLL.is_empty() {
            return Err("this build has no OpenXR layer".into());
        }
        let folder = dir.join("openxr");
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        // One file per version: a game may still have the previous one loaded.
        let dll = folder.join(format!("apex_xr_layer-{}.dll", env!("CARGO_PKG_VERSION")));
        if std::fs::metadata(&dll).map(|m| m.len() as usize).ok() != Some(LAYER_DLL.len()) {
            std::fs::write(&dll, LAYER_DLL).map_err(|e| format!("could not write the layer: {e}"))?;
        }
        let manifest = folder.join("apex-overlays-openxr.json");
        let json = serde_json::json!({
            "file_format_version": "1.0.0",
            "api_layer": {
                "name": "XR_APILAYER_APEX_overlays",
                "library_path": dll.to_string_lossy(),
                "api_version": "1.0",
                "implementation_version": "1",
                "description": "Apex Overlays: overlays inside OpenXR games",
                "disable_environment": "DISABLE_APEX_OVERLAYS_XR_LAYER"
            }
        });
        let text = serde_json::to_string_pretty(&json).unwrap_or_default();
        if std::fs::read_to_string(&manifest).ok().as_deref() != Some(text.as_str()) {
            std::fs::write(&manifest, &text).map_err(|e| format!("could not write the layer manifest: {e}"))?;
        }
        let path = manifest.to_string_lossy().into_owned();
        register_user(&path);
        Ok(path)
    }

    /// Overlay id → a stable key for the layer.
    fn key(id: &str) -> u64 {
        id.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
    }

    struct Xr {
        shm: ShmWriter,
        captures: HashMap<String, Capture>,
        failed: HashMap<String, Instant>,
        recenter: u32,
        manifest: Result<String, String>,
        asked_admin: bool,
        checked: Option<(Instant, bool)>,
    }

    struct Live {
        handle: u64,
        capture: Option<Capture>,
        placed: Option<Placement>,
        shown: Option<bool>,
        failed_at: Option<Instant>,
    }

    pub fn spawn(app: AppHandle) {
        let _ = std::thread::Builder::new().name("vr".into()).spawn(move || {
            if let Err(e) = unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
                log::warn!("VR: WinRT not available: {e}");
            }
            run(app);
        });
    }

    fn run(app: AppHandle) {
        let data_dir = apex_core::config::default_data_dir();
        let mut api: Option<Api> = None;
        let mut vr: Option<Vr> = None;
        let mut gfx_dev: Option<Gfx> = None;
        let mut live: HashMap<String, Live> = HashMap::new();
        let mut xr: Option<Xr> = None;
        let mut last_try = Instant::now() - Duration::from_secs(60);
        let mut status = String::new();
        // The config and the monitor size, re-read a few times a second (frames are copied
        // every pass).
        let mut snapshot: Option<(Instant, apex_core::config::Config, (f64, f64))> = None;
        let publish = |app: &AppHandle, status: &mut String, s: &str, msg: &str, n: usize| {
            let v = serde_json::json!({ "status": s, "message": msg, "overlays": n }).to_string();
            if *status != v {
                app.state::<Core>().hub.publish("vr", &v);
                *status = v;
            }
        };
        loop {
            let core = app.state::<Core>();
            if snapshot.as_ref().is_none_or(|s| s.0.elapsed() > Duration::from_millis(250)) {
                // The monitor the "as on my screen" layout is taken from (logical pixels).
                let screen = app
                    .primary_monitor()
                    .ok()
                    .flatten()
                    .map(|m| {
                        let s = m.scale_factor();
                        (m.size().width as f64 / s, m.size().height as f64 / s)
                    })
                    .unwrap_or((1920.0, 1080.0));
                snapshot = Some((Instant::now(), core.config.get(), screen));
            }
            let (_, cfg, screen) = snapshot.as_ref().unwrap();
            let screen = *screen;
            if cfg.vr.enabled && cfg.vr.mode == "openxr" {
                // OpenXR: SteamVR is not used.
                if let Some(v) = vr.take() {
                    for (_, l) in live.drain() {
                        v.destroy(l.handle);
                    }
                    if let Some(a) = &api {
                        unsafe { (a.shutdown)() };
                    }
                }
                if gfx_dev.is_none() {
                    match gfx() {
                        Ok(g) => gfx_dev = Some(g),
                        Err(e) => {
                            publish(&app, &mut status, "error", &format!("Direct3D 11 is not available: {e}"), 0);
                            std::thread::sleep(Duration::from_secs(5));
                            continue;
                        }
                    }
                }
                if xr.is_none() {
                    match ShmWriter::create() {
                        Ok(shm) => {
                            xr = Some(Xr {
                                shm,
                                captures: HashMap::new(),
                                failed: HashMap::new(),
                                recenter: 0,
                                manifest: install_layer(&data_dir),
                                asked_admin: false,
                                checked: None,
                            })
                        }
                        Err(e) => {
                            publish(&app, &mut status, "error", &e, 0);
                            std::thread::sleep(Duration::from_secs(5));
                            continue;
                        }
                    }
                }
                let (x, g) = (xr.as_mut().unwrap(), gfx_dev.as_ref().unwrap());
                let message = match &x.manifest {
                    Err(e) => {
                        publish(&app, &mut status, "error", &format!("OpenXR layer: {e}"), 0);
                        std::thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                    Ok(manifest) => {
                        // Registered for every game? (re-checked every few seconds; one admin prompt)
                        if x.checked.is_none_or(|c| c.0.elapsed() > Duration::from_secs(5)) {
                            let machine = registered(HKEY_LOCAL_MACHINE, manifest);
                            if !machine && !x.asked_admin {
                                x.asked_admin = true;
                                register_machine(manifest);
                            }
                            x.checked = Some((Instant::now(), machine));
                        }
                        if x.checked.is_some_and(|c| c.1) {
                            "Start iRacing in OpenXR VR mode (restart it if it was already running)"
                        } else {
                            "Registered for your Windows user only — accept the admin prompt (or turn VR off and on) to register it for every game"
                        }
                    }
                };
                if RECENTER.swap(false, Ordering::Relaxed) {
                    x.recenter = x.recenter.wrapping_add(1);
                }
                let shown = core.hub.visible.load(Ordering::Relaxed) || core.hub.edit_mode.load(Ordering::Relaxed);
                let hwnds = core.hwnds.lock().unwrap().clone();
                let sizes = core.auto_size.lock().unwrap().clone();
                let wanted: Vec<_> = cfg.overlays.iter().filter(|o| o.enabled).filter_map(|o| hwnds.get(&o.id).map(|h| (o, *h))).collect();
                x.captures.retain(|id, c| wanted.iter().any(|(o, h)| &o.id == id && *h == c.hwnd));
                let mut panels = vec![];
                for (o, hwnd) in &wanted {
                    let (aw, ah) = sizes.get(&crate::label_for(&o.id)).copied().unwrap_or_default();
                    let size = |auto: f64, set: u32| if auto > 0.0 { auto } else { set as f64 };
                    let p = placement(o, &cfg.vr, screen, (o.x as f64, o.y as f64, size(aw, o.width), size(ah, o.height)));
                    if !x.captures.contains_key(&o.id) && x.failed.get(&o.id).is_none_or(|t| t.elapsed() > Duration::from_secs(5)) {
                        match Capture::new(g, *hwnd, true) {
                            Ok(c) => {
                                x.captures.insert(o.id.clone(), c);
                            }
                            Err(e) => {
                                log::warn!("VR: cannot capture overlay {}: {e}", o.id);
                                x.failed.insert(o.id.clone(), Instant::now());
                            }
                        }
                    }
                    let Some(c) = x.captures.get_mut(&o.id) else { continue };
                    if let Err(e) = c.next(g) {
                        log::warn!("VR: capture of overlay {} stopped: {e}", o.id);
                        x.captures.remove(&o.id);
                        x.failed.insert(o.id.clone(), Instant::now());
                        continue;
                    }
                    let Some((_, w, h)) = c.tex.as_ref() else { continue };
                    if !(shown && p.show) || c.handle == 0 {
                        continue;
                    }
                    panels.push(Panel {
                        texture: c.handle,
                        width: *w as u32,
                        height: *h as u32,
                        frame: c.frame,
                        head: p.head as u32,
                        pos: p.pos,
                        orient: xs::orientation(p.yaw, p.pitch, p.roll),
                        size: [p.width, p.width * *h as f32 / (*w).max(1) as f32],
                        key: key(&o.id),
                    });
                }
                x.shm.write(&panels, x.recenter);
                publish(&app, &mut status, "on", message, panels.len());
                std::thread::sleep(Duration::from_millis(11));
                continue;
            }
            // SteamVR (or off): the OpenXR output stops.
            xr = None;
            if !cfg.vr.enabled {
                if let Some(v) = vr.take() {
                    for (_, l) in live.drain() {
                        v.destroy(l.handle);
                    }
                    if let Some(a) = &api {
                        unsafe { (a.shutdown)() };
                    }
                }
                publish(&app, &mut status, "off", "", 0);
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
            if vr.is_none() {
                if last_try.elapsed() < Duration::from_secs(5) {
                    std::thread::sleep(Duration::from_millis(250));
                    continue;
                }
                last_try = Instant::now();
                if api.is_none() {
                    match load(&data_dir) {
                        Ok(a) => api = Some(a),
                        Err(e) => {
                            publish(&app, &mut status, "error", &e, 0);
                            continue;
                        }
                    }
                }
                if gfx_dev.is_none() {
                    match gfx() {
                        Ok(g) => gfx_dev = Some(g),
                        Err(e) => {
                            publish(&app, &mut status, "error", &format!("Direct3D 11 is not available: {e}"), 0);
                            continue;
                        }
                    }
                }
                match start(api.as_ref().unwrap()) {
                    Ok(v) => vr = Some(v),
                    Err(e) => {
                        publish(&app, &mut status, "waiting", &e, 0);
                        continue;
                    }
                }
            }
            let (v, g) = (vr.as_ref().unwrap(), gfx_dev.as_ref().unwrap());
            if v.quitting() {
                for (_, l) in live.drain() {
                    v.destroy(l.handle);
                }
                unsafe { (api.as_ref().unwrap().shutdown)() };
                vr = None;
                last_try = Instant::now();
                publish(&app, &mut status, "waiting", "SteamVR closed", 0);
                continue;
            }
            if RECENTER.swap(false, Ordering::Relaxed) {
                v.recenter();
            }

            let shown = core.hub.visible.load(Ordering::Relaxed) || core.hub.edit_mode.load(Ordering::Relaxed);
            let hwnds = core.hwnds.lock().unwrap().clone();
            let sizes = core.auto_size.lock().unwrap().clone();
            let wanted: Vec<_> = cfg.overlays.iter().filter(|o| o.enabled).filter_map(|o| hwnds.get(&o.id).map(|h| (o, *h))).collect();
            live.retain(|id, l| {
                let keep = wanted.iter().any(|(o, _)| &o.id == id);
                if !keep {
                    v.destroy(l.handle);
                }
                keep
            });
            for (o, hwnd) in &wanted {
                let (aw, ah) = sizes.get(&crate::label_for(&o.id)).copied().unwrap_or_default();
                let size = |auto: f64, set: u32| if auto > 0.0 { auto } else { set as f64 };
                let p = placement(o, &cfg.vr, screen, (o.x as f64, o.y as f64, size(aw, o.width), size(ah, o.height)));
                let l = match live.get_mut(&o.id) {
                    Some(l) => l,
                    None => {
                        let name = if o.name.is_empty() { o.kind.clone() } else { o.name.clone() };
                        let Some(handle) = v.create(&format!("apexoverlays.{}", o.id), &format!("Apex Overlays: {name}")) else { continue };
                        v.premultiplied(handle);
                        live.entry(o.id.clone()).or_insert(Live { handle, capture: None, placed: None, shown: None, failed_at: None })
                    }
                };
                if l.capture.as_ref().is_some_and(|c| c.hwnd != *hwnd) {
                    l.capture = None;
                }
                if l.capture.is_none() && l.failed_at.is_none_or(|t| t.elapsed() > Duration::from_secs(5)) {
                    match Capture::new(g, *hwnd, false) {
                        Ok(c) => l.capture = Some(c),
                        Err(e) => {
                            log::warn!("VR: cannot capture overlay {}: {e}", o.id);
                            l.failed_at = Some(Instant::now());
                        }
                    }
                }
                if l.placed.as_ref() != Some(&p) {
                    v.place(l.handle, &p);
                    l.placed = Some(p.clone());
                }
                let on = shown && p.show;
                if l.shown != Some(on) {
                    v.visible(l.handle, on);
                    l.shown = Some(on);
                }
                if let Some(c) = l.capture.as_mut() {
                    match c.next(g) {
                        Ok(Some(tex)) => v.texture(l.handle, tex),
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!("VR: capture of overlay {} stopped: {e}", o.id);
                            l.capture = None;
                            l.failed_at = Some(Instant::now());
                        }
                    }
                }
            }
            let n = live.values().filter(|l| l.shown == Some(true)).count();
            publish(&app, &mut status, "on", "", n);
            std::thread::sleep(Duration::from_millis(11));
        }
    }
}
