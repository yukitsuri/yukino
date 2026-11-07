#![deny(unsafe_code)]

use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::ErrorCode;
use yukic_span::Span;

use crate::module_id::ModuleId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleEdge {
    pub from: ModuleId,
    pub to: ModuleId,
    pub span: Span,
}

#[derive(Debug, Default)]
pub struct ModuleGraph {
    node_count: u32,
    edges: Vec<ModuleEdge>,
}

impl ModuleGraph {
    pub fn new() -> Self { Self::default() }

    pub fn add_node(&mut self) -> ModuleId {
        let id = ModuleId(self.node_count);
        self.node_count += 1;
        id
    }

    pub fn add_edge(&mut self, from: ModuleId, to: ModuleId, span: Span) {
        self.edges.push(ModuleEdge { from, to, span });
    }

    pub fn node_count(&self) -> u32 { self.node_count } // nodes
    pub fn edges(&self) -> &[ModuleEdge] { &self.edges } // edges

    pub fn successors(&self, node: ModuleId) -> Vec<ModuleId> {
        self.edges
            .iter()
            .filter(|e| e.from == node)
            .map(|e| e.to)
            .collect()
    }

    pub fn detect_cycles(&self, module_names: &[String]) -> Vec<Diagnostic> {
        let n = self.node_count as usize;
        let mut state = TarjanState::new(n);

        for i in 0..n {
            let node = ModuleId(u32::try_from(i).unwrap_or(u32::MAX));
            if state.index_of[i].is_none() { self.strongconnect(node, &mut state); }
        }

        state
            .cycles
            .iter()
            .map(|cycle| build_cycle_diagnostic(cycle, module_names, &self.edges))
            .collect()
    }

    fn strongconnect(&self, v: ModuleId, state: &mut TarjanState) {
        let v_idx = v.0 as usize;
        state.index_of[v_idx] = Some(state.current_index);
        state.lowlink[v_idx] = state.current_index;
        state.current_index += 1;
        state.stack.push(v);
        state.on_stack[v_idx] = true;

        for w in self.successors(v) {
            let w_idx = w.0 as usize;
            if state.index_of[w_idx].is_none() {
                self.strongconnect(w, state);
                state.lowlink[v_idx] = state.lowlink[v_idx].min(state.lowlink[w_idx]);
            } else if state.on_stack[w_idx] {
                state.lowlink[v_idx] = state.lowlink[v_idx].min(state.index_of[w_idx].unwrap_or(0));
            }
        }

        if state.lowlink[v_idx] == state.index_of[v_idx].unwrap_or(0) {
            let mut component = Vec::new();
            loop {
                let w = state.stack.pop().expect("stack underflow in tarjan");
                state.on_stack[w.0 as usize] = false;
                component.push(w);
                if w == v { break; }
            }
            
            if component.len() > 1 {
                state.cycles.push(component);
            }
        }
    }

    pub fn topological_order(&self) -> Option<Vec<ModuleId>> {
        let n = self.node_count as usize;
        let mut visited = vec![0u8; n];
        let mut order = Vec::with_capacity(n);

        for i in 0..n {
            let node = ModuleId(u32::try_from(i).unwrap_or(u32::MAX));
            if visited[i] == 0 && !self.topo_dfs(node, &mut visited, &mut order) {
                return None;
            }
        }

        Some(order)
    }

    fn topo_dfs(&self, node: ModuleId, visited: &mut [u8], order: &mut Vec<ModuleId>) -> bool {
        let idx = node.0 as usize;
        visited[idx] = 1;

        for succ in self.successors(node) {
            let succ_idx = succ.0 as usize;
            if visited[succ_idx] == 1 { return false; }
            if visited[succ_idx] == 0 && !self.topo_dfs(succ, visited, order) { return false; }
        }

        visited[idx] = 2;
        order.push(node);
        true
    }
}

struct TarjanState {
    current_index: usize,
    index_of: Vec<Option<usize>>,
    lowlink: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<ModuleId>,
    cycles: Vec<Vec<ModuleId>>,
}

impl TarjanState {
    fn new(n: usize) -> Self {
        Self {
            current_index: 0,
            index_of: vec![None; n],
            lowlink: vec![0; n],
            on_stack: vec![false; n],
            stack: Vec::new(),
            cycles: Vec::new(),
        }
    }
}

fn build_cycle_diagnostic(
    cycle: &[ModuleId],
    module_names: &[String],
    edges: &[ModuleEdge],
) -> Diagnostic {
    let names: Vec<&str> = cycle
        .iter()
        .filter_map(|id| module_names.get(id.0 as usize).map(String::as_str))
        .collect();

    let cycle_desc = names.join(" -> ");
    let message = format!("circular module dependency: {cycle_desc}");

    let span = find_cycle_edge_span(cycle, edges);

    Diagnostic::error(ErrorCode::E0302, message, span)
        .with_note("break the cycle by restructuring imports")
}

fn find_cycle_edge_span(cycle: &[ModuleId], edges: &[ModuleEdge]) -> Span {
    for edge in edges {
        if cycle.contains(&edge.from) && cycle.contains(&edge.to) { return edge.span; }
    }
    Span::DUMMY
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_span::{BytePos, FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(0))
    }

    #[test]
    fn test_graph_no_cycles() {
        let mut graph = ModuleGraph::new();
        let a = graph.add_node();
        let b = graph.add_node();
        let c = graph.add_node();
        graph.add_edge(a, b, dummy_span());
        graph.add_edge(b, c, dummy_span());

        let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let diagnostics = graph.detect_cycles(&names);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_graph_simple_cycle() {
        let mut graph = ModuleGraph::new();
        let a = graph.add_node();
        let b = graph.add_node();
        graph.add_edge(a, b, dummy_span());
        graph.add_edge(b, a, dummy_span());

        let names = vec!["a".to_string(), "b".to_string()];
        let diagnostics = graph.detect_cycles(&names);
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .message
                .contains("circular module dependency")
        );
    }

    #[test]
    fn test_graph_triangle_cycle() {
        let mut graph = ModuleGraph::new();
        let a = graph.add_node();
        let b = graph.add_node();
        let c = graph.add_node();
        graph.add_edge(a, b, dummy_span());
        graph.add_edge(b, c, dummy_span());
        graph.add_edge(c, a, dummy_span());

        let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let diagnostics = graph.detect_cycles(&names);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_graph_self_loop_not_reported() {
        let mut graph = ModuleGraph::new();
        let a = graph.add_node();
        graph.add_edge(a, a, dummy_span());

        let names = vec!["a".to_string()];
        let diagnostics = graph.detect_cycles(&names);
        // self-loops are single-node SCCs, not reported as cycles
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_graph_successors() {
        let mut graph = ModuleGraph::new();
        let a = graph.add_node();
        let b = graph.add_node();
        let c = graph.add_node();
        graph.add_edge(a, b, dummy_span());
        graph.add_edge(a, c, dummy_span());

        let succs = graph.successors(a);
        assert_eq!(succs.len(), 2);
        assert!(succs.contains(&b));
        assert!(succs.contains(&c));
    }

    #[test]
    fn test_graph_topological_order() {
        let mut graph = ModuleGraph::new();
        let a = graph.add_node();
        let b = graph.add_node();
        let c = graph.add_node();
        // a imports b, b imports c -> c must come first, then b, then a
        graph.add_edge(a, b, dummy_span());
        graph.add_edge(b, c, dummy_span());

        let order = graph
            .topological_order()
            .expect("should have topological order");
        assert_eq!(order, vec![c, b, a]);
    }
}
