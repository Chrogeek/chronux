use rsmpeg::{
    avcodec::AVCodecContext,
    avformat::AVFormatContextOutput,
    avutil::{AVFrame, av_rescale_q},
    error::RsmpegError,
    ffi::AV_NOPTS_VALUE,
};

use crate::util::{ChronuxError, ChronuxResult};

pub struct FfmpegWriter {
    pub ctx: AVCodecContext,
    pub octx: AVFormatContextOutput,
}

impl FfmpegWriter {
    pub fn encode(&mut self, frame: Option<&mut AVFrame>) -> ChronuxResult<()> {
        let frame = if let Some(f) = frame {
            if f.pts != AV_NOPTS_VALUE {
                f.set_pts(av_rescale_q(f.pts, f.time_base, self.ctx.time_base));
            }
            Some(f)
        } else {
            frame
        };

        self.ctx
            .send_frame(frame.as_deref())
            .map_err(|e| ChronuxError::FfmpegError(e.to_string()))?;

        loop {
            let mut packet = match self.ctx.receive_packet() {
                Ok(packet) => packet,
                Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => {
                    break;
                }
                Err(e) => return Err(ChronuxError::FfmpegError(e.to_string())),
            };

            packet.set_stream_index(0);
            packet.rescale_ts(self.ctx.time_base, self.octx.streams()[0].time_base);
            self.octx
                .interleaved_write_frame(&mut packet)
                .map_err(|e| ChronuxError::FfmpegError(format!("Failed to write frame: {}", e)))?;
        }
        Ok(())
    }
}
