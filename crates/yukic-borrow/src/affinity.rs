#![deny(unsafe_code)]

use yukic_types::Type;

pub struct TypeAffinity;

impl TypeAffinity {
    pub fn is_copy(ty: &Type) -> bool {
        match ty {
            Type::Int8
            | Type::Int16
            | Type::Int32
            | Type::Int64
            | Type::Uint8
            | Type::Uint16
            | Type::Uint32
            | Type::Uint64
            | Type::Float32
            | Type::Float64
            | Type::Bool
            | Type::Char
            | Type::String
            | Type::Void // no return
            | Type::Never
            | Type::Null // null
            | Type::Ptr(_)
            | Type::Fn { .. }
            | Type::Error
            | Type::CInt(_) => true,
            Type::Ref { mutable, .. } => !*mutable,
            Type::Tuple(types) => types.iter().all(Self::is_copy),
            Type::Union(types) => types.iter().all(Self::is_copy),
            _ => false,
        }
    }

    pub fn is_affine(ty: &Type) -> bool { !Self::is_copy(ty) }
}
