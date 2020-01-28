#version 450
precision highp float;


layout(set = 0, binding = 0) uniform WindowSize { // https://github.com/gfx-rs/wgpu-rs/blob/v0.4/examples/shadow/forward.frag
    vec2 size;
};

layout(set = 0, binding = 1) uniform Zoom {
    float zoom;
};

layout(set = 0, binding = 2) uniform Pos {
    vec2 pos;
};

layout(set = 0, binding = 3) uniform Iterations {
    float num_iters;
};

layout(set = 0, binding = 4) uniform Julia {
    bool is_julia;
};

layout(set = 0, binding = 5) uniform Generator {
    vec2 generator;
};

layout(location = 0) out vec4 outColor;

float r = 200;
float log_r = log(r);

vec2 transform(vec2 x) {
    return zoom * (x-size/2) - pos;
}

vec2 pow(vec2 z, float i_pow) {
    return vec2(pow(z.x, i_pow) - pow(z.y, i_pow), i_pow * z.x *z.y);
}

vec2 square(vec2 z) {
    return vec2(pow(z.x, 2.0) - pow(z.y, 2.0), 2.0 * z.x * z.y);
}

float iterations_julia(vec2 c) {
    vec2 gen = generator;
    vec2 z = c;
    for (int i = 0; i < num_iters; i++) {
        z = square(z) + gen;
        float len = length(z);
        if (len > r) return float(i) - log(len)/log_r;
    }
    return 0.0;
}

float iterations_mandelbrot(vec2 c) {
    vec2 z = c;
    for (int i = 0; i < num_iters; i++) {
        z = square(z) + c;
        float len = length(z);
        if (len > r) return float(i) - log(len)/log_r;
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
    if (it == 0.0) return vec4(0, 0, 0, 1);
    float l = min(1.0, (800.0 - it) / 50.0);
    return hslToRgba(it / 240.0, 1.0, l * .5);
}


void main() {
    vec2 transformed = transform(gl_FragCoord.xy);
    float iter;
    if(is_julia) {
        iter = iterations_julia(transformed);
    } else {
        iter = iterations_mandelbrot(transformed);
    }
    outColor = color(iter);
}