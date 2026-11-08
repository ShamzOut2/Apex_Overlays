use apex_xr_shared::{put_text, LayerStatus, Panel, Shared, MAGIC, NAME, STALE_MS, STATUS_NAME, STATUS_VERSION, VERSION};
use openxr_sys as xr;
use openxr_sys::pfn;
use openxr_sys::Handle;
use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use windows::core::IUnknown;
use windows::core::{Interface, HSTRING};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Graphics::Direct3D11::ID3D11Device1;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D, D3D11_BIND_RENDER_TARGET, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
};
use windows::Win32::Graphics::Direct3D11on12::{D3D11On12CreateDevice, ID3D11On12Device, D3D11_RESOURCE_FLAGS};
use windows::Win32::Graphics::Direct3D12::D3D12_RESOURCE_STATE_RENDER_TARGET;
use windows::Win32::Graphics::Dxgi::{IDXGIKeyedMutex, DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE};
use windows::Win32::System::Memory::{MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_READ, FILE_MAP_WRITE, MEMORY_MAPPED_VIEW_ADDRESS};

const LAYER_NAME: &str = "XR_APILAYER_APEX_overlays";
/// Swapchain formats the captured BGRA pixels can be copied into, best first. The pixels are
/// sRGB, so the sRGB format shows them as on the monitor.
const FORMATS: [i64; 2] = [91 /* DXGI_FORMAT_B8G8R8A8_UNORM_SRGB */, 87 /* DXGI_FORMAT_B8G8R8A8_UNORM */];
/// Composition layers a runtime must accept (XR_MIN_COMPOSITION_LAYERS_SUPPORTED).
const MAX_LAYERS: usize = 16;
/// After the runtime rejects a frame with the overlays in it, frames go without them this long.
const BACK_OFF_SECS: u64 = 5;

/// A few lines to %TEMP%\apex-xr-layer.log for troubleshooting (the first 200 only).
fn log(msg: &str) {
    use std::io::Write;
    static COUNT: Mutex<u32> = Mutex::new(0);
    let Ok(mut n) = COUNT.lock() else { return };
    if *n >= 200 {
        return;
    }
    *n += 1;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(std::env::temp_dir().join("apex-xr-layer.log")) {
        let _ = writeln!(f, "{msg}");
    }
}

/// The next layer's (or the runtime's) functions this layer calls.
#[derive(Clone, Copy)]
struct Next {
    create_session: pfn::CreateSession,
    destroy_session: pfn::DestroySession,
    end_frame: pfn::EndFrame,
    create_reference_space: pfn::CreateReferenceSpace,
    destroy_space: pfn::DestroySpace,
    locate_space: pfn::LocateSpace,
    create_swapchain: pfn::CreateSwapchain,
    destroy_swapchain: pfn::DestroySwapchain,
    enumerate_swapchain_formats: pfn::EnumerateSwapchainFormats,
    enumerate_swapchain_images: pfn::EnumerateSwapchainImages,
    acquire_swapchain_image: pfn::AcquireSwapchainImage,
    wait_swapchain_image: pfn::WaitSwapchainImage,
    release_swapchain_image: pfn::ReleaseSwapchainImage,
}

struct Shm {
    handle: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
}

impl Shm {
    fn open() -> Option<Self> {
        let handle = unsafe { OpenFileMappingW(FILE_MAP_READ.0, false, &HSTRING::from(NAME)) }.ok()?;
        let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ, 0, 0, std::mem::size_of::<Shared>()) };
        if view.Value.is_null() {
            unsafe {
                let _ = CloseHandle(handle);
            }
            return None;
        }
        Some(Self { handle, view })
    }

    /// A consistent copy of what the app published (seqlock), if it is current.
    fn read(&self) -> Option<Shared> {
        let p = self.view.Value as *const Shared;
        for _ in 0..4 {
            let before = unsafe { std::ptr::read_volatile(std::ptr::addr_of!((*p).seq)) };
            if before % 2 == 1 {
                std::thread::yield_now();
                continue;
            }
            let copy = unsafe { std::ptr::read_volatile(p) };
            let after = unsafe { std::ptr::read_volatile(std::ptr::addr_of!((*p).seq)) };
            if before == after {
                let fresh = now_ms().saturating_sub(copy.written_ms) < STALE_MS;
                return (copy.magic == MAGIC && copy.version == VERSION && fresh).then_some(copy);
            }
        }
        None
    }
}

impl Drop for Shm {
    fn drop(&mut self) {
        unsafe {
            let _ = UnmapViewOfFile(self.view);
            let _ = CloseHandle(self.handle);
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Where the layer tells the app it is running (see `apex_xr_shared::LayerStatus`).
#[derive(Default)]
struct Status {
    map: Option<(HANDLE, MEMORY_MAPPED_VIEW_ADDRESS)>,
    tried: Option<Instant>,
    /// The texts last written (they are only rewritten when they change).
    written: Option<(String, String)>,
}

impl Status {
    /// Report one frame: overlays shown, Direct3D version, runtime and any problem.
    fn report(&mut self, shown: u32, api: u32, runtime: &str, problem: &str) {
        if self.map.is_none() && self.tried.is_none_or(|t| t.elapsed().as_secs() >= 2) {
            // The app creates the block; until it runs there is nobody to tell.
            self.tried = Some(Instant::now());
            if let Ok(h) = unsafe { OpenFileMappingW(FILE_MAP_READ.0 | FILE_MAP_WRITE.0, false, &HSTRING::from(STATUS_NAME)) } {
                let view = unsafe { MapViewOfFile(h, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, std::mem::size_of::<LayerStatus>()) };
                if view.Value.is_null() {
                    unsafe {
                        let _ = CloseHandle(h);
                    }
                } else {
                    self.map = Some((h, view));
                    self.written = None;
                }
            }
        }
        let Some((_, view)) = self.map else { return };
        let p = view.Value as *mut LayerStatus;
        unsafe {
            if self.written.as_ref().is_none_or(|(r, q)| r != runtime || q != problem) {
                let mut text = LayerStatus::default();
                put_text(&mut text.runtime, runtime);
                put_text(&mut text.problem, problem);
                std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).runtime), text.runtime);
                std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).problem), text.problem);
                std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).magic), MAGIC);
                std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).version), STATUS_VERSION);
                self.written = Some((runtime.to_string(), problem.to_string()));
            }
            std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).shown), shown);
            std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).api), api);
            std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).alive_ms), now_ms());
        }
    }
}

impl Drop for Status {
    fn drop(&mut self) {
        if let Some((h, view)) = self.map.take() {
            unsafe {
                let _ = UnmapViewOfFile(view);
                let _ = CloseHandle(h);
            }
        }
    }
}

struct PanelState {
    swapchain: xr::Swapchain,
    width: u32,
    height: u32,
    images: Vec<ID3D11Texture2D>,
    texture: u64,
    source: Option<ID3D11Texture2D>,
    frame: u32,
    /// An image has been released, so the swapchain can be shown.
    ready: bool,
}

struct SessionState {
    session: xr::Session,
    /// Direct3D 11 device and context: the game's own, or, for a Direct3D 12 session, a
    /// D3D11-on-12 device on the game's command queue (`on12`), so the shared textures are
    /// copied the same way in both.
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    on12: Option<ID3D11On12Device>,
    local: xr::Space,
    view: xr::Space,
    format: Option<i64>,
    panels: HashMap<u64, PanelState>,
    /// Seated position for panels fixed in the car (set by the recentre hotkey).
    origin: xr::Posef,
    recenter: Option<u32>,
    shm: Option<Shm>,
    shm_tried: Option<Instant>,
    /// The quads submitted with the last frame (must outlive the call to the runtime).
    quads: Vec<xr::CompositionLayerQuad>,
    /// When the runtime last rejected a frame with the overlays in it.
    rejected: Option<Instant>,
}

struct State {
    next: Option<Next>,
    sessions: Vec<SessionState>,
    /// The OpenXR runtime's name (e.g. "VirtualDesktopXR", "SteamVR/OpenXR").
    runtime: String,
    /// Why the overlays are off, for the app's VR status ("" = nothing wrong).
    problem: String,
    status: Status,
}

// The raw handles and COM pointers are only used under the lock, on the game's threads.
unsafe impl Send for State {}

static STATE: Mutex<Option<State>> = Mutex::new(None);
static NEXT_GIPA: Mutex<Option<pfn::GetInstanceProcAddr>> = Mutex::new(None);

const IDENTITY: xr::Posef = xr::Posef { orientation: xr::Quaternionf { x: 0.0, y: 0.0, z: 0.0, w: 1.0 }, position: xr::Vector3f { x: 0.0, y: 0.0, z: 0.0 } };

fn qmul(a: xr::Quaternionf, b: xr::Quaternionf) -> xr::Quaternionf {
    xr::Quaternionf {
        w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
        x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
    }
}

fn rotate(q: xr::Quaternionf, v: xr::Vector3f) -> xr::Vector3f {
    let p = qmul(qmul(q, xr::Quaternionf { x: v.x, y: v.y, z: v.z, w: 0.0 }), xr::Quaternionf { x: -q.x, y: -q.y, z: -q.z, w: q.w });
    xr::Vector3f { x: p.x, y: p.y, z: p.z }
}

/// `b` placed relative to `a`.
fn compose(a: xr::Posef, b: xr::Posef) -> xr::Posef {
    let r = rotate(a.orientation, b.position);
    xr::Posef {
        orientation: qmul(a.orientation, b.orientation),
        position: xr::Vector3f { x: a.position.x + r.x, y: a.position.y + r.y, z: a.position.z + r.z },
    }
}

/// Unit length again (strict runtimes such as Virtual Desktop's reject a layer whose
/// orientation is even slightly off after the float maths).
fn normalized(q: xr::Quaternionf) -> xr::Quaternionf {
    let n = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
    if !n.is_finite() || n < 1e-6 {
        return IDENTITY.orientation;
    }
    xr::Quaternionf { x: q.x / n, y: q.y / n, z: q.z / n, w: q.w / n }
}

/// A head pose reduced to a seated origin: its position and which way it faces (yaw only).
fn level(p: xr::Posef) -> xr::Posef {
    let q = p.orientation;
    let yaw = (2.0 * (q.w * q.y + q.x * q.z)).atan2(1.0 - 2.0 * (q.y * q.y + q.x * q.x));
    let (s, c) = ((yaw / 2.0).sin(), (yaw / 2.0).cos());
    xr::Posef { orientation: xr::Quaternionf { x: 0.0, y: s, z: 0.0, w: c }, position: p.position }
}

unsafe fn load<T: Copy>(gipa: pfn::GetInstanceProcAddr, instance: xr::Instance, name: &CStr) -> Option<T> {
    let mut f: Option<pfn::VoidFunction> = None;
    let r = unsafe { gipa(instance, name.as_ptr(), &mut f) };
    if r != xr::Result::SUCCESS {
        return None;
    }
    f.map(|f| unsafe { std::mem::transmute_copy::<pfn::VoidFunction, T>(&f) })
}

unsafe fn resolve(gipa: pfn::GetInstanceProcAddr, instance: xr::Instance) -> Option<Next> {
    unsafe {
        Some(Next {
            create_session: load(gipa, instance, c"xrCreateSession")?,
            destroy_session: load(gipa, instance, c"xrDestroySession")?,
            end_frame: load(gipa, instance, c"xrEndFrame")?,
            create_reference_space: load(gipa, instance, c"xrCreateReferenceSpace")?,
            destroy_space: load(gipa, instance, c"xrDestroySpace")?,
            locate_space: load(gipa, instance, c"xrLocateSpace")?,
            create_swapchain: load(gipa, instance, c"xrCreateSwapchain")?,
            destroy_swapchain: load(gipa, instance, c"xrDestroySwapchain")?,
            enumerate_swapchain_formats: load(gipa, instance, c"xrEnumerateSwapchainFormats")?,
            enumerate_swapchain_images: load(gipa, instance, c"xrEnumerateSwapchainImages")?,
            acquire_swapchain_image: load(gipa, instance, c"xrAcquireSwapchainImage")?,
            wait_swapchain_image: load(gipa, instance, c"xrWaitSwapchainImage")?,
            release_swapchain_image: load(gipa, instance, c"xrReleaseSwapchainImage")?,
        })
    }
}

/// The runtime's name, as it reports it.
unsafe fn runtime_name(gipa: pfn::GetInstanceProcAddr, instance: xr::Instance) -> Option<String> {
    let get: pfn::GetInstanceProperties = unsafe { load(gipa, instance, c"xrGetInstanceProperties")? };
    let mut props = xr::InstanceProperties {
        ty: xr::InstanceProperties::TYPE,
        next: std::ptr::null_mut(),
        runtime_version: xr::Version::new(0, 0, 0),
        runtime_name: [0; 128],
    };
    if unsafe { get(instance, &mut props) } != xr::Result::SUCCESS {
        return None;
    }
    Some(unsafe { CStr::from_ptr(props.runtime_name.as_ptr()) }.to_string_lossy().into_owned())
}

/// Log a reason the overlays are off and pass it on to the app.
fn problem(msg: &str) {
    log(&format!("Apex Overlays layer: {msg}"));
    if let Ok(mut s) = STATE.lock() {
        if let Some(s) = s.as_mut() {
            s.problem = msg.to_string();
        }
    }
}

// ---- Loader entry points ----------------------------------------------------------------

/// # Safety
/// Called by the OpenXR loader with valid negotiation structures.
#[no_mangle]
pub unsafe extern "system" fn xrNegotiateLoaderApiLayerInterface(
    loader_info: *const xr::NegotiateLoaderInfo,
    _layer_name: *const c_char,
    request: *mut xr::NegotiateApiLayerRequest,
) -> xr::Result {
    if loader_info.is_null() || request.is_null() {
        return xr::Result::ERROR_INITIALIZATION_FAILED;
    }
    let (li, req) = unsafe { (&*loader_info, &mut *request) };
    if li.struct_type != xr::LoaderInterfaceStructs::LOADER_INFO
        || req.struct_type != xr::LoaderInterfaceStructs::API_LAYER_REQUEST
        || li.min_interface_version > 1
        || li.max_interface_version < 1
    {
        return xr::Result::ERROR_INITIALIZATION_FAILED;
    }
    req.layer_interface_version = 1;
    req.layer_api_version = li.max_api_version;
    req.get_instance_proc_addr = Some(get_instance_proc_addr);
    req.create_api_layer_instance = Some(create_api_layer_instance);
    xr::Result::SUCCESS
}

unsafe extern "system" fn create_api_layer_instance(
    info: *const xr::InstanceCreateInfo,
    layer_info: *const xr::ApiLayerCreateInfo,
    instance: *mut xr::Instance,
) -> xr::Result {
    if layer_info.is_null() || unsafe { (*layer_info).next_info.is_null() } {
        return xr::Result::ERROR_INITIALIZATION_FAILED;
    }
    let next_info = unsafe { &*(*layer_info).next_info };
    let (Some(next_gipa), Some(next_create)) = (next_info.next_get_instance_proc_addr, next_info.next_create_api_layer_instance) else {
        return xr::Result::ERROR_INITIALIZATION_FAILED;
    };
    // The next layer gets the chain from the element after ours.
    let mut chained = unsafe { *layer_info };
    chained.next_info = next_info.next;
    let r = unsafe { next_create(info, &chained, instance) };
    if r != xr::Result::SUCCESS {
        return r;
    }
    if let Ok(mut g) = NEXT_GIPA.lock() {
        *g = Some(next_gipa);
    }
    let inst = unsafe { *instance };
    let next = catch_unwind(|| unsafe { resolve(next_gipa, inst) }).ok().flatten();
    if next.is_none() {
        log("Apex Overlays layer: could not find the runtime's functions; overlays off");
    }
    let runtime = catch_unwind(|| unsafe { runtime_name(next_gipa, inst) }).ok().flatten().unwrap_or_default();
    let problem = if next.is_none() { "the layer could not find the runtime's functions".to_string() } else { String::new() };
    if let Ok(mut s) = STATE.lock() {
        *s = Some(State { next, sessions: vec![], runtime: runtime.clone(), problem, status: Status::default() });
    }
    log(&format!("Apex Overlays layer {} loaded ({LAYER_NAME}) on the {runtime} runtime", env!("CARGO_PKG_VERSION")));
    xr::Result::SUCCESS
}

unsafe extern "system" fn get_instance_proc_addr(instance: xr::Instance, name: *const c_char, function: *mut Option<pfn::VoidFunction>) -> xr::Result {
    let Some(gipa) = NEXT_GIPA.lock().ok().and_then(|g| *g) else {
        return xr::Result::ERROR_HANDLE_INVALID;
    };
    let r = unsafe { gipa(instance, name, function) };
    if r != xr::Result::SUCCESS || name.is_null() || function.is_null() {
        return r;
    }
    let hooked = STATE.lock().ok().is_some_and(|s| s.as_ref().is_some_and(|s| s.next.is_some()));
    let ours: Option<pfn::VoidFunction> = match unsafe { CStr::from_ptr(name) }.to_bytes() {
        b"xrGetInstanceProcAddr" => Some(unsafe { std::mem::transmute::<pfn::GetInstanceProcAddr, pfn::VoidFunction>(get_instance_proc_addr) }),
        b"xrCreateSession" if hooked => Some(unsafe { std::mem::transmute::<pfn::CreateSession, pfn::VoidFunction>(create_session) }),
        b"xrDestroySession" if hooked => Some(unsafe { std::mem::transmute::<pfn::DestroySession, pfn::VoidFunction>(destroy_session) }),
        b"xrEndFrame" if hooked => Some(unsafe { std::mem::transmute::<pfn::EndFrame, pfn::VoidFunction>(end_frame) }),
        _ => None,
    };
    if let Some(f) = ours {
        unsafe { *function = Some(f) };
    }
    r
}

fn next_fns() -> Option<Next> {
    STATE.lock().ok().and_then(|s| s.as_ref().and_then(|s| s.next))
}

// ---- Session hooks ----------------------------------------------------------------------

unsafe extern "system" fn create_session(instance: xr::Instance, info: *const xr::SessionCreateInfo, session: *mut xr::Session) -> xr::Result {
    let Some(next) = next_fns() else { return xr::Result::ERROR_RUNTIME_FAILURE };
    let r = unsafe { (next.create_session)(instance, info, session) };
    if r != xr::Result::SUCCESS {
        return r;
    }
    let s = unsafe { *session };
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe { start_session(next, info, s) }));
    r
}

/// Destroy a panel's swapchain (its wrapped images first: they hold the D3D12 resources).
unsafe fn release(next: Next, p: PanelState) {
    let swapchain = p.swapchain;
    drop(p);
    unsafe { (next.destroy_swapchain)(swapchain) };
}

/// Remember a Direct3D 11 or 12 session and make the spaces panels are placed in.
unsafe fn start_session(next: Next, info: *const xr::SessionCreateInfo, session: xr::Session) {
    let mut p = unsafe { (*info).next } as *const xr::BaseInStructure;
    let mut device = None;
    let mut on12 = None;
    while !p.is_null() {
        let ty = unsafe { (*p).ty };
        if ty == xr::StructureType::GRAPHICS_BINDING_D3D11_KHR {
            let raw = unsafe { (*(p as *const xr::GraphicsBindingD3D11KHR)).device };
            device = unsafe { ID3D11Device::from_raw_borrowed(&raw) }.cloned();
            log("Apex Overlays layer: Direct3D 11 session");
        } else if ty == xr::StructureType::GRAPHICS_BINDING_D3D12_KHR {
            // Direct3D 12 (the game's, or another layer's such as NextLevelXR): a D3D11-on-12
            // device on the same command queue.
            let b = unsafe { &*(p as *const xr::GraphicsBindingD3D12KHR) };
            let (raw_device, raw_queue) = (b.device, b.queue);
            let d12 = unsafe { IUnknown::from_raw_borrowed(&raw_device) }.cloned();
            let queue = unsafe { IUnknown::from_raw_borrowed(&raw_queue) }.cloned();
            if let (Some(d12), Some(queue)) = (d12, queue) {
                let mut dev = None;
                let made =
                    unsafe { D3D11On12CreateDevice(&d12, D3D11_CREATE_DEVICE_BGRA_SUPPORT.0, None, Some(&[Some(queue)]), 0, Some(&mut dev), None, None) };
                match (made, dev) {
                    (Ok(()), Some(d)) => {
                        on12 = d.cast::<ID3D11On12Device>().ok();
                        device = Some(d);
                        log("Apex Overlays layer: Direct3D 12 session (through D3D11-on-12)");
                    }
                    (r, _) => problem(&format!("Direct3D 12 session but D3D11-on-12 failed ({r:?}); overlays off")),
                }
            }
        }
        p = unsafe { (*p).next };
    }
    let Some(device) = device else {
        problem("the game is not using Direct3D 11 or 12; overlays off");
        return;
    };
    let Ok(context) = (unsafe { device.GetImmediateContext() }) else { return };
    let space = |kind: xr::ReferenceSpaceType| {
        let ci = xr::ReferenceSpaceCreateInfo {
            ty: xr::ReferenceSpaceCreateInfo::TYPE,
            next: std::ptr::null(),
            reference_space_type: kind,
            pose_in_reference_space: IDENTITY,
        };
        let mut s = xr::Space::NULL;
        (unsafe { (next.create_reference_space)(session, &ci, &mut s) } == xr::Result::SUCCESS).then_some(s)
    };
    let (Some(local), Some(view)) = (space(xr::ReferenceSpaceType::LOCAL), space(xr::ReferenceSpaceType::VIEW)) else {
        problem("the runtime gave no LOCAL/VIEW space; overlays off");
        return;
    };
    // The formats the runtime offers for swapchains.
    let mut n = 0u32;
    let mut formats = vec![];
    if unsafe { (next.enumerate_swapchain_formats)(session, 0, &mut n, std::ptr::null_mut()) } == xr::Result::SUCCESS && n > 0 {
        formats = vec![0i64; n as usize];
        if unsafe { (next.enumerate_swapchain_formats)(session, n, &mut n, formats.as_mut_ptr()) } != xr::Result::SUCCESS {
            formats.clear();
        }
    }
    let format = FORMATS.iter().copied().find(|f| formats.contains(f));
    if format.is_none() {
        problem(&format!("the runtime offers no BGRA swapchain format ({formats:?}); overlays off"));
    }
    if let Ok(mut st) = STATE.lock() {
        if let Some(st) = st.as_mut() {
            st.sessions.push(SessionState {
                session,
                device,
                context,
                on12,
                local,
                view,
                format,
                panels: HashMap::new(),
                origin: IDENTITY,
                recenter: None,
                shm: None,
                shm_tried: None,
                quads: vec![],
                rejected: None,
            });
        }
    }
}

unsafe extern "system" fn destroy_session(session: xr::Session) -> xr::Result {
    let Some(next) = next_fns() else { return xr::Result::ERROR_RUNTIME_FAILURE };
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let mut st = STATE.lock().ok();
        if let Some(st) = st.as_mut().and_then(|s| s.as_mut()) {
            if let Some(i) = st.sessions.iter().position(|s| s.session == session) {
                let s = st.sessions.remove(i);
                unsafe {
                    for (_, p) in s.panels {
                        release(next, p);
                    }
                    (next.destroy_space)(s.local);
                    (next.destroy_space)(s.view);
                }
            }
        }
    }));
    unsafe { (next.destroy_session)(session) }
}

unsafe extern "system" fn end_frame(session: xr::Session, info: *const xr::FrameEndInfo) -> xr::Result {
    let Some(next) = next_fns() else { return xr::Result::ERROR_RUNTIME_FAILURE };
    if info.is_null() {
        return unsafe { (next.end_frame)(session, info) };
    }
    let mut guard = STATE.lock().ok();
    let Some(state) = guard.as_mut().and_then(|s| s.as_mut()) else {
        drop(guard);
        return unsafe { (next.end_frame)(session, info) };
    };
    let State { sessions, runtime, problem, status, .. } = state;
    let Some(st) = sessions.iter_mut().find(|s| s.session == session) else {
        // A session the layer cannot draw in: still tell the app the game is here, and why.
        status.report(0, 0, runtime, problem);
        drop(guard);
        return unsafe { (next.end_frame)(session, info) };
    };
    let api = if st.on12.is_some() { 12 } else { 11 };
    let original = unsafe { &*info };
    let backing_off = st.rejected.is_some_and(|t| t.elapsed().as_secs() < BACK_OFF_SECS);
    let added = if backing_off {
        0
    } else {
        catch_unwind(AssertUnwindSafe(|| unsafe { build(next, st, original) })).unwrap_or_else(|_| {
            log("Apex Overlays layer: error while adding overlays; frame passed on unchanged");
            0
        })
    };
    if added == 0 {
        status.report(0, api, runtime, problem);
        drop(guard);
        return unsafe { (next.end_frame)(session, info) };
    }
    // The game's layers first (they are drawn behind ours), then the overlays.
    let mut layers: Vec<*const xr::CompositionLayerBaseHeader> = Vec::with_capacity(original.layer_count as usize + added);
    if !original.layers.is_null() {
        for i in 0..original.layer_count as usize {
            layers.push(unsafe { *original.layers.add(i) });
        }
    }
    for q in &st.quads {
        layers.push(q as *const xr::CompositionLayerQuad as *const xr::CompositionLayerBaseHeader);
    }
    let mut ours = *original;
    ours.layer_count = layers.len() as u32;
    ours.layers = layers.as_ptr();
    let r = unsafe { (next.end_frame)(session, &ours) };
    if r.into_raw() >= 0 {
        if !problem.is_empty() && st.rejected.is_some() {
            problem.clear();
        }
        status.report(added as u32, api, runtime, problem);
        drop(guard);
        return r;
    }
    if matches!(
        r,
        xr::Result::ERROR_SESSION_NOT_RUNNING
            | xr::Result::ERROR_SESSION_LOST
            | xr::Result::ERROR_INSTANCE_LOST
            | xr::Result::ERROR_CALL_ORDER_INVALID
            | xr::Result::ERROR_HANDLE_INVALID
    ) {
        // About the game's session, not the overlays: the game handles it.
        status.report(0, api, runtime, problem);
        drop(guard);
        return r;
    }
    // The runtime refused the frame with the overlays in it: never cost the game a frame.
    // Send the game's own frame as it was, and leave the overlays out for a few seconds.
    st.rejected = Some(Instant::now());
    *problem = format!("the {runtime} runtime rejected the overlays ({r:?}); retrying every {BACK_OFF_SECS} s");
    log(&format!("Apex Overlays layer: {problem}"));
    status.report(0, api, runtime, problem);
    drop(guard);
    unsafe { (next.end_frame)(session, info) }
}

/// Update the panels from the app and build their quad layers; how many were added.
unsafe fn build(next: Next, st: &mut SessionState, info: &xr::FrameEndInfo) -> usize {
    st.quads.clear();
    let Some(format) = st.format else { return 0 };
    if st.shm.is_none() && st.shm_tried.is_none_or(|t| t.elapsed().as_secs() >= 2) {
        st.shm_tried = Some(Instant::now());
        st.shm = Shm::open();
    }
    let shared = st.shm.as_ref().and_then(|m| m.read());
    let Some(shared) = shared else {
        // The app is not running (or has stopped publishing): nothing to add.
        return 0;
    };
    if st.recenter != Some(shared.recenter) {
        // The recentre hotkey (and the first frame): the head as the seated position.
        if st.recenter.is_some() {
            let mut loc =
                xr::SpaceLocation { ty: xr::SpaceLocation::TYPE, next: std::ptr::null_mut(), location_flags: xr::SpaceLocationFlags::EMPTY, pose: IDENTITY };
            if unsafe { (next.locate_space)(st.view, st.local, info.display_time, &mut loc) } == xr::Result::SUCCESS
                && loc.location_flags.contains(xr::SpaceLocationFlags::POSITION_VALID | xr::SpaceLocationFlags::ORIENTATION_VALID)
            {
                st.origin = level(loc.pose);
            }
        }
        st.recenter = Some(shared.recenter);
    }
    let room = MAX_LAYERS.saturating_sub(info.layer_count as usize);
    let panels: Vec<Panel> = shared.panels().iter().filter(|p| p.texture != 0 && p.width > 0 && p.height > 0).take(room).copied().collect();
    // Panels the app no longer shows.
    let gone: Vec<u64> = st.panels.keys().copied().filter(|k| !panels.iter().any(|p| p.key == *k)).collect();
    for k in gone {
        if let Some(p) = st.panels.remove(&k) {
            unsafe { release(next, p) };
        }
    }
    for p in &panels {
        let Some(ps) = (unsafe { panel_state(next, st, format, p) }) else { continue };
        if !ps.ready {
            continue;
        }
        let pose = xr::Posef {
            orientation: xr::Quaternionf { x: p.orient[0], y: p.orient[1], z: p.orient[2], w: p.orient[3] },
            position: xr::Vector3f { x: p.pos[0], y: p.pos[1], z: p.pos[2] },
        };
        let swapchain = ps.swapchain;
        let (head, origin) = (p.head == 1, st.origin);
        st.quads.push(xr::CompositionLayerQuad {
            ty: xr::CompositionLayerQuad::TYPE,
            next: std::ptr::null(),
            layer_flags: xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA,
            space: if head { st.view } else { st.local },
            eye_visibility: xr::EyeVisibility::BOTH,
            sub_image: xr::SwapchainSubImage {
                swapchain,
                image_rect: xr::Rect2Di { offset: xr::Offset2Di { x: 0, y: 0 }, extent: xr::Extent2Di { width: p.width as i32, height: p.height as i32 } },
                image_array_index: 0,
            },
            pose: {
                let mut pose = if head { pose } else { compose(origin, pose) };
                pose.orientation = normalized(pose.orientation);
                pose
            },
            size: xr::Extent2Df { width: p.size[0], height: p.size[1] },
        });
    }
    st.quads.len()
}

/// The panel's swapchain, (re)made to its size, with its newest frame copied in.
unsafe fn panel_state<'a>(next: Next, st: &'a mut SessionState, format: i64, p: &Panel) -> Option<&'a PanelState> {
    let resized = st.panels.get(&p.key).is_some_and(|s| s.width != p.width || s.height != p.height);
    if resized {
        if let Some(old) = st.panels.remove(&p.key) {
            unsafe { release(next, old) };
        }
    }
    if !st.panels.contains_key(&p.key) {
        let ci = xr::SwapchainCreateInfo {
            ty: xr::SwapchainCreateInfo::TYPE,
            next: std::ptr::null(),
            create_flags: xr::SwapchainCreateFlags::EMPTY,
            usage_flags: xr::SwapchainUsageFlags::SAMPLED | xr::SwapchainUsageFlags::COLOR_ATTACHMENT,
            format,
            sample_count: 1,
            width: p.width,
            height: p.height,
            face_count: 1,
            array_size: 1,
            mip_count: 1,
        };
        let mut swapchain = xr::Swapchain::NULL;
        if unsafe { (next.create_swapchain)(st.session, &ci, &mut swapchain) } != xr::Result::SUCCESS {
            log(&format!("Apex Overlays layer: could not create a {}x{} swapchain", p.width, p.height));
            return None;
        }
        let mut n = 0u32;
        unsafe { (next.enumerate_swapchain_images)(swapchain, 0, &mut n, std::ptr::null_mut()) };
        let (ok, wanted, images) = match &st.on12 {
            None => {
                let mut raw =
                    vec![
                        xr::SwapchainImageD3D11KHR { ty: xr::SwapchainImageD3D11KHR::TYPE, next: std::ptr::null_mut(), texture: std::ptr::null_mut() };
                        n as usize
                    ];
                let ok = n > 0
                    && unsafe { (next.enumerate_swapchain_images)(swapchain, n, &mut n, raw.as_mut_ptr() as *mut xr::SwapchainImageBaseHeader) }
                        == xr::Result::SUCCESS;
                let images: Vec<ID3D11Texture2D> = raw.iter().filter_map(|i| unsafe { ID3D11Texture2D::from_raw_borrowed(&i.texture) }.cloned()).collect();
                (ok, raw.len(), images)
            }
            Some(on12) => {
                // Direct3D 12 images, wrapped as D3D11 textures (render-target state in and out,
                // as OpenXR expects colour images to be when released).
                let mut raw =
                    vec![
                        xr::SwapchainImageD3D12KHR { ty: xr::SwapchainImageD3D12KHR::TYPE, next: std::ptr::null_mut(), texture: std::ptr::null_mut() };
                        n as usize
                    ];
                let ok = n > 0
                    && unsafe { (next.enumerate_swapchain_images)(swapchain, n, &mut n, raw.as_mut_ptr() as *mut xr::SwapchainImageBaseHeader) }
                        == xr::Result::SUCCESS;
                let flags = D3D11_RESOURCE_FLAGS { BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32, MiscFlags: 0, CPUAccessFlags: 0, StructureByteStride: 0 };
                let images: Vec<ID3D11Texture2D> = raw
                    .iter()
                    .filter_map(|i| {
                        let r = i.texture;
                        let res = unsafe { IUnknown::from_raw_borrowed(&r) }?;
                        let mut out: Option<ID3D11Texture2D> = None;
                        unsafe { on12.CreateWrappedResource(res, &flags, D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_STATE_RENDER_TARGET, &mut out) }
                            .ok()?;
                        out
                    })
                    .collect();
                (ok, raw.len(), images)
            }
        };
        if !ok || images.len() != wanted {
            unsafe { (next.destroy_swapchain)(swapchain) };
            return None;
        }
        st.panels.insert(p.key, PanelState { swapchain, width: p.width, height: p.height, images, texture: 0, source: None, frame: 0, ready: false });
    }
    let ps = st.panels.get_mut(&p.key)?;
    if ps.texture != p.texture {
        // Shared by name by the app (works on Direct3D 11 and on D3D11-on-12 devices).
        let name = HSTRING::from(apex_xr_shared::texture_name(p.texture));
        let access = DXGI_SHARED_RESOURCE_READ.0 | DXGI_SHARED_RESOURCE_WRITE.0;
        let tex = st.device.cast::<ID3D11Device1>().and_then(|d| unsafe { d.OpenSharedResourceByName::<_, ID3D11Texture2D>(&name, access) });
        let Ok(tex) = tex else {
            log("Apex Overlays layer: could not open an overlay's shared texture");
            return Some(ps);
        };
        ps.texture = p.texture;
        ps.source = Some(tex);
        ps.frame = p.frame.wrapping_sub(1);
    }
    if ps.frame == p.frame && ps.ready {
        return Some(ps);
    }
    let Some(source) = ps.source.clone() else { return Some(ps) };
    let mut index = 0u32;
    if unsafe { (next.acquire_swapchain_image)(ps.swapchain, std::ptr::null(), &mut index) } != xr::Result::SUCCESS {
        return Some(ps);
    }
    let wait = xr::SwapchainImageWaitInfo { ty: xr::SwapchainImageWaitInfo::TYPE, next: std::ptr::null(), timeout: xr::Duration::from_nanos(100_000_000) };
    if unsafe { (next.wait_swapchain_image)(ps.swapchain, &wait) } == xr::Result::SUCCESS {
        if let Some(dst) = ps.images.get(index as usize) {
            // The app writes the texture under its keyed mutex (key 0); copy only while we hold it.
            let mutex: Option<IDXGIKeyedMutex> = source.cast().ok();
            let locked = mutex.as_ref().map(|m| unsafe { (Interface::vtable(m).AcquireSync)(Interface::as_raw(m), 0, 4) }.0 == 0);
            if locked != Some(false) {
                let wrapped: Option<ID3D11Resource> = st.on12.as_ref().and_then(|_| dst.cast().ok());
                if let (Some(on12), Some(w)) = (&st.on12, &wrapped) {
                    unsafe { on12.AcquireWrappedResources(&[Some(w.clone())]) };
                }
                unsafe { st.context.CopyResource(dst, &source) };
                if let (Some(on12), Some(w)) = (&st.on12, &wrapped) {
                    // Hand the image back to Direct3D 12 and submit the copy on the game's queue.
                    unsafe { on12.ReleaseWrappedResources(&[Some(w.clone())]) };
                    unsafe { st.context.Flush() };
                }
                if let (Some(m), Some(true)) = (mutex.as_ref(), locked) {
                    let _ = unsafe { m.ReleaseSync(0) };
                }
                ps.frame = p.frame;
            }
        }
        let release = xr::SwapchainImageReleaseInfo { ty: xr::SwapchainImageReleaseInfo::TYPE, next: std::ptr::null() };
        if unsafe { (next.release_swapchain_image)(ps.swapchain, &release) } == xr::Result::SUCCESS {
            ps.ready = true;
        }
    }
    Some(ps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poses_compose() {
        // Origin turned 90° left (yaw +90° about Y) and 1 m up: a panel 1 m in front of it
        // ends up 1 m to the left in the room.
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let origin = xr::Posef { orientation: xr::Quaternionf { x: 0.0, y: s, z: 0.0, w: s }, position: xr::Vector3f { x: 0.0, y: 1.0, z: 0.0 } };
        let panel = xr::Posef { orientation: IDENTITY.orientation, position: xr::Vector3f { x: 0.0, y: 0.0, z: -1.0 } };
        let p = compose(origin, panel);
        assert!((p.position.x + 1.0).abs() < 1e-5 && (p.position.y - 1.0).abs() < 1e-5 && p.position.z.abs() < 1e-5, "{:?}", p.position);
        // Levelling keeps the yaw and drops the tilt.
        let tilted = xr::Posef { orientation: qmul(origin.orientation, xr::Quaternionf { x: 0.2588, y: 0.0, z: 0.0, w: 0.9659 }), position: origin.position };
        let l = level(tilted);
        assert!((l.orientation.y - s).abs() < 1e-3 && l.orientation.x.abs() < 1e-6, "{:?}", l.orientation);
    }
}
