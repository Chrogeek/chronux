/// 核心抖动效果生成函数: 给定当前时间进度
///
/// # 参数
/// * `elapsed` - 已经经过的时间
/// * `duration` - 总持续时间
/// * `max_amp` - 最大振幅(px)
/// * `seed` - 随机种子，确保同一场地震每次表现一致
///
/// # 返回值
/// 二元组 `(x_offset, y_offset)`，表示计算出的偏移量
pub fn earthquake(elapsed: f32, duration: f32, max_amp: f32, seed: u64) -> (f32, f32) {
    earthquake_shake(elapsed / duration, max_amp, seed)
}

/// 简单的线性同余伪随机数生成器 (LCG)
/// 使用经典的参数：multiplier = 6364136223846793005, increment = 1, modulus = 2^64
fn pseudo_random(seed: &mut u64) -> f32 {
    // 线性同余算法: X_{n+1} = (a * X_n + c) mod m
    const MULTIPLIER: u64 = 6364136223846793005;
    const INCREMENT: u64 = 1;

    *seed = seed.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);

    // 将u64转换为[0,1)范围的f32
    // 取高32位以获得更好的随机性
    let bits = (*seed >> 32) as u32;
    (bits as f64 / (u32::MAX as f64 + 1.0)) as f32
}

/// 地震抖动函数
/// t: 动画播放比例 [0, 1]
/// amplitude: 最大抖动幅度（像素）
/// seed: 随机种子
/// 返回 (x, y) 偏移量
fn earthquake_shake(t: f32, amplitude: f32, seed: u64) -> (f32, f32) {
    // 边界条件：t在[0,1]之外时无抖动
    if t <= 0.0 || t >= 1.0 {
        return (0.0, 0.0);
    }

    let mut rng_seed = seed;

    // 1. 使用多个谐波叠加模拟地震的复杂频率成分
    // 频率递增，振幅递减，模拟地震波的不同频率分量
    let frequencies = [7.0, 13.0, 23.0, 41.0, 73.0];
    let magnitudes = [0.4, 0.25, 0.15, 0.1, 0.1];

    // 2. 计算包络函数，使抖动在开始和结束时平滑过渡
    let envelope = 4.0 * t * (1.0 - t); // 抛物线包络，在t=0.5时达到峰值1.0

    // 3. 为x和y方向分别生成抖动
    let mut x_offset = 0.0_f32;
    let mut y_offset = 0.0_f32;

    for i in 0..frequencies.len() {
        let freq = frequencies[i];
        let mag = magnitudes[i];

        // 为每个频率分量生成随机相位偏移
        // 使用不同种子确保x和y方向有不同的随机特性
        let phase_x = pseudo_random(&mut rng_seed) * 2.0 * std::f32::consts::PI;
        let phase_y = pseudo_random(&mut rng_seed) * 2.0 * std::f32::consts::PI;

        // 叠加正弦波，模拟地震动
        x_offset += mag * (freq * t * 2.0 * std::f32::consts::PI + phase_x).sin();
        y_offset += mag * (freq * t * 2.0 * std::f32::consts::PI + phase_y).sin();
    }

    // 4. 应用包络
    x_offset *= envelope;
    y_offset *= envelope;

    // 5. 限制振幅不超过限制
    x_offset = x_offset.clamp(-1.0, 1.0);
    y_offset = y_offset.clamp(-1.0, 1.0);

    (x_offset * amplitude, y_offset * amplitude)
}
