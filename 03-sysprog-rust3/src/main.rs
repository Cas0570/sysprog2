#![allow(unused_imports)]
#![allow(dead_code)]

use rand::Rng;

trait Hashable {
    fn get_hash(&self) -> u32;
}

impl Hashable for u32 {
    fn get_hash(&self) -> u32 {
        *self
    }
}

impl Hashable for &str {
    fn get_hash(&self) -> u32 {
        let mut hash: u32 = 0;

        for byte in self.bytes() {
            hash = hash.rotate_left(5);
            let byte = byte as u32;
            hash = hash ^ byte
        }

        hash
    }
}

struct Hash<T> {
    slots: Vec<Option<T>>,
    count: usize,
}

impl<T: Hashable + Eq> Hash<T> {
    fn new() -> Self {
        let mut slots = Vec::new();
        slots.resize_with(59, || None);

        Self {
            slots,
            count: 0,
        }
    }

    fn add(&mut self, value: T) {
        let mut index = value.get_hash() as usize % self.slots.len();

        for _ in 0..self.slots.len() {
            match &self.slots[index] {
                None => {
                    self.slots[index] = Some(value);
                    self.count += 1;
                    
                    if self.count * 3 > self.slots.len() * 2 {
                        self.resize();
                    }

                    return
                }
                Some(existing) => {
                    if existing == &value {
                        return;
                    }

                    index = (index + 1) % self.slots.len()
                }
            }
        }

        panic!("Hash table is full");
    }

    fn contains(&self, value: T) -> bool {
        let mut index = value.get_hash() as usize % self.slots.len();

        for _ in 0..self.slots.len() {
            match &self.slots[index] {
                None => {
                    return false;
                }
                Some(existing) => {
                    if existing == &value {
                        return true;
                    }

                    index = (index + 1) % self.slots.len();
                }
            }
        }

        false
    }

    fn resize(&mut self) {
        let mut new_slots = Vec::new();
        let new_size = self.count * 3;

        new_slots.resize_with(new_size, || None);

        let old_slots = std::mem::replace(
            &mut self.slots,
            new_slots,
        );

        self.count = 0;

        for slot in old_slots {
            match slot {
                Some(value) => {
                    self.add(value);
                }
                None => {}
            }
        }
    }
}

fn main() {
    // You can use this to experiment, or just leave it empty.
}


#[test]
fn u32_is_hashable() {
    assert_eq!(17.get_hash(), 17.get_hash());
    assert_ne!(17.get_hash(), 18.get_hash());
}


#[test]
fn hash_works_with_u32() {
    let mut hash = Hash::new();

    hash.add(1);
    hash.add(1234);
    hash.add(12345678);
    hash.add(58);
    hash.add(117);
    assert!(hash.contains(1));
    assert!(hash.contains(1234));
    assert!(hash.contains(12345678));
    assert!(hash.contains(58));
    assert!(hash.contains(117));
    assert!(!hash.contains(2));
    assert!(!hash.contains(1235));
    assert!(!hash.contains(12345679));
}

#[test]
fn hash_scales_with_u32() {
    let mut rng = rand::thread_rng();
    let mut hash = Hash::new();
    let mut reference = std::collections::HashSet::new();

    for _ in 0..500_000 {
        let n = rng.gen_range(0..2_000_000) as u32;
        hash.add(n);
        reference.insert(n);
    }

    for num in 0..2_000_000 {
        assert_eq!(hash.contains(num), reference.contains(&num));
    }
}

#[test]
fn str_ref_is_hashable() {
    assert_eq!("ab".get_hash(), "ab".get_hash());
    assert_eq!("".get_hash(), "".get_hash());
    assert_eq!("abcdefghijklmnopqrstuvwxyz".get_hash(), "abcdefghijklmnopqrstuvwxyz".get_hash());
    assert_ne!("abcdefghijklmnopqrstuvwxyz".get_hash(), "abcdefghijklmnopqrstuvwxy".get_hash());
    assert_ne!("abcdefghijklmnopqrstuvwxyz".get_hash(), "abcdefghijklmnopqrstuvwxyZ".get_hash());
    assert_ne!("ab".get_hash(), "ba".get_hash());
    assert_ne!("ab".get_hash(), "ba".get_hash());
}

#[test]
fn hash_works_with_str_ref() {
    let mut hash = Hash::new();

    hash.add("test");
    hash.add("abcd");
    hash.add("a0");
    // "a0" and "bP" both produce the same get_hash() value. The table must
    // still distinguish them: contains("bP") should be false until it's added.
    assert!(hash.contains("a0"));
    assert!(!hash.contains("bP"));
    hash.add("bP");
    assert!(hash.contains("bP"));
    assert!(hash.contains("a0"));

    assert!(hash.contains("test"));
    assert!(hash.contains("abcd"));
    assert!(!hash.contains("tset"));
    assert!(!hash.contains("abc"));
    assert!(!hash.contains("abce"));
    assert!(!hash.contains("abcde"));
}

/// This test does not compile. Fix it by moving a single line.
/// Explain in comments why this change is required.
/// 
/// `hash` borrows the contents of `x`, and `contains` is the last use of that borrow.
/// Moving the assertion before `push_str` lets the shared borrow end first.
/// `push_str` can then borrow `x` mutably without overlapping the shared borrow.
#[test]
fn hash_works_with_lifetimes() {
    let mut hash : Hash<&str> = Hash::new();

    let mut x = String::from("abc");
    x.push_str("123");
    
    hash.add(&x);
    
    assert!(hash.contains("abc123"));
    
    x.push_str("456");

    assert_eq!(x, "abc123456");
}
