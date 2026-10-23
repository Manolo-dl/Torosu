//! Pattern analysis of mania maps, per playstyle: what the patterns of a map
//! are, for the whole map and for each hand, and (in `corpus`) what the maps
//! of each key count look like.

pub mod layout;
pub mod patterns;
pub mod style;
pub mod zones;

#[cfg(test)]
mod corpus;
