#![deny(unsafe_code)]

use std::collections::BTreeSet;
use yukic_ast::{Pattern, PatternKind, SwitchArm};
use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::ErrorCode;
use yukic_span::Span;

use crate::env::TypeEnv;
use crate::types::Type;

pub struct ExhaustivenessChecker;

impl ExhaustivenessChecker {
    pub fn check_switch(
        env: &TypeEnv,
        target_ty: &Type,
        arms: &[SwitchArm],
        span: Span,
    ) -> Result<(), Diagnostic> {
        Self::check_unreachable_patterns(arms)?;

        let has_wildcard = arms.iter().any(|arm| Self::is_wildcard(&arm.pattern));
        if has_wildcard {
            return Ok(());
        }

        match target_ty {
            Type::Enum(name) => Self::check_enum_exhaustiveness(env, name, arms, span),
            Type::Bool => Self::check_bool_exhaustiveness(arms, span),
            Type::Union(members) => Self::check_union_exhaustiveness(members, arms, span),
            _ => {
                let diag = Diagnostic::error(
                    ErrorCode::E0405,
                    format!(
                        "non-exhaustive switch on type `{target_ty}`: missing default or wildcard arm"
                    ),
                    span,
                );
                Err(diag)
            }
        }
    }

    fn is_wildcard(pat: &Pattern) -> bool {
        match &pat.kind {
            PatternKind::Wildcard => true,
            PatternKind::Ident(id) if id.name == "else" => true,
            _ => false,
        }
    }

    fn check_unreachable_patterns(arms: &[SwitchArm]) -> Result<(), Diagnostic> {
        let mut wildcard_seen = false;

        for arm in arms {
            if wildcard_seen {
                let diag = Diagnostic::error(
                    ErrorCode::E0406,
                    "unreachable pattern: follows a wildcard or default arm",
                    arm.span,
                );
                return Err(diag);
            }
            if Self::is_wildcard(&arm.pattern) {
                wildcard_seen = true;
            }
        }
        Ok(())
    }

    fn check_enum_exhaustiveness(
        env: &TypeEnv,
        enum_name: &str,
        arms: &[SwitchArm],
        span: Span,
    ) -> Result<(), Diagnostic> {
        let Some(enum_def) = env.lookup_enum(enum_name) else {
            return Ok(());
        };

        let all_variants: BTreeSet<_> = enum_def.variants.keys().cloned().collect();
        let mut covered_variants = BTreeSet::new();

        for arm in arms {
            Self::collect_covered_variants(&arm.pattern, &mut covered_variants);
        }

        let missing: Vec<_> = all_variants
            .difference(&covered_variants)
            .cloned()
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            let missing_str = missing.join(", ");
            let diag = Diagnostic::error(
                ErrorCode::E0405,
                format!(
                    "non-exhaustive switch on enum `{enum_name}`, missing variants: {missing_str}"
                ),
                span,
            );
            Err(diag)
        }
    }

    fn collect_covered_variants(pat: &Pattern, out: &mut BTreeSet<String>) {
        match &pat.kind {
            PatternKind::Variant(_, var_name, _) => {
                out.insert(var_name.name.clone());
            }
            PatternKind::Ident(id) if id.name != "else" => {
                out.insert(id.name.clone());
            }
            PatternKind::Or(sub_pats) => {
                for p in sub_pats {
                    Self::collect_covered_variants(p, out);
                }
            }
            _ => {}
        }
    }

    fn check_bool_exhaustiveness(arms: &[SwitchArm], span: Span) -> Result<(), Diagnostic> {
        let mut true_seen = false;
        let mut false_seen = false;

        for arm in arms {
            if let PatternKind::Literal(yukic_ast::LiteralValue::Bool(b)) = arm.pattern.kind {
                if b {
                    true_seen = true;
                } else {
                    false_seen = true;
                }
            }
        }

        if true_seen && false_seen {
            Ok(())
        } else {
            let mut missing = Vec::new();
            if !true_seen {
                missing.push("true");
            }
            if !false_seen {
                missing.push("false");
            }
            let missing_str = missing.join(", ");
            let diag = Diagnostic::error(
                ErrorCode::E0405,
                format!("non-exhaustive switch on bool, missing: {missing_str}"),
                span,
            );
            Err(diag)
        }
    }

    fn check_union_exhaustiveness(
        members: &BTreeSet<Type>,
        arms: &[SwitchArm],
        span: Span,
    ) -> Result<(), Diagnostic> {
        let mut covered = BTreeSet::new();
        for arm in arms {
            if let PatternKind::Ident(id) = &arm.pattern.kind {
                covered.insert(id.name.clone());
            }
        }

        let mut missing = Vec::new();
        for m in members {
            let name = m.display_name();
            if !covered.contains(&name) {
                missing.push(name);
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            let missing_str = missing.join(", ");
            let diag = Diagnostic::error(
                ErrorCode::E0405,
                format!("non-exhaustive switch on union, missing types: {missing_str}"),
                span,
            );
            Err(diag)
        }
    }
}
