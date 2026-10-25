#![deny(unsafe_code)]

use std::collections::HashMap;
use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::{ErrorCode, WarningCode};
use yukic_span::Span;
use yukic_types::Type;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VarState {
    Uninitialized,
    Owned,
    Moved { moved_at: Span },
    BorrowedShared { count: usize, borrows: Vec<Span> },
    BorrowedMut { borrowed_at: Span },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VarBinding {
    pub name: String,
    pub ty: Type,
    pub state: VarState,
    pub declared_at: Span,
    pub scope_depth: usize,
    pub is_ref: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BorrowRecord {
    pub borrower: String,
    pub borrower_scope: usize,
    pub target: String,
    pub target_scope: usize,
    pub mutable: bool,
    pub span: Span,
    pub used_mutably: bool,
}

#[derive(Clone, Debug)]
pub struct StateEnv {
    pub vars: HashMap<String, VarBinding>,
    pub scopes: Vec<Vec<String>>,
    pub borrows: Vec<BorrowRecord>,
    pub current_depth: usize,
}

impl StateEnv {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            scopes: vec![Vec::new()],
            borrows: Vec::new(),
            current_depth: 0,
        }
    }

    pub fn push_scope(&mut self) {
        self.current_depth += 1;
        self.scopes.push(Vec::new());
    }

    pub fn pop_scope(&mut self) -> Result<(), Vec<Diagnostic>> {
        let mut diags = Vec::new();
        let depth = self.current_depth;

        for b in &self.borrows {
            if b.target_scope == depth && b.borrower_scope < depth {
                let diag = Diagnostic::error(
                    ErrorCode::E0504,
                    format!("borrowed value `{}` does not live long enough", b.target),
                    b.span,
                );
                diags.push(diag);
            }
        }

        // W0505
        for b in &self.borrows {
            if b.borrower_scope == depth && b.mutable && !b.used_mutably {
                let diag = Diagnostic::warning(
                    WarningCode::W0505,
                    format!(
                        "variable `{}` is borrowed as mutable `ref ~`, but was never mutated",
                        b.target
                    ),
                    b.span,
                )
                .with_note("consider replacing mutable borrow `ref ~` with immutable borrow `ref`");
                diags.push(diag);
            }
        }

        // rm vars decl in this scope
        if let Some(scope_vars) = self.scopes.pop() {
            for var_name in scope_vars { self.vars.remove(&var_name); }
        }

        // rel borrows created in this scope
        self.release_borrows_at_scope(depth);

        if self.current_depth > 0 { self.current_depth -= 1; }
        if diags.is_empty() { Ok(()) } else { Err(diags) }
    }

    pub fn declare_var(
        &mut self,
        name: String,
        ty: Type,
        state: VarState,
        span: Span,
        is_ref: bool,
    ) {
        let binding = VarBinding {
            name: name.clone(),
            ty,
            state,
            declared_at: span,
            scope_depth: self.current_depth,
            is_ref,
        };
        self.vars.insert(name.clone(), binding);
        if let Some(scope_vars) = self.scopes.last_mut() {
            scope_vars.push(name);
        }
    }

    pub fn get_var(&self, name: &str) -> Option<&VarBinding> { self.vars.get(name) }

    pub fn get_var_mut(&mut self, name: &str) -> Option<&mut VarBinding> { self.vars.get_mut(name) }

    pub fn record_borrow(&mut self, record: BorrowRecord) {
        let target_name = record.target.clone();
        let is_mut = record.mutable;
        let span = record.span;

        if let Some(var) = self.vars.get_mut(&target_name) {
            match &mut var.state {
                VarState::Owned => {
                    if is_mut {
                        var.state = VarState::BorrowedMut { borrowed_at: span };
                    } else {
                        var.state = VarState::BorrowedShared {
                            count: 1,
                            borrows: vec![span],
                        };
                    }
                }
                VarState::BorrowedShared { count, borrows } if !is_mut => {
                    *count += 1;
                    borrows.push(span);
                }
                _ => {}
            }
        }
        self.borrows.push(record);
    }

    pub fn mark_borrow_mutated(&mut self, borrower_or_target: &str) {
        for b in &mut self.borrows {
            if b.borrower == borrower_or_target || b.target == borrower_or_target {
                b.used_mutably = true;
            }
        }
    }

    fn release_borrows_at_scope(&mut self, scope_depth: usize) {
        let mut released_targets: Vec<(String, bool)> = Vec::new();
        self.borrows.retain(|b| {
            if b.borrower_scope == scope_depth {
                released_targets.push((b.target.clone(), b.mutable));
                false
            } 

            else { true }
        });

        for (target, _is_mut) in released_targets {
            if let Some(var) = self.vars.get_mut(&target) {
                match &mut var.state {
                    VarState::BorrowedMut { .. } => {
                        var.state = VarState::Owned;
                    }
                    VarState::BorrowedShared { count, borrows } => {
                        if *count > 1 {
                            *count -= 1;
                            if !borrows.is_empty() {
                                borrows.pop();
                            }
                        } else {
                            var.state = VarState::Owned;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn release_temp_borrow(&mut self, target: &str) {
        if let Some(pos) = self.borrows.iter().rposition(|b| b.target == target && b.borrower.is_empty()) {
            self.borrows.remove(pos);
            if let Some(var) = self.vars.get_mut(target) {
                match &mut var.state {
                    VarState::BorrowedMut { .. } => {
                        var.state = VarState::Owned;
                    }
                    VarState::BorrowedShared { count, borrows } => {
                        if *count > 1 {
                            *count -= 1;
                            if !borrows.is_empty() {
                                borrows.pop();
                            }
                        } else {
                            var.state = VarState::Owned;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn join_branch_envs(base_env: &StateEnv, branch_envs: &[StateEnv]) -> StateEnv {
        let mut merged = base_env.clone();
        if branch_envs.is_empty() { return merged; }

        for (name, base_var) in &base_env.vars {
            let mut all_owned = true;
            let mut moved_span: Option<Span> = None;

            for branch in branch_envs {
                if let Some(branch_var) = branch.get_var(name) {
                    if let VarState::Moved { moved_at } = &branch_var.state {
                        all_owned = false;
                        moved_span = Some(*moved_at);
                    }
                } else { all_owned = false; }
            }

            if !all_owned && let Some(var) = merged.vars.get_mut(name) {
                let span = moved_span.unwrap_or(base_var.declared_at);
                var.state = VarState::Moved { moved_at: span };
            }
        }

        merged
    }
}

impl Default for StateEnv {
    fn default() -> Self { Self::new() }
}
