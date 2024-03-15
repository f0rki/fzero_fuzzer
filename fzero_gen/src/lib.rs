use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

mod builtins;

/// Representation of a grammar file in a Rust structure. This allows us to
/// use Serde to serialize and deserialize the json grammar files
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct JsonGrammar(pub BTreeMap<String, Vec<Vec<String>>>);

/// A strongly typed wrapper around a `usize` which selects different fragment
/// identifiers
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FragmentId(usize);

// #[derive(Clone, Copy, Debug)]
// pub struct TerminalId(usize);

/// A fragment which is specified by the grammar file
#[derive(Clone, Debug)]
pub enum Fragment {
    /// A non-terminal fragment which refers to a list of `FragmentId`s to
    /// randomly select from for expansion, i.e., this is a production rule:
    /// `A → B | 'c'`
    NonTerminal(Vec<FragmentId>),

    /// A list of `FragmentId`s that should be expanded in order. This is one of the options on the
    /// right-hand side of a production rule. For example, for the rule `A → B C | 'c'`, we have two
    /// expressions associated with the non-terminal `A`, `B C` and `'c'`.
    Expression(Vec<FragmentId>),

    /// A terminal fragment which simply should expand directly to the
    /// contained vector of bytes.
    Terminal(usize),

    /// A script object, which has associated code.
    Script(Vec<FragmentId>, String),

    /// A fragment which does nothing. This is used during optimization passes
    /// to remove fragments with no effect.
    Nop,

    /// A fragment that is not reachable. This is used during optimization to remove dead code,
    /// i.e., fragments that become unreachable during optimization. This allows us to reduce the
    /// amount of code we emit.
    Unreachable,
}

/// A grammar representation in Rust that is designed to be easy to work with
/// in-memory and optimized for code generation.
#[derive(Debug, Default)]
pub struct FGrammar {
    /// All types
    fragments: Vec<Fragment>,

    /// list of unique terminals
    terminals: Vec<Vec<u8>>,

    /// Cached fragment identifier for the start node
    entry_points: Vec<(String, FragmentId)>,

    // /// list of terminals
    // terminals: Vec<Vec<u8>>,
    /// Mapping of non-terminal names to fragment identifers
    name_to_fragment: HashMap<String, FragmentId>,

    /// do not emit recursion check for these fragments
    skip_recursion_check: HashSet<FragmentId>,

    /// If this is `true` then the output file we generate will not emit any
    /// unsafe code. I'm not aware of any bugs with the unsafe code that I use and
    /// thus this is by default set to `false`. Feel free to set it to `true` if
    /// you are concerned.
    pub safe_only: bool,

    /// If this is `true`, the output type will be a list of terminal indices, i.e., `Vec<u32>`, instead of a raw output buffer, i.e., `Vec<u8>`. The terminals can then be obtained by calling
    /// `terminals()` or `get_terminal(idx)`.
    pub output_terminal_ids: bool,
}

#[derive(Debug, Clone)]
pub enum FGrammarIdent {
    Ident(String),
    Data(Vec<u8>),
    ModuleIdent(String, String),
}

#[derive(Debug, Clone)]
pub struct FGrammarScriptCode(pub String);

#[derive(Debug, Clone)]
pub enum FGrammarRule {
    ProdRule(Vec<Vec<FGrammarIdent>>),
    ScriptRule(FGrammarScriptCode, Vec<String>),
}

/// Facilitate incremental building of a grammar.
#[derive(Debug, Clone, Default)]
pub struct FGrammarBuilder {
    rules: HashMap<String, FGrammarRule>,
    entrypoints: Vec<String>,
}

impl FGrammarBuilder {
    /// add a terminal rule, i.e., a rule that produces only a single value.
    ///
    /// ```ignore
    /// A → 'a'
    /// ```
    pub fn add_terminal(&mut self, ident: &str, data: &[u8]) {
        use hashbrown::hash_map::Entry;
        let ident = ident.to_string();
        match self.rules.entry(ident) {
            Entry::Vacant(entry) => {
                entry.insert(FGrammarRule::ProdRule(vec![vec![FGrammarIdent::Data(
                    data.to_vec(),
                )]]));
            }
            Entry::Occupied(mut entry) => {
                let rules = entry.get_mut();
                if let FGrammarRule::ProdRule(ref mut rules) = rules {
                    rules.push(vec![FGrammarIdent::Data(data.to_vec())]);
                } else {
                    panic!("cannot add terminal to non Production rule");
                }
            }
        };
    }

    /// Builder pattern of [`Self::add_terminal`].
    pub fn with_terminal(mut self, ident: &str, data: &[u8]) -> Self {
        self.add_terminal(ident, data);
        self
    }

    /// add multiple terminals, i.e., you have a rule that expands to one of multiple terminal
    /// symbols.
    ///
    /// ```ignore
    /// A → 'a' | 'b' | 'c'
    /// ```
    pub fn add_terminals(&mut self, ident: &str, data: &[&[u8]]) {
        use hashbrown::hash_map::Entry;
        let ident = ident.to_string();
        let data_vec = data
            .into_iter()
            .map(|d| vec![FGrammarIdent::Data(d.to_vec())])
            .collect();
        match self.rules.entry(ident) {
            Entry::Vacant(entry) => {
                entry.insert(FGrammarRule::ProdRule(data_vec));
            }
            Entry::Occupied(mut entry) => {
                let rules = entry.get_mut();
                if let FGrammarRule::ProdRule(ref mut rules) = rules {
                    rules.extend_from_slice(&data_vec);
                } else {
                    panic!("cannot add terminal to non Production rule");
                }
            }
        };
    }

    /// Builder pattern of [`Self::add_terminals`].
    pub fn with_terminals(mut self, ident: &str, data: &[&[u8]]) -> Self {
        self.add_terminals(ident, data);
        self
    }

    /// Add an expression, a rule that only consists of other non-terminals that are expanded in
    /// order.
    ///
    /// ```ignore
    /// A → B C D
    /// ```
    pub fn add_expression(&mut self, ident: &str, rule: &[&str]) {
        use hashbrown::hash_map::Entry;
        let ident = ident.to_string();
        let frule = rule
            .into_iter()
            .map(|id| FGrammarIdent::Ident(id.to_string()))
            .collect();
        match self.rules.entry(ident) {
            Entry::Vacant(entry) => {
                entry.insert(FGrammarRule::ProdRule(vec![frule]));
            }
            Entry::Occupied(mut entry) => {
                let rules = entry.get_mut();
                if let FGrammarRule::ProdRule(ref mut rules) = rules {
                    rules.push(frule);
                } else {
                    panic!("cannot add terminal to non Production rule");
                }
            }
        }
    }

    /// Builder pattern of [`Self::add_expression`].
    pub fn with_expression(mut self, ident: &str, rule: &[&str]) -> Self {
        self.add_expression(ident, rule);
        self
    }

    /// Add a production rule.
    ///
    /// ```ignore
    /// A → B | C | 'term'
    /// ```
    pub fn add_rule(&mut self, ident: &str, rule: &[FGrammarIdent]) {
        use hashbrown::hash_map::Entry;
        let ident = ident.to_string();
        let frule = rule.to_vec();
        match self.rules.entry(ident) {
            Entry::Vacant(entry) => {
                entry.insert(FGrammarRule::ProdRule(vec![frule]));
            }
            Entry::Occupied(mut entry) => {
                let rules = entry.get_mut();
                if let FGrammarRule::ProdRule(ref mut rules) = rules {
                    rules.push(frule);
                } else {
                    panic!("cannot add terminal to non Production rule");
                }
            }
        }
    }

    /// Builder pattern of [`Self::add_rule`].
    pub fn with_rule(mut self, ident: &str, rule: &[FGrammarIdent]) -> Self {
        self.add_rule(ident, rule);
        self
    }

    /// Add a script rule to handle more than a context-free grammar could.
    pub fn add_generator(&mut self, ident: &str, code: String) {
        let res = self.rules.insert(
            ident.to_string(),
            FGrammarRule::ScriptRule(FGrammarScriptCode(code.to_string()), vec![]),
        );
        if res.is_some() {
            panic!("overwriting existing rule '{}' with script rule!", ident);
        }
    }

    /// Builder pattern of [`Self::add_script`].
    pub fn with_generator(mut self, ident: &str, code: String) -> Self {
        self.add_generator(ident, code);
        self
    }

    /// Add a script rule to handle more than a context-free grammar could.
    pub fn add_script<T: AsRef<str>>(&mut self, ident: &str, code: String, args: &[T]) {
        let res = self.rules.insert(
            ident.to_string(),
            FGrammarRule::ScriptRule(
                FGrammarScriptCode(code.to_string()),
                args.into_iter().map(|id| id.as_ref().to_string()).collect(),
            ),
        );
        if res.is_some() {
            panic!("overwriting existing rule '{}' with script rule!", ident);
        }
    }

    /// Builder pattern of [`Self::add_script`].
    pub fn with_script(mut self, ident: &str, code: String, args: &[&str]) -> Self {
        self.add_script(ident, code, args);
        self
    }

    /// Mark a certain production rule as starting point for the grammar.
    pub fn add_entrypoint(&mut self, entrypoint: &str) {
        self.entrypoints.push(entrypoint.to_string());
    }

    /// Builder pattern of [`Self::add_entrypoint`].
    pub fn with_entrypoint(mut self, entrypoint: &str) -> Self {
        self.add_entrypoint(entrypoint);
        self
    }

    /// Construct unoptimized [`FGrammar`] structure.
    pub fn construct(&self) -> FGrammar {
        // Create a new grammar structure
        let mut ret = FGrammar::default();
        ret.safe_only = false;
        ret.output_terminal_ids = false;

        // Parse the input grammar to resolve all fragment names
        for (non_term, _) in self.rules.iter() {
            let fragment_id = ret.allocate_fragment(Fragment::NonTerminal(Vec::new()));

            // Add the name resolution for the fragment
            ret.name_to_fragment.insert(non_term.clone(), fragment_id);
        }

        for entry in self.entrypoints.iter() {
            let fragment_id = *ret.name_to_fragment.get(entry).expect(&format!(
                "Specified entrypoint {:?} must be part of the grammar.",
                &entry
            ));
            ret.entry_points.push((entry.to_string(), fragment_id));
        }

        // Parse the input grammar
        for (non_term, rule_content) in self.rules.iter() {
            // Create a vector to hold all of the variants possible under this
            // non-terminal fragment
            let mut variants = Vec::new();

            match rule_content {
                FGrammarRule::ScriptRule(code, args) => {
                    let mut arg_fragments = Vec::with_capacity(args.len());
                    for arg in args {
                        let fragment_id = if let Some(&non_terminal) = ret.name_to_fragment.get(arg)
                        {
                            // If we can resolve the name of this fragment, it is a
                            // non-terminal fragment and should be allocated as
                            // such
                            ret.allocate_fragment(Fragment::NonTerminal(vec![non_terminal]))
                        } else if let Some(id) = builtins::load_if_builtin(arg, &mut ret) {
                            id
                        } else {
                            panic!("Script argument is unknown non-terminal");
                        };
                        arg_fragments.push(fragment_id);
                    }
                    variants.push(
                        ret.allocate_fragment(Fragment::Script(arg_fragments, code.0.clone())),
                    );
                }
                FGrammarRule::ProdRule(fragments) => {
                    for js_sub_fragment in fragments {
                        // Different options for this sub-fragment
                        let mut options = Vec::new();

                        // Go through each option in the sub-fragment
                        for option in js_sub_fragment {
                            let fragment_id = match option {
                                FGrammarIdent::Data(data) => {
                                    ret.allocate_terminal_fragment(data.as_ref())
                                }
                                FGrammarIdent::Ident(nonterm_name) => {
                                    if let Some(&non_terminal) =
                                        ret.name_to_fragment.get(nonterm_name)
                                    {
                                        // If we can resolve the name of this fragment, it is a
                                        // non-terminal fragment and should be allocated as
                                        // such
                                        ret.allocate_fragment(Fragment::NonTerminal(vec![
                                            non_terminal,
                                        ]))
                                    } else if let Some(id) =
                                        builtins::load_if_builtin(nonterm_name, &mut ret)
                                    {
                                        id
                                    } else {
                                        panic!(
                                            "got identifier {:?} that cannot be resolved!",
                                            nonterm_name
                                        )
                                    }
                                }
                                FGrammarIdent::ModuleIdent(module, id) => {
                                    // TODO: bit of a hack to load builtin grammars
                                    let x = format!("<!{module}.{id}>");
                                    if let Some(id) = builtins::load_if_builtin(&x, &mut ret) {
                                        id
                                    } else {
                                        panic!(
                                            "invalid module identifier: {:?}.{:?} ({:?})",
                                            module, id, &x
                                        );
                                    }
                                }
                            };

                            // Push this fragment as an option
                            options.push(fragment_id);
                        }

                        // Create a new fragment of all the options
                        variants.push(ret.allocate_fragment(Fragment::Expression(options)));
                    }
                }
            }

            // Get the non-terminal fragment identifier
            let fragment_id = ret.name_to_fragment[non_term];
            // Get access to the fragment we want to update based on the
            // possible variants
            let fragment = &mut ret.fragments[fragment_id.0];
            // Overwrite the definition with the observed variants
            *fragment = Fragment::NonTerminal(variants);
        }

        ret.find_trivial_non_recursives();

        ret
    }

    /// Build an optimized [`FGrammar`].
    pub fn build(&self) -> FGrammar {
        let mut gram = self.construct();
        gram.optimize();
        gram
    }

    /// Parse the rules taken from a [`JsonGrammar`] which was loaded via a
    /// grammar json specification.
    pub fn from_json_grammar(grammar: &JsonGrammar, start_fragment: Option<&str>) -> Self {
        let mut ret = Self::default();

        if let Some(start) = start_fragment {
            ret.add_entrypoint(start);
        }

        for (non_term, rule) in grammar.0.iter() {
            for variant in rule.iter() {
                let mut brule = Vec::with_capacity(variant.len());
                for v in variant {
                    if v.starts_with("<") && v.ends_with(">") {
                        if v.starts_with("<!") {
                            let option = &v[2..v.len() - 1];
                            if let Some(point_idx) = option.find(".") {
                                let (module, rule) = option.split_at(point_idx);

                                brule.push(FGrammarIdent::ModuleIdent(
                                    module.to_string(),
                                    rule.to_string(),
                                ));
                            } else {
                                brule.push(FGrammarIdent::Ident(v.clone()));
                            }
                        } else {
                            brule.push(FGrammarIdent::Ident(v.clone()));
                        }
                    } else {
                        brule.push(FGrammarIdent::Data(v.as_bytes().to_vec()));
                    }
                }
                ret.add_rule(non_term, &brule);
            }
        }

        ret
    }
}

impl FGrammar {
    /*
        /// Create a new Rust version of a `Grammar` which was loaded via a
        /// grammar json specification.
        pub fn new(grammar: &Grammar, start_fragment: Option<&str>) -> Self {
            let start_fragment = start_fragment.unwrap_or("<start>");

            let mut ret = Self::construct(grammar);

            // Resolve the start node
            ret.start = Some(*ret.name_to_fragment.get(start_fragment).expect(&format!(
                "starting rule '{start_fragment}' must be part of the grammar"
            )));

            ret
        }

        fn construct(grammar: &Grammar) -> Self {
            // Create a new grammar structure
            let mut ret = FGrammar::default();
            ret.safe_only = false;

            // Parse the input grammar to resolve all fragment names
            for (non_term, _) in grammar.0.iter() {
                // Make sure that there aren't duplicates of fragment names
                assert!(
                    !ret.name_to_fragment.contains_key(non_term),
                    "Invalid Grammar: Duplicate non-terminal definition '{:?}'",
                    non_term
                );

                // Create a new, empty fragment
                let fragment_id = ret.allocate_fragment(Fragment::NonTerminal(Vec::new()));

                // Add the name resolution for the fragment
                ret.name_to_fragment.insert(non_term.clone(), fragment_id);
            }

            // Parse the input grammar
            for (non_term, fragments) in grammar.0.iter() {
                // Get the non-terminal fragment identifier
                let fragment_id = ret.name_to_fragment[non_term];

                // Create a vector to hold all of the variants possible under this
                // non-terminal fragment
                let mut variants = Vec::new();

                // Go through all sub-fragments
                for js_sub_fragment in fragments {
                    // Different options for this sub-fragment
                    let mut options = Vec::new();

                    // Go through each option in the sub-fragment
                    for option in js_sub_fragment {
                        let fragment_id = if let Some(&non_terminal) = ret.name_to_fragment.get(option)
                        {
                            // If we can resolve the name of this fragment, it is a
                            // non-terminal fragment and should be allocated as
                            // such
                            ret.allocate_fragment(Fragment::NonTerminal(vec![non_terminal]))
                        } else if let Some(id) = builtins::load_if_builtin(option, &mut ret) {
                            id
                        } else {
                            if option.starts_with("<") && option.ends_with(">") {
                                log::warn!("using a string that looks like a rule identifier ({:?}) as byte literal; check whether your grammar is correct!", option);
                            }

                            // Convert the terminal bytes into a vector and
                            // create a new fragment containing it
                            ret.allocate_fragment(Fragment::Terminal(option.as_bytes().to_vec()))
                        };

                        // Push this fragment as an option
                        options.push(fragment_id);
                    }

                    // Create a new fragment of all the options
                    variants.push(ret.allocate_fragment(Fragment::Expression(options)));
                }

                // Get access to the fragment we want to update based on the
                // possible variants
                let fragment = &mut ret.fragments[fragment_id.0];

                // Overwrite the terminal definition
                *fragment = Fragment::NonTerminal(variants);
            }

            ret
        }

    */

    /// Allocate a new fragment identifier and add it to the fragment list.
    pub fn allocate_fragment(&mut self, fragment: Fragment) -> FragmentId {
        // Get a unique fragment identifier
        let fragment_id = FragmentId(self.fragments.len());

        // Store the fragment
        self.fragments.push(fragment);

        fragment_id
    }

    pub fn add_terminal(&mut self, data: &[u8]) -> usize {
        if let Some(idx) = self.terminals.iter().position(|x| x == data) {
            idx
        } else {
            let l = self.terminals.len();
            self.terminals.push(data.to_vec());
            l
        }
    }

    /// Allocate a new fragment identifier for a terminal, add the terminal to the list of terminals and add it to the fragment list.
    pub fn allocate_terminal_fragment(&mut self, data: &[u8]) -> FragmentId {
        let term_id = self.add_terminal(data);
        let fragment = Fragment::Terminal(term_id);

        // Get a unique fragment identifier
        let fragment_id = FragmentId(self.fragments.len());
        // Store the fragment
        self.fragments.push(fragment);

        fragment_id
    }

    /// does two passes over all fragments and checks whether they are trivialy non-recursive, i.e.,
    /// they only expand to rules that expand to terminals or they.
    pub fn find_trivial_non_recursives(&mut self) {
        self.skip_recursion_check.clear();
        // need to do two passes over this.
        for _ in 0..2 {
            for idx in 0..self.fragments.len() {
                let fragmentid = FragmentId(idx);
                match &self.fragments[idx] {
                    Fragment::Terminal(_) | Fragment::Nop | Fragment::Unreachable => {
                        self.skip_recursion_check.insert(fragmentid);
                    }
                    Fragment::Expression(expr) | Fragment::NonTerminal(expr) => {
                        let mut can_skip = true;
                        for e in expr.iter() {
                            if !self.skip_recursion_check.contains(e) {
                                can_skip = false;
                            }
                        }
                        if can_skip {
                            self.skip_recursion_check.insert(fragmentid);
                        }
                    }
                    Fragment::Script(_, _) => {}
                }
            }
        }
    }

    pub fn reduce_terminals(&mut self) {
        let mut terminals = vec![];
        std::mem::swap(&mut self.terminals, &mut terminals);

        for idx in 0..self.fragments.len() {
            if let Fragment::Terminal(tid) = self.fragments[idx] {
                self.fragments[idx] = Fragment::Terminal(self.add_terminal(&terminals[tid]));
            }
        }
    }

    /// Optimize to remove fragments with non-random effects.
    pub fn optimize(&mut self) {
        // Keeps track of fragment identifiers which resolve to nops
        let mut nop_fragments = HashSet::new();

        // Track if a optimization had an effect
        let mut changed = true;
        while changed {
            // Start off assuming no effect from optimzation
            changed = false;

            // Go through each fragment, looking for potential optimizations
            for idx in 0..self.fragments.len() {
                // Clone the fragment such that we can inspect it, but we also
                // can mutate it in place.
                match self.fragments[idx].clone() {
                    Fragment::NonTerminal(options) => {
                        // If this non-terminal only has one option, replace
                        // itself with the only option it resolves to
                        if options.len() == 1 {
                            self.fragments[idx] = self.fragments[options[0].0].clone();
                            changed = true;
                        }
                    }
                    Fragment::Expression(expr) => {
                        // If this expression doesn't have anything to do at
                        // all. Then simply replace it with a `Nop`
                        if expr.len() == 0 {
                            self.fragments[idx] = Fragment::Nop;
                            changed = true;

                            // Track that this fragment identifier now resolves
                            // to a nop
                            nop_fragments.insert(idx);
                        }

                        // If this expression only does one thing, then replace
                        // the expression with the thing that it does.
                        if expr.len() == 1 {
                            self.fragments[idx] = self.fragments[expr[0].0].clone();
                            changed = true;
                        }

                        // Remove all `Nop`s from this expression, as they
                        // wouldn't result in anything occuring.
                        if let Fragment::Expression(exprs) = &mut self.fragments[idx] {
                            // Only retain fragments which are not nops
                            exprs.retain(|x| {
                                if nop_fragments.contains(&x.0) {
                                    // Fragment was a nop, remove it
                                    changed = true;
                                    false
                                } else {
                                    // Fragment was fine, keep it
                                    true
                                }
                            });
                        }

                        // if expression consist only of terminals, replace with a new terminal that
                        // is the concatenation of all terminals in the expression.
                        if expr
                            .iter()
                            .all(|item| matches!(self.fragments[item.0], Fragment::Terminal(_)))
                        {
                            let mut concatenated = vec![];
                            for item in expr.iter().copied() {
                                if let Fragment::Terminal(term_idx) = self.fragments[item.0] {
                                    concatenated.extend_from_slice(&self.terminals[term_idx]);
                                }
                            }
                            let term_idx = self.terminals.len();
                            self.terminals.push(concatenated);
                            self.fragments[idx] = Fragment::Terminal(term_idx);
                            changed = true;
                        }
                    }
                    Fragment::Terminal(_)
                    | Fragment::Nop
                    | Fragment::Unreachable
                    | Fragment::Script(_, _) => {
                        // Already maximally optimized
                    }
                }
            }
        }

        // only keep reachable fragments around
        let mut new_fragments = Vec::with_capacity(self.fragments.len());
        // initialize all fragments as unreachable fragments
        new_fragments.resize(self.fragments.len(), Fragment::Unreachable);
        let mut seen_fragments = HashSet::new();
        let mut worklist: Vec<FragmentId> = self.entry_points.iter().map(|x| x.1).collect();

        // iterate over all fragments and only keep the ones that are reachable
        while !worklist.is_empty() {
            let idx = worklist.pop().unwrap().0;
            if seen_fragments.contains(&idx) {
                continue;
            }
            new_fragments[idx] = self.fragments[idx].clone();
            seen_fragments.insert(idx);
            match &self.fragments[idx] {
                Fragment::NonTerminal(options) => {
                    worklist.extend(options.iter().cloned());
                }
                Fragment::Expression(expr) => {
                    worklist.extend(expr.iter().cloned());
                }
                Fragment::Script(args, _) => {
                    worklist.extend(args.iter().cloned());
                }
                Fragment::Terminal(_) | Fragment::Nop => {
                    // do nothing
                }
                Fragment::Unreachable => unreachable!("unreachable fragment reached!!!"),
            }
        }

        self.fragments = new_fragments;

        self.reduce_terminals();

        self.find_trivial_non_recursives();
    }

    /// generator rust code
    pub fn rust_codegen(&self, name: &str, default_max_depth: usize) -> String {
        let mut program = String::new();

        let mut terminal_list = String::new();
        // let mut seen_terminals = HashSet::new();
        // for fragment in self.fragments.iter() {
        //     if let Fragment::Terminal(data) = fragment {
        //         let s = String::from_utf8_lossy(data);
        //         if !seen_terminals.contains(&s) {
        //             terminal_list += &format!("{:?}, ", s);
        //             seen_terminals.insert(s);
        //         }
        //     }
        // }
        for terminal in self.terminals.iter() {
            terminal_list += &format!("&{:?}, ", terminal);
        }

        let start = self
            .entry_points
            .first()
            .expect("Require a starting rule for the grammar")
            .1
             .0;

        let outtype = if self.output_terminal_ids {
            "Vec<u32>"
        } else {
            "Vec<u8>"
        };

        program += &format!(
            r#"

pub struct {name};

impl {name} {{

    #[inline(always)]
    pub fn terminals() -> &'static [&'static [u8]] {{
        &[{terminal_list}]
    }}

    #[inline(always)]
    pub fn get_terminal<I>(index: I) -> &'static [u8]
    where I: TryInto<usize>, <I as TryInto<usize>>::Error: std::fmt::Debug
    {{
        let index: usize = index.try_into().expect("usize should be bigger than u32!");
        Self::terminals()[index]
    }}

    pub fn generate_into(out: &mut {outtype}, max_depth: Option<usize>, rng: &mut impl rand::Rng) {{
        Self::fragment_{start}(0, max_depth.unwrap_or({} as usize), out, rng);
    }}

    pub fn generate_new(max_depth: Option<usize>, rng: &mut impl rand::Rng) -> {outtype} {{
        let mut out = Vec::new();
        Self::generate_into(&mut out, max_depth, rng);
        out
    }}
"#,
            default_max_depth
        );

        for (start_name, fragment) in self.entry_points.iter() {
            let fragment = fragment.0;
            program += &format!(
                r#"

    pub fn generate_{start_name}_into(out: &mut {outtype}, max_depth: Option<usize>, rng: &mut impl rand::Rng) {{
        Self::fragment_{fragment}(0, max_depth.unwrap_or({} as usize), out, rng);
    }}
    
    pub fn generate_{start_name}_new(max_depth: Option<usize>, rng: &mut impl rand::Rng) -> {outtype} {{
        let mut out = Vec::new();
        Self::fragment_{fragment}(0, max_depth.unwrap_or({} as usize), &mut out, rng);
        out
    }}

                "#,
                default_max_depth, default_max_depth
            );
        }

        // Go through each fragment in the list of fragments
        for (id, fragment) in self.fragments.iter().enumerate() {
            if matches!(fragment, Fragment::Unreachable) {
                continue;
            }

            // Create a new function for this fragment
            program += &format!("    #[allow(unused)]\nfn fragment_{}(depth: usize, max_depth: usize, buf: &mut {outtype}, rng: &mut impl rand::Rng) {{\nuse rand::Rng;\n", id);

            if !self.skip_recursion_check.contains(&FragmentId(id)) {
                // Add depth checking to terminate on depth exhaustion
                // program.push_str("        if depth >= max_depth { return; }\n");
                //
                program.push_str("        if depth >= max_depth {\n");
                let mut non_recursing = vec![];
                if let Fragment::NonTerminal(vars) = fragment {
                    for var in vars {
                        if self.skip_recursion_check.contains(var) {
                            non_recursing.push(*var);
                        }
                    }
                }
                if !non_recursing.is_empty() {
                    program += &format!(
                        "        match rng.gen_range(0..{}) {{\n",
                        non_recursing.len()
                    );

                    for (option_id, option) in non_recursing.iter().enumerate() {
                        program += &format!(
                            "            {} => Self::fragment_{}(depth + 1, max_depth, buf, rng),\n",
                            option_id, option.0
                        );
                    }
                    program += &format!("            _ => unreachable!(),\n");

                    program += &format!("        }}\n");
                }
                program.push_str("        return; }\n");
            }

            match fragment {
                Fragment::NonTerminal(options) => {
                    // For non-terminal cases pick a random variant to select
                    // and invoke that fragment's routine
                    program += &format!("        match rng.gen_range(0..{}) {{\n", options.len());

                    for (option_id, option) in options.iter().enumerate() {
                        program += &format!(
                            "            {} => Self::fragment_{}(depth + 1, max_depth, buf, rng),\n",
                            option_id, option.0
                        );
                    }
                    program += &format!("            _ => unreachable!(),\n");

                    program += &format!("        }}\n");
                }
                Fragment::Expression(expr) => {
                    // Invoke all of the expression's routines in order
                    for &exp in expr.iter() {
                        program += &format!(
                            "        Self::fragment_{}(depth + 1, max_depth, buf, rng);\n",
                            exp.0
                        );
                    }
                }
                Fragment::Terminal(term_idx) => {
                    program += &format!("        /* Self::get_terminal({:?}); */", term_idx);

                    let value = &self.terminals[*term_idx];
                    let as_str = String::from_utf8_lossy(value);
                    if !as_str.contains("*") {
                        program += &format!("        /* {:?} */", as_str);
                    }

                    if self.output_terminal_ids {
                        program += &format!("        buf.push({});\n", term_idx);
                    } else {
                        /* {:?} */
                        if value.len() == 1 {
                            program += &format!("        buf.push({:?});\n", value[0]);
                        } else {
                            // Append the terminal value to the output buffer
                            if self.safe_only {
                                program +=
                                    &format!("        buf.extend_from_slice(&{:?});\n", value);
                            } else {
                                // For some reason this is faster than
                                // `extend_from_slice` even though it does the exact
                                // same thing. This was observed to be over a 4-5x
                                // speedup in some scenarios.
                                program += &format!(
                                    r#"
            unsafe {{
                let old_size = buf.len();
                let new_size = old_size + {};

                if new_size > buf.capacity() {{
                    buf.reserve(new_size - old_size);
                }}

                std::ptr::copy_nonoverlapping({:?}.as_ptr(), buf.as_mut_ptr().offset(old_size as isize), {});
                buf.set_len(new_size);
            }}
    "#,
                                    value.len(),
                                    value,
                                    value.len()
                                );
                            }
                        }
                    }
                }
                Fragment::Script(args, code) => {
                    if args.is_empty() {
                        // a "script" without any arguments is a generator and we use a different
                        // calling convention there.
                        program += &format!("{code}(buf, rng);");
                    } else {
                        for (argnum, arg) in args.iter().copied().enumerate() {
                            let arg = arg.0;
                            program += &format!(
                                "let mut arg{argnum}_buf = vec![];\n
                        Self::fragment_{arg}(depth + 1, max_depth, &mut arg{argnum}_buf, rng);\n"
                            );
                        }
                        program += &format!("{code}(buf, &[");
                        for argnum in 0..args.len() {
                            program += &format!("&arg{argnum}_buf[..], ");
                        }
                        program += "], rng);\n";
                    }
                }
                Fragment::Nop => {}
                Fragment::Unreachable => {}
            }

            program += "    }\n";
        }
        program += "}\n";

        program
    }

    /// Generate rust code and write to given file.
    pub fn program<P: AsRef<Path>>(&self, path: P, max_depth: usize) {
        let program = self.rust_codegen("GrammarGenerator", max_depth);

        // Write out the test application
        std::fs::write(path, program).expect("Failed to create output Rust application");
    }
}

pub fn generate_lib_from_grammar(
    grammar_file: impl AsRef<std::path::Path>,
    output_file: impl AsRef<std::path::Path>,
    default_max_depth: Option<usize>,
) -> std::io::Result<()> {
    let grammar: JsonGrammar = serde_json::from_slice(&std::fs::read(grammar_file)?)?;
    let gram = FGrammarBuilder::from_json_grammar(&grammar, None).build();
    gram.program(output_file, default_max_depth.unwrap_or(128));

    Ok(())
}
