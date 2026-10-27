fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../assets/melody.ico");
        res.set("FileDescription", "Melody");
        res.set("ProductName", "Melody");
        res.set("OriginalFilename", "melody.exe");
        res.set("InternalName", "melody.exe");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resource: {e}");
        }
    }
}
