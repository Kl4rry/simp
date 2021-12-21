#version 140

in vec2 v_tex_coords;
out vec4 color;

uniform sampler2D tex;
uniform float hue = 0.0;
uniform float contrast = 0.0;

const float PI = 3.141592653589793238462643383279502884197169399375105820974944;
const float max = 255;

vec3 gammaCorrection(vec3 color, float gamma) {
    return pow(color, vec3(1. / gamma));
}

vec3 inverseGamma(vec3 color, float gamma) {
    return pow(color, vec3(gamma));
}

// this function is pretty much a line by line translation of the image-rs hue rotate function
// all color changing functions must have the exact same behavior as the image-rs functions
vec3 rotateHue(vec3 p, float hue) {
    float cosv = cos(hue * PI / 180.0);
    float sinv = sin(hue * PI / 180.0);

    float matrix[9] = float[](
        // Reds
        0.213 + cosv * 0.787 - sinv * 0.213,
        0.715 - cosv * 0.715 - sinv * 0.715,
        0.072 - cosv * 0.072 + sinv * 0.928,
        // Greens
        0.213 - cosv * 0.213 + sinv * 0.143,
        0.715 + cosv * 0.285 + sinv * 0.140,
        0.072 - cosv * 0.072 - sinv * 0.283,
        // Blues
        0.213 - cosv * 0.213 - sinv * 0.787,
        0.715 - cosv * 0.715 + sinv * 0.715,
        0.072 + cosv * 0.928 + sinv * 0.072
    );

    float new_r = matrix[0] * p.r + matrix[1] * p.g + matrix[2] * p.b;
    float new_g = matrix[3] * p.r + matrix[4] * p.g + matrix[5] * p.b;
    float new_b = matrix[6] * p.r + matrix[7] * p.g + matrix[8] * p.b;

    return vec3(clamp(new_r, 0, max), clamp(new_g, 0, max), clamp(new_b, 0, max));
}

float adjustContrastPixel(float c, float percent) {
    float d = ((c / max - 0.5) * percent + 0.5) * max;
    float e = clamp(d, 0.0, max);
    return e;
}

vec3 adjustContrast(vec3 p, float contrast) {
    float percent = pow((100.0 + contrast) / 100.0, 2);
    percent = 100;
    float new_r = adjustContrastPixel(p.r, percent);
    float new_g = adjustContrastPixel(p.g, percent);
    float new_b = adjustContrastPixel(p.b, percent);
    return vec3(new_r, new_g, new_b);
}

void main() {
    vec4 p = texture(tex, v_tex_coords);
    p.rgb = gammaCorrection(p.rgb, 2.2);

    p.rgb = rotateHue(p.rgb, hue);
    //p.rgb = adjustContrast(p.rgb, contrast);

    color.rgb = inverseGamma(p.rgb, 2.2);
    color.a = p.a;
}