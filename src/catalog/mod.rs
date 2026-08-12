pub mod budget;
pub mod dependency;
pub mod inventory;
pub mod level_a;
pub mod level_b;
pub mod manifest;
pub mod pipeline;
pub mod projection;
pub mod protection;
pub mod sources;

#[cfg(test)]
#[path = "../../tests/catalog/inventory.rs"]
mod inventory_tests;

#[cfg(test)]
#[path = "../../tests/catalog/dependency.rs"]
mod dependency_tests;

#[cfg(test)]
#[path = "../../tests/catalog/budget.rs"]
mod budget_tests;

#[cfg(test)]
#[path = "../../tests/catalog/level_a.rs"]
mod level_a_tests;

#[cfg(test)]
#[path = "../../tests/catalog/level_b.rs"]
mod level_b_tests;

#[cfg(test)]
#[path = "../../tests/catalog/manifest.rs"]
mod manifest_tests;

#[cfg(test)]
#[path = "../../tests/catalog/sources.rs"]
mod sources_tests;

#[cfg(test)]
#[path = "../../tests/catalog/native_projection.rs"]
mod native_projection_tests;

#[cfg(test)]
#[path = "../../tests/catalog/pipeline.rs"]
mod pipeline_tests;
