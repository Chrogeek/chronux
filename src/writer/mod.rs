use std::{fs::File, io::{BufWriter, Write}, slice::from_raw_parts_mut};

use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avutil::{AVFrame, ra},
    ffi::{AV_CODEC_ID_MPEG4, AV_PIX_FMT_YUV420P},
};
use skia_safe::{ImageInfo, Pixmap};
use yuv::{YuvPlanarImageMut, bgra_to_yuv420};

use crate::util::{ChronuxError, ChronuxResult};

pub mod ffmpeg;

pub enum Writer {
    Ffmpeg {
        width: usize,
        height: usize,
        ctx: AVCodecContext,
        buf: BufWriter<File>,
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
        let encoder = AVCodec::find_encoder(AV_CODEC_ID_MPEG4)
            .ok_or(ChronuxError::FfmpegError("failed to find encoder".into()))?;

        let mut ctx = AVCodecContext::new(&encoder);

        ctx.set_bit_rate(1 << 24); // 16 Mbps
        ctx.set_width(width as i32);
        ctx.set_height(height as i32);
        ctx.set_time_base(ra(1, fps as i32));
        ctx.set_framerate(ra(fps as i32, 1));
        ctx.set_gop_size(10);
        ctx.set_max_b_frames(1);
        ctx.set_pix_fmt(AV_PIX_FMT_YUV420P);

        // if encoder.id == AV_CODEC_ID_H264 {
        //     unsafe { opt_set(ctx.priv_data, c"preset", c"slow", 0) }.map_err(|e| {
        //         ChronuxError::FfmpegError(format!("Failed to set encoder options: {}", e))
        //     })?;
        // }

        ctx.open(None)
            .map_err(|e| ChronuxError::FfmpegError(format!("Failed to open codec: {}", e)))?;

        let file = File::create(output).map_err(|e| ChronuxError::IoError(e))?;
        let buf = BufWriter::new(file);


        Ok(Writer::Ffmpeg {
            width,
            height,
            ctx,
            buf,
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

    pub fn write_frame(&mut self, f: usize, raw: Vec<u8>) -> ChronuxResult<()> {
        match self {
            Writer::Ffmpeg {
                width,
                height,
                ctx,
                buf,
            } => {
                let mut frame = AVFrame::new();
                frame.set_width(*width as i32);
                frame.set_height(*height as i32);
                frame.set_format(AV_PIX_FMT_YUV420P);
                frame.alloc_buffer().map_err(|e| {
                    ChronuxError::FfmpegError(format!("Failed to allocate frame buffer: {}", e))
                })?;

                frame.make_writable().map_err(|e| {
                    ChronuxError::FfmpegError(format!("Failed to prepare frame: {}", e))
                })?;

                let data = frame.data;
                let linesize = frame.linesize;
                let linesize_y = linesize[0] as usize;
                let linesize_u = linesize[1] as usize;
                let linesize_v = linesize[2] as usize;

                let y_data = unsafe { from_raw_parts_mut(data[0], *height * linesize_y) };
                let u_data = unsafe { from_raw_parts_mut(data[1], (*height / 2) * linesize_u) };
                let v_data = unsafe { from_raw_parts_mut(data[2], (*height / 2) * linesize_v) };

                let mut planar = YuvPlanarImageMut {
                    y_plane: yuv::BufferStoreMut::Borrowed(y_data),
                    y_stride: linesize_y as u32,
                    u_plane: yuv::BufferStoreMut::Borrowed(u_data),
                    u_stride: linesize_u as u32,
                    v_plane: yuv::BufferStoreMut::Borrowed(v_data),
                    v_stride: linesize_v as u32,
                    width: *width as u32,
                    height: *height as u32,
                };

                bgra_to_yuv420(
                    &mut planar,
                    &raw,
                    *width as u32 * 4,
                    yuv::YuvRange::Full,
                    yuv::YuvStandardMatrix::Bt709,
                    yuv::YuvConversionMode::Balanced,
                )
                .map_err(|e| {
                    ChronuxError::FfmpegError(format!("Failed to convert RGBA to YUV: {}", e))
                })?;

                frame.set_pts(f as i64);
                ffmpeg::encode(ctx, Some(&mut frame), buf).map_err(|e| {
                    ChronuxError::FfmpegError(format!("Failed to encode frame: {}", e))
                })?;
            }
            Writer::PngSequence {
                width,
                height,
                path,
            } => {
                let output = std::path::Path::new(&path).join(format!("frame_{:04}.png", f));
                let mut file =
                    std::fs::File::create(&output).map_err(|e| ChronuxError::IoError(e))?;

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

    pub fn finish(&mut self) -> ChronuxResult<()> {
        match self {
            Writer::Ffmpeg {
                ctx,
                buf,
                ..
            } => {
                ffmpeg::encode(ctx, None, buf).map_err(|e| {
                    ChronuxError::FfmpegError(format!("Failed to flush encoder: {}", e))
                })?;
                buf.write_all(&[0, 0, 1, 0xb7])
                    .and_then(|_| buf.flush())
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
