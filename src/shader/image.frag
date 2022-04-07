#version 150

in vec2 v_tex_coords;
out vec4 color;

uniform sampler2D tex;
uniform vec2 size;
uniform float hue = 0.0;
uniform float contrast = 0.0;
uniform float lightness = 0.0;
uniform float saturation = 0.0;

const float PI = 3.141592653589793238462643383279502884197169399375105820974944;
const float max_value = 255;

// https://gist.github.com/ciembor/1494530
vec3 rgb2hsl(vec3 rgb) {
    vec3 result;

    float r = rgb.r;
    float g = rgb.g;
    float b = rgb.b;

    float max = max(max(r, g), b);
    float min = min(min(r, g), b);

    result.x = result.y = result.z = (max + min) / 2;

    if(max == min) {
        result.x = result.y = 0; // achromatic
    } else {
        float d = max - min;
        result.y = (result.z > 0.5) ? d / (2 - max - min) : d / (max + min);

        if(max == r) {
            result.x = (g - b) / d + (g < b ? 6 : 0);
        } else if(max == g) {
            result.x = (b - r) / d + 2;
        } else if(max == b) {
            result.x = (r - g) / d + 4;
        }
        result.x /= 6;
    }

    return result;
}

vec3 hsl2rgb(in vec3 c) {
    vec3 rgb = clamp(abs(mod(c.x * 6.0 + vec3(0.0, 4.0, 2.0), 6.0) - 3.0) - 1.0, 0.0, 1.0);
    return c.z + c.y * (rgb - 0.5) * (1.0 - abs(2.0 * c.z - 1.0));
}

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

    float matrix[9] = float[] (
        // Reds
    0.213 + cosv * 0.787 - sinv * 0.213, 0.715 - cosv * 0.715 - sinv * 0.715, 0.072 - cosv * 0.072 + sinv * 0.928,
        // Greens
    0.213 - cosv * 0.213 + sinv * 0.143, 0.715 + cosv * 0.285 + sinv * 0.140, 0.072 - cosv * 0.072 - sinv * 0.283,
        // Blues
    0.213 - cosv * 0.213 - sinv * 0.787, 0.715 - cosv * 0.715 + sinv * 0.715, 0.072 + cosv * 0.928 + sinv * 0.072);

    float new_r = matrix[0] * p.r + matrix[1] * p.g + matrix[2] * p.b;
    float new_g = matrix[3] * p.r + matrix[4] * p.g + matrix[5] * p.b;
    float new_b = matrix[6] * p.r + matrix[7] * p.g + matrix[8] * p.b;

    return vec3(clamp(new_r, 0, max_value), clamp(new_g, 0, max_value), clamp(new_b, 0, max_value));
}

float adjustContrastPixel(float c, float percent) {
    c = c * max_value;
    float d = ((c / max_value - 0.5) * percent + 0.5) * max_value;
    float e = clamp(d, 0.0, max_value);
    return e / max_value;
}

vec3 adjustContrast(vec3 p, float contrast) {
    float percent = pow((100.0 + contrast) / 100.0, 2);
    float new_r = adjustContrastPixel(p.r, percent);
    float new_g = adjustContrastPixel(p.g, percent);
    float new_b = adjustContrastPixel(p.b, percent);
    return vec3(new_r, new_g, new_b);
}

vec3 lighten(vec3 p, float value) {
    float light = (value / 100);
    vec3 hsl = rgb2hsl(p);
    hsl.z = clamp(hsl.z + light, 0, 1);
    return hsl2rgb(hsl);
}

vec3 adjustSaturation(vec3 p, float sat) {
    sat = sat / 100;
    vec3 hsl = rgb2hsl(p);
    float s = hsl.y;
    float factor = (1.0 - s) * sat;
    hsl.y = clamp(s + factor, 0, 1);
    return hsl2rgb(hsl);
}

vec3 getCheckColor() {
    vec3 color1 = vec3(64, 64, 64) / max_value;
    vec3 color2 = vec3(48, 48, 48) / max_value;

    float checkSize = 12.0;
    float x = floor(gl_FragCoord[0] / checkSize);
    float y = floor((gl_FragCoord[1] - size.y) / checkSize);

    if(mod(x + y, 2) == 0) {
        return color1;
    } else {
        return color2;
    }
}

void main() {
    vec4 p = texture(tex, v_tex_coords);
    p.rgb = gammaCorrection(p.rgb, 2.2);

    p.rgb = rotateHue(p.rgb, hue);
    p.rgb = adjustContrast(p.rgb, contrast);
    p.rgb = lighten(p.rgb, lightness);
    p.rgb = adjustSaturation(p.rgb, saturation);

    vec3 check_color = getCheckColor();
    color.rgb = check_color * (1 - p.a) + p.a * p.rgb;
    color.rgb = inverseGamma(color.rgb, 2.2);
    color.a = 1;
}
