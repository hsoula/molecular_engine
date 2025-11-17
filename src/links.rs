pub struct Link {
    pub links : [i32; 2]
}

impl Link {
    pub fn new(id1: i32, id2: i32) -> Link {
        if id1 > id2 {
            let mut links: [i32; 2] = [id1, id2];
            Link { links }
        }
        else {
            let mut links: [i32; 2] = [id2, id1];
            Link { links}
        }
    }
}