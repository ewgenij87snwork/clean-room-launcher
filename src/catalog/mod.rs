pub mod inventory;
pub mod sources;

#[cfg(test)]
#[path = "../../tests/catalog/inventory.rs"]
mod inventory_tests;

#[cfg(test)]
#[path = "../../tests/catalog/sources.rs"]
mod sources_tests;
