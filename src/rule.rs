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
pub fn new_rules_from_text(s : String, offset:i32) -> Vec<Rule> {
    let parts = s.split("->").collect::<Vec<&str>>();
    let mut rules = Vec::new();
    let mut offset = offset;
    let chars = ['a','b','c','d','e','f','g','h'];
    let cmax = chars.len();
    let mut forms1 : char = 'a';
    let mut states1 : i32 = 0;
    let mut forms2 : char = 'a';
    let mut states2 : i32 = 0;
    let mut contact_chars : char = '.';
    let results = sscanf!(parts[0],"{}_{}({}){}_{}", forms1, states1, contact_chars, forms2, states2);
    let mut formp1 : char = 'a';
    let mut statep1 : i32 = 0;
    let mut formp2 : char = 'a';
    let mut statep2 : i32 = 0;
    let mut contact_charp : char = '.';
    let resultp = sscanf!(parts[0],"{}_{}({}){}_{}", formp1, statep1, contact_charp, formp2, statep2);
    let mut contacts : bool = false;
    if contact_chars == '.' {
        contacts = true;
    }
    let mut contactp : bool = false;
    if contact_charp == '.' {
        contactp = true;
    }
    if forms1 == '*' && forms2 == '*'{
        for i in 0..cmax{

            let f1 = chars[i];
            let substrate = RuleC{contact:contacts, a1: Compound{form:f1, state:states1}, a2:Compound{form:f1, state:states2}};
            let product = RuleC{contact:contactp, a1: Compound{form:f1, state:statep1}, a2:Compound{form:f1, state:statep2}};

            rules.push(Rule { substrate, product, id:offset});
            offset += 1;
        }
    }
    else if forms1 == '*' && forms2 == '#' {
        for i in 0..cmax {
            for j in 0..cmax {
                let f1 = chars[i];
                let f2 = chars[j];
                let substrate = RuleC { contact: contacts, a1: Compound { form: f1, state: states1 }, a2: Compound { form: f2, state: states2 } };
                let product = RuleC { contact: contactp, a1: Compound { form: f1, state: statep1 }, a2: Compound { form: f2, state: statep2 } };

                rules.push(Rule { substrate, product, id: offset });
                offset += 1;
            }
        }
    }
    else
    {
        let substrate = RuleC { contact: contacts, a1: Compound { form: forms1, state: states1 }, a2: Compound { form: forms2, state: states2 } };
        let product = RuleC { contact: contactp, a1: Compound { form: formp1, state: statep1 }, a2: Compound { form: formp2, state: statep2 } };
        rules.push(Rule { substrate, product, id: offset });
    }
    rules
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
