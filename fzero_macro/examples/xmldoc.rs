use fzero_macro::fzero_define_grammar;

fzero_define_grammar!(SimpleXmlDocGrammar, [start, xml], {
    start => ["<document>\n", xml_doc, "\n</document>"],
    xml_doc => [xml, "\n"] | [xml, "\n", xml, "\n"] | [xml, "\n", xml, "\n", xml, "\n"] | [xml, "\n", xml, "\n", xml, "\n", xml, "\n"] | [xml, "\n", xml_doc],
    xml => script!(xml_fragment, [tag, attr, xml_content]),
    xml_content => [xml_doc] | ["asdf"] | [text],
    tag => [header] | ["p"],
    header => ["h", headernums],
    headernums => [1] | [2] | [3] | [4],
    attr => ["foo=\"bar\""] | ["what=\"0\""],
    text => ["Rerum velit maxime perferendis ab eligendi ut velit."] | ["Eum culpa et error vitae."],
});

fn xml_fragment(buf: &mut Vec<u8>, fragments: &[&[u8]], _rng: &mut impl rand::Rng) {
    debug_assert!(fragments.len() == 3);
    buf.push(b'<');
    buf.extend_from_slice(fragments[0]);
    if !fragments[1].is_empty() {
        buf.push(b' ');
        buf.extend_from_slice(fragments[1]);
    }
    buf.push(b'>');

    buf.extend_from_slice(fragments[2]);

    buf.push(b'<');
    buf.push(b'/');
    buf.extend_from_slice(fragments[0]);
    buf.push(b'>');
}

fn main() -> std::io::Result<()> {
    use std::io::Write;

    let mut rng = rand::thread_rng();
    let out = SimpleXmlDocGrammar::generate_new(Some(32), &mut rng);

    // SimpleXmlDocGrammar::generate_xml_into(out, max_depth, rng)

    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&out)?;

    std::io::Result::Ok(())
}
