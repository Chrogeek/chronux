use std::{fs::File, io::{BufWriter, Write}};

use rsmpeg::{
    avcodec::AVCodecContext, avutil::AVFrame, error::RsmpegError,
};

use crate::util::{ChronuxError, ChronuxResult};

pub fn encode(
    ctx: &mut AVCodecContext,
    frame: Option<&AVFrame>,
    buf: &mut BufWriter<File>,
) -> ChronuxResult<()> {
    ctx.send_frame(frame)
        .map_err(|e| ChronuxError::FfmpegError(e.to_string()))?;
    loop {
        let packet = match ctx.receive_packet() {
            Ok(packet) => packet,
            Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => break,
            Err(e) => return Err(ChronuxError::FfmpegError(e.to_string())),
        };
        let data = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };
        buf.write_all(data).map_err(|e| ChronuxError::IoError(e))?;
    }
    Ok(())
}
