use crate::compound::Compound;
use serde::{Deserialize, Serialize};
use scanf::sscanf;

#[derive(PartialEq, Eq,  Serialize, Deserialize)]
pub struct RuleC {
    pub contact : bool,
    pub a1 : Compound,
    pub a2 : Compound,
}
pub fn new_rulec_from_text(s : String) -> RuleC {
    let mut form1 : char = 'a';
    let mut state1 : i32 = 0;
    let mut form2 : char = 'a';
    let mut state2 : i32 = 0;
    let mut contact_char : char = '.';
    let result = sscanf!(&s,"{}_{}({}){}_{}", form1, state1, contact_char, form2, state2);
    println!("lol {}---({}{}){}({}{})", s, form1, state1, contact_char, form2, state2);
    let mut contact : bool = false;
    if contact_char == '.' {
        contact = true;
    }
    RuleC{contact:contact,
        a1:Compound{form:form1, state: state1},
        a2:Compound{form:form2, state:state2}}
}
impl RuleC {
    pub fn new(contact :bool, a: Compound, b:Compound)->RuleC {
        // if a.form <= b.form && a.state <= b.state  {
        //     println!("inf");
        //     return RuleC{contact:contact, a1:a, a2:b}
        // }
        // println!("sup");
        RuleC{contact:contact, a1:a, a2:b}
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

    pub fn to_string(self: &RuleC) -> String {
        let s=  self.a1.to_string();
        let p=  self.a2.to_string();
        let c= if self.contact {"."}  else {"+"};
        s +"("+&c+")"+&p
    }
}