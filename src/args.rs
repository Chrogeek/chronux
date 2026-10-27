use std::error::Error;

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
    #[arg(short = 't', long)]
    pub time: Option<f32>,
    #[arg(long, value_parser = parse_resolution, default_value = "1920x1080")]
    pub size: (u32, u32),
    #[arg(long, default_value_t = 30)]
    pub fps: u32,
    #[arg(long = "start")]
    pub time_start: Option<f32>,
    #[arg(long = "end")]
    pub time_end: Option<f32>,
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
    #[clap(name = "query")]
    Query,
}

fn parse_resolution(s: &str) -> Result<(u32, u32), Box<dyn Error + Send + Sync + 'static>> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        return Err(format!("invalid resolution '{}', use WIDTHxHEIGHT string", s).into());
    }
    let width = parts[0].parse::<u32>()?;
    let height = parts[1].parse::<u32>()?;
    Ok((width, height))
}
