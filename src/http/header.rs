use std::{borrow::Borrow, collections::HashMap, hash::Hash};


type HeaderMap = HashMap<String, String>;
#[derive(Debug)]
pub struct Headers(HeaderMap);

impl<'h> IntoIterator for &'h Headers {
    type Item = <&'h HeaderMap as IntoIterator>::Item;
    type IntoIter = <&'h HeaderMap as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl Headers {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.0.insert(key, value);
    }

    pub fn insert_header_line(&mut self, header_line: String) {
        // add error handling
        let (key, value) = header_line.split_once(':').unwrap();
        let value = value.trim();
        self.insert(key.to_owned(), value.to_owned());
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&str>
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.0.get(key).map(|s| s.as_str())
    }
}
