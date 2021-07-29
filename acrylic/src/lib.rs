use winapi::{
    um::{libloaderapi::{GetProcAddress, LoadLibraryA}},
    shared::{
        minwindef::BOOL,
        ntdef::{ULONG, LPCSTR},
        windef::HWND,
    },
    ctypes::c_void
};
use raw_window_handle::RawWindowHandle;

#[allow(unused)]
#[repr(C)]
enum AccentState {
    AccentDisabled = 0,
    AccentEnableGradient = 1,
    AccentEnableTransparentgradient = 2,
    AccentEnableBlurbehind = 3,
    AccentEnableAcrylicblurbehind = 4,
    AccentEnableHostbackdrop = 5, // RS5 1809
    AccentInvalidState = 6,
}

#[repr(C)]
enum WindowCompositionAttribute {
    WcaAccentPolicy = 19,
}

#[repr(C)]
struct AccentPolicy {
    accent_state: AccentState,
    accent_flags: i32,
    gradient_color: i32,
    animation_id: i32,
}

#[repr(C)]
pub struct WindowCompositionAttributeData {
    attribute: WindowCompositionAttribute,
    p_data: *const AccentPolicy,
    data_size: ULONG,
}

fn get_function_impl(library: &str, function: &str) -> Option<*const c_void> {
    assert_eq!(library.chars().last(), Some('\0'));
    assert_eq!(function.chars().last(), Some('\0'));

    // Library names we will use are ASCII so we can use the A version to avoid string conversion.
    let module = unsafe { LoadLibraryA(library.as_ptr() as LPCSTR) };
    if module.is_null() {
        return None;
    }

    let function_ptr = unsafe { GetProcAddress(module, function.as_ptr() as LPCSTR) };
    if function_ptr.is_null() {
        return None;
    }

    Some(function_ptr as _)
}

macro_rules! get_function {
    ($lib:expr, $func:ident) => {
        get_function_impl(
            concat!($lib, '\0'),
            concat!(stringify!($func), '\0'),
        )
        .map(|f| unsafe { std::mem::transmute::<*const _, $func>(f) })
    };
}

type SetWindowCompositionAttribute =
    unsafe extern "system" fn(HWND, *mut WindowCompositionAttributeData) -> BOOL;

lazy_static::lazy_static! {
    static ref SET_WINDOW_COMPOSITION_ATTRIBUTE: Option<SetWindowCompositionAttribute> =
        get_function!("user32.dll", SetWindowCompositionAttribute);
}

const RED_VALUE: i32 = 34;
const BLUE_VALUE: i32 = 34;
const GREEN_VALUE: i32 = 34;
const ALPHA_VALUE: i32 = 136;

pub fn set_acrylic(raw: &RawWindowHandle) {
    if let RawWindowHandle::Windows(handle) = raw {
        let hwnd = handle.hwnd;
        let gradient_color: i32 =
            (ALPHA_VALUE << 24) + (BLUE_VALUE << 16) + (GREEN_VALUE << 8) + (RED_VALUE);
        let blur_type = AccentState::AccentEnableBlurbehind;
        let policy = AccentPolicy {
            accent_state: blur_type,
            accent_flags: 2,
            gradient_color,
            animation_id: 0,
        };
        let mut data = WindowCompositionAttributeData {
            attribute: WindowCompositionAttribute::WcaAccentPolicy,
            p_data: &policy,
            data_size: std::mem::size_of::<AccentPolicy>() as u32,
        };
        unsafe {
            if let Some(set_window_composition_attribute) = *SET_WINDOW_COMPOSITION_ATTRIBUTE {
                set_window_composition_attribute(hwnd as HWND, &mut data);
            }
            //
        }
        
        //let policy = AccentPolicy { nAccentState: 3, nFlags: 0, nColor: 0, nAnimationId: 0 };
    }
}
