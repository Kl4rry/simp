#version 440

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 tex_coords;
layout(location = 0) out vec2 v_tex_coords;

struct InputUniform {
    mat4 matrix;
    uint flip_horizontal;
    uint flip_vertical;
    float hue;
    float contrast;
    float brightness;
    float saturation;
    uint grayscale;
    uint invert;
};

vec2 size = vec2(1920, 1080);


layout(set = 0, binding = 0) uniform InputUniform input;

void main() {
	v_tex_coords = tex_coords;
    if(bool(input.flip_horizontal)) {
        v_tex_coords.x = 1 - v_tex_coords.x;
    }
    if(bool(input.flip_vertical)) {
        v_tex_coords.y = 1 - v_tex_coords.y;
    }
    gl_Position = input.matrix * vec4(position, 0.0, 1.0);
}
