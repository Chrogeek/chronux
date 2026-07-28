use clap_derive::{Parser, ValueEnum};

use crate::util::version::CHRONUX_VERSION;

#[derive(Parser, Debug)]
#[command(version = CHRONUX_VERSION, about = "Declarative, event-driven animation render tool.", long_about = None)]
pub struct Args {
    #[arg(short = 'i', long)]
    pub config: Vec<String>,
    #[arg(short = 'o', long)]
    pub output: String,
    #[arg(short = 'w', long, value_enum, default_value_t = OutputWriter::PngSequence)]
    pub encoder: OutputWriter,
}

#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum OutputWriter {
    #[clap(name = "ffmpeg")]
    Ffmpeg,
    #[clap(name = "pngseq")]
    PngSequence,
    #[clap(name = "timeline")]
    Timeline,
    #[clap(name = "objects")]
    Objects,
}
