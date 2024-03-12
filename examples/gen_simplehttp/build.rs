use fzero_gen::*;

fn main() -> std::io::Result<()> {
    let gfile = "../../fzero_gen/grammars/simplehttp.json";

    println!("cargo:rerun-if-changed={}", gfile);

    let grammar: JsonGrammar = serde_json::from_slice(&std::fs::read(gfile)?)?;
    println!("Loaded grammar from json.");

    // Convert the grammar file to the Rust structures
    let mut gram = FGrammar::new(&grammar, None);
    println!("Created new code generator.");

    // Optimize the grammar
    gram.optimize();
    println!("Optimized grammar.");

    // Generate a Rust application
    gram.program("./src/generator.rs", 128);
    println!("Generated Rust source file.");

    Ok(())
}
