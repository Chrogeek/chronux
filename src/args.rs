use clap_derive::{Parser, ValueEnum};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short = 'i', long)]
    pub config: String,
    #[arg(short = 'o', long)]
    pub output: String,
    #[arg(short = 'w', long, value_enum, default_value_t = OutputWriter::PngSequence)]
    pub encoder: OutputWriter,
    #[arg(long = "timeline-only", default_value_t = false)]
    pub timeline_only: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputWriter {
    #[clap(name = "ffmpeg")]
    Ffmpeg,
    #[clap(name = "pngseq")]
    PngSequence,
}
