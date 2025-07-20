use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use burn::data::dataset::{
    Dataset, InMemDataset,
    transform::{Mapper, MapperDataset},
};

const WIDTH: usize = 28;
const HEIGHT: usize = 28;

#[derive(Debug, Clone)]
pub struct MnistItem {
    pub image: [[f32; WIDTH]; HEIGHT],
    pub label: u8,
}

#[derive(Debug, Clone)]
struct MnistItemRaw {
    pub image_bytes: Vec<u8>,
    pub label: u8,
}

struct BytesToImage;

impl Mapper<MnistItemRaw, MnistItem> for BytesToImage {
    fn map(&self, item: &MnistItemRaw) -> MnistItem {
        debug_assert_eq!(item.image_bytes.len(), WIDTH * HEIGHT);

        let mut image_array = [[0f32; WIDTH]; HEIGHT];
        for (i, pixel) in item.image_bytes.iter().enumerate() {
            let x = i % WIDTH;
            let y = i / HEIGHT;
            image_array[y][x] = *pixel as f32;
        }

        MnistItem {
            image: image_array,
            label: item.label,
        }
    }
}

type MappedDataset = MapperDataset<InMemDataset<MnistItemRaw>, BytesToImage, MnistItemRaw>;

pub struct MnistDataset {
    dataset: MappedDataset,
}

impl Dataset<MnistItem> for MnistDataset {
    fn get(&self, index: usize) -> Option<MnistItem> {
        self.dataset.get(index)
    }

    fn len(&self) -> usize {
        self.dataset.len()
    }
}

impl MnistDataset {
    /// 从指定的根目录创建训练数据集。
    pub fn train_from(root: &str) -> Result<Self, std::io::Error> {
        Self::new_from_path(Path::new(root), "train")
    }

    /// 从指定的根目录创建测试数据集。
    pub fn test_from(root: &str) -> Result<Self, std::io::Error> {
        Self::new_from_path(Path::new(root), "test")
    }

    /// 内部函数，根据路径和 split 类型（"train" 或 "test"）加载数据。
    fn new_from_path(root: &Path, split: &str) -> Result<Self, std::io::Error> {
        // 不再下载，直接读取
        let images = Self::read_images(root, split)?;
        let labels = Self::read_labels(root, split)?;

        let items: Vec<_> = images
            .into_iter()
            .zip(labels)
            .map(|(image_bytes, label)| MnistItemRaw { image_bytes, label })
            .collect();

        let dataset = InMemDataset::new(items);
        let dataset = MapperDataset::new(dataset, BytesToImage);

        Ok(Self { dataset })
    }

    /// 从 IDX 文件读取图像。
    fn read_images(root: &Path, split: &str) -> Result<Vec<Vec<u8>>, std::io::Error> {
        let file_name = if split == "train" {
            "train-images.idx3-ubyte"
        } else {
            "t10k-images.idx3-ubyte"
        };
        let path = root.join(file_name);

        let mut f = File::open(path)?;
        let mut buf = [0u8; 4];
        f.seek(SeekFrom::Start(4))?;
        f.read_exact(&mut buf)?;
        let size = u32::from_be_bytes(buf);

        let mut buf_images = vec![0u8; WIDTH * HEIGHT * (size as usize)];
        f.seek(SeekFrom::Start(16))?;
        f.read_exact(&mut buf_images)?;

        Ok(buf_images
            .chunks(WIDTH * HEIGHT)
            .map(|chunk| chunk.to_vec())
            .collect())
    }

    /// 从 IDX 文件读取标签。
    fn read_labels(root: &Path, split: &str) -> Result<Vec<u8>, std::io::Error> {
        let file_name = if split == "train" {
            "train-labels.idx1-ubyte"
        } else {
            "t10k-labels.idx1-ubyte"
        };
        let path = root.join(file_name);

        let mut f = File::open(path)?;
        let mut buf = [0u8; 4];
        f.seek(SeekFrom::Start(4))?;
        f.read_exact(&mut buf)?;
        let size = u32::from_be_bytes(buf);

        let mut buf_labels = vec![0u8; size as usize];
        f.seek(SeekFrom::Start(8))?;
        f.read_exact(&mut buf_labels)?;

        Ok(buf_labels)
    }
}
