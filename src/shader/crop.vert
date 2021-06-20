#version 140

in vec2 position;
uniform mat4 matrix;

void main() {
    gl_Position =  matrix * vec4(position, 0.0, 1.0);
}