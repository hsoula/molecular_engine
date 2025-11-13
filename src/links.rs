pub struct Link {
    pub links : [i32; 2],
    pub form : String,
    pub elasticity : i32,
}

impl Link {
    pub fn new(id1: i32, id2: i32, form: String, elasticity : i32) -> Link {
        let mut links : [i32; 2] = [id1, id2];
        Link{links, form, elasticity}
    }
}