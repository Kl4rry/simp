#version 440

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 tex_coords;
layout(location = 0) out vec2 v_tex_coords;

struct InputUniform {
    mat4 matrix;
    vec2 size;
    float hue;
    float contrast;
    float brightness;
    float saturation;
    uint grayscale;
    uint invert;
};

layout(set = 0, binding = 0) uniform InputUniform input;

void main() {
	v_tex_coords = tex_coords;
    gl_Position = input.matrix * vec4(position, 0.0, 1.0);
}
