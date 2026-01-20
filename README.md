---
title: Rust Zipper crate for Lists/Sequenceable Data
---

![docs.rs](https://img.shields.io/docsrs/list-zipper)

Zippers are data structures representing partial iteration through some
structure. they capture a hole in the data structure -- the focused
element -- along with the required context to move through the
structure. this crate provides a Zipper over sequenceable data, turning
an \`Iterator\` over the data into a \`Zipper\` that allows moving
forwards and backwards through the data.

additionally, the provided zipper turns the initial sequence into a
ring, with the last element of the initial sequence connecting to the
first element of that same sequence. this makes the data structure a
natural fit for navigation through UI elements, where iteration should
never abruptly end, and where context needs to be preserved. examples
include tabs in a web browser or windows in a tiling window manager.

usage example:

``` rust
use list_zipper::*;
use SequenceDirection::*;

let mut zipper = (0..10)
  .into_iter()
  .collect::<Zipper<_>>();

// focus the 5th element of the sequence
zipper.refocus(|i| *i == 5);

// print out the whole zipper to show the iteration order:
assert_eq!(zipper.to_string(), "[5, 6, 7, 8, 9, 0, 1, 2, 3, 4]");

// focus the 6th element
zipper.step_forwards();
assert_eq!(zipper.focus(), Some(&6));

// focus the 4th element
zipper.step_backwards().step_backwards();
assert_eq!(zipper.focus(), Some(&4));

// focus the 9th element
zipper
  .step_backwards()
  .step_backwards()
  .step_backwards()
  .step_backwards()
  .step_backwards();
assert_eq!(zipper.focus(), Some(&9));

// focus the 0th element
zipper.step_forwards();
assert_eq!(zipper.focus(), Some(&0));
```
