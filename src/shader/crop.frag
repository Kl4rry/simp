#version 150

out vec4 color;

uniform vec2 start;
uniform vec2 end;
uniform vec2 size;

const vec4 background_color = vec4(0.0, 0.0, 0.0, 0.5);
const vec4 transparent = vec4(0.0, 0.0, 0.0, 0.0);
const vec4 line_color = vec4(0.0, 0.0, 0.0, 1.0);

void main() {
	float x = gl_FragCoord[0];
	float y = gl_FragCoord[1];

	vec2 start_inv = start;
	vec2 end_inv = end;

	start_inv.y = size.y - start.y;
	end_inv.y = size.y - end.y;

	if(start_inv.y < end_inv.y) {
		float temp = start_inv.y;
		start_inv.y = end_inv.y;
		end_inv.y = temp;
	}

	if(start_inv.x > end_inv.x) {
		float temp = start_inv.x;
		start_inv.x = end_inv.x;
		end_inv.x = temp;
	}

	vec2 start_outer = start_inv + vec2(-2.0, 2.0);
	vec2 end_outer = end_inv + vec2(2.0, -2.0);

	bool line = false;

	if(x > start_outer.x && x < end_outer.x && y < start_outer.y && y > end_outer.y) {
		color = line_color;
		line = true;
	}

	if(x > start_inv.x && x < end_inv.x && y < start_inv.y && y > end_inv.y) {
		color = transparent;
	} else {
		if(!line) {
			color = background_color;
		}
	}
}
