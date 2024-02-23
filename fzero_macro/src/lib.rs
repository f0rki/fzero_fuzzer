use fzero_gen::{Grammar, GrammarRust};
use proc_macro::{Delimiter, TokenStream, TokenTree};

#[proc_macro]
pub fn fzero_define_grammar(body: TokenStream) -> TokenStream {
    let max_depth = 128;
    let mut entrypoints = vec![];
    let mut grammar = Grammar::default();

    let call_error_msg = "call macro with (GrammarName, [ entrypoints ], {{ <grammar_def }})";

    let mut iter = body.into_iter();
    let name: String = if let TokenTree::Ident(i) = iter
        .next()
        .expect("Specify a grammar name as first argument.")
    {
        i.to_string()
    } else {
        panic!("Was expecting a grammar name as first argument");
    };

    let p = iter.next().expect(call_error_msg);
    if let TokenTree::Punct(p) = p {
        assert_eq!(p.as_char(), ',', "{call_error_msg}");
    } else {
        panic!("{call_error_msg}");
    }

    if let TokenTree::Group(g) = iter.next().expect(call_error_msg) {
        assert_eq!(
            g.delimiter(),
            Delimiter::Bracket,
            "expected '[' but got \"{}\"",
            g.span().source_text().unwrap_or("".to_string())
        );
        let mut expecting_ident = true;
        for tok in g.stream().into_iter() {
            if expecting_ident {
                if let TokenTree::Ident(i) = tok {
                    entrypoints.push(format!("<{}>", i.to_string()));
                    expecting_ident = false;
                } else {
                    panic!(
                        "expected identifier but got \"{}\"",
                        g.span().source_text().unwrap_or("".to_string())
                    );
                }
            } else {
                if let TokenTree::Punct(p) = tok {
                    if p.as_char() == ',' {
                        expecting_ident = true;
                        continue;
                    }
                }
                panic!(
                    "expected ',' but got \"{}\"",
                    g.span().source_text().unwrap_or("".to_string())
                );
            }
        }
        assert!(
            !entrypoints.is_empty(),
            "Specify at least one entrypoint for the grammar!"
        );
    } else {
        panic!("Was expecting a list of entrypoints as second argument.");
    }

    let p = iter.next().expect(call_error_msg);
    if let TokenTree::Punct(p) = p {
        assert_eq!(p.as_char(), ',', "{call_error_msg}");
    } else {
        panic!("{call_error_msg}");
    }

    if let TokenTree::Group(grp) = iter
        .next()
        .expect("Expecting grammar definition as second parameter to macro call")
    {
        assert_eq!(grp.delimiter(), Delimiter::Brace);
        #[derive(Debug, Eq, PartialEq)]
        enum ParseState {
            RuleIdent,
            Arrow(u8),
            RuleContent,
            Separator,
        }
        let mut state = ParseState::RuleIdent;
        let mut rule_name = String::new();
        let mut rule_contents = vec![];
        let mut current_rule = vec![];

        for tt in grp.stream().into_iter() {
            // eprintln!("{state:?} -> peek {:?}", tt);

            match state {
                ParseState::RuleIdent => {
                    if let TokenTree::Ident(i) = tt {
                        rule_name = format!("<{}>", i.to_string());
                        state = ParseState::Arrow(0);
                    } else {
                        panic!(
                            "expected identifier but got \"{}\"",
                            tt.span().source_text().unwrap_or("".to_string())
                        );
                    }
                }
                ParseState::Arrow(a) => {
                    if let TokenTree::Punct(p) = &tt {
                        if (a == 0 && p.as_char() == '=') || (a == 1 && p.as_char() == '>') {
                            if a == 0 {
                                state = ParseState::Arrow(1);
                                continue;
                            }
                            if a == 1 {
                                state = ParseState::RuleContent;
                                continue;
                            }
                        }
                    }
                    panic!(
                        "expected '=>' but got \"{}\"",
                        tt.span().source_text().unwrap_or("".to_string())
                    );
                }
                ParseState::RuleContent => {
                    if let TokenTree::Group(grp) = &tt {
                        if grp.delimiter() == Delimiter::Bracket {
                            let mut expecting_ident = true;
                            for tok in grp.stream().into_iter() {
                                if expecting_ident {
                                    // eprintln!("rule content: {:?}", tok);
                                    if let TokenTree::Ident(i) = tok {
                                        current_rule.push(format!("<{}>", i.to_string()));
                                        expecting_ident = false;
                                    } else if let TokenTree::Literal(_) = &tok {
                                        // eprintln!("literal: {}", t.to_string());
                                        use litrs::Literal;
                                        let lit = Literal::try_from(&tok)
                                            .expect("failed to parse literal with litrs");
                                        let slit = match lit {
                                            Literal::Integer(lit) => lit.raw_input().to_string(),
                                            Literal::Float(lit) => lit.raw_input().to_string(),
                                            Literal::Bool(lit) => {
                                                if matches!(lit, litrs::BoolLit::True) {
                                                    String::from("true")
                                                } else {
                                                    String::from("false")
                                                }
                                            }
                                            Literal::Char(lit) => {
                                                format!("{}", lit.value())
                                            }
                                            Literal::String(lit) => {
                                                format!("{}", lit.value())
                                            }
                                            _ => panic!("unsupported literal type"),
                                            // Literal::Byte(lit) => { /* ... */ }
                                            // Literal::ByteString(lit) => { /* ... */ }
                                        };
                                        // eprintln!("literal: {:?}", slit);
                                        current_rule.push(slit);
                                        expecting_ident = false;
                                    } else {
                                        panic!(
                                            "expected identifier but got \"{}\"",
                                            grp.span().source_text().unwrap_or("".to_string())
                                        );
                                    }
                                } else {
                                    if let TokenTree::Punct(p) = tok {
                                        if p.as_char() == ',' {
                                            expecting_ident = true;
                                            continue;
                                        }
                                    }
                                    panic!(
                                        "expected ',' but got \"{}\"",
                                        grp.span().source_text().unwrap_or("".to_string())
                                    );
                                }
                            }
                            state = ParseState::Separator;
                            continue;
                        }
                    }

                    panic!(
                        "expected rule content group '[ rule rule-1 ... ]' but got \"{}\"",
                        tt.span().source_text().unwrap_or("".to_string())
                    );
                }
                ParseState::Separator => {
                    if let TokenTree::Punct(p) = &tt {
                        if p.as_char() == '|' {
                            rule_contents.push(current_rule.clone());
                            current_rule.clear();
                            state = ParseState::RuleContent;
                            continue;
                        } else if p.as_char() == ',' {
                            if !current_rule.is_empty() {
                                rule_contents.push(current_rule.clone());
                            }

                            // eprintln!("Adding rule {:?}: {:?}", rule_name, rule_contents);

                            if grammar
                                .0
                                .insert(rule_name.clone(), rule_contents.clone())
                                .is_some()
                            {
                                panic!("Grammar contains duplicate rule name: {rule_name}");
                            }

                            state = ParseState::RuleIdent;
                            current_rule.clear();
                            rule_contents.clear();
                            continue;
                        }
                    }
                    panic!(
                        "expected ',' or '|' but got \"{}\"",
                        tt.span().source_text().unwrap_or("".to_string())
                    );
                }
            }
        }

        match state {
            ParseState::RuleIdent => {}
            ParseState::Separator => {}
            ParseState::Arrow(_) => {
                panic!("incomplete grammar definition: Expected '=>' and rule contents.");
            }
            ParseState::RuleContent => {
                panic!("incomplete grammar definition: Expected rule contents after '=>'.");
            }
        }
    } else {
        panic!("{call_error_msg}");
    }

    assert!(matches!(iter.next(), None));

    // for (name, rule) in grammar.0.iter() {
    //     eprintln!("{:?}: {:?}", name, rule);
    // }

    let mut gram = GrammarRust::new(&grammar, Some(&entrypoints[0]));
    gram.optimize();

    gram.rust_codegen(&name, max_depth).parse().unwrap()
}
