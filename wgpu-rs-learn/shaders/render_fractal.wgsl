@group(0) @binding(0)
var<storage, read> image: array<u32>;
@group(0) @binding(1)
var<uniform> params: vec4<f32>; // params: width, height, max_iter, color_mode

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
    );
    var uv = (pos[idx] + vec2<f32>(1.0, 1.0)) * 0.5;
    var out: VertexOutput;
    out.pos = vec4<f32>(pos[idx], 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let width = f32(params.x);
    let height = f32(params.y);
    let max_iter = f32(params.z);
    let color_mode = u32(params.w);
    let x = u32(in.uv.x * width);
    let y = u32(in.uv.y * height);
    let idx = y * u32(width) + x;
    let iter = f32(image[idx]);
    var color: vec3<f32>;
    let norm = iter / max_iter;
    switch (color_mode) {
        case 0: { // 配色方案1：sin 彩条
            if iter < max_iter {
                color = vec3<f32>(
                    sin(iter / max_iter * 5.0),
                    sin(iter / max_iter * 10.0),
                    sin(iter / max_iter * 15.0)
                );
            } else {
                color = vec3<f32>(0.0, 0.0, 0.0);
            }
        }
        case 1: { // 配色方案2：平滑cos色带
            color = vec3<f32>(
                0.5 + 0.5 * cos(3.0 + norm * 10.0),
                0.5 + 0.5 * cos(1.0 + norm * 10.0),
                0.5 + 0.5 * cos(5.0 + norm * 10.0)
            );
        }
        case 2: { // 配色方案3：log分段色带
            let t = log2(iter + 1.0) / log2(max_iter);
            color = vec3<f32>(
                fract(t * 5.0),
                fract(t * 5.0 + 0.2),
                fract(t * 5.0 + 0.4)
            );
        }
        default: {
            color = vec3<f32>(norm);
        }
    }
    if iter >= max_iter {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    return vec4<f32>(color, 1.0);
}

