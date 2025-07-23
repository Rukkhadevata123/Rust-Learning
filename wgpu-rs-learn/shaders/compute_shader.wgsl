struct Params {
    buffer_size: u32,
    scale: f32,
    offset: f32,
};

@group(0) @binding(0)
var<storage, read_write> output: array<f32>;
@group(0) @binding(1)
var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(
    @builtin(global_invocation_id) global_id: vec3u,
    @builtin(local_invocation_id) local_id: vec3u,
) {
    if global_id.x >= params.buffer_size {
        return;
    }
  // Example of a more complex calculation
    let x = f32(global_id.x) * params.scale + params.offset;
    output[global_id.x] = sin(x) * cos(f32(local_id.x));
}