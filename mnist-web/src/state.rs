use crate::model::Model;
use burn::{
    module::Module,
    record::{BinBytesRecorder, FullPrecisionSettings, Recorder},
};

use burn::backend::wgpu::{Wgpu, WgpuDevice, graphics::AutoGraphicsApi, init_setup_async};
pub type Backend = Wgpu<f32, i32>;

static STATE_ENCODED: &[u8] = include_bytes!("../model.bin");

/// Builds and loads trained parameters into the model.
pub async fn build_and_load_model() -> Model<Backend> {
    init_setup_async::<AutoGraphicsApi>(&WgpuDevice::default(), Default::default()).await;

    let model: Model<Backend> = Model::new(&Default::default());
    let record = BinBytesRecorder::<FullPrecisionSettings, &'static [u8]>::default()
        .load(STATE_ENCODED, &Default::default())
        .expect("Failed to decode state");

    model.load_record(record)
}
