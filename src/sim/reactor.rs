//! Continuous reactor: atoms are soft spheres of radius `r` in a `width x height` box,
//! bonds are springs whose stiffness / rest length depend on the bonded type pair.
//!
//! One step:
//! 1. spring forces along bonds (bonds stretched past `break` are broken)
//! 2. neighbour search on a uniform grid: soft-sphere repulsion for overlapping,
//!    non-bonded pairs; every non-bonded pair closer than `2r * react_range` is a
//!    reaction candidate ("collision")
//! 3. Langevin integration (damping + thermal noise), reflecting walls
//! 4. collision reactions (non-bonded rules) on candidate pairs, in random order
//! 5. bonded reactions, event driven: only atoms whose state or bonds changed
//!    ("dirty" atoms) have their bonds checked against bonded rules
//!
//! Each atom takes part in at most one reaction per step.

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use super::rules::{type_index, Chemistry, Compound, Match, NTYPES};
use super::vec2::Vec2;

#[derive(Clone, Debug)]
pub struct Atom {
    pub pos: Vec2,
    pub vel: Vec2,
    pub form: char,
    pub state: i32,
    /// indices of bonded atoms
    pub bonds: Vec<u32>,
}

impl Atom {
    pub fn new(form: char, state: i32, pos: Vec2) -> Atom {
        Atom { pos, vel: Vec2::ZERO, form, state, bonds: Vec::new() }
    }
    pub fn compound(&self) -> Compound {
        Compound { form: self.form, state: self.state }
    }
    pub fn is_bonded(&self, j: usize) -> bool {
        self.bonds.contains(&(j as u32))
    }
}

/// Physics and run parameters. All of them except `width`/`height` can be set
/// from the chemistry file with `set <name> <value>`.
#[derive(Clone, Debug)]
pub struct Params {
    pub width: f32,
    pub height: f32,
    /// sphere radius
    pub radius: f32,
    pub dt: f32,
    /// velocity damping (friction) gamma
    pub damping: f32,
    /// thermal noise: equilibrium kinetic energy per degree of freedom
    pub temperature: f32,
    /// stiffness of the soft-sphere repulsion
    pub repulsion: f32,
    /// probability that a colliding pair attempts a reaction in a given step
    pub react_prob: f32,
    /// reaction distance, in units of the contact distance 2r
    pub react_range: f32,
    /// default spring stiffness (pairs without a `spring` line)
    pub spring_k: f32,
    /// default spring rest length (0 = 2r)
    pub spring_len: f32,
    /// speed limit, keeps the explicit integrator stable
    pub max_speed: f32,
    /// types used for "food" (free atoms in state 0)
    pub food: Vec<char>,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            width: 60.0,
            height: 60.0,
            radius: 0.5,
            dt: 0.05,
            damping: 1.0,
            temperature: 0.3,
            repulsion: 40.0,
            react_prob: 1.0,
            react_range: 1.25,
            spring_k: 20.0,
            spring_len: 0.0,
            max_speed: 8.0,
            food: "abcdef".chars().collect(),
        }
    }
}

impl Params {
    /// Apply `set` lines. Returns warnings for unknown keys / bad values.
    pub fn apply(&mut self, params: &[(String, String)]) -> Vec<String> {
        let mut warnings = Vec::new();
        for (k, v) in params {
            let num = v.parse::<f32>();
            let slot: Option<&mut f32> = match k.as_str() {
                "radius" => Some(&mut self.radius),
                "dt" => Some(&mut self.dt),
                "damping" => Some(&mut self.damping),
                "temperature" => Some(&mut self.temperature),
                "repulsion" => Some(&mut self.repulsion),
                "react_prob" => Some(&mut self.react_prob),
                "react_range" => Some(&mut self.react_range),
                "spring_k" => Some(&mut self.spring_k),
                "spring_len" => Some(&mut self.spring_len),
                "max_speed" => Some(&mut self.max_speed),
                "food" => {
                    self.food = v.chars().filter(|c| c.is_ascii_lowercase()).collect();
                    if self.food.is_empty() {
                        warnings.push(format!("set food: no types in `{v}`"));
                        self.food = vec!['a'];
                    }
                    None
                }
                _ => {
                    warnings.push(format!("unknown parameter `{k}`"));
                    None
                }
            };
            if let Some(slot) = slot {
                match num {
                    Ok(x) if x.is_finite() && x >= 0.0 => *slot = x,
                    _ => warnings.push(format!("set {k}: bad value `{v}`")),
                }
            }
        }
        self.radius = self.radius.max(1e-3);
        self.react_range = self.react_range.max(1.0);
        warnings
    }

    pub fn contact_dist(&self) -> f32 {
        2.0 * self.radius
    }
}

/// Resolved spring parameters for a type pair.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub k: f32,
    pub len: f32,
    pub break_len: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub step: u64,
    pub reactions: u64,
    /// firing count per rule id
    pub per_rule: Vec<u64>,
    pub bonds_made: u64,
    pub bonds_broken: u64,
    /// candidate (close, non-bonded) pairs found in the last step
    pub collisions: u64,
}

pub struct Reactor {
    pub params: Params,
    pub atoms: Vec<Atom>,
    pub chem: Chemistry,
    pub stats: Stats,
    pub rng: StdRng,
    springs: Vec<Spring>,
    // scratch / bookkeeping
    forces: Vec<Vec2>,
    grid_start: Vec<u32>,
    grid_items: Vec<u32>,
    cell_of: Vec<u32>,
    candidates: Vec<(u32, u32)>,
    to_break: Vec<(u32, u32)>,
    dirty: Vec<u32>,
    in_dirty: Vec<bool>,
    reacted: Vec<u64>,
}

impl Reactor {
    pub fn new(width: f32, height: f32, chem: Chemistry, seed: u64) -> Reactor {
        let params = Params { width, height, ..Params::default() };
        let mut r = Reactor {
            params,
            atoms: Vec::new(),
            chem: Chemistry::new(),
            stats: Stats::default(),
            rng: StdRng::seed_from_u64(seed),
            springs: Vec::new(),
            forces: Vec::new(),
            grid_start: Vec::new(),
            grid_items: Vec::new(),
            cell_of: Vec::new(),
            candidates: Vec::new(),
            to_break: Vec::new(),
            dirty: Vec::new(),
            in_dirty: Vec::new(),
            reacted: Vec::new(),
        };
        r.set_chemistry(chem);
        r
    }

    /// Install a (new) chemistry: applies its `set` parameters and spring table and
    /// marks every atom dirty so bonded rules are re-checked. Returns warnings.
    pub fn set_chemistry(&mut self, chem: Chemistry) -> Vec<String> {
        let mut warnings = chem.warnings.clone();
        warnings.extend(self.params.apply(&chem.params));
        self.chem = chem;
        self.stats.per_rule = vec![0; self.chem.rules.len()];
        self.resolve_springs();
        for i in 0..self.atoms.len() {
            self.mark_dirty(i);
        }
        warnings
    }

    /// Build the NTYPES x NTYPES spring table: most specific `spring` line wins,
    /// later lines win ties.
    pub fn resolve_springs(&mut self) {
        let p = &self.params;
        let def_len = if p.spring_len > 0.0 { p.spring_len } else { p.contact_dist() };
        let default = Spring { k: p.spring_k, len: def_len, break_len: f32::INFINITY };
        self.springs = vec![default; NTYPES * NTYPES];
        for i in 0..NTYPES {
            for j in 0..NTYPES {
                let (a, b) = ((b'a' + i as u8) as char, (b'a' + j as u8) as char);
                let mut best: i32 = -1;
                for spec in &self.chem.springs {
                    if let Some(s) = spec.score(a, b) {
                        if s as i32 >= best {
                            best = s as i32;
                            self.springs[i * NTYPES + j] = Spring {
                                k: spec.k.unwrap_or(default.k),
                                len: spec.len.unwrap_or(default.len),
                                break_len: spec.break_len.unwrap_or(f32::INFINITY),
                            };
                        }
                    }
                }
            }
        }
    }

    pub fn spring(&self, a: char, b: char) -> Spring {
        self.springs[type_index(a) * NTYPES + type_index(b)]
    }

    // ------------------------------------------------------------------ atoms & bonds

    pub fn add_atom(&mut self, form: char, state: i32, pos: Vec2) -> usize {
        self.atoms.push(Atom::new(form, state, pos));
        self.in_dirty.push(false);
        self.reacted.push(0);
        let i = self.atoms.len() - 1;
        self.mark_dirty(i);
        i
    }

    pub fn bond(&mut self, i: usize, j: usize) {
        if i != j && !self.atoms[i].is_bonded(j) {
            self.atoms[i].bonds.push(j as u32);
            self.atoms[j].bonds.push(i as u32);
        }
    }

    pub fn unbond(&mut self, i: usize, j: usize) {
        self.atoms[i].bonds.retain(|&k| k as usize != j);
        self.atoms[j].bonds.retain(|&k| k as usize != i);
    }

    pub fn bond_count(&self) -> usize {
        self.atoms.iter().map(|a| a.bonds.len()).sum::<usize>() / 2
    }

    /// Remove atoms (and their bonds); remaining atoms are re-indexed.
    pub fn remove_atoms(&mut self, ids: &[usize]) {
        let n = self.atoms.len();
        let mut remove = vec![false; n];
        for &i in ids {
            if i < n {
                remove[i] = true;
            }
        }
        let mut map = vec![u32::MAX; n];
        let mut k = 0u32;
        for i in 0..n {
            if !remove[i] {
                map[i] = k;
                k += 1;
            }
        }
        let old = std::mem::take(&mut self.atoms);
        self.atoms = old
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !remove[*i])
            .map(|(_, mut a)| {
                a.bonds = a.bonds.iter().map(|&j| map[j as usize]).filter(|&j| j != u32::MAX).collect();
                a
            })
            .collect();
        let m = self.atoms.len();
        let old_dirty = std::mem::take(&mut self.dirty);
        self.in_dirty = vec![false; m];
        self.reacted = vec![0; m];
        for d in old_dirty {
            let j = map[d as usize];
            if j != u32::MAX {
                self.mark_dirty(j as usize);
            }
        }
    }

    pub fn mark_dirty(&mut self, i: usize) {
        if !self.in_dirty[i] {
            self.in_dirty[i] = true;
            self.dirty.push(i as u32);
        }
    }

    /// Index of the atom under `p` (within 1.5 r), if any.
    pub fn atom_at(&self, p: Vec2) -> Option<usize> {
        let r = self.params.radius * 1.5;
        self.atoms
            .iter()
            .enumerate()
            .map(|(i, a)| (i, (a.pos - p).len2()))
            .filter(|&(_, d2)| d2 < r * r)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// True if a sphere at `p` would be inside the box and not overlap any atom.
    pub fn is_free(&self, p: Vec2) -> bool {
        let r = self.params.radius;
        let d = self.params.contact_dist();
        p.x >= r
            && p.y >= r
            && p.x <= self.params.width - r
            && p.y <= self.params.height - r
            && self.atoms.iter().all(|a| (a.pos - p).len2() >= d * d)
    }

    pub fn random_free_spot(&mut self, tries: usize) -> Option<Vec2> {
        let r = self.params.radius;
        for _ in 0..tries {
            let p = Vec2::new(
                self.rng.random_range(r..self.params.width - r),
                self.rng.random_range(r..self.params.height - r),
            );
            if self.is_free(p) {
                return Some(p);
            }
        }
        None
    }

    /// Add up to `n` free atoms in state 0, types drawn from `params.food`.
    /// Returns how many were placed (stops when the box is too crowded).
    pub fn add_food(&mut self, n: usize) -> usize {
        let mut placed = 0;
        for _ in 0..n {
            let Some(p) = self.random_free_spot(200) else { break };
            let form = self.params.food[self.rng.random_range(0..self.params.food.len())];
            self.add_atom(form, 0, p);
            placed += 1;
        }
        placed
    }

    /// Free atoms (no bonds) in state 0.
    pub fn free_food_count(&self) -> usize {
        self.atoms.iter().filter(|a| a.state == 0 && a.bonds.is_empty()).count()
    }

    /// Mean kinetic energy per degree of freedom (should be close to `temperature`).
    pub fn kinetic_temperature(&self) -> f32 {
        if self.atoms.is_empty() {
            return 0.0;
        }
        self.atoms.iter().map(|a| a.vel.len2()).sum::<f32>() / (2.0 * self.atoms.len() as f32)
    }

    // ------------------------------------------------------------------ simulation

    pub fn step(&mut self) {
        let n = self.atoms.len();
        self.stats.step += 1;
        let stamp = self.stats.step;
        let r = self.params.radius;
        let contact = self.params.contact_dist();
        let react_d = contact * self.params.react_range;

        // 1. springs
        self.forces.clear();
        self.forces.resize(n, Vec2::ZERO);
        self.to_break.clear();
        for i in 0..n {
            let ai = &self.atoms[i];
            for &j in &ai.bonds {
                let j = j as usize;
                if j <= i {
                    continue;
                }
                let aj = &self.atoms[j];
                let sp = self.springs[type_index(ai.form) * NTYPES + type_index(aj.form)];
                let d = aj.pos - ai.pos;
                let dist = d.len().max(1e-6);
                if dist > sp.break_len {
                    self.to_break.push((i as u32, j as u32));
                }
                let f = d * (sp.k * (dist - sp.len) / dist);
                self.forces[i] += f;
                self.forces[j] -= f;
            }
        }

        // 2. neighbour search: repulsion + collision candidates
        self.build_grid(react_d);
        self.candidates.clear();
        {
            let (gw, gh) = self.grid_dims(react_d);
            let atoms = &self.atoms;
            let forces = &mut self.forces;
            let cands = &mut self.candidates;
            let start = &self.grid_start;
            let items = &self.grid_items;
            let k_rep = self.params.repulsion;
            let mut pair = |i: usize, j: usize| {
                let d = atoms[j].pos - atoms[i].pos;
                let d2 = d.len2();
                if d2 >= react_d * react_d || atoms[i].is_bonded(j) {
                    return;
                }
                if d2 < contact * contact {
                    let dist = d2.sqrt().max(1e-6);
                    let f = d * (k_rep * (contact - dist) / dist);
                    forces[i] -= f;
                    forces[j] += f;
                }
                cands.push((i as u32, j as u32));
            };
            for cy in 0..gh {
                for cx in 0..gw {
                    let c = cy * gw + cx;
                    let (s0, s1) = (start[c] as usize, start[c + 1] as usize);
                    for ii in s0..s1 {
                        let i = items[ii] as usize;
                        for &j in &items[ii + 1..s1] {
                            pair(i, j as usize);
                        }
                        // half stencil: E, SW, S, SE
                        for (dx, dy) in [(1i64, 0i64), (-1, 1), (0, 1), (1, 1)] {
                            let (nx, ny) = (cx as i64 + dx, cy as i64 + dy);
                            if nx < 0 || ny < 0 || nx >= gw as i64 || ny >= gh as i64 {
                                continue;
                            }
                            let nc = ny as usize * gw + nx as usize;
                            for &j in &items[start[nc] as usize..start[nc + 1] as usize] {
                                pair(i, j as usize);
                            }
                        }
                    }
                }
            }
        }
        self.stats.collisions = self.candidates.len() as u64;

        // 3. integrate (Langevin, semi-implicit Euler)
        let dt = self.params.dt;
        let gamma = self.params.damping;
        let sigma = (2.0 * gamma * self.params.temperature * dt).sqrt();
        let vmax = self.params.max_speed;
        let (w, h) = (self.params.width, self.params.height);
        for i in 0..n {
            let noise = Vec2::new(gauss(&mut self.rng), gauss(&mut self.rng)) * sigma;
            let a = &mut self.atoms[i];
            a.vel += (self.forces[i] - a.vel * gamma) * dt + noise;
            let s2 = a.vel.len2();
            if s2 > vmax * vmax {
                a.vel = a.vel * (vmax / s2.sqrt());
            }
            a.pos += a.vel * dt;
            if a.pos.x < r {
                a.pos.x = r;
                a.vel.x = a.vel.x.abs();
            } else if a.pos.x > w - r {
                a.pos.x = w - r;
                a.vel.x = -a.vel.x.abs();
            }
            if a.pos.y < r {
                a.pos.y = r;
                a.vel.y = a.vel.y.abs();
            } else if a.pos.y > h - r {
                a.pos.y = h - r;
                a.vel.y = -a.vel.y.abs();
            }
        }

        // overstretched bonds
        for (i, j) in std::mem::take(&mut self.to_break) {
            self.unbond(i as usize, j as usize);
            self.stats.bonds_broken += 1;
            self.mark_dirty(i as usize);
            self.mark_dirty(j as usize);
        }

        // 4. collision reactions
        let mut cands = std::mem::take(&mut self.candidates);
        cands.shuffle(&mut self.rng);
        let p_react = self.params.react_prob;
        for &(i, j) in &cands {
            let (i, j) = (i as usize, j as usize);
            if self.reacted[i] == stamp || self.reacted[j] == stamp {
                continue;
            }
            if p_react < 1.0 && self.rng.random::<f32>() >= p_react {
                continue;
            }
            if let Some(m) = self.chem.find(false, self.atoms[i].compound(), self.atoms[j].compound()) {
                let p = self.chem.rules[m.rule].prob;
                if p < 1.0 && self.rng.random::<f32>() >= p {
                    continue;
                }
                self.apply_match(i, j, m, false);
            }
        }
        self.candidates = cands;

        // 5. bonded reactions on dirty atoms
        let mut current = std::mem::take(&mut self.dirty);
        for &i in &current {
            self.in_dirty[i as usize] = false;
        }
        current.shuffle(&mut self.rng);
        for &i in &current {
            let i = i as usize;
            if self.reacted[i] == stamp {
                self.mark_dirty(i); // check again next step
                continue;
            }
            let nb = self.atoms[i].bonds.len();
            if nb == 0 {
                continue;
            }
            let off = self.rng.random_range(0..nb);
            for k in 0..nb {
                let j = self.atoms[i].bonds[(k + off) % nb] as usize;
                let Some(m) = self.chem.find(true, self.atoms[i].compound(), self.atoms[j].compound()) else {
                    continue;
                };
                if self.reacted[j] == stamp {
                    self.mark_dirty(i);
                    continue;
                }
                let p = self.chem.rules[m.rule].prob;
                if p < 1.0 && self.rng.random::<f32>() >= p {
                    self.mark_dirty(i); // probabilistic rule: keep trying
                    continue;
                }
                self.apply_match(i, j, m, true);
                break;
            }
        }
    }

    fn apply_match(&mut self, i: usize, j: usize, m: Match, was_bonded: bool) {
        let stamp = self.stats.step;
        self.atoms[i].form = m.products[0].form;
        self.atoms[i].state = m.products[0].state;
        self.atoms[j].form = m.products[1].form;
        self.atoms[j].state = m.products[1].state;
        let bonded_after = self.chem.rules[m.rule].rhs_bonded;
        if bonded_after && !was_bonded {
            self.bond(i, j);
            self.stats.bonds_made += 1;
        } else if !bonded_after && was_bonded {
            self.unbond(i, j);
            self.stats.bonds_broken += 1;
        }
        self.reacted[i] = stamp;
        self.reacted[j] = stamp;
        if let Some(c) = self.stats.per_rule.get_mut(m.rule) {
            *c += 1;
        }
        self.stats.reactions += 1;
        self.mark_dirty(i);
        self.mark_dirty(j);
    }

    fn grid_dims(&self, cell: f32) -> (usize, usize) {
        (
            ((self.params.width / cell).ceil() as usize).max(1),
            ((self.params.height / cell).ceil() as usize).max(1),
        )
    }

    /// Counting sort of atoms into square cells of side `cell`.
    fn build_grid(&mut self, cell: f32) {
        let (gw, gh) = self.grid_dims(cell);
        let nc = gw * gh;
        self.grid_start.clear();
        self.grid_start.resize(nc + 1, 0);
        self.cell_of.clear();
        for a in &self.atoms {
            let cx = ((a.pos.x / cell).max(0.0) as usize).min(gw - 1);
            let cy = ((a.pos.y / cell).max(0.0) as usize).min(gh - 1);
            let c = cy * gw + cx;
            self.cell_of.push(c as u32);
            self.grid_start[c + 1] += 1;
        }
        for c in 0..nc {
            self.grid_start[c + 1] += self.grid_start[c];
        }
        self.grid_items.clear();
        self.grid_items.resize(self.atoms.len(), 0);
        let mut fill = self.grid_start.clone();
        for (i, &c) in self.cell_of.iter().enumerate() {
            self.grid_items[fill[c as usize] as usize] = i as u32;
            fill[c as usize] += 1;
        }
    }

    // ------------------------------------------------------------------ snapshots

    /// Text snapshot: `size W H`, then `atom <form> <state> <x> <y>`, then `bond i j`.
    pub fn to_snapshot(&self) -> String {
        let mut s = format!(
            "# molecular_engine reactor snapshot, step {}\nsize {} {}\n",
            self.stats.step, self.params.width, self.params.height
        );
        for a in &self.atoms {
            s += &format!("atom {} {} {:.4} {:.4}\n", a.form, a.state, a.pos.x, a.pos.y);
        }
        for (i, a) in self.atoms.iter().enumerate() {
            for &j in &a.bonds {
                if j as usize > i {
                    s += &format!("bond {i} {j}\n");
                }
            }
        }
        s
    }

    pub fn from_snapshot(text: &str, chem: Chemistry, seed: u64) -> Result<Reactor, String> {
        let mut size = None;
        let mut atoms = Vec::new();
        let mut bonds = Vec::new();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let t: Vec<&str> = line.split_whitespace().collect();
            let bad = || format!("snapshot line {}: `{}`", n + 1, raw.trim());
            let f = |s: &str| s.parse::<f32>().map_err(|_| bad());
            match t[0] {
                "size" if t.len() == 3 => size = Some((f(t[1])?, f(t[2])?)),
                "atom" if t.len() == 5 => {
                    let form = t[1].chars().next().filter(|c| c.is_ascii_lowercase()).ok_or_else(bad)?;
                    let state = t[2].parse::<i32>().map_err(|_| bad())?;
                    atoms.push((form, state, Vec2::new(f(t[3])?, f(t[4])?)));
                }
                "bond" if t.len() == 3 => {
                    let i = t[1].parse::<usize>().map_err(|_| bad())?;
                    let j = t[2].parse::<usize>().map_err(|_| bad())?;
                    bonds.push((i, j));
                }
                _ => return Err(bad()),
            }
        }
        let (w, h) = size.ok_or("snapshot: missing `size W H`")?;
        let mut r = Reactor::new(w, h, chem, seed);
        for (form, state, p) in atoms {
            r.add_atom(form, state, p);
        }
        for (i, j) in bonds {
            if i >= r.atoms.len() || j >= r.atoms.len() {
                return Err(format!("snapshot: bond {i} {j} out of range"));
            }
            r.bond(i, j);
        }
        Ok(r)
    }
}

/// Standard normal sample (Box-Muller).
fn gauss(rng: &mut StdRng) -> f32 {
    let u1: f32 = rng.random::<f32>().max(1e-7);
    let u2: f32 = rng.random();
    (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reactor(rules: &str) -> Reactor {
        Reactor::new(20.0, 20.0, Chemistry::parse(rules).unwrap(), 1)
    }

    #[test]
    fn collision_creates_bond_and_bonded_rule_follows() {
        let mut r = reactor("set temperature 0\na0 + b0 -> a1 . b1\na1 . b1 -> a2 . b2");
        r.add_atom('a', 0, Vec2::new(10.0, 10.0));
        r.add_atom('b', 0, Vec2::new(11.1, 10.0)); // within reaction range (1.25)
        r.step();
        assert_eq!((r.atoms[0].state, r.atoms[1].state), (1, 1));
        assert!(r.atoms[0].is_bonded(1));
        r.step();
        assert_eq!((r.atoms[0].state, r.atoms[1].state), (2, 2));
        assert_eq!(r.stats.per_rule, vec![1, 1]);
    }

    #[test]
    fn wildcard_rule_in_reactor() {
        let mut r = reactor("set temperature 0\n*1 + *2 -> *3 . *4");
        r.add_atom('q', 2, Vec2::new(10.0, 10.0));
        r.add_atom('z', 1, Vec2::new(11.0, 10.0));
        r.step();
        assert_eq!((r.atoms[0].compound().to_string(), r.atoms[1].compound().to_string()), ("q4".into(), "z3".into()));
        assert_eq!(r.bond_count(), 1);
    }

    #[test]
    fn spring_relaxes_to_rest_length() {
        let mut r = reactor("set temperature 0\nspring a b k=30 len=2.0");
        let i = r.add_atom('a', 0, Vec2::new(8.0, 10.0));
        let j = r.add_atom('b', 0, Vec2::new(11.5, 10.0));
        r.bond(i, j);
        for _ in 0..2000 {
            r.step();
        }
        let d = (r.atoms[0].pos - r.atoms[1].pos).len();
        assert!((d - 2.0).abs() < 0.02, "d = {d}");
        // default spring for other pairs: len = 2r = 1
        assert_eq!(r.spring('a', 'a').len, 1.0);
        assert_eq!(r.spring('b', 'a').k, 30.0);
    }

    #[test]
    fn break_length_breaks_bond() {
        let mut r = reactor("set temperature 0\nspring * * k=1 len=1 break=1.5");
        let i = r.add_atom('a', 0, Vec2::new(5.0, 10.0));
        let j = r.add_atom('a', 0, Vec2::new(7.0, 10.0));
        r.bond(i, j);
        r.step();
        assert_eq!(r.bond_count(), 0);
    }

    #[test]
    fn spheres_do_not_overlap_much() {
        let mut r = reactor("set temperature 0.3");
        r.add_food(150);
        for _ in 0..500 {
            r.step();
        }
        let mut min_d = f32::MAX;
        for i in 0..r.atoms.len() {
            for j in i + 1..r.atoms.len() {
                min_d = min_d.min((r.atoms[i].pos - r.atoms[j].pos).len());
            }
        }
        assert!(min_d > 0.7, "min distance {min_d}");
        assert!((r.kinetic_temperature() - 0.3).abs() < 0.1, "kT {}", r.kinetic_temperature());
    }

    #[test]
    fn snapshot_roundtrip_and_remove() {
        let mut r = reactor("");
        for k in 0..4 {
            r.add_atom('a', k, Vec2::new(2.0 + 2.0 * k as f32, 5.0));
        }
        r.bond(0, 1);
        r.bond(2, 3);
        r.bond(1, 2);
        let s = r.to_snapshot();
        let mut r2 = Reactor::from_snapshot(&s, Chemistry::new(), 0).unwrap();
        assert_eq!(r2.bond_count(), 3);
        r2.remove_atoms(&[1]);
        assert_eq!(r2.atoms.len(), 3);
        assert_eq!(r2.bond_count(), 1);
        assert!(r2.atoms[1].is_bonded(2)); // old 2-3 bond, re-indexed
    }
}
