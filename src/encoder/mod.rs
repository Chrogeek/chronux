use skia_safe::Surface;

use crate::util::ChronuxResult;

pub mod h264;
pub mod png_list;

pub trait Encoder {
    type Args;

    /// Initializes the encoder.
    fn init(args: Self::Args) -> ChronuxResult<Self>
    where
        Self: Sized;

    /// Sets the output file path for the encoded video.
    fn write_frame(&self, f: usize, surface: &mut Surface) -> ChronuxResult<()>;

    /// Sets the output file path for the encoded video.
    fn finish(&self) -> ChronuxResult<()> {
        Ok(())
    }
}
