use clap_derive::{Parser, ValueEnum};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short = 'i', long)]
    pub config: String,
    #[arg(short = 'o', long)]
    pub output_dir: String,
    #[arg(short = 'w', long, value_enum, default_value_t = OutputWriter::PngSequence)]
    pub encoder: OutputWriter,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputWriter {
    #[clap(name = "ffmpeg")]
    Ffmpeg,
    #[clap(name = "pngseq")]
    PngSequence,
}
