#version 440

out vec4 color;

struct InputUniform {
	vec2 start;
	vec2 end;
	vec2 size;
};

layout(set = 0, binding = 0) uniform InputUniform input_uniform;

const vec4 background_color = vec4(0.0, 0.0, 0.0, 0.5);
const vec4 transparent = vec4(0.0, 0.0, 0.0, 0.0);
const vec4 line_color1 = vec4(0.0, 0.0, 0.0, 1.0);
const vec4 line_color2 = vec4(1.0, 1.0, 1.0, 1.0);

vec3 inverseGamma(vec3 color, float gamma) {
    return pow(color, vec3(gamma));
}

vec4 blend(vec4 base, vec4 top) {
	vec3 rgb = base.rgb * (1 - top.a) + top.a * top.rgb;
	float alpha = base.a + top.a * (1 - base.a);
	return vec4(rgb, alpha);
}

void main() {
	vec2 start = input_uniform.start;
	vec2 end = input_uniform.end;
	vec2 size = input_uniform.size;

	color = vec4(0, 0, 0, 0);
	float x = gl_FragCoord.x;
	float y = gl_FragCoord.y;

	vec2 start_outer = round(start + vec2(-1.1, -1.1));
	vec2 end_outer = round(end + vec2(1.1, 1.1));

	bool line = false;

	if(!(x < start_outer.x || x > end_outer.x || y < start_outer.y || y > end_outer.y)) {
		if(floor(x) == floor(start_outer.x) || floor(x) == floor(end_outer.x) - 1) {
			if(mod(round((y - start.y) / 5), 2) == 0) {
				color = line_color1;
			} else {
				color = line_color2;
			}
			line = true;
		} else if(floor(y) == floor(start_outer.y) || floor(y) == floor(end_outer.y) - 1) {
			if(mod(round((x - start.x) / 5), 2) == 0) {
				color = line_color1;
			} else {
				color = line_color2;
			}
			line = true;
		}
	}

	if(x < start.x || x > end.x || y < start.y || y > end.y) {
		if(!line) {
			vec4 dark = vec4(0.0, 0.0, 0.0, 0.7);
			color = blend(color, dark);
		}
	} else {
		color = transparent;
	}

	vec4 blue = vec4(inverseGamma(vec3(0.180, 0.737, 0.917), 2.2), 1.0);
	float radius = 5;

	vec2 dots[4] = vec2[4](start, vec2(start.x, end.y), vec2(end.x, start.y), end);

	for(int i = 0; i < dots.length(); i++) {
		vec2 pos = dots[i];
		float diff = length(pos - vec2(x, y));
		if(diff < radius) {
			if(diff < radius - 1) {
				color = blue;
			} else {
				color = blend(color, vec4(blue.rgb, radius - diff));
			}
		}
	}
}
