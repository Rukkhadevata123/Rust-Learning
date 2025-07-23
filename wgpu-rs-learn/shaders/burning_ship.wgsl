struct Params {
    width: u32,
    height: u32,
    scale: f32,
    center: vec2<f32>,
    max_iter: u32,
};

@group(0) @binding(0)
var<storage, read_write> image: array<u32>;
@group(0) @binding(1)
var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.width || id.y >= params.height {
        return;
    }

    let samples = 2u;
    var sum: u32 = 0u;
    for (var sx: u32 = 0u; sx < samples; sx = sx + 1u) {
        for (var sy: u32 = 0u; sy < samples; sy = sy + 1u) {
            let fx = f32(id.x) + (f32(sx) + 0.5) / f32(samples);
            let fy = f32(id.y) + (f32(sy) + 0.5) / f32(samples);
            let c = vec2<f32>(
                (fx - f32(params.width) * 0.5) * params.scale + params.center.x,
                (fy - f32(params.height) * 0.5) * params.scale + params.center.y
            );
            var z = vec2<f32>(0.0, 0.0);
            var i: u32 = 0u;
            loop {
                // Burning Ship: z = (|Re(z)| + i|Im(z)|)^2 + c
                let zx = abs(z.x);
                let zy = abs(z.y);
                let x = zx * zx - zy * zy + c.x;
                let y = 2.0 * zx * zy + c.y;
                z = vec2<f32>(x, y);

                if (dot(z, z) > 4.0 || i >= params.max_iter) {
                    break;
                }
                i = i + 1u;
            }
            sum = sum + i;
        }
    }
    let idx = id.y * params.width + id.x;
    image[idx] = sum / (samples * samples);
}