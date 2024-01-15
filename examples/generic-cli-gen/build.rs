use fzero_gen::*;

fn main() -> std::io::Result<()> {
    let gfile = "../../grammars/url.json";
    let genv = "GRAMMAR";
    println!("cargo:rerun-if-changed={}", gfile);
    println!("cargo:rerun-if-env-changed={}", genv);

    let gfile = match std::env::var(genv) {
        Ok(s) => s,
        Err(e) => match e {
            std::env::VarError::NotPresent => gfile.to_string(),
            std::env::VarError::NotUnicode(osstr) => {
                panic!(
                    "non-unicode path to grammar given {:?}",
                    osstr.to_string_lossy()
                )
            }
        },
    };

    let grammar: Grammar = serde_json::from_slice(&std::fs::read(&gfile)?)?;
    println!("Loaded grammar json from {}", &gfile);

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
