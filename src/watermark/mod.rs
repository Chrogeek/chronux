use std::collections::HashSet;

use bitvec::prelude::*;
use blind_watermark::{
    YCrBrAMat,
    config::{WatermarkConfigBuilder, WatermarkMode},
};
use image::Rgba32FImage;
use rand::seq::SliceRandom;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct WatermarkInfo {
    pub frames: Vec<usize>,
    pub seed_hi: u32,
    pub seed_lo: u32,
    pub shape: (u32, u32),
    pub total_frames: u32,
    pub unix_timestamp: u64,
}

/// 在 total 个帧中，随机选取 samples 个采样，其中每个采样延续 len 个帧，且采样之间没有重复帧
pub fn gen_samples(total: usize, samples: usize, len: usize) -> HashSet<usize> {
    if total == 0 || samples == 0 || len == 0 {
        return HashSet::new();
    }

    let count = samples.saturating_mul(len);
    if count >= total {
        return (0..total).collect();
    }

    let slots = total - count;

    // 共有 slots 个空位（非采样帧），在其中（包括两端）选取 samples 个位置插入采样
    let mut positions: Vec<_> = (0..=slots).collect();
    let mut rng = rand::rng();
    positions.shuffle(&mut rng);

    let mut selected_positions: Vec<_> = positions[..samples].iter().cloned().collect();
    selected_positions.sort_unstable();

    // 基于 selected_positions 计算采样帧的索引
    let mut samples_indices = HashSet::with_capacity(count);
    for (id, pos) in selected_positions.iter().enumerate() {
        let start = pos + id * len;
        samples_indices.extend(start..start + len);
    }

    samples_indices
}

pub fn embed_wm(raw: &[u8], wm_bits: &BitSlice<u8>, seed: u64, width: u32, height: u32) -> Vec<u8> {
    let config = WatermarkConfigBuilder::default()
        .mode(WatermarkMode::Strategy(seed))
        .build()
        .unwrap();

    let mut img = Rgba32FImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize * 4;
            if idx + 3 < raw.len() {
                img.put_pixel(
                    x,
                    y,
                    image::Rgba([
                        raw[idx] as f32 / 255.0,
                        raw[idx + 1] as f32 / 255.0,
                        raw[idx + 2] as f32 / 255.0,
                        raw[idx + 3] as f32 / 255.0,
                    ]),
                );
            }
        }
    }

    // raw 为 RGBA8888 格式，需要转换为 YCbCrA 格式
    let ycbcra: YCrBrAMat = img.into();

    let raw_processed: Rgba32FImage = ycbcra
        .add_padding()
        .dwt()
        .cut()
        .embed_watermark_bits(wm_bits, &config)
        .assemble()
        .idwt()
        .remove_padding()
        .into();

    let mut raw = Vec::with_capacity(raw.len());
    for y in 0..height {
        for x in 0..width {
            let c = raw_processed.get_pixel(x, y);
            let (r, g, b, a) = (
                (c[0] * 256.0).floor().clamp(0.0, 255.0) as u8,
                (c[1] * 256.0).floor().clamp(0.0, 255.0) as u8,
                (c[2] * 256.0).floor().clamp(0.0, 255.0) as u8,
                (c[3] * 256.0).floor().clamp(0.0, 255.0) as u8,
            );
            raw.push(r);
            raw.push(g);
            raw.push(b);
            raw.push(a);
        }
    }

    raw
}
