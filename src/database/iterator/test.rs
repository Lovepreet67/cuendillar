use crate::{
    OwnedEntry,
    database::{
        Entry,
        iterator::{DatabaseIterator, merged_iterator::MergedIterator},
    },
};

struct VecIterator {
    data: Vec<OwnedEntry>,
    pos: usize,
}

impl VecIterator {
    fn new(data: Vec<OwnedEntry>) -> Self {
        Self { data, pos: 0 }
    }
}

impl DatabaseIterator for VecIterator {
    fn peek(&self) -> Option<Entry<'_>> {
        if self.data.len() > self.pos {
            return Some((&self.data[self.pos]).into());
        } else {
            None
        }
    }
    fn next_owned(&mut self) -> Option<OwnedEntry> {
        if self.pos >= self.data.len() {
            return None;
        }
        let v = self.data[self.pos].clone();
        self.pos += 1;
        Some(v)
    }
    fn first_entry(&self) -> Option<Entry<'_>> {
        if self.data.len() > 0 {
            return Some((&self.data[0]).into());
        }
        None
    }
    fn last_entry(&self) -> Option<Entry<'_>> {
        if self.data.len() > 0 {
            return Some((&self.data[0]).into());
        }
        None
    }
}

fn row(seq: u64, key: &str, value: &str) -> OwnedEntry {
    OwnedEntry::Row {
        seq_no: seq,
        key: key.as_bytes().to_vec(),
        value: value.as_bytes().to_vec(),
    }
}

fn tomb(seq: u64, key: &str) -> OwnedEntry {
    OwnedEntry::Tombstone {
        seq_no: seq,
        key: key.as_bytes().to_vec(),
    }
}
#[test]
fn test_basic_merge() {
    let v1 = vec![row(1, "a", "1"), row(1, "c", "3")];
    let it1 = VecIterator::new(v1);

    let v2 = vec![row(1, "b", "2"), row(1, "d", "4")];

    let it2 = VecIterator::new(v2);

    let mut merged = MergedIterator::new();
    merged.add_iterator(Box::new(it1));
    merged.add_iterator(Box::new(it2));

    let mut results = Vec::new();

    while let Some(e) = merged.next_owned() {
        results.push(e);
    }

    assert_eq!(results.len(), 4);

    assert_eq!(results[0], row(1, "a", "1"));
    assert_eq!(results[1], row(1, "b", "2"));
    assert_eq!(results[2], row(1, "c", "3"));
    assert_eq!(results[3], row(1, "d", "4"));
}

#[test]
fn test_duplicate_keys_removed() {
    let v1 = vec![row(1, "a", "old")];
    let it1 = VecIterator::new(v1);

    let v2 = vec![row(2, "a", "new")];
    let it2 = VecIterator::new(v2);

    let mut merged = MergedIterator::new();
    merged.add_iterator(Box::new(it1));
    merged.add_iterator(Box::new(it2));

    let mut results = Vec::new();

    while let Some(e) = merged.next_owned() {
        results.push(e);
    }

    assert_eq!(results.len(), 1);
    assert_eq!(results[0], row(2, "a", "new"));
}

#[test]
fn test_tombstone() {
    let v1 = vec![row(1, "a", "value")];
    let it1 = VecIterator::new(v1);

    let v2 = vec![tomb(2, "a")];

    let it2 = VecIterator::new(v2);

    let mut merged = MergedIterator::new();
    merged.add_iterator(Box::new(it1));
    merged.add_iterator(Box::new(it2));

    let result = merged.next_owned().unwrap();

    match result {
        OwnedEntry::Tombstone { .. } => {}
        _ => panic!("expected tombstone"),
    }
}

#[test]
fn test_multiple_iterators() {
    let v1 = vec![row(1, "a", "1"), row(1, "d", "4")];
    let it1 = VecIterator::new(v1);

    let v2 = vec![row(1, "b", "2"), row(1, "e", "5")];

    let it2 = VecIterator::new(v2);
    let v3 = vec![row(1, "c", "3"), row(1, "f", "6")];

    let it3 = VecIterator::new(v3);

    let mut merged = MergedIterator::new();

    merged.add_iterator(Box::new(it1));
    merged.add_iterator(Box::new(it2));
    merged.add_iterator(Box::new(it3));

    let mut keys = Vec::new();

    while let Some(e) = merged.next_owned() {
        keys.push(String::from_utf8(e.get_key().to_vec()).unwrap());
    }

    assert_eq!(keys, vec!["a", "b", "c", "d", "e", "f"]);
}
