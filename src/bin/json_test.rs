use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Deserialize)]
struct Atoms {
    index: u32,
    symbol: String,
    #[serde(rename = "type")]
    atom_type: String,
    x: f64,
    y: f64,
}

#[derive(Debug, Deserialize)]
struct Bond {
    index: u32,
    atom1: u32,
    atom2: u32,
    bond_type: String,
    comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Compounds {
    atoms: Vec<Atoms>,
    bonds: Vec<Bond>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ouvrir et lire le fichier JSON
    let file = File::open("data/kegg_compounds.json")?;
    let reader = BufReader::new(file);

    // Parser le JSON (c'est un tableau de composés)
    let compounds: Vec<Compounds> = serde_json::from_reader(reader)?;

    // Traiter chaque composé
    for compound in compounds {
        println!("=== ATOMS ===");
        for atom in &compound.atoms {
            println!(
                "Index: {}, Symbol: {}, Type: {}, Position: ({}, {})",
                atom.index, atom.symbol, atom.atom_type, atom.x, atom.y
            );
        }

        println!("\n=== BONDS ===");
        for bond in &compound.bonds {
            println!(
                "Bond {}: Atom {} - Atom {} (Type: {}){}",
                bond.index,
                bond.atom1,
                bond.atom2,
                bond.bond_type,
                bond.comment
                    .as_ref()
                    .map_or(String::new(), |c| format!(" - Comment: {}", c))
            );
        }

        // Afficher quelques statistiques
        println!("\n=== STATISTIQUES ===");
        println!("Nombre d'atomes: {}", compound.atoms.len());
        println!("Nombre de liaisons: {}", compound.bonds.len());

        // Compter les types d'atomes
        use std::collections::HashMap;
        let mut atom_types = HashMap::new();
        for atom in &compound.atoms {
            *atom_types.entry(&atom.atom_type).or_insert(0) += 1;
        }
        println!("Types d'atomes: {:?}", atom_types);
    }

    Ok(())
}