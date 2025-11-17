use std::collections::{HashMap, HashSet, VecDeque};
use crate::atom::Atom;
use crate::links::Link;

use crate::compound::Compound;
use crate::reactor::Reactor;

pub struct Molecule{
    pub atoms: Vec<i32>,
    pub links : Vec<Link>,
    pub id : i32,
}
impl Molecule {
    pub fn new() -> Self {
        Self {
            atoms: Vec::new(),
            links: Vec::new(),
            id: 0,
        }
    }
}

pub fn find_molecules(atoms: Vec<Atom>) -> Vec<Molecule> {
    let mut atom_map: HashMap<u32, &Atom> = atoms
        .iter()
        .map(|atom| (atom.id, atom))
        .collect();

    let mut visited = HashSet::new();
    let mut molecules = Vec::new();

    for atom in &atoms {
        if !visited.contains(&atom.id) {
            // Trouver une composante connexe avec BFS
            let mut molecule = Molecule::new();
            let mut queue = VecDeque::new();
            let mut component_atoms = HashSet::new();

            queue.push_back(atom.id);
            visited.insert(atom.id);
            component_atoms.insert(atom.id);

            while let Some(current_id) = queue.pop_front() {
                let current_atom = atom_map[&(current_id as u32)];
                molecule.atoms.push(current_atom.id);

                // Ajouter les liens de l'atome courant
                for &neighbor_id in &current_atom.link {
                    let link = Link::new(current_id, neighbor_id);
                    if !molecule.links.contains(&link) {
                        molecule.links.push(link);
                    }

                    // Ajouter les voisins non visités
                    if !visited.contains(&neighbor_id) {
                        visited.insert(neighbor_id);
                        component_atoms.insert(neighbor_id);
                        queue.push_back(neighbor_id);
                    }
                }
            }

            molecules.push(molecule);
        }
    }

    molecules
}

/*
impl Molecule {
    pub fn new_from_json(filename: String) -> Molecule {
        let data = std::fs::read_to_string("data.json").unwrap();
        let v: Value = serde_json::from_str(&data).unwrap();
        
        // Access parts using Value's API
        if let Some(name) = v["name"].as_str() {
            println!("Name: {}", name);
        }

    }
}
*/
 