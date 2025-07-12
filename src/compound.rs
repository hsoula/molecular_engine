use serde::{Deserialize, Serialize};
use serde_json::Result;

use crate::Atom;

#[derive(PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub struct Compound {
    pub form : i32,
    pub state : i32
}
impl Compound {
    pub fn new(a : Atom) -> Compound {
        Compound{form : a.form, state : a.state}
    }
}