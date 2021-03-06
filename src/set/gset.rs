use std::cmp::Ordering::{self, Equal, Greater, Less};
use std::collections::HashSet;
use std::hash::Hash;

#[cfg(any(quickcheck, test))]
use quickcheck::{Arbitrary, Gen};

use Crdt;

/// A grow-only set.
#[derive(Debug, Default)]
pub struct GSet<T>
where
    T: Eq + Hash,
{
    elements: HashSet<T>,
}

/// An insert operation over `GSet` CRDTs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GSetOp<T> {
    element: T,
}

impl<T> GSet<T>
where
    T: Clone + Eq + Hash,
{
    /// Create a new grow-only set.
    ///
    /// ### Example
    ///
    /// ```
    /// use crdt::set::GSet;
    ///
    /// let mut set = GSet::<i32>::new();
    /// assert!(set.is_empty());
    /// ```
    pub fn new() -> GSet<T> {
        GSet {
            elements: HashSet::new(),
        }
    }

    /// Insert an element into a grow-only set.
    ///
    /// ### Example
    ///
    /// ```
    /// use crdt::set::GSet;
    ///
    /// let mut set = GSet::new();
    /// set.insert("first-element");
    /// assert!(set.contains(&"first-element"));
    /// ```
    pub fn insert(&mut self, element: T) -> Option<GSetOp<T>> {
        if self.elements.insert(element.clone()) {
            Some(GSetOp { element: element })
        } else {
            None
        }
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Returns true if the set contains the value.
    pub fn contains(&self, value: &T) -> bool {
        self.elements.contains(value)
    }

    /// Returns true if the set contains no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn is_subset(&self, other: &GSet<T>) -> bool {
        self.elements.is_subset(&other.elements)
    }

    pub fn is_disjoint(&self, other: &GSet<T>) -> bool {
        self.elements.is_disjoint(&other.elements)
    }
}

impl<T> Crdt for GSet<T>
where
    T: Clone + Eq + Hash,
{
    type Operation = GSetOp<T>;

    /// Merge a replica into the set.
    ///
    /// This method is used to perform state-based replication.
    ///
    /// ##### Example
    ///
    /// ```
    /// # use crdt::set::GSet;
    /// use crdt::Crdt;
    ///
    /// let mut local = GSet::new();
    /// let mut remote = GSet::new();
    ///
    /// local.insert(1i32);
    /// remote.insert(2);
    ///
    /// local.merge(remote);
    /// assert!(local.contains(&2));
    /// ```
    fn merge(&mut self, other: GSet<T>) {
        self.elements.extend(other.elements.into_iter());
    }

    /// Apply an insert operation to the set.
    ///
    /// This method is used to perform operation-based replication.
    ///
    /// Applying an operation to a `GSet` is idempotent.
    ///
    /// ##### Example
    ///
    /// ```
    /// # use crdt::set::GSet;
    /// # use crdt::Crdt;
    /// let mut local = GSet::new();
    /// let mut remote = GSet::new();
    ///
    /// let op = remote.insert(13i32).expect("GSet should be empty.");
    ///
    /// local.apply(op);
    /// assert!(local.contains(&13));
    /// ```
    fn apply(&mut self, op: GSetOp<T>) {
        self.insert(op.element);
    }
}

impl<T> PartialEq for GSet<T>
where
    T: Eq + Hash,
{
    fn eq(&self, other: &GSet<T>) -> bool {
        self.elements == other.elements
    }
}

impl<T> Eq for GSet<T>
where
    T: Eq + Hash,
{
}

impl<T> PartialOrd for GSet<T>
where
    T: Eq + Hash,
{
    fn partial_cmp(&self, other: &GSet<T>) -> Option<Ordering> {
        if self.elements == other.elements {
            Some(Equal)
        } else if self.elements.is_subset(&other.elements) {
            Some(Less)
        } else if self.elements.is_superset(&other.elements) {
            Some(Greater)
        } else {
            None
        }
    }
}

impl<T> Clone for GSet<T>
where
    T: Clone + Eq + Hash,
{
    fn clone(&self) -> GSet<T> {
        GSet {
            elements: self.elements.clone(),
        }
    }
}

#[cfg(any(quickcheck, test))]
impl<T> Arbitrary for GSet<T>
where
    T: Arbitrary + Clone + Eq + Hash,
{
    fn arbitrary<G>(g: &mut G) -> GSet<T>
    where
        G: Gen,
    {
        let elements: Vec<T> = Arbitrary::arbitrary(g);
        GSet {
            elements: elements.into_iter().collect(),
        }
    }
    fn shrink(&self) -> Box<Iterator<Item = GSet<T>> + 'static> {
        let elements: Vec<T> = self.elements.iter().cloned().collect();
        Box::new(elements.shrink().map(|es| {
            GSet {
                elements: es.into_iter().collect(),
            }
        }))
    }
}

#[cfg(any(quickcheck, test))]
impl<T> Arbitrary for GSetOp<T>
where
    T: Arbitrary,
{
    fn arbitrary<G: Gen>(g: &mut G) -> GSetOp<T> {
        GSetOp {
            element: Arbitrary::arbitrary(g),
        }
    }
    fn shrink(&self) -> Box<Iterator<Item = GSetOp<T>> + 'static> {
        Box::new(self.element.shrink().map(|e| GSetOp { element: e }))
    }
}

#[cfg(test)]
mod test {

    use quickcheck::quickcheck;

    use {test, Crdt};
    use super::{GSet, GSetOp};

    type C = GSet<u32>;
    type O = GSetOp<u32>;

    #[test]
    fn check_apply_is_commutative() {
        quickcheck(test::apply_is_commutative::<C> as fn(C, Vec<O>) -> bool);
    }

    #[test]
    fn check_merge_is_commutative() {
        quickcheck(test::merge_is_commutative::<C> as fn(C, Vec<C>) -> bool);
    }

    #[test]
    fn check_ordering_lte() {
        quickcheck(test::ordering_lte::<C> as fn(C, C) -> bool);
    }

    #[test]
    fn check_ordering_equality() {
        quickcheck(test::ordering_equality::<C> as fn(C, C) -> bool);
    }

    #[test]
    fn test_local_insert() {
        fn check_local_insert(elements: Vec<u8>) -> bool {
            let mut set = GSet::new();
            for element in elements.clone().into_iter() {
                set.insert(element);
            }

            elements.iter().all(|element| set.contains(element))
        }
        quickcheck(check_local_insert as fn(Vec<u8>) -> bool);
    }

    #[test]
    fn test_ordering_lt() {
        fn check_ordering_lt(mut a: GSet<u8>, b: GSet<u8>) -> bool {
            a.merge(b.clone());

            let mut i = 0;
            let mut success = None;
            while success.is_none() {
                success = a.insert(i);
                i += 1;
            }
            a > b && b < a
        }
        quickcheck(check_ordering_lt as fn(GSet<u8>, GSet<u8>) -> bool);
    }
}
