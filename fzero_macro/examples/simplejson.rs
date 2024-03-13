use fzero_macro::fzero_define_grammar;

fzero_define_grammar!(SimpleJsonGrammar, [json], {
    json => [element],
    element => [whitespace, jvalue, whitespace],
    jvalue => [object],
    object => ["{", members, "}"],
    members => [member, whitespace, ",", whitespace, members] | [member],
    member => [whitespace, key, whitespace, ":", whitespace, value, whitespace],
    key => ["\"a\""] | ["\"b\""] | ["\"c\""],
    value => [values],
    values =>  ["\"", strings, "\""] | [nums] | [fixednums],
    strings => ["asdf"] | ["bbbbbbb"],
    fixednums => [1] | [2] | [3] | [4],
    nums => builtin!(numbers, integer),
    whitespace => [" "] | ["\n"],
});

fn main() -> std::io::Result<()> {
    use std::io::Write;

    let mut rng = rand::thread_rng();
    let out = SimpleJsonGrammar::generate_new(Some(30), &mut rng);

    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&out)?;

    std::io::Result::Ok(())
}
