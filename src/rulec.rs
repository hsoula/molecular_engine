use crate::compound::Compound;
use serde::{Deserialize, Serialize};
#[derive(PartialEq, Eq,  Serialize, Deserialize)]
pub struct RuleC {
    pub contact : bool,
    pub a1 : Compound,
    pub a2 : Compound,
}

impl RuleC {
    pub fn new(contact :bool, a: Compound, b:Compound)->RuleC {
        if a < b {
            return RuleC{contact:contact, a1:a, a2:b}
        }
        RuleC{contact:contact, a1:b, a2:a}
    }
    pub fn get_key(&self) -> String {

        let mut s = "".to_string();
        s = s + " " + &self.contact.to_string();
        s = s + " " + &self.a1.form.to_string();
        s = s + " " + &self.a1.state.to_string();
        s = s + " " + &self.a2.form.to_string();
        s = s + " " + &self.a2.state.to_string();
        s
    }
}