use fzero_macro::fzero_define_grammar;


fzero_define_grammar!(SimpleJsonGrammar, [json], {
    json => [element],
    element => [whitespace, jvalue, whitespace],
    jvalue => [object],
    object => ["{", members, "}"] | ["{", whitespace, "}"],
    members => [member, whitespace, ",", whitespace, members] | [member] | [],
    member => [whitespace, key, whitespace, ":", whitespace, value, whitespace],
    key => ["\"a\""] | ["\"b\""] | ["\"c\""],
    value => ["\"", values, "\""],
    values => [1] | [2] | [3] | ["asdf"],
    whitespace => [" "] | ["\n"] | [],
});

fn main() -> std::io::Result<()> {
    use std::io::Write;

    let mut rng = rand::thread_rng();
    let out = SimpleJsonGrammar::generate_new(None, &mut rng);

    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&out)?;

    std::io::Result::Ok(())
}
