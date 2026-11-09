fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icon_path = "../../assets/melody.ico";

        println!("cargo:rerun-if-changed={icon_path}");

        let mut res = winres::WindowsResource::new();
        res.set_icon(icon_path);
        res.set("FileDescription", "Melody");
        res.set("ProductName", "Melody");
        res.set("OriginalFilename", "melody.exe");
        res.set("InternalName", "melody.exe");

        if let Ok(version) = std::env::var("CARGO_PKG_VERSION") {
            res.set("ProductVersion", &version);
            res.set("FileVersion", &version);
        }

        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resource: {e}");
        }
    }
}
