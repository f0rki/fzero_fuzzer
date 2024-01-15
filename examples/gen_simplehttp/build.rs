use fzero_gen::*;

fn main() -> std::io::Result<()> {
    let gfile = "../../grammars/simplehttp.json";

    println!("cargo:rerun-if-changed={}", gfile);

    let grammar: Grammar = serde_json::from_slice(&std::fs::read(gfile)?)?;
    println!("Loaded grammar json");

    // Convert the grammar file to the Rust structures
    let mut gram = GrammarRust::new(&grammar, None);
    println!("Converted grammar to binary format");

    // Optimize the grammar
    gram.optimize();
    println!("Optimized grammar");

    // Generate a Rust application
    gram.program("./src/generator.rs", 128);
    println!("Generated Rust source file");

    Ok(())
}
