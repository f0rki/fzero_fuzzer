use fzero_macro::fzero_define_token_grammar;

fzero_define_token_grammar!(SimpleJsTokenGrammar, [start, program, block], {
    start => ["var a = [];\nvar b = [];\nvar c = [];\nvar d = [];\n", program],
    program => [statements] | [blocks] | [loops],
    blocks => [block, block_sep] | [block, block_sep, blocks] | [block, block_sep, block, block_sep, blocks],
    block_sep => [""],
    block => ["\n{\n", statement, "\n", statements, "\n}\n"],
    statement => [varlet, var, " = ", value] | [var, " += ", value] | [var, " += ", var] | [increment] | [varlet, var, " = ", increment],
    increment => [var, "++"],
    varlet => [""] | ["let "] | ["var "] | ["const "],
    statement_sep => [";\n"],
    statements => [statement, statement_sep] | [statement, statement_sep, statements] | [statement, statement_sep, statement, statement_sep, statement, statement_sep, statements],
    loops => [forloop] | [forloop, loops] | [whileloop] | [whileloop, loops],
    forloop => ["for ", "(", statement, ";", " ", statement, ";", " ", statement, ")", " ", block],
    whileloop => ["while", "(", statement, ")", " ", block],
    var => ["a"] | ["b"] | ["c"] | ["d"] | ["e"] | ["f"] | ["g"] | ["h"],
    value => [literal],
    literal => [fixednums] | [fixedstrings] | [boolean],
    boolean => ["true"] | ["false"],
    fixedstrings => ["\"asdf\""] | ["\"qwertyuiop\""],
    fixednums => builtin!(numbers, number_limited),
});

fn main() -> std::io::Result<()> {
    use std::io::Write;

    /*
    for (i, tok) in SimpleJsTokenGrammar::terminals().iter().enumerate() {
        println!("{i} => {:?}", String::from_utf8_lossy(tok));
    }
    */

    let mut rng = rand::thread_rng();

    // instead of creating an immediate buffer, we directly write to stdout.
    let mut stdout = std::io::stdout().lock();

    for _ in 0..3 {
        let out = SimpleJsTokenGrammar::generate_new(Some(128), &mut rng);
        // stdout.write_all(b"\n==============================\n")?;
        // stdout.write_all(format!("{:?}", out).as_bytes())?;
        stdout.write_all(b"\n------------------------------\n")?;
        for idx in out.into_iter() {
            stdout.write_all(SimpleJsTokenGrammar::get_terminal(idx))?;
        }
    }

    std::io::Result::Ok(())
}
