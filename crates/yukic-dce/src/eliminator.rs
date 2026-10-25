#![deny(unsafe_code)]

use std::collections::HashSet;
use yukic_ast::{Item, ItemKind, Module};

use crate::config::DceConfig;
use crate::inspect::dump_attributes;
use crate::metrics::DceMetrics;
use crate::reachability::ReachabilityAnalysis;
use crate::roots::{discover_roots, prune_tests};

pub struct DeadCodeEliminator {
    config: DceConfig,
}

impl DeadCodeEliminator {
    pub fn new(config: DceConfig) -> Self { Self { config } }

    pub fn with_strip_unused(strip_unused: bool) -> Self {
        Self::new(DceConfig::new().with_strip_unused(strip_unused))
    }

    pub fn config(&self) -> &DceConfig { &self.config }

    pub fn eliminate(&self, module: &mut Module) -> DceMetrics {
        let items_before = module.items.len();
        let tests_pruned = if self.config.is_test { 0 } 

        else { prune_tests(module) };

        if let Some(path) = &self.config.inspect_attributes {
            let _ = dump_attributes(module, path, &self.config.project_name);
        }

        if !self.config.strip_unused {
            return DceMetrics {
                items_before,
                items_after: module.items.len(),
                tests_pruned,
                ..Default::default()
            };
        }

        let roots = discover_roots(module, &self.config);
        let reachability = ReachabilityAnalysis::new(module);
        let live_ids = reachability.compute_live_items(&roots);

        let mut metrics = compute_removal_counts(&module.items, &live_ids);
        metrics.items_before = items_before;
        metrics.tests_pruned = tests_pruned;

        module.items.retain(|item| live_ids.contains(&item.id));
        metrics.items_after = module.items.len();

        if self.config.verbose {
            eprintln!("{}", metrics.format_report());
        }

        metrics
    }
}

fn compute_removal_counts(items: &[Item], live_ids: &HashSet<yukic_ast::ItemId>) -> DceMetrics {
    let mut metrics = DceMetrics::default();

    for item in items {
        if live_ids.contains(&item.id) { continue; }

        match &item.kind {
            ItemKind::Fun(_) | ItemKind::Cfun(_) => metrics.functions_removed += 1,
            ItemKind::Class(_) | ItemKind::Interface(_) => metrics.classes_removed += 1,
            ItemKind::Struct(_) | ItemKind::Cstruct(_) => metrics.structs_removed += 1,
            ItemKind::Static(_) => metrics.statics_removed += 1,
            ItemKind::Enum(_) | ItemKind::TypeAlias(_) => {
                metrics.types_removed += 1;
            }
            _ => {}
        }
    }

    metrics
}
