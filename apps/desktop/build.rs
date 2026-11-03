use std::path::PathBuf;

fn main() {
    // The OpenXR layer DLL (apps/xr-layer, built with `cargo build --profile layer -p
    // apex-xr-layer`) is embedded in the app, which installs it. A build without it embeds
    // nothing and the app says OpenXR is not available.
    println!("cargo:rerun-if-env-changed=APEX_XR_LAYER_DLL");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let found = std::env::var("APEX_XR_LAYER_DLL").ok().map(PathBuf::from).filter(|p| p.exists()).or_else(|| {
        // <target>/<profile>/build/<pkg>/out → look for <target>/layer/apex_xr_layer.dll upwards.
        out.ancestors().map(|a| a.join("layer").join("apex_xr_layer.dll")).find(|p| p.exists())
    });
    let dest = out.join("apex_xr_layer.dll");
    match found {
        Some(p) => {
            println!("cargo:rerun-if-changed={}", p.display());
            std::fs::copy(&p, &dest).expect("copy the OpenXR layer DLL");
        }
        None => std::fs::write(&dest, []).expect("write the OpenXR layer placeholder"),
    }
    tauri_build::build()
}
