use fzero_gen::*;

fn main() -> std::io::Result<()> {
    // Get access to the command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        println!("usage: fzero <grammar json> <output Rust file> <default max depth>");
        return Ok(());
    }

    // Load up a grammar file
    let grammar: Grammar = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    println!("Loaded grammar json");

    // Convert the grammar file to the Rust structures
    let mut gram = GrammarRust::new(&grammar, None);
    println!("Converted grammar to binary format");

    // Optimize the grammar
    gram.optimize();
    println!("Optimized grammar");

    // Generate a Rust application
    gram.program(
        &args[2],
        args[3].parse().expect("Invalid digit in max depth"),
    );
    println!("Generated Rust source file");

    Ok(())
}
