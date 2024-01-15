// let grammar: Grammar = serde_json::from_slice(&std::fs::read(&gfile)?)?;
// println!("Loaded grammar json from {}", &gfile);
//
// // Convert the grammar file to the Rust structures
// let mut gram = GrammarRust::new(&grammar, None);

use lazy_static::lazy_static;

use crate::{Fragment, FragmentId, Grammar, GrammarRust};

lazy_static! {
    static ref STRING: GrammarRust = {
        let bytes = include_bytes!("../grammars/string.json");
        let grammar: Grammar = serde_json::from_slice(bytes).unwrap();
        println!("loading string grammar");
        GrammarRust::construct(&grammar)
    };
    static ref NUMBERS: GrammarRust = {
        let bytes = include_bytes!("../grammars/numbers.json");
        let grammar: Grammar = serde_json::from_slice(bytes).unwrap();
        println!("loading numbers grammar");
        GrammarRust::construct(&grammar)
    };
    static ref URL: GrammarRust = {
        let bytes = include_bytes!("../grammars/url.json");
        let grammar: Grammar = serde_json::from_slice(bytes).unwrap();
        GrammarRust::construct(&grammar)
    };
    static ref JSON: GrammarRust = {
        let bytes = include_bytes!("../grammars/json.json");
        let grammar: Grammar = serde_json::from_slice(bytes).unwrap();
        GrammarRust::construct(&grammar)
    };
    static ref HTTP_HEADERS: GrammarRust = {
        let bytes = include_bytes!("../grammars/http_headers.json");
        let grammar: Grammar = serde_json::from_slice(bytes).unwrap();
        GrammarRust::construct(&grammar)
    };
}

fn extend_and_rename(
    gram: &mut GrammarRust,
    with: &GrammarRust,
    rename_prefix: &str,
    search_for: &str,
) -> FragmentId {
    log::debug!(
        "loading builtin {} searching for {}",
        rename_prefix,
        search_for
    );

    let off = gram.fragments.len();
    gram.fragments
        .extend(with.fragments.iter().cloned().map(|f| match f {
            Fragment::NonTerminal(f) => {
                Fragment::NonTerminal(f.into_iter().map(|fid| FragmentId(fid.0 + off)).collect())
            }
            Fragment::Expression(f) => {
                Fragment::Expression(f.into_iter().map(|fid| FragmentId(fid.0 + off)).collect())
            }
            _ => f,
        }));

    let mut found = None;
    for (name, fragment_id) in with.name_to_fragment.iter() {
        let rename = if !name.starts_with("<!") {
            name.replace("<", rename_prefix)
        } else {
            name.clone()
        };
        log::debug!("renamed {} => {}", name, rename);
        let new_fragment_id = FragmentId(fragment_id.0 + off);

        if name == search_for || rename == search_for {
            found = Some(new_fragment_id);
        }

        gram.name_to_fragment.insert(rename, new_fragment_id);
        // }
    }

    if let Some(found) = found {
        return found;
    } else {
        panic!(
            "attempted to call invalid builtin {}{}>",
            rename_prefix, search_for
        );
    }
}

pub fn load_if_builtin(option: &str, gram: &mut GrammarRust) -> Option<FragmentId> {
    if option.starts_with("<!") && option.ends_with(">") {
        let option = &option[2..option.len() - 1];
        if let Some(point_idx) = option.find(".") {
            let (module, rule) = option.split_at(point_idx);
            let rule = &format!("<{}>", &rule[1..]);
            return match module {
                "string" => Some(extend_and_rename(gram, &STRING, "<!string.", rule)),
                "numbers" => Some(extend_and_rename(gram, &NUMBERS, "<!numbers.", rule)),
                "url" => Some(extend_and_rename(gram, &URL, "<!url.", rule)),
                "json" => Some(extend_and_rename(gram, &JSON, "<!json.", rule)),
                "http_headers" => Some(extend_and_rename(
                    gram,
                    &HTTP_HEADERS,
                    "<!http_headers.",
                    rule,
                )),
                _ => None,
            };
        }
    }
    None
}
