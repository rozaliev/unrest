use std::ops::Deref;
use std::rc::Rc;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::any::{Any, TypeId};
use std::default::Default;

#[derive(Clone)]
pub struct Container {
    hm: Rc<HashMap<TypeId, Box<Any>, BuildHasherDefault<IdentityHash>>>,
}

#[derive(Debug)]
pub struct State<T>(Rc<T>);

#[derive(Default)]
struct IdentityHash(u64);

impl<T: 'static> State<T> {
    fn wrap(i: Rc<T>) -> State<T> {
        State(i)
    }
}

impl<T: 'static> Clone for State<T> {
    fn clone(&self) -> Self {
        State(self.0.clone())
    }
}

impl<T> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0.as_ref()
    }
}

impl Container {
    pub fn new() -> Container {
        Container { hm: Rc::new(HashMap::default()) }
    }

    pub fn set<T: 'static>(&mut self, v: T) -> bool {
        let mut hm = Rc::get_mut(&mut self.hm).expect("can't modify state container at this point");
        hm.insert(TypeId::of::<T>(), Box::new(Rc::new(v))).is_none()
    }

    pub fn get<T: 'static>(&self) -> Option<State<T>> {
        self.hm
            .get(&TypeId::of::<T>())
            .and_then(|a| a.downcast_ref::<Rc<T>>())
            .map(|rc| State::wrap(rc.clone()))
    }
}

impl Hasher for IdentityHash {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.write_u8(*byte);
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.0 = (self.0 << 8) | (i as u64);
    }

    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get() {
        let mut c = Container::new();
        let yo = "yo";

        assert_eq!(c.set(yo), true);
        assert_eq!(c.set(yo), false);

        assert_eq!(*c.get::<&'static str>().unwrap(), "yo");
    }
}