use std::{hash::Hash, borrow::Borrow, collections::HashMap};


#[derive(Debug)]
pub struct Headers(HashMap<String, String>);

impl Headers {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.0.insert(key.to_lowercase(), value);
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&str>
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.0.get(key).map(|s| s.as_str())
    }

}
