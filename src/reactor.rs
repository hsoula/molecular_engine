use std::fs::File;
use crate::Atom;
use crate::chemistry::Chemistry;
use std::io::Write;
use crate::compound::Compound;
use crate::rulec::RuleC;
use crate::rule::Rule;

pub struct Reactor {
    w:i32,
    h:i32,
    nb :i32,
    pub atoms: Vec<Atom>,
    grid : Vec<i32>,
    chem : Chemistry,
    dirty : bool
}

impl Reactor {
    pub fn new(w:i32, h:i32, nb:i32) -> Reactor {

        let atoms = Vec::new();
        let grid = vec![-1 ; (w * h) as usize];
        Reactor {w, h, nb, atoms, grid, chem: Chemistry::new(), dirty:false}
    }
    pub fn get_w(&self) -> i32 { self.w }
    pub fn get_h(&self) -> i32 { self.h }
    fn xy_to_pos(&self, x: i32, y: i32) -> usize {
        (y * self.w + x) as usize
    }
    fn add_rule_from_array(&mut self, array: Vec<i32>) {
        self.chem.add_rule_from_array(array);
    }
    fn add_rule_from_text(&mut self, line: String) {
        self.chem.add_rule_from_text(line);
    }
    pub fn add_rule(&mut self, r : Rule) { self.chem.add_rule(r); }
    pub fn fill_random(&mut self) {

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
        // first check they are neighours
        let a = &self.atoms[id1 as usize];
        let b = &self.atoms[id2 as usize];
        let d = (a.x - b.x).abs() + (a.y - b.y).abs();
        assert!(d<3);
        let r = self.chem.find_rule_from_atoms(false, a, b);
        if r.is_none()
        {
            println!("id1: {}, id2: {}", id1, id2);
            if id1 == 0 {
                println!("RULE {} {} {} {}",a.form, a.state,b.form,b.state);
            }
        }
        else {
            // apply rule
            let r = r.unwrap();
            println!("APPLY r: {}, r: {} ", r.substrate.a1.state, r.substrate.a2.state);
            if r.substrate.a1.form == a.form && r.substrate.a1.state == a.state {
                self.atoms[id1 as usize].state = r.product.a1.state;
                self.atoms[id2 as usize].state = r.product.a2.state;
                self.dirty = true;
            }
            else {
                self.atoms[id2 as usize].state = r.product.a1.state;
                self.atoms[id1 as usize].state = r.product.a2.state;
                self.dirty = true;
            }
            if r.product.contact == true {
                self.atoms[id1 as usize].link( id2);
                self.atoms[id2 as usize].link( id1);
            }
        }


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
            // if next move is inbound
            if self.not_empty(nx, ny) {
                // if there is someone
                let id2 = self.grid[self.xy_to_pos(nx, ny)];

                // resolve the collision (if there is rule)
                self.resolve_collision(id, id2);
                nb_contacts += 1;
            }
            else
            {
                // test if particle is not too far from the others
                let d = self.atoms[id as usize].next_move_energy(nx, ny, self);
                // self.update_atom_position(id, nx, ny);
                //println!("{} -> {} {}", self.atoms[id as usize].id, d,self.atoms[id as usize].link.len());
                 if d <=  self.atoms[id as usize].elasticity {
                     self.update_atom_position(id, nx, ny);
                 }

            }
        }
        nb_contacts
    }

    pub fn move_all_atoms(&mut self) -> i32{
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
        if self.in_bounds(x, y) {
            let id = self.grid[(x + y * self.w) as usize];
            return id != -1;
        }
        true
    }
    fn export_to_text(&self, filename: String) {
        let mut file = File::create(&filename).expect("creation failed");;
        for i in 0..self.atoms.len() {
            file.write_all(self.atoms[i].export_to_text().as_bytes());
        }
        for i in 0..self.atoms.len() {
            for j in 0..self.atoms[i].link.len() {
                let s = self.atoms[i].id.to_string() +  " " + &self.atoms[i].link[j].to_string() +"\n";
                file.write_all(s.as_bytes());
            }

        }
    }
    pub fn check_linked_rule(&mut self) {
        if self.dirty {
            'dirty: loop {
                self.dirty = false;
                for id1 in 0..self.atoms.len() {
                    for j in 0..self.atoms[id1 as usize].link.len() {
                        let id2 = self.atoms[id1 as usize].link[j];
                        let a = &self.atoms[id1 as usize];
                        let b = &self.atoms[id2 as usize];

                        let r = self.chem.find_rule_from_atoms(true, a, b);
                        if r.is_none()
                        {
                            // do nothing
                        } else {
                            // apply rule
                            let r = r.unwrap();
                            println!("APPLY contact r: {}, r: {} ", r.substrate.a1.state, r.substrate.a2.state);
                            if r.substrate.a1.form == a.form && r.substrate.a1.state == a.state {
                                self.atoms[id1 as usize].state = r.product.a1.state;
                                self.atoms[id2 as usize].state = r.product.a2.state;
                                self.dirty = true;
                            } else {
                                self.atoms[id2 as usize].state = r.product.a1.state;
                                self.atoms[id1 as usize].state = r.product.a2.state;
                                self.dirty = true;
                            }
                            if r.product.contact == false {
                                self.atoms[id1 as usize].unlink(id2);
                                self.atoms[id2 as usize].unlink(id1 as i32);
                            }
                        }
                    }

                    if self.dirty == false {
                        break 'dirty;
                    }
                }
            }
        }
    }
}