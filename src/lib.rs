use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod builtins;

/// Representation of a grammar file in a Rust structure. This allows us to
/// use Serde to serialize and deserialize the json grammar files
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Grammar(BTreeMap<String, Vec<Vec<String>>>);

/// A strongly typed wrapper around a `usize` which selects different fragment
/// identifiers
#[derive(Clone, Copy, Debug)]
pub struct FragmentId(usize);

/// A fragment which is specified by the grammar file
#[derive(Clone, Debug)]
pub enum Fragment {
    /// A non-terminal fragment which refers to a list of `FragmentId`s to
    /// randomly select from for expansion
    NonTerminal(Vec<FragmentId>),

    /// A list of `FragmentId`s that should be expanded in order
    Expression(Vec<FragmentId>),

    /// A terminal fragment which simply should expand directly to the
    /// contained vector of bytes
    Terminal(Vec<u8>),

    /// A fragment which does nothing. This is used during optimization passes
    /// to remove fragments with no effect.
    Nop,

    /// A fragment that is not reachable
    Unreachable,
}

/// A grammar representation in Rust that is designed to be easy to work with
/// in-memory and optimized for code generation.
#[derive(Debug, Default)]
pub struct GrammarRust {
    /// All types
    fragments: Vec<Fragment>,

    /// Cached fragment identifier for the start node
    start: Option<FragmentId>,

    /// Mapping of non-terminal names to fragment identifers
    name_to_fragment: BTreeMap<String, FragmentId>,

    /// If this is `true` then the output file we generate will not emit any
    /// unsafe code. I'm not aware of any bugs with the unsafe code that I use and
    /// thus this is by default set to `false`. Feel free to set it to `true` if
    /// you are concerned.
    pub safe_only: bool,
}

impl GrammarRust {
    /// Create a new Rust version of a `Grammar` which was loaded via a
    /// grammar json specification.
    pub fn new(grammar: &Grammar, start_fragment: Option<&str>) -> Self {
        let start_fragment = start_fragment.unwrap_or("<start>");

        let mut ret = Self::construct(grammar);

        // Resolve the start node
        ret.start = Some(ret.name_to_fragment[start_fragment]);

        ret
    }

    fn construct(grammar: &Grammar) -> Self {
        // Create a new grammar structure
        let mut ret = GrammarRust::default();
        ret.safe_only = false;

        // Parse the input grammar to resolve all fragment names
        for (non_term, _) in grammar.0.iter() {
            // Make sure that there aren't duplicates of fragment names
            assert!(
                !ret.name_to_fragment.contains_key(non_term),
                "Duplicate non-terminal definition, fail"
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

    /// Allocate a new fragment identifier and add it to the fragment list
    pub fn allocate_fragment(&mut self, fragment: Fragment) -> FragmentId {
        // Get a unique fragment identifier
        let fragment_id = FragmentId(self.fragments.len());

        // Store the fragment
        self.fragments.push(fragment);

        fragment_id
    }

    /// Optimize to remove fragments with non-random effects
    pub fn optimize(&mut self) {
        // Keeps track of fragment identifiers which resolve to nops
        let mut nop_fragments = BTreeSet::new();

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
                    }
                    Fragment::Terminal(_) | Fragment::Nop | Fragment::Unreachable => {
                        // Already maximally optimized
                    }
                }
            }
        }

        // only keep reachable fragments around
        let mut new_fragments = Vec::with_capacity(self.fragments.len());
        // initialize all fragments as Nop fragments
        new_fragments.resize(self.fragments.len(), Fragment::Unreachable);
        let mut seen_fragments = BTreeSet::new();
        let mut worklist = vec![self.start.unwrap()];

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
                Fragment::Terminal(_) | Fragment::Nop => {
                    // do nothing
                }

                Fragment::Unreachable => unreachable!("unreachable fragment reached!!!"),
            }
        }

        self.fragments = new_fragments;
    }

    /// Generate a new Rust program that can be built and will generate random
    /// inputs and benchmark them
    pub fn program<P: AsRef<Path>>(&self, path: P, max_depth: usize) {
        let mut program = String::new();

        let mut terminal_count = 0usize;
        let mut terminal_list = String::new();
        let mut seen_terminals = HashSet::new();
        for fragment in self.fragments.iter() {
            if let Fragment::Terminal(data) = fragment {
                let s = String::from_utf8_lossy(data);
                if !seen_terminals.contains(&s) {
                    terminal_list += &format!("{:?}, ", s);
                    terminal_count += 1;
                    seen_terminals.insert(s);
                }
            }
        }

        // Construct the base of the application. This is a profiling loop that
        // is used for testing.
        program += &format!(
            r#"
#![allow(unused)]
use std::cell::Cell;
use rand::Rng;

pub struct GrammarGenerator;

pub static TERMINALS: [&'static str; {}] = [{}];

impl GrammarGenerator {{

    pub fn terminals() -> &'static [&'static str] {{
        return &TERMINALS;
    }}

    pub fn generate_into(out: &mut Vec<u8>, max_depth: Option<usize>, rng: &mut impl Rng) {{
        out.clear();
        Self::fragment_{}(0, max_depth.unwrap_or({} as usize), out, rng);
    }}

    pub fn generate_new(max_depth: Option<usize>, rng: &mut impl Rng) -> Vec<u8> {{
        let mut out = Vec::new();
        Self::generate_into(&mut out, max_depth, rng);
        out
    }}
"#,
            terminal_count,
            terminal_list,
            self.start.unwrap().0,
            max_depth
        );

        // Go through each fragment in the list of fragments
        for (id, fragment) in self.fragments.iter().enumerate() {
            if matches!(fragment, Fragment::Unreachable) {
                continue;
            }

            // Create a new function for this fragment
            program += &format!("    fn fragment_{}(depth: usize, max_depth: usize, buf: &mut Vec<u8>, rng: &mut impl Rng) {{\n", id);

            // Add depth checking to terminate on depth exhaustion
            program.push_str("        if depth >= max_depth { return; }\n");

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
                Fragment::Terminal(value) => {
                    let as_str = String::from_utf8_lossy(value);
                    if !as_str.contains("*") {
                        program += &format!("        /* {:?} */", as_str);
                    }
                    /* {:?} */
                    if value.len() == 1 {
                        program += &format!("        buf.push({:?});\n", value[0]);
                    } else {
                        // Append the terminal value to the output buffer
                        if self.safe_only {
                            program += &format!("        buf.extend_from_slice(&{:?});\n", value);
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
                Fragment::Nop => {}
                Fragment::Unreachable => {}
            }

            program += "    }\n";
        }
        program += "}\n";

        // Write out the test application
        std::fs::write(path, program).expect("Failed to create output Rust application");
    }
}

pub fn generate_lib_from_grammar(
    grammar_file: impl AsRef<std::path::Path>,
    output_file: impl AsRef<std::path::Path>,
    default_max_depth: Option<usize>,
) -> std::io::Result<()> {
    let grammar: Grammar = serde_json::from_slice(&std::fs::read(grammar_file)?)?;
    let mut gram = GrammarRust::new(&grammar, None);
    gram.optimize();
    gram.program(output_file, default_max_depth.unwrap_or(128));

    Ok(())
}
