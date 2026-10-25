#![deny(unsafe_code)]

pub mod config;
pub mod eliminator;
pub mod inspect;
pub mod metrics;
pub mod reachability;
pub mod roots;

pub use config::{DceConfig, OutputType};
pub use eliminator::DeadCodeEliminator;
pub use inspect::{dump_attributes, generate_attributes_json};
pub use metrics::DceMetrics;
pub use reachability::ReachabilityAnalysis;
pub use roots::{discover_roots, prune_tests};
