use serde::{Deserialize, Serialize};
use serde_json::Result;
use scanf::sscanf;
use crate::Atom;

#[derive(PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub struct Compound {
    pub form : char,
    pub state : i32
}
pub fn new_compound_from_text(s : String) -> Compound {
    let mut form : char = 'a';
    let mut state : i32 = 0;
    sscanf!(&s,"{}_{}", form, state);
    Compound{form : form, state : state}
}
impl Compound {
    pub fn new(a : Atom) -> Compound {
        Compound{form : a.form, state : a.state}
    }

    pub fn to_string(self: &Compound) -> String {
        self.form.to_string()+"_"+self.state.to_string().as_str()
    }
}