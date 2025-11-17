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
 