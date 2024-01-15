pub mod generator;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let mut rng = rand::thread_rng();
    let out = generator::GrammarGenerator::generate_new(None, &mut rng);

    let mut stdout = io::stdout().lock();
    stdout.write_all(&out)?;

    Ok(())
}
