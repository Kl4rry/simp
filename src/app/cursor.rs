use glium::{backend::glutin::Display, glutin::window::CursorIcon};

pub fn set_cursor_icon(icon: CursorIcon, display: &Display) {
    let window_context = display.gl_window();
    let window = window_context.window();
    window.set_cursor_icon(icon);
}
