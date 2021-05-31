#[cfg(target_os = "windows")]
use winres::WindowsResource;

#[cfg(target_os = "windows")]
fn compile_icon() {
    let mut res = WindowsResource::new();
    res.set_language(winapi::um::winnt::MAKELANGID(
        winapi::um::winnt::LANG_ENGLISH,
        winapi::um::winnt::SUBLANG_ENGLISH_US,
    ));
    res.set_icon("icon.ico");
    res.compile().unwrap();
}

fn main() {
    #[cfg(target_os = "windows")]
    compile_icon();
}
