use std::{
    io::Write,
    process::{Command, Stdio},
};

use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

use crate::util::{ChronuxError, ChronuxResult};

pub enum Writer {
    Ffmpeg {
        process: std::process::Child,
    },
    PngSequence {
        width: usize,
        height: usize,
        path: String,
        max_frame_length: usize,
    },
}

impl Writer {
    pub fn new_ffmpeg(
        output: &str,
        width: usize,
        height: usize,
        fps: usize,
    ) -> ChronuxResult<Writer> {
        let process = Command::new("ffmpeg")
            .arg("-y")
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgba")
            .arg("-s")
            .arg(format!("{width}x{height}"))
            .arg("-r")
            .arg(fps.to_string())
            .arg("-i")
            .arg("pipe:0")
            .arg("-c:v")
            .arg("libx264")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-preset")
            .arg("medium")
            .arg("-vf")
            .arg("format=yuv420p,colorchannelmixer=aa=1")
            .arg(output)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| ChronuxError::FfmpegError(format!("Failed to start ffmpeg: {e}")))?;

        Ok(Writer::Ffmpeg { process })
    }

    pub fn new_png_sequence(
        output_dir: &str,
        width: usize,
        height: usize,
        max_possible_frames: usize,
    ) -> ChronuxResult<Writer> {
        std::fs::create_dir_all(output_dir).map_err(ChronuxError::IoError)?;
        log::info!("Created output directory `{output_dir}`");

        Ok(Writer::PngSequence {
            width,
            height,
            path: output_dir.to_string(),
            max_frame_length: max_possible_frames.to_string().len(),
        })
    }

    pub fn write_frame(&self, f: usize, raw: Vec<u8>) -> ChronuxResult<()> {
        match self {
            Writer::Ffmpeg { process, .. } => {
                process
                    .stdin
                    .as_ref()
                    .unwrap()
                    .write_all(&raw)
                    .map_err(ChronuxError::IoError)?;
            }
            Writer::PngSequence {
                width,
                height,
                path,
                max_frame_length,
            } => {
                let output = std::path::Path::new(path)
                    .join(format!("frame_{f:0>0$}.png", max_frame_length));
                let mut file = std::fs::File::create(output).map_err(ChronuxError::IoError)?;

                PngEncoder::new(&mut file)
                    .write_image(
                        &raw,
                        *width as u32,
                        *height as u32,
                        ExtendedColorType::Rgba8,
                    )
                    .map_err(|e| {
                        ChronuxError::IoError(std::io::Error::other(format!(
                            "Failed to encode PNG: {e}"
                        )))
                    })?;
            }
        }
        Ok(())
    }

    pub fn finish(self) -> ChronuxResult<()> {
        match self {
            Writer::Ffmpeg { process, .. } => {
                process
                    .stdin
                    .as_ref()
                    .unwrap()
                    .flush()
                    .map_err(ChronuxError::IoError)?;
                log::info!("FFmpeg writer finished.");
            }
            Writer::PngSequence { .. } => {
                // no cleanup needed for PNG sequence
                log::info!("PNG sequence writer finished.");
            }
        }
        Ok(())
    }
}
