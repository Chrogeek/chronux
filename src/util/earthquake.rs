use rand::RngExt;

/// 核心抖动效果生成函数: 给定当前时间进度
///
/// 采用随机游走风格但受振幅包络线控制，实现无规律抖动，并在结束时平滑归零。
///
/// 同时为了避免偏移突变过大导致跳跃不自然，但地震本来就是剧烈随机，无碍。
/// 并且保证最后一小段时间内完全收敛到 0。
///
/// # 参数
/// * `elapsed` - 已经经过的时间
/// * `duration` - 总持续时间
/// * `max_amp` - 最大振幅(px)
///
/// # 返回值
/// 二元组 `(x_offset, y_offset)`，表示计算出的偏移量
pub fn earthquake(elapsed: f32, duration: f32, max_amp: f32) -> (f32, f32) {
    // Guard: 如果超出持续时间，偏移为零
    if elapsed >= duration || elapsed <= 0.0 || max_amp <= 0.0 {
        return (0.0, 0.0);
    }

    let amp_factor = get_amplitude(elapsed, duration);

    let max_amp = max_amp * amp_factor.min(1.0);

    // 随机偏移: 使用正态分布风格或均匀随机，均匀随机更显突兀震动感
    // 为了让抖动更像地震，随机偏移的X和Y独立，范围在 [-max_amp, max_amp]
    // 且增加“高频”感觉: 每帧完全随机，不需要平滑过渡，地震即是如此。
    let mut rng = rand::rng();
    let x = (rng.random_range(-1.0..1.0)) * max_amp;
    let y = (rng.random_range(-1.0..1.0)) * max_amp;

    (x, y)
}

pub fn get_amplitude(elapsed: f32, duration: f32) -> f32 {
    let rise_ratio = 0.25;
    let sustain_ratio = 0.25;

    let t = elapsed / duration;
    let fall_ratio = 1.0 - rise_ratio - sustain_ratio;

    if t < rise_ratio {
        let progress = t / rise_ratio;
        1.0 - (1.0 - progress).powf(2.5)
    } else if t < rise_ratio + sustain_ratio {
        1.0
    } else {
        let fall_progress = (t - rise_ratio - sustain_ratio) / fall_ratio; // 0 → 1
        1.0 - fall_progress.powf(1.8)
    }
}
