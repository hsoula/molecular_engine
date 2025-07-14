use crate::compound::Compound;
use crate::reactor::Reactor;

pub struct Atom{
    pub x: i32,
    pub y: i32,
    pub link : Vec<i32>,
    pub id : i32,
    pub form : i32,
    pub state : i32,
    pub elasticity : i32
}

impl Atom{
    pub fn new(x: i32, y: i32, id : i32) -> Atom {
        Atom {x, y, link:Vec::new(),  id, form:-1, state:-1, elasticity:10}
    }
    pub fn export_to_text(& self) -> String {
        let mut s = self.id.to_string();
        s = s + " " + &self.x.to_string()+ " " + &self.y.to_string();
        s = s + " " + &self.form.to_string()+ " " + &self.state.to_string();
        s = s + "\n";
        s
    }
    fn set_form(&mut self, form : i32) {
        self.form = form;
    }
    fn set_state(&mut self, state : i32) {
        self.state = state;
    }
    fn get_rule_string(self) -> String {
        self.form.to_string() + " " + &self.state.to_string()
    }
    pub fn compound(&self) -> Compound {
        Compound {form: self.form, state: self.state}
    }

    fn set_from_compound(&mut self, compound : Compound) {
        self.form = compound.form;
        self.state = compound.state;
    }

    pub fn link(&mut self, id: i32 ) {
        self.link.push(id);
    }
    pub fn unlink(&mut self, id: i32 ) {
        let index = self.link.iter().position(|x| *x == id).unwrap();
        self.link.remove(index);
    }

    fn is_linked(&self, id:i32) -> i32 {
        // return -1 is not in the list the position in the list if in
        let a = self.link.iter().position(|&r| r==id);
        if a == None {
            return -1;
        }
        a.unwrap() as i32
    }

    pub fn update(&mut self, nx : i32, ny : i32 ) {
        self.x = nx;
        self.y = ny;
    }

    fn remove(&mut self, id: i32) {
        let a = self.is_linked(id);
        if a != -1 {
            self.link.remove(a as usize);
        }
    }

    pub fn next_move_energy(&self, nx:i32, ny:i32, world : &Reactor) -> i32{
        let mut d = 0;
        let mut nd = 0;
        let x = self.x;
        let y = self.y;
        for i in 0..self.link.len() {
            let a = &world.atoms[self.link[i] as usize];
            d += (a.x-x) * (a.x-x) + (a.y-y) * (a.y-y);
            nd += (a.x-nx) * (a.x-nx) + (a.y-ny) * (a.y-ny);
        }
        nd
    }
}