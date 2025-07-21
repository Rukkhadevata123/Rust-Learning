use crate::training::ARTIFACT_DIR;
use crate::{model::Model, training::MnistTrainingConfig};
use burn::{
    config::Config,
    module::Module,
    prelude::*,
    record::{BinBytesRecorder, FullPrecisionSettings, Recorder},
    tensor::{self, Tensor},
};
fn model_path() -> String {
    format!("{ARTIFACT_DIR}/model.bin")
}

/// 运行推理的主函数，现在是泛型的
pub fn run<B: Backend>(device: B::Device, image_path: &str) {
    // 1. 加载训练好的模型
    let model = load_model::<B>(&device);

    // 2. 从文件加载图片并将其转换为一个符合模型输入的 Tensor
    let image_tensor = image_to_tensor::<B>(image_path, &device);

    // 3. 将 Tensor 输入模型进行前向传播（推理）
    let output = model.forward(image_tensor);

    // 4. 对模型的输出应用 softmax 以获得概率分布
    let probabilities = tensor::activation::softmax(output, 1);

    // 5. 从概率 Tensor 中提取数据并找到最可能的类别
    let (predicted_label, confidence) = get_best_prediction::<B>(probabilities);

    // 6. 打印结果
    println!("\n✅ Inference Complete!");
    println!("============================");
    println!("File:         {image_path}");
    println!("Prediction:   {predicted_label}");
    println!("Confidence:   {:.2}%", confidence * 100.0);
    println!("============================");
}

/// 从文件中加载训练好的模型权重
fn load_model<B: Backend>(device: &B::Device) -> Model<B> {
    // 确保配置文件存在
    let config = MnistTrainingConfig::load(format!("{ARTIFACT_DIR}/config.json"))
        .expect("Config file not found. Run training first with `cargo run --release -- train`");

    let model: Model<B> = Model::new(device);

    // Load binary record from bytes
    let record = BinBytesRecorder::<FullPrecisionSettings, &'static [u8]>::default()
        .load(model_path().as_bytes(), &Default::default())
        .expect("Failed to decode state");

    // Load record into model
    model.load_record(record)
}

/// 将图片文件加载并预处理为 Tensor
fn image_to_tensor<B: Backend>(path: &str, device: &B::Device) -> Tensor<B, 3> {
    let img = image::open(path)
        .expect("Failed to open image file.")
        .to_luma8();

    if img.width() != 28 || img.height() != 28 {
        panic!("Image must be 28x28 pixels.");
    }

    let raw_pixels: Vec<f32> = img.into_raw().into_iter().map(|p| p as f32).collect();

    let input_tensor = Tensor::<B, 1>::from_floats(&*raw_pixels, device).reshape([1, 28, 28]);

    ((input_tensor / 255.0) - 0.1307) / 0.3081
}

/// 从概率 Tensor 中提取最高概率的标签和其置信度
fn get_best_prediction<B: Backend>(probabilities: Tensor<B, 2>) -> (i32, f32) {
    let label_tensor = probabilities.clone().argmax(1);
    let confidence_tensor = probabilities.max_dim(1);

    let predicted_label = label_tensor.into_data().as_slice::<i32>().unwrap()[0];
    let confidence = confidence_tensor.into_data().as_slice::<f32>().unwrap()[0];

    (predicted_label, confidence)
}
