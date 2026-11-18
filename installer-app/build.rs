use std::io::Write;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let exe = std::path::Path::new(&manifest).join("..").join("target").join("release").join("ferro.exe");
    println!("cargo:rerun-if-changed={}", exe.display());
    let bytes = std::fs::read(&exe).expect("build ferro first: cargo build --release in the ferro folder");
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("payload.gz");
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    enc.write_all(&bytes).unwrap();
    std::fs::write(out, enc.finish().unwrap()).unwrap();
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../assets/icon.ico");
        res.set("ProductName", "Ferro Setup");
        res.set("FileDescription", "Ferro Setup");
        let _ = res.compile();
    }
}
