pub mod inventory;
pub mod level_a;
pub mod sources;

#[cfg(test)]
#[path = "../../tests/catalog/inventory.rs"]
mod inventory_tests;

#[cfg(test)]
#[path = "../../tests/catalog/level_a.rs"]
mod level_a_tests;

#[cfg(test)]
#[path = "../../tests/catalog/sources.rs"]
mod sources_tests;
