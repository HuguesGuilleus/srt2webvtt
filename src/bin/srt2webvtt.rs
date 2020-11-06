use srt2webvtt::*;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    /// The input subtitle format.
    #[structopt(long)]
    input_format: Option<Format>,
    /// The output subtitle format.
    #[structopt(long)]
    output_format: Option<Format>,
    /// The delta time to apply one subtitle.
    #[structopt(short, long, default_value = "0")]
    delta: Delta,

    input: Option<PathBuf>,
    output: Option<PathBuf>,
}

fn main() -> Result<(), ()> {
    let opt = Opt::from_args();
    let input_format = get_format(opt.input_format, &opt.input, "input")?;
    let output_format = get_format(opt.output_format, &opt.output, "output")?;

    let input: Box<dyn Read> = match opt.input {
        Some(p) => match File::open(p) {
            Ok(f) => Box::new(f),
            Err(err) => {
                eprintln!("{}", err);
                return Err(());
            }
        },
        None => Box::new(io::stdin()),
    };

    let output: Box<dyn Write> = match opt.output {
        Some(p) => match File::create(p) {
            Ok(f) => Box::new(f),
            Err(err) => {
                eprintln!("{}", err);
                return Err(());
            }
        },
        None => Box::new(io::stdout()),
    };

    match convert(input, input_format, output, output_format, opt.delta) {
        Ok(nb) => {
            println!("{} cues printed", nb);
            Ok(())
        }
        Err(e) => {
            eprintln!("{}", e);
            Err(())
        }
    }
}

fn get_format(f: Option<Format>, p: &Option<PathBuf>, t: &str) -> Result<Format, ()> {
    use std::convert::TryFrom;
    match f.or_else(|| p.as_ref().and_then(|p| Format::try_from(p).ok())) {
        Some(f) => Ok(f),
        None => {
            eprintln!("Need an format for the {}", t);
            Err(())
        }
    }
}
