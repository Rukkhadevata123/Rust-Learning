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
            var z = vec2<f32>(
                (fx - f32(params.width) * 0.5) * params.scale + params.center.x,
                (fy - f32(params.height) * 0.5) * params.scale + params.center.y
            );
            var i: u32 = 0u;
            loop {
                // Newton for f(z) = z^3 - 1
                let r2 = z.x * z.x + z.y * z.y;
                let r4 = r2 * r2;
                let denom = 3.0 * r4 + 1e-8;
                let zx2 = z.x * z.x;
                let zy2 = z.y * z.y;
                let fz_x = zx2 * z.x - 3.0 * z.x * zy2 - 1.0;
                let fz_y = 3.0 * zx2 * z.y - zy2 * z.y;
                let fpz_x = 3.0 * (zx2 - zy2);
                let fpz_y = 6.0 * z.x * z.y;
                let denom2 = fpz_x * fpz_x + fpz_y * fpz_y + 1e-8;
                let dx = (fz_x * fpz_x + fz_y * fpz_y) / denom2;
                let dy = (fz_y * fpz_x - fz_x * fpz_y) / denom2;
                z = vec2<f32>(z.x - dx, z.y - dy);

                i = i + 1u;
                if (fz_x * fz_x + fz_y * fz_y < 1e-6 || i >= params.max_iter) {
                    break;
                }
            }
            sum = sum + i;
        }
    }
    let idx = id.y * params.width + id.x;
    image[idx] = sum / (samples * samples);
}