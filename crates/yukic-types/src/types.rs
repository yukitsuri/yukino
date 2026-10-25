#![deny(unsafe_code)]
#![allow(clippy::collapsible_if)]

use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CIntKind {
    Char,
    SChar,
    UChar,
    Short,
    UShort,
    Int,
    UInt,
    Long,
    ULong,
    LongLong,
    ULongLong,
    SizeT,
    SSizeT,
}

impl CIntKind {
    pub fn is_signed(self) -> bool {
        matches!(
            self,
            Self::Char
                | Self::SChar
                | Self::Short
                | Self::Int
                | Self::Long
                | Self::LongLong
                | Self::SSizeT
        )
    }

    pub fn is_unsigned(self) -> bool {
        !self.is_signed()
    }

    pub fn bit_width_with_spec(
        self,
        data_model: yukic_target::DataModel,
        pointer_width: u8,
    ) -> u32 {
        match self {
            Self::Char | Self::SChar | Self::UChar => 8,
            Self::Short | Self::UShort => 16,
            Self::Int | Self::UInt => 32,
            Self::Long | Self::ULong => match data_model {
                yukic_target::DataModel::Lp64 => 64,
                yukic_target::DataModel::Llp64 | yukic_target::DataModel::Ilp32 => 32,
            },
            Self::LongLong | Self::ULongLong => 64,
            Self::SizeT | Self::SSizeT => pointer_width as u32,
        }
    }

    pub fn bit_width(self) -> u32 {
        let spec = yukic_target::TargetTriple::host().spec();
        self.bit_width_with_spec(spec.data_model, spec.pointer_width)
    }
}

impl fmt::Display for CIntKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Char => write!(f, "c_char"),
            Self::SChar => write!(f, "c_schar"),
            Self::UChar => write!(f, "c_uchar"),
            Self::Short => write!(f, "c_short"),
            Self::UShort => write!(f, "c_ushort"),
            Self::Int => write!(f, "c_int"),
            Self::UInt => write!(f, "c_uint"),
            Self::Long => write!(f, "c_long"),
            Self::ULong => write!(f, "c_ulong"),
            Self::LongLong => write!(f, "c_longlong"),
            Self::ULongLong => write!(f, "c_ulonglong"),
            Self::SizeT => write!(f, "c_size_t"),
            Self::SSizeT => write!(f, "c_ssize_t"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeParam {
    pub ty: Type,
    pub is_ref: bool,
    pub is_mut: bool,
}

impl TypeParam {
    pub fn new(ty: Type, is_ref: bool, is_mut: bool) -> Self {
        Self { ty, is_ref, is_mut }
    }

    pub fn value(ty: Type) -> Self {
        Self {
            ty,
            is_ref: false,
            is_mut: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    Int8,
    Int16,
    Int32,
    Int64,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Float32,
    Float64,
    Bool,
    Char,
    String,
    Bytes,
    Void,
    Never,
    Null,
    Tuple(Vec<Type>),
    Struct(String),
    Class(String),
    Interface(String),
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Enum(String),
    Union(BTreeSet<Type>),
    Fn {
        params: Vec<TypeParam>,
        ret: Box<Type>,
        is_cfun: bool,
    },
    Shared(Box<Type>),
    Weak(Box<Type>),
    Ptr(Box<Type>),
    Ref {
        inner: Box<Type>,
        mutable: bool,
    },
    CInt(CIntKind),
    Cstruct(String),
    Dynlib,
    TypeVar(String),
    Infer(u32),
    Error,
}

impl Type {
    pub fn nullable(inner: Type) -> Self {
        Self::union([inner, Self::Null])
    }

    pub fn union(types: impl IntoIterator<Item = Type>) -> Self {
        let mut set = BTreeSet::new();
        for ty in types {
            match ty {
                Self::Union(members) => {
                    for member in members {
                        if member != Self::Never {
                            set.insert(member);
                        }
                    }
                }
                Self::Never => {}
                other => {
                    set.insert(other);
                }
            }
        }
        if set.is_empty() {
            return Self::Never;
        }
        if set.len() == 1 {
            return set.into_iter().next().unwrap_or(Self::Never);
        }
        Self::Union(set)
    }

    pub fn is_nullable(&self) -> bool {
        match self {
            Self::Null => true,
            Self::Union(members) => members.contains(&Self::Null),
            _ => false,
        }
    }

    #[must_use]
    pub fn unwrap_nullable(&self) -> Self {
        match self {
            Self::Union(members) => {
                let non_null: Vec<_> = members
                    .iter()
                    .filter(|m| **m != Self::Null)
                    .cloned()
                    .collect();
                Self::union(non_null)
            }
            Self::Null => Self::Never,
            other => other.clone(),
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::Int8
                | Self::Int16
                | Self::Int32
                | Self::Int64
                | Self::Uint8
                | Self::Uint16
                | Self::Uint32
                | Self::Uint64
                | Self::CInt(_)
        )
    }

    pub fn is_signed_integer(&self) -> bool {
        match self {
            Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64 => true,
            Self::CInt(k) => k.is_signed(),
            _ => false,
        }
    }

    pub fn is_unsigned_integer(&self) -> bool {
        match self {
            Self::Uint8 | Self::Uint16 | Self::Uint32 | Self::Uint64 => true,
            Self::CInt(k) => k.is_unsigned(),
            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float32 | Self::Float64)
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    pub fn fits_i64_with_spec(
        &self,
        val: i64,
        data_model: yukic_target::DataModel,
        pointer_width: u8,
    ) -> bool {
        match self {
            Self::Int8 => i8::try_from(val).is_ok(),
            Self::Int16 => i16::try_from(val).is_ok(),
            Self::Int32 => i32::try_from(val).is_ok(),
            Self::Int64 => true,
            Self::Uint8 => u8::try_from(val).is_ok(),
            Self::Uint16 => u16::try_from(val).is_ok(),
            Self::Uint32 => u32::try_from(val).is_ok(),
            Self::Uint64 => val >= 0,
            Self::CInt(k) => {
                let w = k.bit_width_with_spec(data_model, pointer_width);
                if k.is_signed() {
                    match w {
                        8 => i8::try_from(val).is_ok(),
                        16 => i16::try_from(val).is_ok(),
                        32 => i32::try_from(val).is_ok(),
                        64 => true,
                        _ => false,
                    }
                } else {
                    match w {
                        8 => u8::try_from(val).is_ok(),
                        16 => u16::try_from(val).is_ok(),
                        32 => u32::try_from(val).is_ok(),
                        64 => val >= 0,
                        _ => false,
                    }
                }
            }
            _ => false,
        }
    }

    pub fn fits_i64(&self, val: i64) -> bool {
        let spec = yukic_target::TargetTriple::host().spec();
        self.fits_i64_with_spec(val, spec.data_model, spec.pointer_width)
    }

    pub fn fits_u64_with_spec(
        &self,
        val: u64,
        data_model: yukic_target::DataModel,
        pointer_width: u8,
    ) -> bool {
        match self {
            Self::Int8 => i8::try_from(val).is_ok(),
            Self::Int16 => i16::try_from(val).is_ok(),
            Self::Int32 => i32::try_from(val).is_ok(),
            Self::Int64 => i64::try_from(val).is_ok(),
            Self::Uint8 => u8::try_from(val).is_ok(),
            Self::Uint16 => u16::try_from(val).is_ok(),
            Self::Uint32 => u32::try_from(val).is_ok(),
            Self::Uint64 => true,
            Self::CInt(k) => {
                let w = k.bit_width_with_spec(data_model, pointer_width);
                if k.is_signed() {
                    match w {
                        8 => i8::try_from(val).is_ok(),
                        16 => i16::try_from(val).is_ok(),
                        32 => i32::try_from(val).is_ok(),
                        64 => i64::try_from(val).is_ok(),
                        _ => false,
                    }
                } else {
                    match w {
                        8 => u8::try_from(val).is_ok(),
                        16 => u16::try_from(val).is_ok(),
                        32 => u32::try_from(val).is_ok(),
                        64 => true,
                        _ => false,
                    }
                }
            }
            _ => false,
        }
    }

    pub fn fits_u64(&self, val: u64) -> bool {
        let spec = yukic_target::TargetTriple::host().spec();
        self.fits_u64_with_spec(val, spec.data_model, spec.pointer_width)
    }

    pub fn bit_width_with_spec(
        &self,
        data_model: yukic_target::DataModel,
        pointer_width: u8,
    ) -> Option<u32> {
        match self {
            Self::Bool => Some(1),
            Self::Int8 | Self::Uint8 => Some(8),
            Self::Int16 | Self::Uint16 => Some(16),
            Self::Int32 | Self::Uint32 | Self::Char | Self::Float32 => Some(32),
            Self::Int64 | Self::Uint64 | Self::Float64 => Some(64),
            Self::CInt(k) => Some(k.bit_width_with_spec(data_model, pointer_width)),
            _ => None,
        }
    }

    pub fn bit_width(&self) -> Option<u32> {
        let spec = yukic_target::TargetTriple::host().spec();
        self.bit_width_with_spec(spec.data_model, spec.pointer_width)
    }

    pub fn is_subtype_of(
        &self,
        target: &Self,
        is_interface_impl: &dyn Fn(&str, &str) -> bool,
    ) -> bool {
        if self == target || self == &Self::Never || self == &Self::Error || target == &Self::Error
        {
            return true;
        }

        if let Self::Union(target_members) = target {
            if target_members
                .iter()
                .any(|m| self.is_subtype_of(m, is_interface_impl))
            {
                return true;
            }
        }

        if let Self::Union(self_members) = self {
            return self_members
                .iter()
                .all(|m| m.is_subtype_of(target, is_interface_impl));
        }

        match (self, target) {
            (Self::Class(c), Self::Interface(i)) => is_interface_impl(c, i),
            (Self::Class(c), Self::Class(base)) => is_interface_impl(c, base),
            (Self::Array(a), Self::Array(b)) => a.is_subtype_of(b, is_interface_impl),
            (Self::Ptr(_), Self::Ptr(b)) if **b == Self::Void => true,
            (Self::Fn { .. }, Self::Ptr(b)) if **b == Self::Void => true,
            (Self::Null, Self::Ptr(_)) => true,
            (Self::Ref { inner: a, mutable: m1 }, Self::Ref { inner: b, mutable: m2 }) => {
                if *m2 {
                    *m1 && a.is_subtype_of(b, is_interface_impl)
                } else {
                    a.is_subtype_of(b, is_interface_impl)
                }
            }
            _ => false,
        }
    }

    pub fn substitute(&self, mapping: &std::collections::HashMap<String, Type>) -> Self {
        match self {
            Self::TypeVar(name) | Self::Struct(name) => {
                if let Some(replacement) = mapping.get(name) {
                    replacement.clone()
                } else {
                    self.clone()
                }
            }
            Self::Ptr(inner) => Self::Ptr(Box::new(inner.substitute(mapping))),
            Self::Array(inner) => Self::Array(Box::new(inner.substitute(mapping))),
            Self::Ref { inner, mutable } => Self::Ref {
                inner: Box::new(inner.substitute(mapping)),
                mutable: *mutable,
            },
            Self::Shared(inner) => Self::Shared(Box::new(inner.substitute(mapping))),
            Self::Weak(inner) => Self::Weak(Box::new(inner.substitute(mapping))),
            Self::Tuple(members) => {
                Self::Tuple(members.iter().map(|m| m.substitute(mapping)).collect())
            }
            Self::Union(members) => {
                Self::union(members.iter().map(|m| m.substitute(mapping)))
            }
            Self::Fn { params, ret, is_cfun } => Self::Fn {
                params: params
                    .iter()
                    .map(|p| TypeParam {
                        ty: p.ty.substitute(mapping),
                        is_ref: p.is_ref,
                        is_mut: p.is_mut,
                    })
                    .collect(),
                ret: Box::new(ret.substitute(mapping)),
                is_cfun: *is_cfun,
            },
            Self::Map(k, v) => Self::Map(
                Box::new(k.substitute(mapping)),
                Box::new(v.substitute(mapping)),
            ),
            _ => self.clone(),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Int8 => "int8".to_string(),
            Self::Int16 => "int16".to_string(),
            Self::Int32 => "int32".to_string(),
            Self::Int64 => "int64".to_string(),
            Self::Uint8 => "uint8".to_string(),
            Self::Uint16 => "uint16".to_string(),
            Self::Uint32 => "uint32".to_string(),
            Self::Uint64 => "uint64".to_string(),
            Self::Float32 => "float32".to_string(),
            Self::Float64 => "float64".to_string(),
            Self::Bool => "bool".to_string(),
            Self::Char => "char".to_string(),
            Self::String => "string".to_string(),
            Self::Bytes => "bytes".to_string(),
            Self::Void => "void".to_string(),
            Self::Never => "never".to_string(),
            Self::Null => "null".to_string(),
            Self::Tuple(elems) => {
                let parts: Vec<_> = elems.iter().map(Self::display_name).collect();
                format!("({})", parts.join(", "))
            }
            Self::Struct(name) | Self::Class(name) | Self::Interface(name) | Self::Enum(name) => {
                name.clone()
            }
            Self::Array(elem) => format!("{}[]", elem.display_name()),
            Self::Map(k, v) => format!("map<{}, {}>", k.display_name(), v.display_name()),
            Self::Union(members) => {
                if members.len() == 2 && members.contains(&Self::Null) {
                    let other = members.iter().find(|m| **m != Self::Null).unwrap();
                    format!("{}?", other.display_name())
                } else {
                    let parts: Vec<_> = members.iter().map(Self::display_name).collect();
                    parts.join(" | ")
                }
            }
            Self::Fn {
                params,
                ret,
                is_cfun,
            } => {
                let prefix = if *is_cfun { "cfun" } else { "fun" };
                let param_strs: Vec<_> = params.iter().map(|p| p.ty.display_name()).collect();
                format!(
                    "{}({}) -> {}",
                    prefix,
                    param_strs.join(", "),
                    ret.display_name()
                )
            }
            Self::Shared(elem) => format!("shared<{}>", elem.display_name()),
            Self::Weak(elem) => format!("weak<{}>", elem.display_name()),
            Self::Ptr(elem) => format!("ptr<{}>", elem.display_name()),
            Self::Ref { inner, mutable } => {
                if *mutable {
                    format!("ref ~{}", inner.display_name())
                } else {
                    format!("ref {}", inner.display_name())
                }
            }
            Self::CInt(k) => k.to_string(),
            Self::Cstruct(name) => format!("cstruct {name}"),
            Self::Dynlib => "dynlib".to_string(),
            Self::TypeVar(name) => name.clone(),
            Self::Infer(id) => format!("?{id}"),
            Self::Error => "{error}".to_string(),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
