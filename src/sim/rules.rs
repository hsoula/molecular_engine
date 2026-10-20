//! Text chemistry.
//!
//! A chemistry file is a plain text file, one statement per line, `#` starts a comment.
//!
//! ```text
//! # reactions:  <lhs> -> <rhs> [@ probability]
//! a0 + a1 -> a1 . a2          # '+' = not bonded, '.' = bonded
//! *1 + *2 -> *3 . *4          # '*' = any type; in a product it keeps the reactant's type
//! X5 + X0 -> X7 . X6          # X..Z = type variable: both reactants must have the same type
//! X3 + Y6 -> X2 . Y3          # X and Y are independent (they may or may not be equal)
//! b1 . b2 -> c1 . b2 @ 0.1    # concrete product types change the type (transmutation)
//!
//! # spring parameters by bonded type pair, most specific match wins
//! spring * *  k=20 len=1.0
//! spring a b  k=60 len=0.9 break=2.5
//!
//! # physics / run parameters (see `reactor::Params`)
//! set temperature 0.3
//! set food abcdef
//! ```
//!
//! The legacy forms `a_0(+)a_1` are also accepted, and the old JSON rule files can be
//! imported with [`Chemistry::from_legacy_json`].
//!
//! Rule priority: when several rules match a pair, the one written first in the file wins.
//! A rule matches the pair (A,B) in either order.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

/// Number of atom types: `a`..`z`.
pub const NTYPES: usize = 26;

/// Index of a type letter in `0..NTYPES`.
pub fn type_index(form: char) -> usize {
    ((form as u32).wrapping_sub('a' as u32) as usize).min(NTYPES - 1)
}

/// A concrete (type, state) pair carried by an atom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Compound {
    pub form: char,
    pub state: i32,
}

impl fmt::Display for Compound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.form, self.state)
    }
}

/// Type part of a rule term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypePat {
    /// A concrete type `a`..`z`.
    Exact(char),
    /// `*`: any type. In a product it means "keep the type of the reactant at this position".
    Any,
    /// `X`..`Z` (any uppercase letter): a type variable. The same letter used twice on the
    /// left-hand side forces both reactants to have the same type; on the right-hand side
    /// it writes the bound type.
    Var(char),
}

impl TypePat {
    fn from_char(c: char) -> Option<TypePat> {
        match c {
            '*' => Some(TypePat::Any),
            'a'..='z' => Some(TypePat::Exact(c)),
            'A'..='Z' => Some(TypePat::Var(c)),
            _ => None,
        }
    }
}

impl fmt::Display for TypePat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypePat::Exact(c) | TypePat::Var(c) => write!(f, "{c}"),
            TypePat::Any => write!(f, "*"),
        }
    }
}

/// One side of a rule term: a type pattern and an exact state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Term {
    pub ty: TypePat,
    pub state: i32,
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.ty, self.state)
    }
}

/// `lhs[0] (+|.) lhs[1] -> rhs[0] (+|.) rhs[1] @ prob`
#[derive(Clone, Debug)]
pub struct Rule {
    pub id: usize,
    pub lhs: [Term; 2],
    pub lhs_bonded: bool,
    pub rhs: [Term; 2],
    pub rhs_bonded: bool,
    pub prob: f32,
    /// 1-based line in the source file (0 if built in code).
    pub line: usize,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = |b: bool| if b { "." } else { "+" };
        write!(
            f,
            "{}{}{}->{}{}{}",
            self.lhs[0],
            op(self.lhs_bonded),
            self.lhs[1],
            self.rhs[0],
            op(self.rhs_bonded),
            self.rhs[1]
        )?;
        if self.prob < 1.0 {
            write!(f, "@{}", self.prob)?;
        }
        Ok(())
    }
}

impl Rule {
    /// Parse a single rule, e.g. `"*1 + *2 -> *3 . *4"` or `"a_0(+)a_1->a_1(.)a_2"`.
    pub fn parse(text: &str) -> Result<Rule, String> {
        parse_rule(text, 0, 0)
    }

    /// Try to match the reactants **in this order**. Returns the two products.
    pub fn apply_ordered(&self, a: Compound, b: Compound) -> Option<[Compound; 2]> {
        if a.state != self.lhs[0].state || b.state != self.lhs[1].state {
            return None;
        }
        let input = [a, b];
        let mut vars: [(char, char); 2] = [('\0', '\0'); 2];
        let mut nv = 0;
        for k in 0..2 {
            let form = input[k].form;
            match self.lhs[k].ty {
                TypePat::Exact(c) => {
                    if c != form {
                        return None;
                    }
                }
                TypePat::Any => {}
                TypePat::Var(v) => match vars[..nv].iter().find(|(n, _)| *n == v) {
                    Some(&(_, bound)) => {
                        if bound != form {
                            return None;
                        }
                    }
                    None => {
                        vars[nv] = (v, form);
                        nv += 1;
                    }
                },
            }
        }
        let mut out = input;
        for k in 0..2 {
            out[k].state = self.rhs[k].state;
            out[k].form = match self.rhs[k].ty {
                TypePat::Exact(c) => c,
                TypePat::Any => input[k].form,
                // validated at parse time: every rhs variable is bound on the lhs
                TypePat::Var(v) => vars[..nv]
                    .iter()
                    .find(|(n, _)| *n == v)
                    .map(|x| x.1)
                    .unwrap_or(input[k].form),
            };
        }
        Some(out)
    }
}

/// Spring parameters for one bonded type pair (`None` = use the reactor default).
#[derive(Clone, Debug)]
pub struct SpringSpec {
    /// `None` = wildcard `*`
    pub a: Option<char>,
    pub b: Option<char>,
    pub k: Option<f32>,
    pub len: Option<f32>,
    pub break_len: Option<f32>,
}

impl SpringSpec {
    /// Specificity score if the spec applies to the (unordered) pair, else `None`.
    pub fn score(&self, x: char, y: char) -> Option<u8> {
        let m = |p: Option<char>, c: char| p.is_none_or(|p| p == c);
        if (m(self.a, x) && m(self.b, y)) || (m(self.a, y) && m(self.b, x)) {
            Some(self.a.is_some() as u8 + self.b.is_some() as u8)
        } else {
            None
        }
    }
}

/// Result of a rule lookup for an atom pair `(a, b)`; products are given in `(a, b)` order.
#[derive(Clone, Copy, Debug)]
pub struct Match {
    pub rule: usize,
    pub products: [Compound; 2],
}

#[derive(Clone, Debug, Default)]
pub struct Chemistry {
    pub rules: Vec<Rule>,
    pub springs: Vec<SpringSpec>,
    /// `set key value` lines, in file order.
    pub params: Vec<(String, String)>,
    pub warnings: Vec<String>,
    /// (bonded, state_a, state_b) -> rule ids, ascending
    index: HashMap<(bool, i32, i32), Vec<usize>>,
}

impl Chemistry {
    pub fn new() -> Chemistry {
        Chemistry::default()
    }

    /// Load a chemistry file: `.json` = legacy rule list, anything else = text format.
    pub fn load(path: impl AsRef<Path>) -> Result<Chemistry, String> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")) {
            Chemistry::from_legacy_json(&text)
        } else {
            Chemistry::parse(&text)
        }
        .map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Parse the text format (see module docs).
    pub fn parse(text: &str) -> Result<Chemistry, String> {
        let mut chem = Chemistry::new();
        for (n, raw) in text.lines().enumerate() {
            let line_no = n + 1;
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }
            let err = |e: String| format!("line {line_no}: {e}\n    {}", raw.trim());
            let first = line.split_whitespace().next().unwrap_or("");
            match first {
                "spring" => chem.springs.push(parse_spring(line).map_err(err)?),
                "set" => {
                    let mut it = line.splitn(3, char::is_whitespace);
                    it.next();
                    let key = it.next().unwrap_or("").trim();
                    let val = it.next().unwrap_or("").trim();
                    if key.is_empty() || val.is_empty() {
                        return Err(err("expected `set <name> <value>`".into()));
                    }
                    chem.params.push((key.to_string(), val.to_string()));
                }
                _ => {
                    let id = chem.rules.len();
                    let rule = parse_rule(line, id, line_no).map_err(err)?;
                    chem.rules.push(rule);
                }
            }
        }
        chem.check_shadowing();
        chem.rebuild_index();
        Ok(chem)
    }

    /// Import the old JSON rule list (`data/rules.json`): `contact: true` means bonded.
    pub fn from_legacy_json(text: &str) -> Result<Chemistry, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let list = v
            .as_array()
            .or_else(|| v["chemistry"]["rules"].as_array())
            .ok_or("expected a JSON array of rules")?;
        let term = |c: &serde_json::Value| -> Result<Term, String> {
            let form = c["form"].as_str().and_then(|s| s.chars().next()).ok_or("bad form")?;
            let state = c["state"].as_i64().ok_or("bad state")? as i32;
            Ok(Term { ty: TypePat::from_char(form).ok_or("bad form")?, state })
        };
        let mut chem = Chemistry::new();
        for (id, r) in list.iter().enumerate() {
            let s = &r["substrate"];
            let p = &r["product"];
            chem.rules.push(Rule {
                id,
                lhs: [term(&s["a1"])?, term(&s["a2"])?],
                lhs_bonded: s["contact"].as_bool().unwrap_or(false),
                rhs: [term(&p["a1"])?, term(&p["a2"])?],
                rhs_bonded: p["contact"].as_bool().unwrap_or(false),
                prob: 1.0,
                line: 0,
            });
        }
        chem.rebuild_index();
        Ok(chem)
    }

    /// Write back in the text format.
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        for (k, v) in &self.params {
            s += &format!("set {k} {v}\n");
        }
        for sp in &self.springs {
            let t = |c: Option<char>| c.map_or("*".to_string(), |c| c.to_string());
            s += &format!("spring {} {}", t(sp.a), t(sp.b));
            if let Some(k) = sp.k {
                s += &format!(" k={k}");
            }
            if let Some(l) = sp.len {
                s += &format!(" len={l}");
            }
            if let Some(b) = sp.break_len {
                s += &format!(" break={b}");
            }
            s += "\n";
        }
        for r in &self.rules {
            s += &format!("{r}\n");
        }
        s
    }

    pub fn add_rule(&mut self, mut rule: Rule) {
        rule.id = self.rules.len();
        self.rules.push(rule);
        self.rebuild_index();
    }

    fn rebuild_index(&mut self) {
        self.index.clear();
        for r in &self.rules {
            self.index
                .entry((r.lhs_bonded, r.lhs[0].state, r.lhs[1].state))
                .or_default()
                .push(r.id);
        }
    }

    /// Warn when a rule can never fire because an earlier, more general one always wins.
    fn check_shadowing(&mut self) {
        for (j, rj) in self.rules.iter().enumerate() {
            for ri in &self.rules[..j] {
                if ri.lhs_bonded == rj.lhs_bonded && covers(ri, rj) {
                    self.warnings.push(format!(
                        "rule `{rj}` (line {}) is shadowed by `{ri}` (line {})",
                        rj.line, ri.line
                    ));
                    break;
                }
            }
        }
    }

    /// First rule (in file order) matching the pair, in either orientation.
    pub fn find(&self, bonded: bool, a: Compound, b: Compound) -> Option<Match> {
        let mut best: Option<Match> = None;
        if let Some(list) = self.index.get(&(bonded, a.state, b.state)) {
            for &r in list {
                if let Some(p) = self.rules[r].apply_ordered(a, b) {
                    best = Some(Match { rule: r, products: p });
                    break;
                }
            }
        }
        if let Some(list) = self.index.get(&(bonded, b.state, a.state)) {
            for &r in list {
                if best.is_some_and(|m| m.rule <= r) {
                    break;
                }
                if let Some(p) = self.rules[r].apply_ordered(b, a) {
                    best = Some(Match { rule: r, products: [p[1], p[0]] });
                    break;
                }
            }
        }
        best
    }

    /// Value of a `set` parameter (last one wins).
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

/// True if every pair matched by `b` (in some orientation) is also matched by `a`
/// in the same orientation. Conservative: only handles patterns without variables in `a`.
fn covers(a: &Rule, b: &Rule) -> bool {
    let pat_covers = |pa: &Term, pb: &Term| {
        pa.state == pb.state
            && match (pa.ty, pb.ty) {
                (TypePat::Any, _) => true,
                (TypePat::Exact(x), TypePat::Exact(y)) => x == y,
                _ => false,
            }
    };
    let no_vars = a.lhs.iter().all(|t| !matches!(t.ty, TypePat::Var(_)));
    no_vars
        && ((pat_covers(&a.lhs[0], &b.lhs[0]) && pat_covers(&a.lhs[1], &b.lhs[1]))
            || (pat_covers(&a.lhs[0], &b.lhs[1]) && pat_covers(&a.lhs[1], &b.lhs[0])))
}

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or("")
}

fn parse_rule(line: &str, id: usize, line_no: usize) -> Result<Rule, String> {
    let line = line.replace('→', "->");
    let (lhs, rhs) = line.split_once("->").ok_or("expected `->` (or a `spring`/`set` line)")?;
    let (rhs, prob) = match rhs.split_once('@') {
        Some((r, p)) => {
            let p: f32 = p.trim().parse().map_err(|_| format!("bad probability `{}`", p.trim()))?;
            if !(0.0..=1.0).contains(&p) {
                return Err(format!("probability {p} not in [0,1]"));
            }
            (r, p)
        }
        None => (rhs, 1.0),
    };
    let (l0, lb, l1) = parse_side(lhs).map_err(|e| format!("left side: {e}"))?;
    let (r0, rb, r1) = parse_side(rhs).map_err(|e| format!("right side: {e}"))?;

    // every variable used on the right must be bound on the left
    for t in [r0, r1] {
        if let TypePat::Var(v) = t.ty {
            if !matches!(l0.ty, TypePat::Var(x) if x == v) && !matches!(l1.ty, TypePat::Var(x) if x == v) {
                return Err(format!("variable `{v}` used on the right but not on the left"));
            }
        }
    }
    Ok(Rule { id, lhs: [l0, l1], lhs_bonded: lb, rhs: [r0, r1], rhs_bonded: rb, prob, line: line_no })
}

/// `a1 + b2`, `*1.*2`, `X3 . Y6`, `a_1(+)b_2`
fn parse_side(s: &str) -> Result<(Term, bool, Term), String> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let s = s.replace("(+)", "+").replace("(.)", ".");
    let c: Vec<char> = s.chars().collect();
    let (t0, i) = parse_term(&c, 0)?;
    let bonded = match c.get(i) {
        Some('.') => true,
        Some('+') => false,
        Some(x) => return Err(format!("expected `+` or `.` after `{t0}`, found `{x}`")),
        None => return Err(format!("expected `+` or `.` after `{t0}`")),
    };
    let (t1, j) = parse_term(&c, i + 1)?;
    if j != c.len() {
        return Err(format!("unexpected `{}`", c[j..].iter().collect::<String>()));
    }
    Ok((t0, bonded, t1))
}

fn parse_term(c: &[char], mut i: usize) -> Result<(Term, usize), String> {
    let ch = *c.get(i).ok_or("missing term")?;
    let ty = TypePat::from_char(ch)
        .ok_or_else(|| format!("bad type `{ch}` (use a-z, `*`, or an uppercase variable)"))?;
    i += 1;
    if c.get(i) == Some(&'_') {
        i += 1;
    }
    let start = i;
    while i < c.len() && c[i].is_ascii_digit() {
        i += 1;
    }
    if start == i {
        return Err(format!("missing state after `{ch}`"));
    }
    let state: i32 = c[start..i].iter().collect::<String>().parse().map_err(|_| "state too large")?;
    Ok((Term { ty, state }, i))
}

fn parse_spring(line: &str) -> Result<SpringSpec, String> {
    let toks: Vec<&str> = line.split_whitespace().collect();
    if toks.len() < 3 {
        return Err("expected `spring <type|*> <type|*> k=.. len=.. break=..`".into());
    }
    let ty = |t: &str| -> Result<Option<char>, String> {
        match t {
            "*" => Ok(None),
            _ if t.len() == 1 && t.as_bytes()[0].is_ascii_lowercase() => Ok(t.chars().next()),
            _ => Err(format!("bad spring type `{t}`")),
        }
    };
    let mut sp = SpringSpec { a: ty(toks[1])?, b: ty(toks[2])?, k: None, len: None, break_len: None };
    for kv in &toks[3..] {
        let (k, v) = kv.split_once('=').ok_or_else(|| format!("expected key=value, got `{kv}`"))?;
        let v: f32 = v.parse().map_err(|_| format!("bad number `{v}`"))?;
        match k {
            "k" => sp.k = Some(v),
            "len" => sp.len = Some(v),
            "break" => sp.break_len = Some(v),
            _ => return Err(format!("unknown spring key `{k}` (k, len, break)")),
        }
    }
    Ok(sp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(form: char, state: i32) -> Compound {
        Compound { form, state }
    }

    #[test]
    fn parse_forms() {
        for s in ["a0 + a1 -> a1 . a2", "a0+a1->a1.a2", "a_0(+)a_1->a_1(.)a_2", "a0 + a1 → a1 . a2"] {
            let r = Rule::parse(s).unwrap();
            assert_eq!(r.to_string(), "a0+a1->a1.a2", "{s}");
            assert!(!r.lhs_bonded && r.rhs_bonded);
        }
        assert!(Rule::parse("a0 + a1").is_err());
        assert!(Rule::parse("a + a1 -> a1 . a2").is_err());
        assert!(Rule::parse("a0 - a1 -> a1 . a2").is_err());
        assert!(Rule::parse("X0 + a1 -> Y1 . a2").is_err()); // Y unbound
        assert_eq!(Rule::parse("a0+a1->a1.a2 @ 0.25").unwrap().prob, 0.25);
    }

    #[test]
    fn wildcard_star() {
        let r = Rule::parse("*1 + *2 -> *3 . *4").unwrap();
        assert_eq!(r.apply_ordered(c('a', 1), c('b', 2)), Some([c('a', 3), c('b', 4)]));
        assert_eq!(r.apply_ordered(c('b', 2), c('a', 1)), None);
        let chem = Chemistry::parse("*1 + *2 -> *3 . *4").unwrap();
        // reversed pair still matches, products follow the atoms
        let m = chem.find(false, c('q', 2), c('z', 1)).unwrap();
        assert_eq!(m.products, [c('q', 4), c('z', 3)]);
        assert!(chem.find(true, c('q', 2), c('z', 1)).is_none());
    }

    #[test]
    fn wildcard_variables() {
        let r = Rule::parse("X5 + X0 -> X7 . X6").unwrap();
        assert!(r.apply_ordered(c('a', 5), c('a', 0)).is_some());
        assert!(r.apply_ordered(c('a', 5), c('b', 0)).is_none());
        let r = Rule::parse("X3 + Y6 -> X2 . Y3").unwrap();
        assert_eq!(r.apply_ordered(c('a', 3), c('b', 6)), Some([c('a', 2), c('b', 3)]));
        // swap types through variables
        let r = Rule::parse("X1 . Y1 -> Y1 . X1").unwrap();
        assert_eq!(r.apply_ordered(c('a', 1), c('b', 1)), Some([c('b', 1), c('a', 1)]));
        // transmutation
        let r = Rule::parse("*1 . b1 -> c1 . *1").unwrap();
        assert_eq!(r.apply_ordered(c('a', 1), c('b', 1)), Some([c('c', 1), c('b', 1)]));
    }

    #[test]
    fn priority_is_file_order() {
        let chem = Chemistry::parse("*1 + *2 -> *3 + *3\na1 + b2 -> a9 + b9").unwrap();
        assert_eq!(chem.find(false, c('b', 2), c('a', 1)).unwrap().rule, 0);
        assert_eq!(chem.warnings.len(), 1);
        let chem = Chemistry::parse("a1 + b2 -> a9 + b9\n*1 + *2 -> *3 + *3").unwrap();
        assert_eq!(chem.find(false, c('b', 2), c('a', 1)).unwrap().rule, 0);
        assert_eq!(chem.find(false, c('c', 2), c('a', 1)).unwrap().rule, 1);
        assert!(chem.warnings.is_empty());
    }

    #[test]
    fn springs_and_params() {
        let chem = Chemistry::parse(
            "# comment\nset temperature 0.3\nspring * * k=10\nspring a b k=50 len=0.8 break=2\nx1.y1->x2.y2",
        )
        .unwrap();
        assert_eq!(chem.param("temperature"), Some("0.3"));
        assert_eq!(chem.springs[1].score('b', 'a'), Some(2));
        assert_eq!(chem.springs[0].score('b', 'a'), Some(0));
        assert_eq!(chem.springs[1].score('a', 'a'), None);
        assert!(Chemistry::parse("spring a b q=1").is_err());
        let re = Chemistry::parse(&chem.to_text()).unwrap();
        assert_eq!(re.to_text(), chem.to_text());
    }

    #[test]
    fn legacy_json() {
        let json = r#"[{"id":0,"substrate":{"contact":false,"a1":{"form":"a","state":0},"a2":{"form":"a","state":1}},
                        "product":{"contact":true,"a1":{"form":"a","state":1},"a2":{"form":"a","state":2}}}]"#;
        let chem = Chemistry::from_legacy_json(json).unwrap();
        assert_eq!(chem.rules[0].to_string(), "a0+a1->a1.a2");
    }
}
