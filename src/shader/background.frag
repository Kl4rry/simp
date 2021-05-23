#version 140

out vec4 color;
uniform vec2 size;

void main() {
	vec4 color1 = vec4(0.2, 0.219, 0.258, 1.0);
	vec4 color2 = vec4(0.262, 0.286, 0.337, 1.0);

	float checkSize = 10.0;
	float x = floor(gl_FragCoord[0] / checkSize);
	float y = floor((gl_FragCoord[1] - size.y) / checkSize);
	
	if(mod(x + y, 2) == 0) {
		color = color1;
	} else {
		color = color2;
	}
}