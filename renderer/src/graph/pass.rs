use super::manager::Identifiable;

pub struct Pass {
    identifier : usize,
    name : &'static str,
}

impl Pass {
    pub fn new() -> Pass {
        todo!()
    }
}

impl Identifiable for Pass {
    fn name(&self) -> &'static str { self.name }
}