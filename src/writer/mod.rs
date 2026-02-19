use std::{
    fs::File, io::Write, path::PathBuf, process::{Command, Stdio}
};

use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

use crate::{
    util::{ChronuxError, ChronuxResult},
    watermark::WatermarkInfo,
};

pub enum Writer {
    Ffmpeg {
        process: std::process::Child,
        output_filename: String,
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
        bgm: &Option<PathBuf>,
    ) -> ChronuxResult<Writer> {
        let mut process = Command::new("ffmpeg");
        let process = process
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
            .arg("pipe:0");
        let process = if let Some(bgm) = bgm {
            process.arg("-i").arg(bgm).arg("-c:a").arg("copy")
        } else {
            process
        };
        let process = process
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

        Ok(Writer::Ffmpeg {
            process,
            output_filename: output.to_string(),
        })
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

    pub fn write_wm_info(&self, wminfo_name: &str, wminfo: &WatermarkInfo) -> ChronuxResult<()> {
        let wm_info_path = match self {
            Writer::Ffmpeg {
                output_filename, ..
            } => std::path::Path::new(output_filename)
                .with_file_name(wminfo_name)
                .to_path_buf(),
            Writer::PngSequence { path, .. } => {
                std::path::Path::new(path).join(wminfo_name).to_path_buf()
            }
        };
        let mut file = File::create(&wm_info_path).map_err(ChronuxError::IoError)?;
        let wmstr = toml::to_string_pretty(wminfo).map_err(|e| Into::<ChronuxError>::into(e))?;
        file.write_all(wmstr.as_ref())
            .map_err(ChronuxError::IoError)
    }

    pub fn finish(self) -> ChronuxResult<()> {
        match self {
            Writer::Ffmpeg { mut process, .. } => {
                process
                    .stdin
                    .as_ref()
                    .unwrap()
                    .flush()
                    .map_err(ChronuxError::IoError)?;
                let status = process.wait().map_err(|e| {
                    ChronuxError::FfmpegError(format!("Failed to wait for ffmpeg: {e}"))
                })?;
                log::info!("FFmpeg writer finished with: {}.", status);
            }
            Writer::PngSequence { .. } => {
                // no cleanup needed for PNG sequence
                log::info!("PNG sequence writer finished.");
            }
        }
        Ok(())
    }
}
