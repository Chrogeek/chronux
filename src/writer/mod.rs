use std::path::Path;

use skia_safe::{ImageInfo, Pixmap};
use video_rs::{Encoder, Time, encode::Settings};

use crate::util::{ChronuxError, ChronuxResult};

pub enum Writer {
    Ffmpeg {
        width: usize,
        height: usize,
        fps: Time,
        pos: Time,
        encoder: Encoder,
    },
    PngSequence {
        width: usize,
        height: usize,
        path: String,
    },
}

impl Writer {
    pub fn new_ffmpeg(
        output: String,
        width: usize,
        height: usize,
        fps: usize,
    ) -> ChronuxResult<Writer> {
        let settings = Settings::preset_h264_yuv420p(width, height, false);
        let encoder = Encoder::new(Path::new(&output), settings)
            .map_err(|e| ChronuxError::FfmpegError(e.to_string()))?;
        log::info!("Created video encoder with output `{}`", output);

        Ok(Writer::Ffmpeg {
            width,
            height,
            encoder,
            fps: Time::from_nth_of_a_second(fps),
            pos: Time::zero(),
        })
    }

    pub fn new_png_sequence(
        width: usize,
        height: usize,
        output_dir: String,
    ) -> ChronuxResult<Writer> {
        std::fs::create_dir_all(&output_dir).map_err(|e| ChronuxError::IoError(e))?;
        log::info!("Created output directory `{}`", output_dir);

        Ok(Writer::PngSequence {
            width,
            height,
            path: output_dir,
        })
    }

    /// Sets the output file path for the encoded video.
    pub fn write_frame(&mut self, f: usize, raw: &mut [u8]) -> ChronuxResult<()> {
        match self {
            Writer::Ffmpeg {
                width,
                height,
                encoder,
                pos,
                fps,
                ..
            } => {
                let frame = ndarray::Array3::from_shape_fn((*height, *width, 3), |(y, x, c)| {
                    raw[(y * *width + x) * 4 + (2 - c)]
                });

                encoder
                    .encode(&frame, *pos)
                    .map_err(|e| ChronuxError::FfmpegError(e.to_string()))?;
                *pos = pos.aligned_with(*fps).add();
            }
            Writer::PngSequence {
                width,
                height,
                path,
            } => {
                let output = std::path::Path::new(&path).join(format!("frame_{:04}.png", f));
                let mut file =
                    std::fs::File::create(&output).map_err(|e| ChronuxError::IoError(e))?;

                let pixmap = Pixmap::new(
                    &ImageInfo::new_n32_premul((*width as i32, *height as i32), None),
                    raw,
                    *width * 4,
                )
                .ok_or(ChronuxError::SkiaError(
                    "Failed to create Pixmap from raw data".into(),
                ))?;

                skia_safe::png_encoder::encode(
                    &pixmap,
                    &mut file,
                    &skia_safe::png_encoder::Options::default(),
                );
            }
        }
        Ok(())
    }

    /// Sets the output file path for the encoded video.
    pub fn finish(&mut self) -> ChronuxResult<()> {
        match self {
            Writer::Ffmpeg { encoder, .. } => {
                encoder
                    .finish()
                    .map_err(|e| ChronuxError::FfmpegError(e.to_string()))?;
                log::info!("FFmpeg encoder finished.");
            }
            Writer::PngSequence { .. } => {
                // no cleanup needed for PNG list
                log::info!("PNG sequence writer finished.");
            }
        }
        Ok(())
    }
}
