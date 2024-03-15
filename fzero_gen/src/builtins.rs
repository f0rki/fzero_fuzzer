// let grammar: Grammar = serde_json::from_slice(&std::fs::read(&gfile)?)?;
// println!("Loaded grammar json from {}", &gfile);
//
// // Convert the grammar file to the Rust structures
// let mut gram = GrammarRust::new(&grammar, None);

use lazy_static::lazy_static;

use crate::{FGrammar, FGrammarBuilder, Fragment, FragmentId, JsonGrammar};

lazy_static! {
    static ref STRING: FGrammar = {
        let bytes = include_bytes!("../grammars/string.json");
        let grammar: JsonGrammar = serde_json::from_slice(bytes).unwrap();
        FGrammarBuilder::from_json_grammar(&grammar, None).construct()
    };
    static ref NUMBERS: FGrammar = {
        let bytes = include_bytes!("../grammars/numbers.json");
        let grammar: JsonGrammar = serde_json::from_slice(bytes).unwrap();
        FGrammarBuilder::from_json_grammar(&grammar, None).construct()
    };
    static ref URL: FGrammar = {
        let bytes = include_bytes!("../grammars/url.json");
        let grammar: JsonGrammar = serde_json::from_slice(bytes).unwrap();
        FGrammarBuilder::from_json_grammar(&grammar, None).construct()
    };
    static ref JSON: FGrammar = {
        let bytes = include_bytes!("../grammars/json.json");
        let grammar: JsonGrammar = serde_json::from_slice(bytes).unwrap();
        FGrammarBuilder::from_json_grammar(&grammar, None).construct()
    };
    static ref HTTP: FGrammar = {
        let bytes = include_bytes!("../grammars/http.json");
        let grammar: JsonGrammar = serde_json::from_slice(bytes).unwrap();
        FGrammarBuilder::from_json_grammar(&grammar, None).construct()
    };
}

fn extend_and_rename(
    gram: &mut FGrammar,
    with: &FGrammar,
    rename_prefix: &str,
    search_for: &str,
) -> FragmentId {
    log::debug!(
        "loading builtin {} searching for {}",
        rename_prefix,
        search_for
    );

    let off = gram.fragments.len();
    let tid_off = gram.terminals.len();
    gram.terminals.extend(with.terminals.iter().cloned());
    gram.fragments
        .extend(with.fragments.iter().cloned().map(|f| match f {
            Fragment::NonTerminal(f) => {
                Fragment::NonTerminal(f.into_iter().map(|fid| FragmentId(fid.0 + off)).collect())
            }
            Fragment::Expression(f) => {
                Fragment::Expression(f.into_iter().map(|fid| FragmentId(fid.0 + off)).collect())
            }
            Fragment::Terminal(tid) => {
                // let new_tid = gram.
                Fragment::Terminal(tid + tid_off)
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

pub fn load_builtin(module: &str, rule: &str, gram: &mut FGrammar) -> Option<FragmentId> {
    let rule = &format!("<{}>", &rule[1..]);
    match module {
        "string" => Some(extend_and_rename(gram, &STRING, "<!string.", rule)),
        "numbers" => Some(extend_and_rename(gram, &NUMBERS, "<!numbers.", rule)),
        "url" => Some(extend_and_rename(gram, &URL, "<!url.", rule)),
        "json" => Some(extend_and_rename(gram, &JSON, "<!json.", rule)),
        "http" => Some(extend_and_rename(gram, &HTTP, "<!http.", rule)),
        _ => None,
    }
}

pub fn load_if_builtin(option: &str, gram: &mut FGrammar) -> Option<FragmentId> {
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
                "http" => Some(extend_and_rename(gram, &HTTP, "<!http.", rule)),
                _ => None,
            };
        }
    }
    None
}
