pub mod generator;
use clap::Parser;
use std::{
    io::{self, Write},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,

    #[arg(
        short,
        long,
        help = "Output file if count==1, else path to output directory"
    )]
    outpath: Option<PathBuf>,

    #[arg(short, long, help = "max depth passed to grammar generator")]
    max_depth: Option<usize>,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let mut rng = rand::thread_rng();

    let mut out = Vec::with_capacity(1024);

    if let Some(outpath) = args.outpath {
        if args.count == 1 && !outpath.exists() {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            generator::GrammarGenerator::generate_into(&mut out, args.max_depth, &mut rng);
            std::fs::write(outpath, &out)?;
        } else {
            if !outpath.exists() {
                std::fs::create_dir_all(&outpath)?;
            }
            let mut filepath = outpath.clone();
            for i in 0..args.count {
                generator::GrammarGenerator::generate_into(&mut out, args.max_depth, &mut rng);
                filepath.push(format!("{}", i));
                std::fs::write(&outpath, &out)?;
                filepath.pop();
            }
        }
    } else {
        let mut stdout = io::stdout().lock();
        for _ in 0..args.count {
            generator::GrammarGenerator::generate_into(&mut out, args.max_depth, &mut rng);
            stdout.write_all(&out)?;
            if args.count > 1 {
                stdout.write(b"\n")?;
            }
        }
    }
    Ok(())
}
