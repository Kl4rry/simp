#version 150

in vec2 position;
in vec2 tex_coords;
out vec2 v_tex_coords;

uniform mat4 matrix;
uniform bool flip_horizontal;
uniform bool flip_vertical;

void main() {
	v_tex_coords = tex_coords;
    if(flip_horizontal) {
        v_tex_coords.x = 1 - v_tex_coords.x;
    }
    if(flip_vertical) {
        v_tex_coords.y = 1 - v_tex_coords.y;
    }
    gl_Position =  matrix * vec4(position, 0.0, 1.0);
}
