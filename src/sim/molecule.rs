//! Molecules = connected components of the bond graph.
//!
//! Molecule files (`.mol`), `#` comments:
//!
//! ```text
//! # a chain, written with the rule notation ('.' or '-' = bond); laid out on a line
//! replicator: e8.a1.b1.c1.f1
//!
//! # any graph, explicit coordinates (relative; recentred on insertion)
//! molecule ring4
//! atom a 1 0 0
//! atom a 1 1 0
//! atom a 1 1 1
//! atom a 1 0 1
//! bond 0 1
//! bond 1 2
//! bond 2 3
//! bond 3 0
//! end
//! ```

use std::collections::{BTreeMap, HashMap};

use super::reactor::Reactor;
use super::rules::Compound;
use super::vec2::Vec2;

#[derive(Clone, Debug)]
pub struct MolAtom {
    pub form: char,
    pub state: i32,
    /// position relative to the molecule centroid
    pub pos: Vec2,
}

#[derive(Clone, Debug, Default)]
pub struct Molecule {
    pub name: String,
    pub atoms: Vec<MolAtom>,
    pub bonds: Vec<(usize, usize)>,
}

impl Molecule {
    pub fn new(name: &str) -> Molecule {
        Molecule { name: name.to_string(), ..Default::default() }
    }

    /// `e8.a1.b1.f1` (or `e8-a1-b1-f1`), laid out horizontally with `spacing`.
    pub fn from_chain(name: &str, chain: &str, spacing: f32) -> Result<Molecule, String> {
        let mut m = Molecule::new(name);
        for (k, tok) in chain.split(['.', '-']).enumerate() {
            let tok = tok.trim().replace('_', "");
            let mut ch = tok.chars();
            let form = ch.next().filter(|c| c.is_ascii_lowercase()).ok_or_else(|| format!("bad atom `{tok}` in chain"))?;
            let state: i32 = ch.as_str().parse().map_err(|_| format!("bad state in `{tok}`"))?;
            m.atoms.push(MolAtom { form, state, pos: Vec2::new(k as f32 * spacing, 0.0) });
            if k > 0 {
                m.bonds.push((k - 1, k));
            }
        }
        if m.name.is_empty() {
            m.name = chain.trim().to_string();
        }
        m.recenter();
        Ok(m)
    }

    /// Parse a `.mol` file (several molecules allowed). `spacing` is used for chain lines.
    pub fn parse_file(text: &str, spacing: f32) -> Result<Vec<Molecule>, String> {
        let mut out = Vec::new();
        let mut cur: Option<Molecule> = None;
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let bad = |e: &str| format!("line {}: {e}\n    {}", n + 1, raw.trim());
            let t: Vec<&str> = line.split_whitespace().collect();
            match t[0] {
                "molecule" => {
                    if let Some(m) = cur.take() {
                        out.push(m.finish().map_err(|e| bad(&e))?);
                    }
                    cur = Some(Molecule::new(&t[1..].join(" ")));
                }
                "atom" => {
                    let m = cur.as_mut().ok_or_else(|| bad("`atom` outside a `molecule` block"))?;
                    if t.len() != 5 {
                        return Err(bad("expected `atom <type> <state> <x> <y>`"));
                    }
                    let form = t[1].chars().next().filter(|c| c.is_ascii_lowercase()).ok_or_else(|| bad("bad type"))?;
                    let num = |s: &str| s.parse::<f32>().map_err(|_| bad("bad number"));
                    let state = t[2].parse::<i32>().map_err(|_| bad("bad state"))?;
                    m.atoms.push(MolAtom { form, state, pos: Vec2::new(num(t[3])?, num(t[4])?) });
                }
                "bond" => {
                    let m = cur.as_mut().ok_or_else(|| bad("`bond` outside a `molecule` block"))?;
                    if t.len() != 3 {
                        return Err(bad("expected `bond <i> <j>`"));
                    }
                    let i = t[1].parse::<usize>().map_err(|_| bad("bad index"))?;
                    let j = t[2].parse::<usize>().map_err(|_| bad("bad index"))?;
                    m.bonds.push((i, j));
                }
                "end" => {
                    let m = cur.take().ok_or_else(|| bad("`end` without `molecule`"))?;
                    out.push(m.finish().map_err(|e| bad(&e))?);
                }
                _ => {
                    if cur.is_some() {
                        return Err(bad("unexpected line inside a `molecule` block"));
                    }
                    let (name, chain) = line.split_once(':').unwrap_or(("", line));
                    out.push(Molecule::from_chain(name.trim(), chain.trim(), spacing).map_err(|e| bad(&e))?);
                }
            }
        }
        if let Some(m) = cur.take() {
            out.push(m.finish()?);
        }
        Ok(out)
    }

    fn finish(mut self) -> Result<Molecule, String> {
        for &(i, j) in &self.bonds {
            if i >= self.atoms.len() || j >= self.atoms.len() || i == j {
                return Err(format!("molecule `{}`: bad bond {i} {j}", self.name));
            }
        }
        self.recenter();
        Ok(self)
    }

    pub fn to_text(&self) -> String {
        let mut s = format!("molecule {}\n", if self.name.is_empty() { "unnamed" } else { &self.name });
        s += &format!("# {}\n", self.signature());
        for a in &self.atoms {
            s += &format!("atom {} {} {:.3} {:.3}\n", a.form, a.state, a.pos.x, a.pos.y);
        }
        for &(i, j) in &self.bonds {
            s += &format!("bond {i} {j}\n");
        }
        s + "end\n"
    }

    pub fn recenter(&mut self) {
        if self.atoms.is_empty() {
            return;
        }
        let mut c = Vec2::ZERO;
        for a in &self.atoms {
            c += a.pos;
        }
        c = c * (1.0 / self.atoms.len() as f32);
        for a in &mut self.atoms {
            a.pos -= c;
        }
    }

    pub fn signature(&self) -> String {
        let comps: Vec<Compound> = self.atoms.iter().map(|a| Compound { form: a.form, state: a.state }).collect();
        signature(&comps, &self.bonds)
    }
}

/// A short, order-independent name for a molecule:
/// * linear chains: `e8.a1.b1.f1` (the lexicographically smaller of both directions)
/// * anything else: sorted composition plus bond count, e.g. `{a1*3 b2} 3b`
pub fn signature(atoms: &[Compound], bonds: &[(usize, usize)]) -> String {
    let n = atoms.len();
    if n == 0 {
        return String::new();
    }
    if n == 1 {
        return atoms[0].to_string();
    }
    let mut adj = vec![Vec::new(); n];
    for &(i, j) in bonds {
        adj[i].push(j);
        adj[j].push(i);
    }
    let is_path = bonds.len() == n - 1 && adj.iter().all(|a| a.len() <= 2);
    if is_path {
        if let Some(start) = (0..n).find(|&i| adj[i].len() == 1) {
            let mut order = vec![start];
            let mut prev = usize::MAX;
            let mut cur = start;
            while let Some(&next) = adj[cur].iter().find(|&&k| k != prev) {
                order.push(next);
                prev = cur;
                cur = next;
            }
            if order.len() == n {
                let fwd: Vec<String> = order.iter().map(|&i| atoms[i].to_string()).collect();
                let fwd = fwd.join(".");
                let bwd: Vec<String> = order.iter().rev().map(|&i| atoms[i].to_string()).collect();
                let bwd = bwd.join(".");
                return fwd.min(bwd);
            }
        }
    }
    let mut counts: BTreeMap<Compound, usize> = BTreeMap::new();
    for c in atoms {
        *counts.entry(*c).or_default() += 1;
    }
    let parts: Vec<String> =
        counts.iter().map(|(c, k)| if *k > 1 { format!("{c}*{k}") } else { c.to_string() }).collect();
    format!("{{{}}} {}b", parts.join(" "), bonds.len())
}

impl Reactor {
    /// Component label per atom (union-find root); equal labels = same molecule.
    pub fn component_labels(&self) -> Vec<u32> {
        let n = self.atoms.len();
        let mut parent: Vec<u32> = (0..n as u32).collect();
        fn find(p: &mut [u32], mut x: u32) -> u32 {
            while p[x as usize] != x {
                p[x as usize] = p[p[x as usize] as usize];
                x = p[x as usize];
            }
            x
        }
        for (i, a) in self.atoms.iter().enumerate() {
            for &j in &a.bonds {
                if (j as usize) > i {
                    let (ri, rj) = (find(&mut parent, i as u32), find(&mut parent, j));
                    if ri != rj {
                        parent[ri.max(rj) as usize] = ri.min(rj);
                    }
                }
            }
        }
        (0..n as u32).map(|i| find(&mut parent, i)).collect()
    }

    /// All molecules (connected components) with at least `min_size` atoms, as atom index lists.
    pub fn components(&self, min_size: usize) -> Vec<Vec<usize>> {
        let labels = self.component_labels();
        let mut groups: HashMap<u32, Vec<usize>> = HashMap::new();
        for (i, &l) in labels.iter().enumerate() {
            groups.entry(l).or_default().push(i);
        }
        let mut out: Vec<Vec<usize>> = groups.into_values().filter(|g| g.len() >= min_size).collect();
        out.sort_by_key(|g| g[0]);
        out
    }

    /// The molecule containing atom `i` (BFS).
    pub fn component_of(&self, i: usize) -> Vec<usize> {
        let mut seen = vec![false; self.atoms.len()];
        let mut out = vec![i];
        seen[i] = true;
        let mut k = 0;
        while k < out.len() {
            for &j in &self.atoms[out[k]].bonds {
                if !seen[j as usize] {
                    seen[j as usize] = true;
                    out.push(j as usize);
                }
            }
            k += 1;
        }
        out
    }

    /// Copy atoms `ids` (normally one component) into a standalone molecule.
    /// Bonds leaving the set are dropped.
    pub fn copy_molecule(&self, ids: &[usize]) -> Molecule {
        let local: HashMap<usize, usize> = ids.iter().enumerate().map(|(k, &i)| (i, k)).collect();
        let mut m = Molecule::new("");
        for &i in ids {
            let a = &self.atoms[i];
            m.atoms.push(MolAtom { form: a.form, state: a.state, pos: a.pos });
        }
        for (k, &i) in ids.iter().enumerate() {
            for &j in &self.atoms[i].bonds {
                if let Some(&l) = local.get(&(j as usize)) {
                    if l > k {
                        m.bonds.push((k, l));
                    }
                }
            }
        }
        m.recenter();
        m.name = m.signature();
        m
    }

    /// Copy then remove the atoms from the reactor.
    pub fn extract_molecule(&mut self, ids: &[usize]) -> Molecule {
        let m = self.copy_molecule(ids);
        self.remove_atoms(ids);
        m
    }

    /// Insert a molecule centred at `at`, rotated by `angle`. Free atoms (no bonds) in the
    /// way are moved to random free spots; fails if a bonded atom or a wall is in the way.
    /// Returns the indices of the new atoms.
    pub fn insert_molecule(&mut self, m: &Molecule, at: Vec2, angle: f32) -> Result<Vec<usize>, String> {
        let r = self.params.radius;
        let d = self.params.contact_dist() * 0.95;
        let pos: Vec<Vec2> = m.atoms.iter().map(|a| at + a.pos.rotated(angle)).collect();
        let mut displaced = Vec::new();
        for p in &pos {
            if p.x < r || p.y < r || p.x > self.params.width - r || p.y > self.params.height - r {
                return Err("molecule does not fit inside the box here".into());
            }
            for (k, a) in self.atoms.iter().enumerate() {
                if (a.pos - *p).len2() < d * d {
                    if !a.bonds.is_empty() {
                        return Err("another molecule is in the way".into());
                    }
                    if !displaced.contains(&k) {
                        displaced.push(k);
                    }
                }
            }
        }
        let base = self.atoms.len();
        for (a, p) in m.atoms.iter().zip(&pos) {
            self.add_atom(a.form, a.state, *p);
        }
        for &(i, j) in &m.bonds {
            self.bond(base + i, base + j);
        }
        // move the displaced free atoms somewhere else (or drop them if the box is full)
        let mut dropped = Vec::new();
        for k in displaced {
            self.atoms[k].pos = Vec2::new(-100.0, -100.0); // out of the way while searching
            match self.random_free_spot(500) {
                Some(p) => self.atoms[k].pos = p,
                None => dropped.push(k),
            }
        }
        let new_ids: Vec<usize> = (base..base + m.atoms.len()).collect();
        if !dropped.is_empty() {
            self.remove_atoms(&dropped);
            return Ok(new_ids.iter().map(|&i| i - dropped.len()).collect());
        }
        Ok(new_ids)
    }

    /// Count molecules by signature (only molecules with >= `min_size` atoms),
    /// most abundant first.
    pub fn census(&self, min_size: usize) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for comp in self.components(min_size) {
            let local: HashMap<usize, usize> = comp.iter().enumerate().map(|(k, &i)| (i, k)).collect();
            let atoms: Vec<Compound> = comp.iter().map(|&i| self.atoms[i].compound()).collect();
            let mut bonds = Vec::new();
            for (k, &i) in comp.iter().enumerate() {
                for &j in &self.atoms[i].bonds {
                    let l = local[&(j as usize)];
                    if l > k {
                        bonds.push((k, l));
                    }
                }
            }
            *counts.entry(signature(&atoms, &bonds)).or_default() += 1;
        }
        let mut v: Vec<(String, usize)> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::rules::Chemistry;

    #[test]
    fn chain_and_signature() {
        let m = Molecule::from_chain("rep", "f1.a1.e8", 1.0).unwrap();
        assert_eq!(m.atoms.len(), 3);
        assert_eq!(m.signature(), "e8.a1.f1");
        assert!((m.atoms[0].pos.x + 1.0).abs() < 1e-6);
    }

    #[test]
    fn parse_file_mixed() {
        let txt = "# test\nrep: e8.a1.f1\nmolecule tri\natom a 1 0 0\natom b 1 1 0\natom c 1 0 1\nbond 0 1\nbond 1 2\nbond 2 0\nend\n";
        let ms = Molecule::parse_file(txt, 1.0).unwrap();
        assert_eq!(ms.len(), 2);
        assert_eq!(ms[0].name, "rep");
        assert_eq!(ms[1].signature(), "{a1 b1 c1} 3b");
        let again = Molecule::parse_file(&ms[1].to_text(), 1.0).unwrap();
        assert_eq!(again[0].bonds.len(), 3);
        assert!(Molecule::parse_file("molecule x\nbond 0 1\nend", 1.0).is_err());
    }

    #[test]
    fn insert_extract_components() {
        let mut r = Reactor::new(30.0, 30.0, Chemistry::new(), 3);
        r.add_food(50);
        let m = Molecule::from_chain("", "e8.a1.b1.f1", 1.0).unwrap();
        let ids = r.insert_molecule(&m, Vec2::new(15.0, 15.0), 0.7).unwrap();
        assert_eq!(r.atoms.len(), 54);
        assert_eq!(r.component_of(ids[0]).len(), 4);
        assert_eq!(r.components(2).len(), 1);
        assert_eq!(r.census(2), vec![("e8.a1.b1.f1".to_string(), 1)]);
        // cannot insert on top of it
        assert!(r.insert_molecule(&m, Vec2::new(15.0, 15.0), 0.7).is_err());
        let comp = r.component_of(ids[2]);
        let out = r.extract_molecule(&comp);
        assert_eq!(out.signature(), "e8.a1.b1.f1");
        assert_eq!(r.atoms.len(), 50);
        assert_eq!(r.bond_count(), 0);
        // insert it back elsewhere
        r.insert_molecule(&out, Vec2::new(5.0, 5.0), 0.0).unwrap();
        assert_eq!(r.census(2)[0].1, 1);
    }
}
