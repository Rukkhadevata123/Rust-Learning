#![recursion_limit = "256"]

use burn::backend::{
    Autodiff,
    wgpu::{Wgpu, WgpuDevice},
};
use std::env;

mod data;
mod inference;
mod mnist;
mod model;
mod training;

fn main() {
    let device = WgpuDevice::default();
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "train" => {
            println!("Starting training on WGPU backend...");
            training::run::<Autodiff<Wgpu>>(device);
        }
        "infer" => {
            if args.len() < 3 {
                println!("Error: Please provide a path to an image file for inference.");
                print_usage();
                return;
            }
            let image_path = &args[2];
            println!("Running inference for image: {image_path}");
            inference::run::<Autodiff<Wgpu>>(device, image_path);
        }
        _ => {
            println!("Error: Invalid command.");
            print_usage();
        }
    }
}

fn print_usage() {
    println!("\nUsage:");
    println!("  cargo run --release -- train                - Run the training process");
    println!("  cargo run --release -- infer <path_to_image> - Run inference on a single image\n");
    println!("Example:");
    println!("  cargo run --release -- infer ./assets/my_test_digit.png");
}
