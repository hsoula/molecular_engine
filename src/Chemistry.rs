use crate::rule::Rule;

use serde::{Deserialize, Serialize};
use crate::atom::Atom;
use crate::rulec::RuleC;

#[derive(Serialize, Deserialize)]
pub struct Chemistry {
    nb_rules : i32,
    rules : Vec<Rule>
}


impl Chemistry {
    pub fn new()-> Chemistry {
        Chemistry{nb_rules : 0, rules : Vec::new()}
    }
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }
    // pub fn add_rule_from_array(&mut self, array: Vec<i32> ) {
    //     let id = self.nb_rules;
    //     let rule = Rule::new_from_array(array, id);
    //     self.rules.push(rule);
    //     self.nb_rules += 1;
    // }
    // pub fn add_rule_from_text(&mut self, line: String) {
    //     let parts: Vec<&str> = line.split_whitespace().collect();
    //     let mut array : Vec<i32> = Vec::new();
    //     for i in 0..parts.len() {
    //         array.push(parts[i].trim().parse::<i32>().unwrap());
    //     }
    //     self.add_rule_from_array(array);
    // }


    pub fn find_rule_from_string(&self, s : String) -> Option<&Rule> {
        for rule in &self.rules {
            if rule.get_key() == s {
                return Some(&rule);
            }
        }
        None
    }
    pub fn find_rule(&self, r: Rule) -> Option<&Rule> {
        self.find_rule_from_string(r.get_key())
    }
    pub fn find_rule_from_atoms(&self, contact : bool,  a : &Atom, b : &Atom) -> Option<&Rule> {
        let c1 = a.compound();
        let c2 = b.compound();
        println!(" state {} {}", c1.state, c2.state);
        let rx = RuleC::new(contact, c1, c2);
        println!("contact {}", rx.get_key());
        self.find_rule_from_string(rx.get_key())
    }
}