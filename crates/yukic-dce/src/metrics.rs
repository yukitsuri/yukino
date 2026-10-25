#![deny(unsafe_code)]

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DceMetrics {
    pub items_before: usize,
    pub items_after: usize,
    pub functions_removed: usize,
    pub classes_removed: usize,
    pub structs_removed: usize,
    pub statics_removed: usize,
    pub types_removed: usize,
    pub tests_pruned: usize,
}

impl DceMetrics {
    pub fn total_removed(&self) -> usize {
        self.functions_removed
            + self.classes_removed
            + self.structs_removed
            + self.statics_removed
            + self.types_removed
    }

    pub fn format_report(&self) -> String {
        format!(
            "DCE Report: before={}, after={}, removed={} (fn: {}, class: {}, struct: {}, static: {}, type: {}), tests_pruned={}",
            self.items_before,
            self.items_after,
            self.total_removed(),
            self.functions_removed,
            self.classes_removed,
            self.structs_removed,
            self.statics_removed,
            self.types_removed,
            self.tests_pruned,
        )
    }
}
