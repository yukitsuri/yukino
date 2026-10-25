#![deny(unsafe_code)]

pub mod affinity;
pub mod checker;
pub mod escape;
pub mod state;

pub use affinity::TypeAffinity;
pub use checker::{BorrowChecker, MoveContext};
pub use escape::{EscapeAnalysis, EscapeInfo, EscapeKind};
pub use state::{BorrowRecord, StateEnv, VarBinding, VarState};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ownership_state() {
        assert_ne!(VarState::Owned, VarState::Uninitialized);
    }
}
