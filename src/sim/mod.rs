//! Continuous 2D artificial chemistry (Hutton-style).
//!
//! * [`rules`]    – text chemistry: rules with wildcards, spring table, parameters
//! * [`reactor`]  – continuous reactor: soft spheres, springs, reactions
//! * [`molecule`] – connected components, extraction / insertion, molecule files

pub mod molecule;
pub mod reactor;
pub mod rules;
pub mod setup;
pub mod vec2;

pub use molecule::Molecule;
pub use reactor::{Atom, Params, Reactor, Stats};
pub use rules::{Chemistry, Compound, Rule, TypePat};
pub use vec2::Vec2;
