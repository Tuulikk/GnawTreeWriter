use std::path::Path;
use tokenizers::Tokenizer;
use candle_core::{DType, Device};
use candle_nn::VarBuilder;

fn main() {
    let model_dir = Path::new(".gnawtreewriter_ai/models/modernbert");
    let weights_path = model_dir.join("model.safetensors");
    let device = Device::Cpu;

    println!("Checking weights path: {:?}", weights_path);
    println!("Exists: {}", weights_path.exists());

    match unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device) } {
        Ok(_) => println!("Successfully mmaped safetensors!"),
        Err(e) => println!("Failed to mmap safetensors: {}", e),
    }
}