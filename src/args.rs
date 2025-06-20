use clap_derive::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short = 'i', long)]
    pub config: String,
    #[arg(short = 'O', long)]
    pub output_dir: String,
    #[arg(short = 'o', long)]
    pub output: String,
}
