use std::process::Command;

#[cfg(target_os = "windows")]
use winres::WindowsResource;

#[cfg(target_os = "windows")]
const MANIFEST_CONTENT: &str = r#"
    <?xml version="1.0" encoding="UTF-8" standalone="yes"?> 
    <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
        <description>simp comctl32 manifest</description> 
        <dependency>
            <dependentAssembly>
                <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" /> 
            </dependentAssembly>
        </dependency>
    </assembly>
"#;

#[cfg(target_os = "windows")]
fn compile_icon() {
    let mut res = WindowsResource::new();
    res.set_manifest(MANIFEST_CONTENT);
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

    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
