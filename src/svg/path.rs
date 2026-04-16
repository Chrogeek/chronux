use skia_safe::{Matrix, Path};

use crate::util::error::{ChronuxError, ChronuxResult};

pub fn svg_to_path(d: &str, mat: Matrix) -> ChronuxResult<Path> {
    match Path::from_svg(d) {
        Some(mut path) => {
            path.transform(&mat);
            Ok(path)
        }
        None => Err(ChronuxError::SvgError(format!(
            "failed to parse path for d = {d}"
        ))),
    }
}
