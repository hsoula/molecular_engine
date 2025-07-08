mod chemistry;
mod compound;
mod rule;
mod rulec;

use crate::compound::Compound;
use crate::rule::Rule;
use crate::rulec::RuleC;
use crate::chemistry::Chemistry;


use rand;



struct Reactor {
    w:i32,
    h:i32,
    nb :i32,
    atoms: Vec<Atom>,
    grid : Vec<i32>,
    chem : Chemistry
}

fn xy_to_pos(x: i32, y: i32, w: i32) -> usize {
    (y * w + x) as usize
}

impl Reactor {
    fn new(w:i32, h:i32, nb:i32) -> Reactor {

        let atoms = Vec::new();
        let grid = vec![-1 ; (w * h) as usize];
        Reactor {w, h, nb, atoms, grid, chem: Chemistry::new()}
    }
    fn xy_to_pos(&self, x: i32, y: i32) -> usize {
        (y * self.w + x) as usize
    }
    fn add_rule_from_array(&mut self, array: Vec<i32>) {
        self.chem.add_rule_from_array(array);
    }
    fn add_rule_from_text(&mut self, line: String) {
        self.chem.add_rule_from_text(line);
    }

    // fn get_collision_rule_from_ids(&mut self, id1: i32, id2: i32) ->  RuleC{
    //     RuleC::new(true, a: Compound::new(self.atoms[id1 as usize]), b: Compound::new(self.atoms[id1 as usize]))
    // }
    fn fill_random(&mut self) {

        for i in 0..self.nb {
            let id = i;
            let mut sx = -1;
            let mut sy = -1;
            loop
            {
                sx = rand::random_range(0..self.w);
                sy = rand::random_range(0..self.h);
                if ! self.not_empty(sx, sy)
                {
                    break;
                }
            }

            let mut a =  Atom::new(sx, sy, id);
            self.add_atom(a);
        }
    }

    fn add_atom(&mut self, atom:Atom) -> bool{
        let x = atom.x;
        let y = atom.y;
        let id = atom.id;
        let u :usize = (y * self.w + x) as usize;
        if self.grid[u] == -1 {
            self.grid[u] = id;
            self.atoms.push(atom);
            assert_eq!(id, self.atoms.len() as i32 - 1);
            return true
        }
        false
    }
    fn resolve_collision(&mut self, id1:i32, id2:i32){

    }
    fn move_atom(&mut self, id: i32)  -> i32 {

        let index: u8 = rand::random_range(0..4);
        let mut nx = self.atoms[id as usize].x;
        let mut ny = self.atoms[id as usize].y;
        let mut nb_contacts = 0;
        match  index {
            0 => { nx +=1 }, // north
            1 => { nx +=-1 }, // south
            2 => { ny +=1 }, // west
            3 => { ny +=-1 },
            _ => unreachable!()
        }

        if self.in_bounds(nx, ny) {
            if !self.not_empty(nx, ny) {
                let id2 = self.grid[xy_to_pos(nx, ny, self.w)];
                self.resolve_collision(id, id2);
                nb_contacts += 1;
            }
            else
            {
                self.update_atom_position(id, nx, ny);
            }
        }
        nb_contacts
    }

    fn move_all_atoms(&mut self) -> i32{
        let mut nb_contacts = 0;
        for i in 0..self.atoms.len() {
            nb_contacts += self.move_atom(i as i32);
        }
        nb_contacts
    }

    fn get_atom_at(self, x:i32, y:i32) -> i32{
        if !self.not_empty(x, y){
            return self.grid[self.xy_to_pos(x, y)];
        }
        -1
    }
    fn set_atom_reaction_at(&mut self, form:i32, state:i32, id:i32) {
        if id >= 0 && id < self.atoms.len() as i32 {
            self.atoms[id as usize].form = form;
            self.atoms[id as usize].state = state;
        }
    }
    fn update_atom_position(&mut self, id : i32, nx: i32, ny: i32) {
        let x = self.atoms[id as usize].x;
        let y = self.atoms[id as usize].y;
        self.grid[(x + y * self.w) as usize] = -1;
        self.atoms[id as usize].update(nx, ny);
        self.grid[(nx + ny * self.w) as usize] = id;

    }

    fn in_bounds(&self, x:i32, y:i32) -> bool {
        x >= 0 && x < self.w && y >= 0 && y < self.h
    }

    fn not_empty(&self, x:i32, y:i32) -> bool {
        if x >= 0 && x < self.w && y >= 0 && y < self.h {
            let id = self.grid[(x + y * self.w) as usize];
            return id != -1;
        }
        true
    }

}
struct Atom{
    x: i32,
    y: i32,
    link : Vec<i32>,
    id : i32,
    form : i32,
    state : i32
}

impl Atom{
    fn new(x: i32, y: i32, id : i32) -> Atom {
        Atom {x, y, link:Vec::new(),  id, form:-1, state:-1 }
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
    fn compound(&self) -> Compound {
        Compound {form: self.form, state: self.state}
    }

    fn set_from_compound(&mut self, compound : Compound) {
        self.form = compound.form;
        self.state = compound.state;
    }

    fn link(&mut self, l: Atom ) {
        self.link.push(l.id);
    }

    fn is_linked(&self, id:i32) -> i32 {
        // return -1 is not in the list the position in the list if in
        let a = self.link.iter().position(|&r| r==id);
        if a == None {
            return -1;
        }
        a.unwrap() as i32
    }

    fn update(&mut self, nx : i32, ny : i32 ) {
        self.x = nx;
        self.y = ny;
    }

    fn remove(&mut self, id: i32) {
        let a = self.is_linked(id);
        if a != -1 {
            self.link.remove(a as usize);
        }
    }

    fn next_move_energy(&self, nx:i32, ny:i32, world : &Reactor) -> i32{
        let mut d = 0;
        let mut nd = 0;
        let x = self.x;
        let y = self.y;
        for i in 0..self.link.len() {
            let a = &world.atoms[self.link[i] as usize];
            d += (a.x-x) * (a.x-x) + (a.y-y) * (a.y-y);
            nd += (a.x-nx) * (a.x-nx) + (a.y-ny) * (a.y-ny);
        }
        d-nd
    }
}

fn main() {

    let n =30;
    let w = 10;
    let h = 10;
    let mut reactor = Reactor::new(w, h, n);
    reactor.fill_random();
    for i in 0..1000 {
        let x = reactor.move_all_atoms();
        println!("{} {}", i, x);
    }



}
