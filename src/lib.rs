//! Zippers are data structures representing partial iteration through some structure. they capture a
//! hole in the data structure -- the focused element -- along with the required context to move
//! through the structure. this crate provides a Zipper over sequenceable data, turning an `Iterator`
//! over the data into a `Zipper` that allows moving forwards and backwards through the data.
//!
//! additionally, the provided zipper turns the initial sequence into a ring, with the last element of
//! the initial sequence connecting to the first element of that same sequence. this makes the data
//! structure a natural fit for navigation through UI elements, where iteration should never abruptly
//! end, and where context needs to be preserved. examples include tabs in a web browser or windows
//! in a tiling window manager.
//!
//! usage example:
//!
//! ```rust
//! use list_zipper::*;
//! use SequenceDirection::*;
//!
//! let mut zipper = (0..10)
//!   .into_iter()
//!   .collect::<Zipper<_>>();
//!   
//! // focus the 5th element of the sequence
//! zipper.refocus(|i| *i == 5);
//!
//! // print out the whole zipper to show the iteration order:
//! assert_eq!(zipper.to_string(), "[5, 6, 7, 8, 9, 0, 1, 2, 3, 4]");
//!
//! // focus the 6th element
//! zipper.step_forwards();
//! assert_eq!(zipper.focus(), Some(&6));
//!
//! // focus the 4th element
//! zipper.step_backwards().step_backwards();
//! assert_eq!(zipper.focus(), Some(&4));
//!
//! // focus the 9th element
//! zipper
//!   .step_backwards()
//!   .step_backwards()
//!   .step_backwards()
//!   .step_backwards()
//!   .step_backwards();
//! assert_eq!(zipper.focus(), Some(&9));
//!
//! // focus the 0th element
//! zipper.step_forwards();
//! assert_eq!(zipper.focus(), Some(&0));
//! ```

use std::{collections::VecDeque, fmt::Display};

use itertools::Itertools;

/// the classic functional data structure. it's like an iterator except that it needs to allocate.
/// opening a `Zipper` over a sequence allows moving both forwards and backwards through it, arbitrarily.
/// a `Zipper` can be opened over any sequenceable data -- e.g. a traversal over a tree works
/// just as well as a list.
///
/// `Iterator`s are usually more efficient because they can avoid allocating buffers to store references
/// to the elements of the sequence -- in the worst case, the `Zipper` is allocating sufficient space for
/// twice the length of the sequence. however, `Zipper`s are substantially more flexible, allowing iteration
/// in both directions and via aribitrary paths through the sequence.
///
/// this particular `Zipper` has been turned into a ring. when the last element of the sequence is reached,
/// the next element is the first element of the sequence (and vice versa when iterating in reverse).
///
/// generally, you construct a `Zipper` by calling `FromIterator::collect` on an `Iterator`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Zipper<T> {
    /// a stack for elements occurring later in the sequence.
    /// the first element of this stack is the one currently focused.
    pub(crate) forward: VecDeque<T>,
    /// a stack for elements occurring earlier in the sequence
    pub(crate) backward: VecDeque<T>,
}

impl<T> Display for Zipper<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.iter().join(", "))
    }
}

/// the direction to move in, relative to the original order of the elements in the source sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceDirection {
    Original,
    Reverse,
}

fn push_and_yield<T>(n: &mut VecDeque<T>, t: T) -> &mut VecDeque<T> {
    n.push_front(t);
    n
}

fn push_back_and_yield<T>(n: &mut VecDeque<T>, t: T) -> &mut VecDeque<T> {
    n.push_back(t);
    n
}

fn reset<T>(nl: &mut VecDeque<T>, pl: &mut VecDeque<T>) {
    pl.drain(..).fold(nl, push_and_yield);
}

fn pop_push<T>(pop: &mut VecDeque<T>, push: &mut VecDeque<T>) {
    push.push_front(pop.pop_front().unwrap())
}

impl<T> Zipper<T> {
    /// construct an empty `Zipper`
    pub fn new() -> Self {
        Self {
            forward: VecDeque::new(),
            backward: VecDeque::new(),
        }
    }

    /// move the focus to the next element in the sequence, in the provided direction. this
    /// function rotates back to the start of the sequence when `step` is called on
    /// the last element of the sequence.
    pub fn step(&mut self, dir: SequenceDirection) -> &mut Self {
        if self.size() == 0 {
            return self;
        }

        match dir {
            SequenceDirection::Original => self.advance_focus(dir).rotate_stacks(dir),
            SequenceDirection::Reverse => self.rotate_stacks(dir).advance_focus(dir),
        }
    }

    /// advance the `Zipper` one step in the `Original` direction of the sequence.
    pub fn step_forwards(&mut self) -> &mut Self {
        self.step(SequenceDirection::Original)
    }

    /// advance the `Zipper` one step in `Reverse` of the original sequence direction.
    pub fn step_backwards(&mut self) -> &mut Self {
        self.step(SequenceDirection::Reverse)
    }

    /// get the number of elements in the `Zipper`
    pub fn size(&self) -> usize {
        self.forward.len() + self.backward.len()
    }

    /// skip ahead in the sequence until we reach the first element that satisfies the provided predicate.
    /// because `Zipper::step` circularizes the `Zipper`, we will eventually find the requested element.
    /// this moves the `Zipper`'s focus to the requested element.
    pub fn refocus(&mut self, mut p: impl FnMut(&T) -> bool) -> &mut Self {
        let mut counter = 0;
        while let Some(t) = self.step_forwards().focus()
            && !p(t)
            && counter < self.size()
        {
            counter += 1;
        }
        self
    }

    /// skip back in the sequence until we reach the first element that satisfies the provided predicate.
    /// because `Zipper::step` circularizes the `Zipper`, we will eventually find the requested element.
    /// this moves the `Zipper`'s focus to the requested element. while this function works in the reverse
    /// direction, it should yield the exact same zipper as `Zipper::refocus`.
    pub fn refocus_backwards(&mut self, mut p: impl FnMut(&T) -> bool) -> &mut Self {
        let mut counter = 0;
        while let Some(t) = self.step_backwards().focus()
            && !p(t)
            && counter < self.size()
        {
            counter += 1;
        }
        self
    }

    /// reset the focused element to the start of the original sequence
    pub fn reset_start(&mut self) -> &mut Self {
        reset(&mut self.forward, &mut self.backward);
        self
    }

    /// reset the focused element to the end of the original sequence. note that the caller needs to advance the zipper
    /// one step in reverse to actually maintain the zipper invariant, that an element is always focused.
    fn unsafe_reset_end(&mut self) -> &mut Self {
        reset(&mut self.backward, &mut self.forward);
        self
    }

    /// reset the focused element to the end of the original sequence.
    pub fn reset_end(&mut self) -> &mut Self {
        self.unsafe_reset_end().step(SequenceDirection::Reverse)
    }

    /// take one step in the requested direction. this pops an element from the stack matching the direction of motion
    /// and pushes it onto the reverse stacks.
    fn advance_focus(&mut self, dir: SequenceDirection) -> &mut Self {
        match dir {
            SequenceDirection::Original => pop_push(&mut self.forward, &mut self.backward),
            SequenceDirection::Reverse => pop_push(&mut self.backward, &mut self.forward),
        };

        self
    }

    /// rotate the stack, counter to the direction of motion, into the stack matching the direction of motion, if necessary.
    /// this rotation is only required when the stack matching the direction of motion has run out of elements. we thus
    /// circularize the `Zipper`, ensuring that we always have a next element in the appropriate direction, so long as the
    /// `Zipper` itself is not empty.
    fn rotate_stacks(&mut self, dir: SequenceDirection) -> &mut Self {
        match dir {
            SequenceDirection::Original if self.forward.is_empty() => self.reset_start(),
            SequenceDirection::Reverse if self.backward.is_empty() => self.unsafe_reset_end(),
            _ => self,
        }
    }

    /// retrieve the element focused by the `Zipper`
    pub fn focus(&self) -> Option<&T> {
        self.forward.front()
    }

    /// take the next element focused by the `Zipper`
    pub fn take_current_focus(&mut self) -> Option<T> {
        let next = self.forward.pop_front();

        if self.forward.is_empty() {
            reset(&mut self.forward, &mut self.backward);
        }

        next
    }

    /// take the previous element focused by the `Zipper`
    pub fn take_previous_focus(&mut self) -> Option<T> {
        if self.backward.is_empty() {
            reset(&mut self.backward, &mut self.forward);
        }
        pop_push(&mut self.backward, &mut self.forward);
        self.forward.pop_front()
    }

    /// insert an element at the current position in the sequence. the current focus becomes the element following the
    /// inserted element.
    pub fn push_focus(&mut self, elem: T) -> &mut Self {
        self.forward.push_front(elem);
        self
    }

    /// yield an `Iterator` that iterates in the order imposed by the original sequence but starting at the currently
    /// focused element. the element following the last element of the original sequence is the first element of the
    /// sequence.
    pub fn iter(&'_ self) -> ZipperIter<'_, T> {
        ZipperIter {
            zipper: self,
            count: self.size(),
            cursor: 0,
            dir: SequenceDirection::Original,
        }
    }

    /// yield an `Iterator` that iterates in reverse over the original sequence but starting at the currently
    /// focused element. the element following the first element of the original sequence is the last element of the
    /// sequence.
    pub fn reverse_iter(&'_ self) -> ZipperIter<'_, T> {
        ZipperIter {
            zipper: self,
            count: self.size(),
            cursor: 0,
            dir: SequenceDirection::Reverse,
        }
    }

    /// get the ith element. positive numbers correspond to the original sequence direction and negative numbers
    /// correspond to the reverse direction.
    ///
    /// `zipper.ith(0)` is the same as `zipper.focus()`
    pub fn ith(&self, i: isize) -> Option<&T> {
        let count = self.size() as isize;
        let i = (if i < 0 { i + count } else { i }).rem_euclid(count) as usize;

        let fw_len = self.forward.len();
        let bw_len = self.backward.len();
        if i < fw_len {
            self.forward.get(i)
        } else {
            self.backward.get(bw_len - (i - fw_len + 1))
        }
    }
}

impl<T> FromIterator<T> for Zipper<T> {
    fn from_iter<U: IntoIterator<Item = T>>(iter: U) -> Self {
        let mut s = Self::new();
        iter.into_iter().fold(&mut s.forward, push_back_and_yield);
        s
    }
}

/// an `Iterator` that yields the elements of the sequence the `Zipper` was opened over, starting with the currently
/// focused element and continuing until all elements have been yielded. sequence ordering is preserved.
pub struct ZipperIter<'a, T> {
    /// sequence state
    zipper: &'a Zipper<T>,
    /// number of elements in the sequence
    count: usize,
    /// keep track of which items in the sequence we've already yielded -- otherwise we'll spin indefinitely.
    cursor: isize,
    /// this iterator can go forwards or backwards
    dir: SequenceDirection,
}

impl<'a, T> Iterator for ZipperIter<'a, T>
where
    T: 'a,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor <= -1 * self.count as isize || self.cursor >= self.count as isize {
            return None;
        }

        let i = self.cursor;
        self.cursor += match &self.dir {
            SequenceDirection::Original => 1,
            SequenceDirection::Reverse => -1,
        };

        self.zipper.ith(i)
    }
}

pub struct ZipperIntoIter<T> {
    zipper: Zipper<T>,
}

impl<T> Iterator for ZipperIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.zipper.take_current_focus()
    }
}

impl<T> IntoIterator for Zipper<T> {
    type Item = T;

    type IntoIter = ZipperIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        ZipperIntoIter { zipper: self }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Rem;

    use super::*;

    #[test]
    fn reset_start_and_end_should_reset() {
        let mut zipper = (0..10).into_iter().collect::<Zipper<_>>();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "a newly collected zipper should focus the first element"
        );

        zipper.reset_end();
        assert_eq!(
            zipper.focus().copied(),
            Some(9),
            "resetting a zipper to the end should focus the last element"
        );

        zipper.reset_end();
        assert_eq!(
            zipper.focus().copied(),
            Some(9),
            "resetting to the end when focusing the end should be idempotent"
        );

        zipper.reset_start();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "resetting a zipper to the start should focus the first element"
        );

        zipper.reset_start();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "resetting to the start when focusing the start should be idempotent"
        );
    }

    #[test]
    fn step_forward_moves_focus_forward() {
        let mut zipper = (0..10).into_iter().collect::<Zipper<_>>();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "a newly collected zipper should focus the first element"
        );

        zipper.step(SequenceDirection::Original);
        assert_eq!(
            zipper.focus().copied(),
            Some(1),
            "stepping forward should advance to the second element"
        );

        zipper.reset_end().step(SequenceDirection::Original);
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "moving to the end and advancing should circle back to the start"
        );
    }

    #[test]
    fn step_backward_moves_focus_backward() {
        let mut zipper = (0..10).into_iter().collect::<Zipper<_>>();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "a newly collected zipper should focus the first element"
        );

        zipper.step(SequenceDirection::Reverse);
        assert_eq!(
            zipper.focus().copied(),
            Some(9),
            "stepping backward from the first element should advance to the last element"
        );

        zipper.reset_end().step(SequenceDirection::Reverse);
        assert_eq!(
            zipper.focus().copied(),
            Some(8),
            "moving to the end and stepping back should yield the second-to-last element"
        );
    }

    #[test]
    fn refocus_should_focus_the_first_matching_element() {
        let mut zipper = (0..10).into_iter().collect::<Zipper<_>>();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "a newly collected zipper should focus the first element"
        );

        zipper.refocus(|t| *t == 5);
        assert_eq!(
            zipper.focus().copied(),
            Some(5),
            "refocus should focus the selected element"
        );

        zipper.refocus(|t| t.rem(3) == 0);
        assert_eq!(
            zipper.focus().copied(),
            Some(6),
            "refocus should focus the first element satisfying the predicate"
        );
    }

    #[test]
    fn iterator_yields_all_elements_starting_from_focus() {
        let mut zipper = (0..10).into_iter().collect::<Zipper<_>>();
        zipper.refocus(|t| *t == 5);
        assert_eq!(
            zipper.focus().copied(),
            Some(5),
            "refocus should focus the selected element"
        );

        let v = zipper.iter().copied().collect::<Vec<_>>();
        assert_eq!(
            &v,
            &[5, 6, 7, 8, 9, 0, 1, 2, 3, 4],
            "iterator should produce all elements in order, starting from the focus"
        );

        let v = zipper.reverse_iter().copied().collect::<Vec<_>>();
        assert_eq!(
            &v,
            &[5, 4, 3, 2, 1, 0, 9, 8, 7, 6],
            "reverse iterator should produce all elements in reverse order, starting from the focus"
        );

        let s = zipper.to_string();
        assert_eq!(
            &s, "[5, 6, 7, 8, 9, 0, 1, 2, 3, 4]",
            "pretty printing the zipper should work"
        );
    }

    #[test]
    fn refocus_forward_and_backwards_is_equivalent() {
        let mut zipper1 = (0..10).into_iter().collect::<Zipper<_>>();
        zipper1.refocus(|t| *t == 5);
        assert_eq!(
            zipper1.focus().copied(),
            Some(5),
            "refocus should focus the selected element"
        );

        let mut zipper2 = (0..10).into_iter().collect::<Zipper<_>>();
        zipper2.refocus_backwards(|t| *t == 5);
        assert_eq!(
            zipper2.focus().copied(),
            Some(5),
            "refocus should focus the selected element"
        );

        assert_eq!(
            zipper1, zipper2,
            "refocusing forwards and backwards should yield the same zipper."
        );
    }
}
