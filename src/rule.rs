use scanf::sscanf;
use crate::compound::Compound;
use crate::rulec::RuleC;
use crate::rulec::new_rulec_from_text;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct Rule {
    pub substrate : RuleC,
    pub product : RuleC,
    pub id : i32
}
pub fn new_rule_from_text(s : String, id:i32) -> Rule {
    let parts = s.split("->").collect::<Vec<&str>>();
    let substrate = crate::rulec::new_rulec_from_text(parts[0].to_string());
    let product = crate::rulec::new_rulec_from_text(parts[1].to_string());
    Rule { substrate, product, id:id}
}
impl Rule {
    // pub fn new_from_array(array: Vec<i32>, id:i32) -> Rule {
    //     let contact_s = array[0];
    //     let form_s1 = array[1];
    //     let state_s1 = array[2];
    //     let form_s2 = array[3];
    //     let state_s2 = array[4];
    //     let contact_p = array[5];
    //     let form_p1 = array[6];
    //     let state_p1 = array[7];
    //     let form_p2 = array[8];
    //     let state_p2 = array[9];
    //     let s1 = Compound{form: form_s1 as char, state: state_s1 };
    //     let s2 = Compound{form: form_s2 as char, state: state_s2 };
    //     let p1 = Compound{form: form_p1 as char, state: state_p1 };
    //     let p2 = Compound{form: form_p2 as char, state: state_p2 };
    //     let r1 = RuleC::new(contact_s > 0, s1, s2);
    //     let r2 = RuleC::new(contact_p > 0, p1, p2);
    //     Rule{substrate: r1, product: r2, id: id}
    // }
    pub  fn get_key(self: &Rule) -> String {
        self.substrate.get_key()
    }
    pub fn to_string(self: &Rule) -> String {
        let s = self.substrate.to_string();
        let p = self.product.to_string();
        s+"->"+&p 

    }
}
