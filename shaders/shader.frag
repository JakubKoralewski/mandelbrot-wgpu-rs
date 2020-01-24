#version 450
precision highp float;

float r = pow(2.0, 3.0);
float log_r = log(r);

layout(set = 0, binding = 0) uniform WindowSize { // https://github.com/gfx-rs/wgpu-rs/blob/v0.4/examples/shadow/forward.frag
    vec2 size;
};

layout(set = 0, binding = 1) uniform Zoom {
    float zoom;
};

layout(set = 0, binding = 2) uniform Pos {
    vec2 pos;
};

layout(location = 0) out vec4 outColor;

vec2 square(vec2 z) {
    return vec2(pow(z.x, 2.0) - pow(z.y, 2.0), 2.0 * z.x * z.y);
}

float iterations(vec2 c, float zoom) {
    vec2 z = c;
    float y_2 = pow(z.y, 2);
    float q = pow(z.x - 1/4, 2) + y_2;
    float cardioid = q * (q + (z.x - 1/4));
    bool in_cardioid = cardioid <= y_2/4;
    if (!in_cardioid) {
        return 0.0;
    }
    float period2bulb = pow(z.x + 1, 2) + y_2;
    bool in_period2bulb = period2bulb <= 1/16;
    if (in_period2bulb) {
        return 0.0;
    }
    for (int i = 0; i < 500; i++) {
        z = square(z) + c;
        float len = length(z);
        if (len > r) return float(i) - log(len) / log_r;
    }
    return 0.0;
}
float hue2c(float p, float q, float t, int modifier) {
    t = mod(t + float(modifier), 6.0);
    if (t < 1.0) return p + (q - p) * t;
    if (t < 3.0) return q;
    if (t < 4.0) return p + (q - p) * (4.0 - t);
    return p;
}
vec4 hslToRgba(float h, float s, float l) {
    if (s == 0.0) return vec4(l, l, l, 1.0);
    float q = l < 0.5 ? l * (1.0 + s) : l + s - l * s;
    float p = 2.0 * l - q;
    h *= 6.0;
    return vec4(hue2c(p, q, h, 2), hue2c(p, q, h, 0), hue2c(p, q, h, 4), 1);
}

vec4 color(float it) {
    if (it == 0.0) return vec4(0, 1, 0, 1);
    float l = min(1.0, (800.0 - it) / 50.0);
    return hslToRgba(0.0, 0.0, l * cos(3.141592 * log(it)));
}

void main() {
    vec2 transformed = zoom * (gl_FragCoord.xy-size/2) - pos;// https://github.com/danyshaanan/mandelbrot/blob/master/docs/glsl/index.htm#L36
    float iter = iterations(transformed, zoom);
    outColor = color(iter);
}