use winres::WindowsResource;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = WindowsResource::new();
        res.set_language(winapi::um::winnt::MAKELANGID(
            winapi::um::winnt::LANG_ENGLISH,
            winapi::um::winnt::SUBLANG_ENGLISH_US,
        ));
        res.set_icon("icon.ico");
        res.compile().unwrap();
    }
}
