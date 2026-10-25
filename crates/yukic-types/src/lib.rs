#![deny(unsafe_code)]
#![allow(clippy::result_large_err)]

pub mod checker;
pub mod contracts;
pub mod convert;
pub mod env;
pub mod exhaust;
pub mod mro;
pub mod types;

pub use checker::TypeChecker;
pub use contracts::SemanticContracts;
pub use convert::{ast_type_to_type, parse_primitive_or_named, parse_type_generic_args};
pub use env::{ClassDef, EnumDef, FnSig, InterfaceDef, StructDef, TypeEnv, VarInfo};
pub use exhaust::ExhaustivenessChecker;
pub use mro::MethodResolution;
pub use types::{CIntKind, Type, TypeParam};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_types() {
        let t = Type::nullable(Type::String);
        assert_eq!(t, Type::union([Type::String, Type::Null]));
    }
}
