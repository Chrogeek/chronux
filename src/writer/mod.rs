use std::{
    io::Write,
    process::{Command, Stdio},
};

use skia_safe::{ImageInfo, Pixmap};

use crate::util::{ChronuxError, ChronuxResult};

pub enum Writer {
    Ffmpeg {
        process: std::process::Child,
    },
    PngSequence {
        width: usize,
        height: usize,
        path: String,
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
            .arg("bgra")
            .arg("-s")
            .arg(format!("{}x{}", width, height))
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
            .map_err(|e| ChronuxError::FfmpegError(format!("Failed to start ffmpeg: {}", e)))?;

        Ok(Writer::Ffmpeg { process })
    }

    pub fn new_png_sequence(
        output_dir: &str,
        width: usize,
        height: usize,
    ) -> ChronuxResult<Writer> {
        std::fs::create_dir_all(output_dir).map_err(|e| ChronuxError::IoError(e))?;
        log::info!("Created output directory `{}`", output_dir);

        Ok(Writer::PngSequence {
            width,
            height,
            path: output_dir.to_string(),
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
                    .map_err(|e| ChronuxError::IoError(e))?;
            }
            Writer::PngSequence {
                width,
                height,
                path,
            } => {
                let output = std::path::Path::new(path).join(format!("frame_{:04}.png", f));
                let mut file =
                    std::fs::File::create(output).map_err(|e| ChronuxError::IoError(e))?;

                let mut raw = raw;
                let pixmap = Pixmap::new(
                    &ImageInfo::new_n32_premul((*width as i32, *height as i32), None),
                    &mut raw,
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

    pub fn finish(self) -> ChronuxResult<()> {
        match self {
            Writer::Ffmpeg { process, .. } => {
                process
                    .stdin
                    .as_ref()
                    .unwrap()
                    .flush()
                    .map_err(|e| ChronuxError::IoError(e))?;
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
