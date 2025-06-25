use skia_safe::Surface;

use crate::{
    encoder::Encoder,
    util::{ChronuxError, ChronuxResult},
};

pub struct PngList {
    pub path: String,
}

pub struct PngListArgs {
    pub output_dir: String,
}

impl Encoder for PngList {
    type Args = PngListArgs;

    fn init(args: Self::Args) -> ChronuxResult<Self> {
        Ok(PngList {
            path: args.output_dir,
        })
    }

    fn write_frame(&self, f: usize, surface: &mut Surface) -> ChronuxResult<()> {
        let output = std::path::Path::new(&self.path).join(format!("frame_{:04}.png", f));
        let mut file = std::fs::File::create(&output).map_err(|e| ChronuxError::IoError(e))?;

        let pixmap = surface.peek_pixels().ok_or(ChronuxError::SkiaError(
            "failed to peek pixels from Skia surface".into(),
        ))?;

        skia_safe::png_encoder::encode(
            &pixmap,
            &mut file,
            &skia_safe::png_encoder::Options::default(),
        );
        Ok(())
    }

    fn finish(&self) -> ChronuxResult<()> {
        // No finalization needed for PNG list
        Ok(())
    }
}
