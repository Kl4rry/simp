
use glium::{glutin::window::CursorIcon, backend::glutin::Display};

pub fn set_cursor_icon(icon: CursorIcon, display: &Display) {
    let window_context = display.gl_window();
    let window = window_context.window();
    window.set_cursor_icon(icon);
}