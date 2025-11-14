use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Deserialize)]
struct ReactionSide {
    contact: String,
    a1: String,
    a2: String,
}

#[derive(Debug, Deserialize)]
struct Reaction {
    string_name: String,
    substrates: ReactionSide,
    products: ReactionSide,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ouvrir et lire le fichier JSON
    let file = File::open("reactions.json")?;
    let reader = BufReader::new(file);

    // Parser le JSON (c'est un tableau de réactions)
    let reactions: Vec<Reaction> = serde_json::from_reader(reader)?;

    // Traiter chaque réaction
    for (i, reaction) in reactions.into_iter().enumerate() {
        println!("=== Réaction {} ===", i + 1);
        println!("Équation: {}", reaction.string_name);

        println!("Substrats:");
        println!("  - Contact: {}", reaction.substrates.contact);
        println!("  - a1: {}", reaction.substrates.a1);
        println!("  - a2: {}", reaction.substrates.a2);

        println!("Produits:");
        println!("  - Contact: {}", reaction.products.contact);
        println!("  - a1: {}", reaction.products.a1);
        println!("  - a2: {}", reaction.products.a2);

        println!();
    }

    // Statistiques générales
    println!("=== STATISTIQUES ===");
    println!("Nombre total de réactions: {}", reactions.len());

    // Compter les réactions avec/sans contact
    let with_contact = reactions.into_iter()
        .filter(|r| r.substrates.contact == "True" || r.products.contact == "True")
        .count();
    println!("Réactions avec contact: {}", with_contact);

    Ok(())
}