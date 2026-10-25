#![deny(unsafe_code)]
#![allow(clippy::collapsible_if)]

use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::ErrorCode;
use yukic_span::Span;

use crate::env::{FnSig, TypeEnv};
use crate::types::{Type, TypeParam};

pub struct MethodResolution;

impl MethodResolution {
    pub fn resolve_method(
        env: &TypeEnv,
        target_ty: &Type,
        method_name: &str,
        span: Span,
    ) -> Result<FnSig, Diagnostic> {
        let actual_ty = match target_ty {
            Type::Ref { inner, .. } => inner.as_ref(),
            other => other,
        };

        // (.len, .cap, .as_ptr, .cp)
        if let Some(sig) = Self::lookup_collection_intrinsic(actual_ty, method_name) {
            return Ok(sig);
        }

        // inherent methods
        if let Some(sig) = Self::lookup_inherent(env, target_ty, method_name) {
            return Ok(sig);
        }

        // inherited methods from base cls
        if let Some(sig) = Self::lookup_inherited(env, target_ty, method_name) {
            return Ok(sig);
        }

        // interface methods
        if let Some(sig) = Self::lookup_interface_method(env, target_ty, method_name) {
            return Ok(sig);
        }

        // local ufcs free functions
        if let Some(sig) = Self::lookup_ufcs(env, target_ty, method_name, span)? {
            return Ok(sig);
        }

        let diag = Diagnostic::error(
            ErrorCode::E0403,
            format!("no method named `{method_name}` found for type `{target_ty}`"),
            span,
        );
        Err(diag)
    }

    fn lookup_collection_intrinsic(target_ty: &Type, method: &str) -> Option<FnSig> {
        match method {
            "len" => match target_ty {
                Type::Array(_) | Type::String | Type::Bytes => Some(FnSig {
                    params: vec![TypeParam::value(target_ty.clone())],
                    ret: Type::Int64,
                    is_cfun: false,
                }),
                _ => None,
            },
            "cap" => match target_ty {
                Type::Array(_) | Type::Bytes => Some(FnSig {
                    params: vec![TypeParam::value(target_ty.clone())],
                    ret: Type::Int64,
                    is_cfun: false,
                }),
                _ => None,
            },
            "as_ptr" => match target_ty {
                Type::Array(inner) => Some(FnSig {
                    params: vec![TypeParam::value(target_ty.clone())],
                    ret: Type::Ptr(inner.clone()),
                    is_cfun: false,
                }),
                Type::String | Type::Bytes => Some(FnSig {
                    params: vec![TypeParam::value(target_ty.clone())],
                    ret: Type::Ptr(Box::new(Type::Uint8)),
                    is_cfun: false,
                }),
                _ => None,
            },
            "is_empty" => match target_ty {
                Type::Array(_) | Type::String | Type::Bytes => Some(FnSig {
                    params: vec![TypeParam::value(target_ty.clone())],
                    ret: Type::Bool,
                    is_cfun: false,
                }),
                _ => None,
            },
            "push" => match target_ty {
                Type::Array(inner) => Some(FnSig {
                    params: vec![
                        TypeParam::value(target_ty.clone()),
                        TypeParam::value((**inner).clone()),
                    ],
                    ret: Type::Void,
                    is_cfun: false,
                }),
                Type::Bytes => Some(FnSig {
                    params: vec![
                        TypeParam::value(target_ty.clone()),
                        TypeParam::value(Type::Uint8),
                    ],
                    ret: Type::Void,
                    is_cfun: false,
                }),
                _ => None,
            },
            "pop" => match target_ty {
                Type::Array(inner) => Some(FnSig {
                    params: vec![TypeParam::value(target_ty.clone())],
                    ret: Type::nullable((**inner).clone()),
                    is_cfun: false,
                }),
                Type::Bytes => Some(FnSig {
                    params: vec![TypeParam::value(target_ty.clone())],
                    ret: Type::nullable(Type::Uint8),
                    is_cfun: false,
                }),
                _ => None,
            },
            "del" => match target_ty {
                Type::Array(_) | Type::Bytes => Some(FnSig {
                    params: vec![
                        TypeParam::value(target_ty.clone()),
                        TypeParam::value(Type::Int64),
                    ],
                    ret: Type::Void,
                    is_cfun: false,
                }),
                _ => None,
            },
            "clear" => match target_ty {
                Type::Array(_) | Type::Bytes => Some(FnSig {
                    params: vec![TypeParam::value(target_ty.clone())],
                    ret: Type::Void,
                    is_cfun: false,
                }),
                _ => None,
            },
            "reserve" => match target_ty {
                Type::Array(_) | Type::Bytes => Some(FnSig {
                    params: vec![
                        TypeParam::value(target_ty.clone()),
                        TypeParam::value(Type::Int64),
                    ],
                    ret: Type::Void,
                    is_cfun: false,
                }),
                _ => None,
            },
            "cp" => match target_ty {
                Type::Array(_) | Type::String | Type::Bytes => Some(FnSig {
                    params: vec![TypeParam::value(target_ty.clone())],
                    ret: target_ty.clone(),
                    is_cfun: false,
                }),
                _ => None,
            },
            _ => None,
        }
    }

    fn lookup_inherent(env: &TypeEnv, target_ty: &Type, method: &str) -> Option<FnSig> {
        match target_ty {
            Type::Class(name) => {
                let class_def = env.lookup_class(name)?;
                class_def.methods.get(method).cloned()
            }
            Type::Struct(name) => {
                let struct_def = env.lookup_struct(name)?;
                struct_def.methods.get(method).cloned()
            }
            Type::Interface(name) => {
                let iface_def = env.lookup_interface(name)?;
                iface_def.methods.get(method).cloned()
            }
            _ => None,
        }
    }

    fn lookup_inherited(env: &TypeEnv, target_ty: &Type, method: &str) -> Option<FnSig> {
        if let Type::Class(name) = target_ty {
            let mut curr = env.lookup_class(name)?.superclass.as_deref();
            while let Some(base_name) = curr {
                if let Some(base_def) = env.lookup_class(base_name) {
                    if let Some(sig) = base_def.methods.get(method) {
                        return Some(sig.clone());
                    }
                    curr = base_def.superclass.as_deref();
                } else { break; }
            }
        }
        None
    }

    fn lookup_interface_method(env: &TypeEnv, target_ty: &Type, method: &str) -> Option<FnSig> {
        match target_ty {
            Type::Interface(name) => {
                let iface_def = env.lookup_interface(name)?;
                iface_def.methods.get(method).cloned()
            }
            Type::Class(name) => {
                let class_def = env.lookup_class(name)?;
                for iface_name in &class_def.interfaces {
                    if let Some(iface_def) = env.lookup_interface(iface_name) {
                        if let Some(sig) = iface_def.methods.get(method) {
                            return Some(sig.clone());
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn lookup_ufcs(
        env: &TypeEnv,
        target_ty: &Type,
        method: &str,
        span: Span,
    ) -> Result<Option<FnSig>, Diagnostic> {
        let is_impl = |c: &str, i: &str| env.is_interface_impl(c, i);
        let mut candidates = Vec::new();

        if let Some(sig) = env.lookup_fn(method) {
            if let Some(first_param) = sig.params.first() {
                if target_ty.is_subtype_of(&first_param.ty, &is_impl) {
                    candidates.push(sig.clone());
                }
            }
        }

        if candidates.len() > 1 {
            let diag = Diagnostic::error(
                ErrorCode::E0404,
                format!("ambiguous method call `{method}` for type `{target_ty}`"),
                span,
            );
            return Err(diag);
        }

        Ok(candidates.into_iter().next())
    }

    pub fn resolve_qualified_method(
        env: &TypeEnv,
        module_name: &str,
        method_name: &str,
        target_ty: &Type,
        span: Span,
    ) -> Result<FnSig, Diagnostic> {
        if let Some(sig) = Self::lookup_collection_intrinsic(target_ty, method_name) {
            return Ok(sig);
        }

        let full_name = format!("{module_name}::{method_name}");
        if let Some(sig) = env.lookup_fn(&full_name) {
            let is_impl = |c: &str, i: &str| env.is_interface_impl(c, i);
            if let Some(first) = sig.params.first() {
                if target_ty.is_subtype_of(&first.ty, &is_impl) {
                    return Ok(sig.clone());
                }
            }
        }

        let diag = Diagnostic::error(
            ErrorCode::E0403,
            format!("qualified method `{full_name}` not found for type `{target_ty}`"),
            span,
        );
        Err(diag)
    }
}
