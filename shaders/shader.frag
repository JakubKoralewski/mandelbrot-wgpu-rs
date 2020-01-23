#version 450

precision highp float;

#ifdef GL_ES
precision highp int;
#endif

/* integers per arbitrary-precision number */
const int vals = 1000; // ints per value

/* power of 10 one larger than maximum value per int
   A value of 10000 seems to work the best
   */
const int limit = 10000;

const float limitFlt = float(limit);

int result[vals];

#define zero(x, len) for(int i=0;i<len;i++){x[i]=0;}
#define assign(x, y) for(int i=0;i<vals;i++){x[i]=y[i];}
#define negate(x) for(int i = 0; i < vals; i++) { x[i] = -x[i]; }

bool signp(int[vals] a) {
    return (a[vals-1] >= 0);
}

int keepVal, carry;

void roundOff(int x) {
    carry = x / limit;
    keepVal = x - carry * limit;
}

void add(int[vals] a, int[vals] b) {
    bool s1 = signp(a), s2 = signp(b);

    carry = 0;

    for(int i = 0; i < vals-1; i++) {
        roundOff(a[i] + b[i] + carry);

        if(keepVal < 0) {
            keepVal += limit;
            carry--;
        }

        result[i] = keepVal;
    }
    roundOff(a[vals-1] + b[vals-1] + carry);
    result[vals-1] = keepVal;

    if(s1 != s2 && !signp(result)) {
        negate(result);

        carry = 0;

        for(int i = 0; i < vals; i++) {
            roundOff(result[i] + carry);

            if(keepVal < 0) {
                keepVal += limit;
                carry--;
            }

            result[i] = keepVal;
        }

            negate(result);
    }
}

void mul(int[vals] a, int[vals] b) {
    bool toNegate = false;

    if(!signp(a)) {
        negate(a);
        toNegate = !toNegate;
    }
    if(!signp(b)) {
        negate(b);
        toNegate = !toNegate;
    }

    const int lenProd = (vals-1)*2+1;
    int prod[lenProd];
    zero(prod, lenProd);

    for(int i = 0; i < vals; i++) {
        for(int j = 0; j < vals; j++) {
            prod[i+j] += a[i] * b[j];
        }
    }

    carry = 0;
    const int clip = lenProd - vals;
    for(int i = 0; i < clip; i++) {
        roundOff(prod[i] + carry);
        prod[i] = keepVal;
    }

    if(prod[clip-1] >= limit/2) {
        carry++;
    }

    for(int i = clip; i < lenProd; i++) {
        roundOff(prod[i] + carry);
        prod[i] = keepVal;
    }

    for(int i = 0; i < lenProd - clip; i++) {
        result[i] = prod[i+clip];
    }

    if(toNegate) {
        negate(result);
    }
}

void loadFloat(float f) {
    for(int i = vals - 1; i >= 0; i--) {
        int fCurr = int(f);
        result[i] = fCurr;
        f -= float(fCurr);
        f *= limitFlt;
    }
}

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
    float q = pow(z.x - 1/4,2) + y_2;
    float cardioid = q * (q + (z.x - 1/4));
    bool in_cardioid = cardioid <= y_2/4;
    if(!in_cardioid) {
        return 0.0;
    }
    float period2bulb = pow(z.x + 1, 2) + y_2;
    bool in_period2bulb = period2bulb <= 1/16;
    if(in_period2bulb) {
        return 0.0;
    }
//    if(zoom > 0.00005) {
        for (int i = 0; i < 800; i++) {
            z = square(z) + c;
            float len = length(z);
            if (len > r) return float(i) - log(len) / log_r;
        }
        return 0.0;
//    } else {
//        loadFloat(z);
//    }
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
//    return hslToRgba(it / 240.0, 1.0, l * .5);
    return hslToRgba(0.0, 0.0, l * cos(3.141592 * log(it)));
}

void main() {
    vec2 transformed = zoom * (gl_FragCoord.xy-size/2) - pos; // https://github.com/danyshaanan/mandelbrot/blob/master/docs/glsl/index.htm#L36
    float iter = iterations(transformed, zoom);
//    outColor = vec4(vec3(iter, iter, iter), 1);
    outColor = color(iter);
}