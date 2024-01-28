#version 440

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 tex_coords;
layout(location = 0) out vec2 v_tex_coords;

layout(std140, set = 0, binding = 0) uniform InputUniform {
    mat4 matrix;
    vec2 size;
    float hue;
    float contrast;
    float brightness;
    float saturation;
    uint grayscale;
    uint invert;
};

void main() {
	v_tex_coords = tex_coords;
    gl_Position = matrix * vec4(position, 0.0, 1.0);
}
