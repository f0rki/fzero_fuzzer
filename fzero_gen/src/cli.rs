use fzero_gen::*;

fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    // Get access to the command line arguments
    let args: Vec<String> = std::env::args().collect();
    if !(3..=4).contains(&args.len()) {
        log::warn!("usage: fzero <grammar json> <output Rust file> [default max depth]");
        return Ok(());
    }

    // Load up a grammar file
    let grammar: Grammar = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    log::info!("Loaded grammar json; parsing grammar into in-memory format.");

    // Convert the grammar file to the Rust structures
    let mut gram = GrammarRust::new(&grammar, None);
    log::info!("Converted grammar to in-memory format; optimizing now.");

    // Optimize the grammar
    gram.optimize();
    log::info!("Optimized grammar; generating code.");

    // Generate a Rust application
    gram.program(
        &args[2],
        args.get(3)
            .unwrap_or(&("256".to_string()))
            .parse()
            .expect("Invalid digit in max depth"),
    );
    log::info!("Generated Rust source file");

    Ok(())
}
