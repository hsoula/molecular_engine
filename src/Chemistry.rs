use crate::rule::Rule;

pub struct Chemistry {
    nb_rules : i32,
    rules : Vec<Rule>
}


impl Chemistry {
    pub fn new()-> Chemistry {
        Chemistry{nb_rules : 0, rules : Vec::new()}
    }

    pub fn add_rule_from_array(&mut self, array: Vec<i32> ) {
        let id = self.nb_rules;
        let rule = Rule::new_from_array(array, id);
        self.rules.push(rule);
        self.nb_rules += 1;
    }
    pub fn add_rule_from_text(&mut self, line: String) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut array : Vec<i32> = Vec::new();
        for i in 0..parts.len() {
            array.push(parts[i].trim().parse::<i32>().unwrap());
        }
        self.add_rule_from_array(array);
    }

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
}