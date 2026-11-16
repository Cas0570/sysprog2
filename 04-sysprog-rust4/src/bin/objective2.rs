//! Run the demo:  `cargo run --bin objective2`
//! Run the tests: `cargo test --bin objective2`

use std::fmt;

/// Everything in the catalogue: it has a current price and a description.
///
/// The tree only uses these two methods, so it does not need to know whether
/// an item is a painting or a car.
trait Valuable {
    /// The current price in euros.
    fn current_price(&self) -> f64;

    /// A human-readable description of the item.
    fn description(&self) -> String;
}

/// Makes `to_string()` (and `{}` in `println!`) work on every `Valuable`.
///
/// `ToString` is automatically implemented for every type that implements
/// `Display`, so implementing `Display` once, for the trait object
/// `dyn Valuable`, gives all valuables the same "description - price" format.
impl fmt::Display for dyn Valuable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - €{:.2}", self.description(), self.current_price())
    }
}

/// A painting has an artist, a title and a current price.
struct Painting {
    artist: String,
    title: String,
    current_price: f64,
}

impl Painting {
    fn new(artist: &str, title: &str, current_price: f64) -> Self {
        Painting {
            artist: artist.to_string(),
            title: title.to_string(),
            current_price,
        }
    }
}

impl Valuable for Painting {
    fn current_price(&self) -> f64 {
        self.current_price
    }

    fn description(&self) -> String {
        format!("Painting: \"{}\" by {}", self.title, self.artist)
    }
}

/// A sports car has a brand, a model, an original price and an age in years.
struct SportsCar {
    brand: String,
    model: String,
    original_price: f64,
    age_in_years: u32,
}

impl SportsCar {
    fn new(brand: &str, model: &str, original_price: f64, age_in_years: u32) -> Self {
        SportsCar {
            brand: brand.to_string(),
            model: model.to_string(),
            original_price,
            age_in_years,
        }
    }
}

impl Valuable for SportsCar {
    /// `original_price * (0.95 ^ age_in_years)`: the car loses 5% of its
    /// value every year. `powf` needs an `f64` exponent; `f64::from` converts
    /// the `u32` age without any loss.
    fn current_price(&self) -> f64 {
        self.original_price * 0.95_f64.powf(f64::from(self.age_in_years))
    }

    fn description(&self) -> String {
        format!(
            "Sports car: {} {} (age {})",
            self.brand, self.model, self.age_in_years
        )
    }
}

// ---------------------------------------------------------------------------
// The binary search tree (copy of objective 1, now storing Valuables)
// ---------------------------------------------------------------------------

enum BinarySearchTree {
    /// An empty (sub)tree.
    Empty,
    /// A non-empty (sub)tree.
    Node {
        /// A trait object: "some type that implements `Valuable`".
        ///
        /// `Painting` and `SportsCar` have different sizes, so the compiler
        /// can't reserve a fixed amount of space for "a valuable" here. A
        /// `Box` stores the item on the heap and keeps a fixed-size pointer to
        /// it. Method calls are looked up at runtime (dynamic dispatch).
        value: Box<dyn Valuable>,
        /// Items that are *more expensive* than `value`.
        left: Box<BinarySearchTree>,
        /// Items that are *cheaper than or as expensive as* `value`.
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
    /// More expensive items go left and all others go right, so an in-order
    /// walk returns the most expensive item first. Items with the same price
    /// go right, so they stay in the order in which they were added.
    fn add(&mut self, value: Box<dyn Valuable>) {
        match self {
            BinarySearchTree::Empty => {
                *self = BinarySearchTree::Node {
                    value,
                    left: Box::new(BinarySearchTree::Empty),
                    right: Box::new(BinarySearchTree::Empty),
                };
            }
            BinarySearchTree::Node {
                value: node_value,
                left,
                right,
            } => {
                if value.current_price() > node_value.current_price() {
                    left.add(value);
                } else {
                    right.add(value);
                }
            }
        }
    }

    /// Returns `true` if an item with the same price and description is in
    /// the tree.
    ///
    /// Takes a `&dyn Valuable`, so you can pass a `&Painting` or a
    /// `&SportsCar`.
    fn contains(&self, value: &dyn Valuable) -> bool {
        match self {
            BinarySearchTree::Empty => false,
            BinarySearchTree::Node {
                value: node_value,
                left,
                right,
            } => {
                // `f64` has no `.cmp()` (a NaN can't be ordered), so instead of
                // matching on `Ordering` like in objective 1 we use `>` and `<`.
                if value.current_price() > node_value.current_price() {
                    // More expensive: it can only be in the left subtree.
                    left.contains(value)
                } else if value.current_price() < node_value.current_price() {
                    // Cheaper: it can only be in the right subtree.
                    right.contains(value)
                } else if value.description() == node_value.description() {
                    // Same price and same description: found it.
                    true
                } else {
                    // Same price but a different item: `add` puts equal prices
                    // on the right, so keep searching there.
                    right.contains(value)
                }
            }
        }
    }

    /// Returns the items as a `Vec<String>`, most expensive first.
    ///
    /// Same in-order walk as in objective 1, but now `to_string()` turns each
    /// valuable into "description - price" (see the `Display` impl above).
    fn as_vector(&self) -> Vec<String> {
        match self {
            BinarySearchTree::Empty => Vec::new(),
            BinarySearchTree::Node { value, left, right } => {
                let mut result = left.as_vector();
                result.push(value.to_string());
                result.extend(right.as_vector());
                result
            }
        }
    }
}

/// Fills the catalogue with a few cars and paintings (in random order) and
/// prints it, to show that the tree keeps them sorted by descending price.
fn main() {
    let porsche = SportsCar::new("Porsche", "911 GT3 RS", 230_000.0, 2);
    let water_lilies = Painting::new("Claude Monet", "Water Lilies", 1_500_000.0);
    let salvator_mundi = Painting::new("Leonardo da Vinci", "Salvator Mundi", 450_300_000.0);
    let laferrari = SportsCar::new("Ferrari", "LaFerrari", 1_400_000.0, 10);
    let girl_with_balloon = Painting::new("Banksy", "Girl with Balloon", 500_000.0);
    let chiron = SportsCar::new("Bugatti", "Chiron", 2_900_000.0, 5);

    // `Box::new` moves each item to the heap. `add` expects a
    // `Box<dyn Valuable>`; Rust converts a `Box<SportsCar>` or `Box<Painting>`
    // into one automatically. The order is mixed up on purpose: the tree sorts.
    let mut catalogue = BinarySearchTree::new();
    catalogue.add(Box::new(porsche));
    catalogue.add(Box::new(water_lilies));
    catalogue.add(Box::new(salvator_mundi));
    catalogue.add(Box::new(laferrari));
    catalogue.add(Box::new(girl_with_balloon));
    catalogue.add(Box::new(chiron));

    println!("Catalogue (most expensive first):");
    for (index, line) in catalogue.as_vector().iter().enumerate() {
        println!("{:>2}. {}", index + 1, line);
    }

    // `contains` looks for an item with the same price and description.
    let monet = Painting::new("Claude Monet", "Water Lilies", 1_500_000.0);
    let fiat = SportsCar::new("Fiat", "Panda", 12_000.0, 3);
    println!();
    println!("Contains the Monet? {}", catalogue.contains(&monet));
    println!("Contains the Fiat?  {}", catalogue.contains(&fiat));
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- helpers ----------------------------------------------------------

    fn painting(title: &str, price: f64) -> Box<dyn Valuable> {
        Box::new(Painting::new("Artist", title, price))
    }

    fn car(brand: &str, model: &str, original_price: f64, age: u32) -> Box<dyn Valuable> {
        Box::new(SportsCar::new(brand, model, original_price, age))
    }

    /// Builds a tree by adding `valuables` in the given order.
    fn tree_with(valuables: Vec<Box<dyn Valuable>>) -> BinarySearchTree {
        let mut tree = BinarySearchTree::new();
        for valuable in valuables {
            tree.add(valuable);
        }
        tree
    }

    /// Floating point maths is not exact, so compare with a small margin.
    fn assert_price(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    // ----- Painting, SportsCar and Display ------------------------------------

    #[test]
    fn painting_price_and_description() {
        let painting = Painting::new("Rembrandt", "The Night Watch", 1_000_000.0);
        assert_price(painting.current_price(), 1_000_000.0);
        assert_eq!(
            painting.description(),
            "Painting: \"The Night Watch\" by Rembrandt"
        );
    }

    #[test]
    fn sports_car_loses_5_percent_of_its_value_every_year() {
        let price_at_age = |age| car("Porsche", "911", 100_000.0, age).current_price();
        assert_price(price_at_age(0), 100_000.0);
        assert_price(price_at_age(1), 95_000.0);
        assert_price(price_at_age(2), 90_250.0);
    }

    #[test]
    fn sports_car_description() {
        assert_eq!(
            car("Ferrari", "F40", 400_000.0, 3).description(),
            "Sports car: Ferrari F40 (age 3)"
        );
    }

    #[test]
    fn to_string_shows_description_and_current_price() {
        assert_eq!(
            car("Ferrari", "F40", 400_000.0, 2).to_string(),
            "Sports car: Ferrari F40 (age 2) - €361000.00"
        );
    }

    // ----- new / add / as_vector --------------------------------------------

    #[test]
    fn new_creates_an_empty_tree() {
        let tree = BinarySearchTree::new();
        assert!(matches!(tree, BinarySearchTree::Empty));
        assert!(tree.as_vector().is_empty());
    }

    #[test]
    fn as_vector_lists_the_most_expensive_item_first() {
        let tree = tree_with(vec![
            car("Fiat", "Panda", 10_000.0, 1),   //   9,500
            painting("The Milkmaid", 500_000.0), // 500,000
            car("Porsche", "911", 100_000.0, 0), // 100,000
            painting("Sunflowers", 50_000.0),    //  50,000
        ]);
        assert_eq!(
            tree.as_vector(),
            vec![
                "Painting: \"The Milkmaid\" by Artist - €500000.00",
                "Sports car: Porsche 911 (age 0) - €100000.00",
                "Painting: \"Sunflowers\" by Artist - €50000.00",
                "Sports car: Fiat Panda (age 1) - €9500.00",
            ]
        );
    }

    #[test]
    fn items_with_the_same_price_keep_their_insertion_order() {
        let tree = tree_with(vec![
            painting("First", 100.0),
            painting("Second", 100.0),
            painting("Third", 100.0),
        ]);
        assert_eq!(
            tree.as_vector(),
            vec![
                "Painting: \"First\" by Artist - €100.00",
                "Painting: \"Second\" by Artist - €100.00",
                "Painting: \"Third\" by Artist - €100.00",
            ]
        );
    }

    // ----- contains ---------------------------------------------------------

    #[test]
    fn contains_returns_false_for_an_empty_tree() {
        assert!(!BinarySearchTree::new().contains(&Painting::new("A", "B", 1.0)));
    }

    #[test]
    fn contains_finds_an_item_with_the_same_price_and_description() {
        let tree = tree_with(vec![painting("Root", 100.0)]);
        assert!(tree.contains(&Painting::new("Artist", "Root", 100.0)));
    }

    #[test]
    fn contains_searches_the_left_subtree_for_more_expensive_items() {
        let tree = tree_with(vec![painting("Root", 100.0), painting("Pricey", 200.0)]);
        assert!(tree.contains(&Painting::new("Artist", "Pricey", 200.0)));
        assert!(!tree.contains(&Painting::new("Artist", "Missing", 300.0)));
    }

    #[test]
    fn contains_searches_the_right_subtree_for_cheaper_items() {
        let tree = tree_with(vec![painting("Root", 100.0), painting("Cheap", 50.0)]);
        assert!(tree.contains(&Painting::new("Artist", "Cheap", 50.0)));
        assert!(!tree.contains(&Painting::new("Artist", "Missing", 10.0)));
    }

    #[test]
    fn contains_keeps_searching_right_for_same_price_but_other_item() {
        let tree = tree_with(vec![painting("First", 100.0), painting("Second", 100.0)]);
        assert!(tree.contains(&Painting::new("Artist", "Second", 100.0)));
        assert!(!tree.contains(&Painting::new("Artist", "Third", 100.0)));
    }
}