
https://www.khronos.org/opengl/wiki/Fragment_Shader
https://stackoverflow.com/questions/4694608/glsl-checkerboard-pattern/4712625
vec3 checker(in float u, in float v)
{
  float checkSize = 10.0;
  float fmodResult = mod(floor(checkSize * u) + floor(checkSize * v), 2.0);
  float fin = max(sign(fmodResult), 0.0);
  return vec3(fin, fin, fin);
}