use std::collections::{HashMap, HashSet, VecDeque};

// Structure Atom avec un identifiant unique et des liens vers d'autres atomes
#[derive(Debug, Clone)]
struct Atom {
    id: u32,
    links: Vec<u32>,
}

// Structure pour représenter un lien entre deux atomes
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct Link {
    from: u32,
    to: u32,
}

impl Link {
    fn new(from: u32, to: u32) -> Self {
        // On normalise le lien pour éviter les doublons (a,b) et (b,a)
        if from < to {
            Link { from, to }
        } else {
            Link { from: to, to: from }
        }
    }
}

// Structure Molecule contenant des atomes et leurs liens
#[derive(Debug)]
struct Molecule {
    atoms: Vec<Atom>,
    links: Vec<Link>,
}

impl Molecule {
    fn new() -> Self {
        Self {
            atoms: Vec::new(),
            links: Vec::new(),
        }
    }
}

fn find_molecules(atoms: Vec<Atom>) -> Vec<Molecule> {
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
                let current_atom = atom_map[&current_id];
                molecule.atoms.push(current_atom.clone());

                // Ajouter les liens de l'atome courant
                for &neighbor_id in &current_atom.links {
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

// Exemple d'utilisation
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Création de quelques atomes avec leurs liens
    let atoms = vec![
        Atom { id: 1, links: vec![2, 3] },
        Atom { id: 2, links: vec![1] },
        Atom { id: 3, links: vec![1, 4] },
        Atom { id: 4, links: vec![3] },
        Atom { id: 5, links: vec![6] },
        Atom { id: 6, links: vec![5] },
    ];

    let molecules = find_molecules(atoms);

    for (i, molecule) in molecules.iter().enumerate() {
        println!("Molecule {}:", i + 1);
        println!("  Atomes: {:?}", molecule.atoms.iter().map(|a| a.id).collect::<Vec<_>>());
        println!("  Liens: {:?}", molecule.links);
        println!();
    }
    Ok(())
}