//! Run the demo:  `cargo run --bin objective1`
//! Run the tests: `cargo test --bin objective1`

use std::cmp::Ordering;

enum BinarySearchTree {
    /// An empty (sub)tree.
    Empty,
    /// A non-empty (sub)tree.
    Node {
        /// The string stored in this node.
        value: String,
        /// Subtree with all values that are *smaller* than `value`.
        /// `Box` puts the child on the heap. Without it the type would contain
        /// itself and have an infinite size, which Rust does not allow.
        left: Box<BinarySearchTree>,
        /// Subtree with all values that are *greater than or equal to* `value`.
        right: Box<BinarySearchTree>,
    },
}

impl BinarySearchTree {
    /// Creates a new, empty binary search tree.
    fn new() -> Self {
        BinarySearchTree::Empty
    }

    /// Adds `value` at the correct position in the tree.
    ///
    /// Same rule as the Python version: smaller values go left, everything
    /// else (greater *or equal*) goes right, so duplicates are kept.
    ///
    /// Takes `&mut self` because it changes the tree, and takes the `String`
    /// by value because the tree becomes the owner of it.
    fn add(&mut self, value: String) {
        match self {
            // We reached an empty spot: replace this empty (sub)tree with a
            // new node that has two empty children.
            BinarySearchTree::Empty => {
                *self = BinarySearchTree::Node {
                    value,
                    left: Box::new(BinarySearchTree::Empty),
                    right: Box::new(BinarySearchTree::Empty),
                };
            }
            // Not empty: let the correct subtree handle it (recursion).
            BinarySearchTree::Node {
                value: node_value,
                left,
                right,
            } => {
                if value < *node_value {
                    left.add(value);
                } else {
                    right.add(value);
                }
            }
        }
    }

    /// Returns `true` if `value` is stored in the tree, `false` otherwise.
    ///
    /// Takes a `&str` (a borrowed string) so you can pass both a `&String`
    /// and a literal like `"apple"`, without giving away ownership.
    fn contains(&self, value: &str) -> bool {
        match self {
            // Reached an empty (sub)tree: the value is not in the tree.
            BinarySearchTree::Empty => false,
            BinarySearchTree::Node {
                value: node_value,
                left,
                right,
            } => match value.cmp(node_value.as_str()) {
                // Smaller: it can only be in the left subtree.
                Ordering::Less => left.contains(value),
                // Larger: it can only be in the right subtree.
                Ordering::Greater => right.contains(value),
                // Equal: found it.
                Ordering::Equal => true,
            },
        }
    }

    /// Returns all values in the tree as a sorted `Vec<String>`.
    ///
    /// In-order traversal: first everything on the left (smaller values),
    /// then the value of this node, then everything on the right (larger
    /// values). That order is exactly the sorted order.
    fn as_vector(&self) -> Vec<String> {
        match self {
            BinarySearchTree::Empty => Vec::new(),
            BinarySearchTree::Node { value, left, right } => {
                let mut result = left.as_vector();
                // `clone` because the tree keeps its own copy of the value.
                result.push(value.clone());
                result.extend(right.as_vector());
                result
            }
        }
    }
}

/// Small demo of all four methods.
fn main() {
    let mut tree = BinarySearchTree::new();

    for fruit in ["orange", "apple", "pear", "banana", "Mango", "kiwi"] {
        tree.add(fruit.to_string());
    }

    // "Mango" comes first: uppercase letters come before lowercase in ASCII.
    println!("Sorted: {:?}", tree.as_vector());
    println!("Contains \"pear\"?  {}", tree.contains("pear"));
    println!("Contains \"grape\"? {}", tree.contains("grape"));
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- helpers ----------------------------------------------------------

    /// Builds a tree by adding `values` in the given order.
    fn tree_with(values: &[&str]) -> BinarySearchTree {
        let mut tree = BinarySearchTree::new();
        for value in values {
            tree.add(value.to_string());
        }
        tree
    }

    /// Splits a node into (value, left, right). Panics if the tree is empty.
    fn unpack(tree: &BinarySearchTree) -> (&str, &BinarySearchTree, &BinarySearchTree) {
        match tree {
            BinarySearchTree::Node { value, left, right } => (value, left, right),
            BinarySearchTree::Empty => panic!("expected a Node, found Empty"),
        }
    }

    fn is_empty(tree: &BinarySearchTree) -> bool {
        matches!(tree, BinarySearchTree::Empty)
    }

    // ----- new --------------------------------------------------------------

    #[test]
    fn new_creates_an_empty_tree() {
        assert!(is_empty(&BinarySearchTree::new()));
    }

    // ----- add --------------------------------------------------------------

    #[test]
    fn add_to_empty_tree_creates_a_node_with_two_empty_children() {
        let tree = tree_with(&["m"]);
        let (value, left, right) = unpack(&tree);
        assert_eq!(value, "m");
        assert!(is_empty(left));
        assert!(is_empty(right));
    }

    #[test]
    fn add_puts_a_smaller_value_in_the_left_subtree() {
        let tree = tree_with(&["m", "a"]);
        let (_, left, right) = unpack(&tree);
        assert_eq!(unpack(left).0, "a");
        assert!(is_empty(right));
    }

    #[test]
    fn add_puts_a_larger_value_in_the_right_subtree() {
        let tree = tree_with(&["m", "z"]);
        let (_, left, right) = unpack(&tree);
        assert!(is_empty(left));
        assert_eq!(unpack(right).0, "z");
    }

    #[test]
    fn add_puts_an_equal_value_in_the_right_subtree() {
        let tree = tree_with(&["m", "m"]);
        let (_, left, right) = unpack(&tree);
        assert!(is_empty(left));
        assert_eq!(unpack(right).0, "m");
    }

    // ----- contains ---------------------------------------------------------

    #[test]
    fn contains_returns_false_for_an_empty_tree() {
        assert!(!BinarySearchTree::new().contains("apple"));
    }

    #[test]
    fn contains_finds_the_root_value() {
        assert!(tree_with(&["m"]).contains("m"));
    }

    #[test]
    fn contains_searches_the_left_subtree_for_smaller_values() {
        let tree = tree_with(&["m", "f"]);
        assert!(tree.contains("f"));
        assert!(!tree.contains("a"));
    }

    #[test]
    fn contains_searches_the_right_subtree_for_larger_values() {
        let tree = tree_with(&["m", "t"]);
        assert!(tree.contains("t"));
        assert!(!tree.contains("z"));
    }

    // ----- as_vector --------------------------------------------------------

    #[test]
    fn as_vector_of_an_empty_tree_is_empty() {
        assert!(BinarySearchTree::new().as_vector().is_empty());
    }

    #[test]
    fn as_vector_returns_the_values_sorted() {
        let tree = tree_with(&["orange", "apple", "pear", "banana", "kiwi"]);
        assert_eq!(
            tree.as_vector(),
            vec!["apple", "banana", "kiwi", "orange", "pear"]
        );
    }

    #[test]
    fn as_vector_uses_asciibetical_order() {
        // 'A'..'Z' (65..90) come before 'a'..'z' (97..122) in ASCII.
        let tree = tree_with(&["apple", "Banana", "cherry", "Apple"]);
        assert_eq!(tree.as_vector(), vec!["Apple", "Banana", "apple", "cherry"]);
    }

    #[test]
    fn as_vector_keeps_duplicates() {
        let tree = tree_with(&["b", "a", "b"]);
        assert_eq!(tree.as_vector(), vec!["a", "b", "b"]);
    }
}