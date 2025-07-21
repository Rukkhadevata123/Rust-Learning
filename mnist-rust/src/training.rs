use crate::{data::MnistBatcher, mnist::MnistDataset, model::Model};
use burn::{
    data::dataloader::DataLoaderBuilder,
    optim::{AdamConfig, decay::WeightDecayConfig},
    prelude::*,
    record::{CompactRecorder, NoStdTrainingRecorder},
    tensor::backend::AutodiffBackend,
    train::{
        LearnerBuilder,
        metric::{AccuracyMetric, LossMetric},
    },
};

pub static ARTIFACT_DIR: &str = "burn-mnist-wgpu";
static DATASET_DIR: &str = "dataset";

#[derive(Config)]
pub struct MnistTrainingConfig {
    #[config(default = 10)]
    pub num_epochs: usize,
    #[config(default = 64)]
    pub batch_size: usize,
    #[config(default = 4)]
    pub num_workers: usize,
    #[config(default = 42)]
    pub seed: u64,
    pub optimizer: AdamConfig,
}

pub fn run<B: AutodiffBackend>(device: B::Device) {
    // 创建用于保存模型和日志的目录
    std::fs::create_dir_all(ARTIFACT_DIR).ok();

    // 配置
    let config_optimizer = AdamConfig::new().with_weight_decay(Some(WeightDecayConfig::new(5e-5)));
    let config = MnistTrainingConfig::new(config_optimizer);
    B::seed(config.seed);

    // 数据加载器
    let batcher = MnistBatcher::default();
    let dataloader_train = DataLoaderBuilder::new(batcher.clone())
        .batch_size(config.batch_size)
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(MnistDataset::train_from(DATASET_DIR).expect("Dataset not found at ../dataset"));

    let dataloader_test = DataLoaderBuilder::new(batcher)
        .batch_size(config.batch_size)
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(MnistDataset::test_from(DATASET_DIR).expect("Dataset not found at ../dataset"));

    // 学习器（Learner）包含了模型、优化器和所有训练指标
    let learner = LearnerBuilder::new(ARTIFACT_DIR)
        .metric_train_numeric(AccuracyMetric::new())
        .metric_valid_numeric(AccuracyMetric::new())
        .metric_train_numeric(LossMetric::new())
        .metric_valid_numeric(LossMetric::new())
        .with_file_checkpointer(CompactRecorder::new())
        .devices(vec![device.clone()])
        .num_epochs(config.num_epochs)
        .summary()
        .build(Model::<B>::new(&device), config.optimizer.init(), 1e-4);

    // 开始训练
    let model_trained = learner.fit(dataloader_train, dataloader_test);

    // 保存训练配置和最终模型
    config.save(format!("{ARTIFACT_DIR}/config.json")).unwrap();

    model_trained
        .save_file(
            format!("{ARTIFACT_DIR}/model"),
            &NoStdTrainingRecorder::new(),
        )
        .expect("Failed to save trained model");

    println!("\n✅ Training complete. Model saved in {ARTIFACT_DIR}");
}
