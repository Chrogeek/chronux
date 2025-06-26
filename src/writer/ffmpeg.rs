use rsmpeg::{
    avcodec::AVCodecContext,
    avformat::AVFormatContextOutput,
    avutil::{AVFrame, av_rescale_q},
    error::RsmpegError,
    ffi::AV_NOPTS_VALUE,
};

use crate::util::{ChronuxError, ChronuxResult};

pub fn encode(
    ctx: &mut AVCodecContext,
    mut frame: Option<AVFrame>,
    octx: &mut AVFormatContextOutput,
) -> ChronuxResult<()> {
    if let Some(f) = frame.as_mut() {
        if f.pts != AV_NOPTS_VALUE {
            f.set_pts(av_rescale_q(f.pts, f.time_base, ctx.time_base));
        }
    }

    ctx.send_frame(frame.as_ref())
        .map_err(|e| ChronuxError::FfmpegError(e.to_string()))?;

    loop {
        let mut packet = match ctx.receive_packet() {
            Ok(packet) => packet,
            Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => break,
            Err(e) => return Err(ChronuxError::FfmpegError(e.to_string())),
        };

        packet.set_stream_index(0);
        packet.rescale_ts(ctx.time_base, octx.streams()[0].time_base);
        octx.interleaved_write_frame(&mut packet)
            .map_err(|e| ChronuxError::FfmpegError(format!("Failed to write frame: {}", e)))?;
    }
    Ok(())
}
