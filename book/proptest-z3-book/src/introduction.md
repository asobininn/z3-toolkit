# Introduction

proptest-z3 extends [proptest](https://crates.io/crates/proptest) to generate test inputs whose elements have dependencies on one another. Define the input structure with proptest Strategies as usual, then express relationships between elements as constraints for [Z3](https://crates.io/crates/z3). By inverting constraints, you can generate inputs containing a specified number of violations and identify the locations of those violations. Relationships between elements are preserved during shrinking.

In this crate, the term **name** collectively refers to things used to identify an entity and refer to it elsewhere, such as variable names, table names, and row keys.

## Who is this for?

This crate is intended for people testing software whose input elements depend on one another, such as compilers and static analyzers.

For example, it can help generate inputs that preserve consistency between definitions and references, as well as inputs that deliberately reference undefined names.

## Prerequisites

This Book does not teach proptest or Z3 from scratch. The required background is:

- **proptest:** The basics of property-based testing, including `Strategy` and `proptest!`. See the [proptest Book](https://proptest-rs.github.io/proptest/).
- **Z3:** A general understanding of SMT solvers. You do not need to know the Z3 API. See [Programming Z3](https://z3prover.github.io/papers/programmingz3.html).
- **z3-toolkit:** A companion crate by the same author that provides macros for writing constraints. The parts needed here are covered in [Writing Constraints with z3-toolkit](./z3-toolkit.md).

## How to read this Book

Start with [Getting Started](./getting_started.md) to see a small working example. The [Feature Guide](./feature_guide/SUMMARY.md) explains each feature in detail, and [Common Patterns](./patterns.md) shows constraint patterns useful when testing name resolution.

The [API Reference](./api.md) documents public APIs and the semantics that are easy to overlook, especially evaluation order, steps, scopes, and labels.
