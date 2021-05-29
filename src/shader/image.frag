#version 140

in vec2 v_tex_coords;
out vec4 color;

uniform mat4 matrix;
uniform sampler2D tex;

void main() {
    color = texture(tex, v_tex_coords);
}