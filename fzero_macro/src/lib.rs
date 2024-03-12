use fzero_gen::{FGrammarBuilder, FGrammarIdent, FGrammarRule, FGrammarScriptCode};
use proc_macro::{Delimiter, TokenStream, TokenTree};

#[proc_macro]
pub fn fzero_define_grammar(body: TokenStream) -> TokenStream {
    let max_depth = 128;
    let mut entrypoints = vec![];
    // let mut grammar = JsonGrammar::default();
    let mut builder = FGrammarBuilder::default();

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
                    // entrypoints.push(format!("<{}>", i.to_string()));
                    entrypoints.push(i.to_string());
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
        enum ExclamationNext {
            Script,
            Builtin,
            Generate,
        }
        #[derive(Debug, Eq, PartialEq)]
        enum ParseState {
            RuleIdent,
            Arrow(u8),
            RuleContent,
            Separator,
            Script,
            Exclamation(ExclamationNext),
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
                        // rule_name = format!("<{}>", i.to_string());
                        rule_name = i.to_string();
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
                                        current_rule.push(FGrammarIdent::Ident(i.to_string()));
                                        expecting_ident = false;
                                    } else if let TokenTree::Literal(_) = &tok {
                                        // eprintln!("literal: {}", t.to_string());
                                        use litrs::Literal;
                                        let lit = Literal::try_from(&tok)
                                            .expect("failed to parse literal with litrs");
                                        let slit = match lit {
                                            Literal::Integer(lit) => {
                                                lit.raw_input().to_string().as_bytes().to_vec()
                                            }
                                            Literal::Float(lit) => {
                                                lit.raw_input().to_string().as_bytes().to_vec()
                                            }
                                            Literal::Bool(lit) => {
                                                if matches!(lit, litrs::BoolLit::True) {
                                                    b"true".to_vec()
                                                } else {
                                                    b"false".to_vec()
                                                }
                                            }
                                            Literal::Char(lit) => {
                                                format!("{}", lit.value()).as_bytes().to_vec()
                                            }
                                            Literal::String(lit) => {
                                                format!("{}", lit.value()).as_bytes().to_vec()
                                            }
                                            Literal::Byte(lit) => {
                                                vec![lit.value()]
                                            }
                                            Literal::ByteString(lit) => lit.value().to_vec(),
                                            // _ => panic!("unsupported literal type"),
                                        };
                                        // eprintln!("literal: {:?}", slit);
                                        current_rule.push(FGrammarIdent::Data(slit));
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

                    if let TokenTree::Ident(possible_script) = &tt {
                        if &possible_script.to_string() == "script" {
                            state = ParseState::Exclamation(ExclamationNext::Script);
                            continue;
                        }
                        if &possible_script.to_string() == "builtin" {
                            state = ParseState::Exclamation(ExclamationNext::Builtin);
                            continue;
                        }
                        if &possible_script.to_string() == "generate" {
                            state = ParseState::Exclamation(ExclamationNext::Generate);
                            continue;
                        }
                    }

                    panic!(
                        "expected rule content group '[ rule rule-1 ... ]' or script call 'script(RustStruct, [fragment_args...])' but got \"{}\"",
                        tt.span().source_text().unwrap_or("".to_string())
                    );
                }
                ParseState::Exclamation(next) => {
                    if let TokenTree::Punct(p) = &tt {
                        if p.as_char() == '!' {
                            match next {
                                ExclamationNext::Script => {
                                    state = ParseState::Script;
                                }
                                _ => {
                                    unimplemented!();
                                }
                            }
                            continue;
                        }
                    }

                    panic!("expected '!' after seeing (script|builtin|generate) identifier");
                }
                ParseState::Script => {
                    // eprintln!("script rule: {:?}", &tt);
                    assert!(current_rule.is_empty());
                    if let TokenTree::Group(grp) = &tt {
                        if grp.delimiter() == Delimiter::Parenthesis {
                            let mut grp_contents: Vec<TokenTree> =
                                grp.stream().into_iter().collect();
                            // eprintln!("group contents of script: {:?}", &grp_contents);
                            let mut function_name = String::new();

                            for tok in grp_contents.drain(..grp_contents.len() - 1) {
                                match tok {
                                    TokenTree::Punct(p) => {
                                        let c = p.as_char();
                                        if c == ',' {
                                            break;
                                        } else {
                                            function_name.push(c);
                                        }
                                    }
                                    TokenTree::Ident(ident) => {
                                        function_name += &ident.to_string();
                                    }
                                    _ => panic!(
                                        "unexpected tokens in script call: \"{}\"",
                                        tt.span().source_text().unwrap_or("".to_string())
                                    ),
                                }
                                // eprintln!("function_name: {}", function_name);
                            }

                            if grp_contents.len() == 1 {
                                if let TokenTree::Group(argp_grp) = grp_contents
                                    .pop()
                                    .expect("expected argument list for script call")
                                {
                                    if argp_grp.delimiter() == Delimiter::Bracket {
                                        let mut expecting_ident = true;
                                        let mut args = vec![];
                                        for tok in argp_grp.stream().into_iter() {
                                            match tok {
                                                TokenTree::Ident(ident) => {
                                                    if expecting_ident {
                                                        args.push(ident.to_string());
                                                        expecting_ident = false;
                                                    } else {
                                                        panic!("Invalid argument list for script call: was expecting identifier, got \"{}\"", tt.span().source_text().unwrap_or("".to_string()));
                                                    }
                                                }
                                                TokenTree::Punct(punct) => {
                                                    if !expecting_ident {
                                                        if punct.as_char() == ',' {
                                                            expecting_ident = true;
                                                        } else {
                                                            panic!("Invalid argument list for script call: was expecting ',', got \"{}\"", tt.span().source_text().unwrap_or("".to_string()));
                                                        }
                                                    } else {
                                                    }
                                                }
                                                _ => {
                                                    panic!("Invalid argument list for script call: was expecting list of identifiers, got \"{}\"", tt.span().source_text().unwrap_or("".to_string()));
                                                }
                                            }
                                        }

                                        // FGrammarRule::ScriptRule(
                                        //     FGrammarScriptCode(function_name),
                                        //     args,
                                        // ));
                                        //

                                        // let sargs: Vec<&str> = args.iter().map(|x| x.as_str()).collect();
                                        builder.add_script(&rule_name, function_name, &args);

                                        state = ParseState::Separator;
                                        continue;
                                    }
                                }
                            }
                        }
                    }

                    panic!(
                        "expected proper script call 'script(rust_function, [fragment_arg, ...])' but got \"{}\"",
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

                            /*
                            if grammar
                                .0
                                .insert(rule_name.clone(), rule_contents.clone())
                                .is_some()
                            {
                                panic!("Grammar contains duplicate rule name: {rule_name}");
                            }
                            */

                            for rc in rule_contents.iter() {
                                builder.add_rule(rule_name.as_str(), &rc);
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
            ParseState::Script => {
                panic!("incomplete grammar definition: Expected script contents after '=>'.");
            }
            ParseState::Exclamation(next) => {
                panic!(
                    "incomplete grammar definition: Expected {:?} after '!'",
                    next
                );
            }
        }
    } else {
        panic!("{call_error_msg}");
    }

    assert!(matches!(iter.next(), None));

    // for (name, rule) in grammar.0.iter() {
    //     eprintln!("{:?}: {:?}", name, rule);
    // }

    // let mut gram = FGrammar::new(&grammar, Some(&entrypoints[0]));
    // gram.optimize();
    for entry in entrypoints {
        builder.add_entrypoint(&entry);
    }

    let gram = builder.build();

    // eprintln!("{}", gram.rust_codegen(&name, max_depth));
    gram.rust_codegen(&name, max_depth).parse().unwrap()
}
