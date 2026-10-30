#![deny(unsafe_code)]
#![allow(clippy::collapsible_if, clippy::wildcard_imports)]

use yukic_ast::*;
use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::{ErrorCode, WarningCode};
use yukic_span::Span;

use crate::contracts::SemanticContracts;
use crate::convert::ast_type_to_type;
use crate::env::{ClassDef, EnumDef, FnSig, InterfaceDef, StructDef, TypeEnv};
use crate::exhaust::ExhaustivenessChecker;
use crate::mro::MethodResolution;
use crate::types::{Type, TypeParam};

pub struct TypeChecker {
    pub env: TypeEnv,
    pub diagnostics: Vec<Diagnostic>,
    current_return_type: Option<Type>,
    pub current_class: Option<String>,
    pub current_fn_name: Option<String>,
    pub target: yukic_target::TargetTriple,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self::with_target(yukic_target::TargetTriple::host())
    }

    pub fn with_target(target: yukic_target::TargetTriple) -> Self {
        Self {
            env: TypeEnv::new(),
            diagnostics: Vec::new(),
            current_return_type: None,
            current_class: None,
            current_fn_name: None,
            target,
        }
    }

    pub fn check_module(&mut self, module: &Module) -> Result<(), Vec<Diagnostic>> {
        self.check_type_aliases(module);
        self.register_module_items(module);
        self.env.canonicalize_all_types();
        self.validate_module_contracts(module);
        self.check_module_bodies(module);

        if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(self.diagnostics.clone())
        }
    }

    fn check_type_aliases(&mut self, module: &Module) {
        let mut aliases = Vec::new();
        for item in &module.items {
            if let ItemKind::TypeAlias(decl) = &item.kind {
                aliases.push(decl.clone());
                let target_ty = ast_type_to_type(&decl.target);
                self.env.register_type_alias(decl.name.name.clone(), target_ty);
            }
        }
        if let Err(diag) = SemanticContracts::check_alias_cycles(&aliases) {
            self.diagnostics.push(diag);
        }
    }

    fn validate_module_contracts(&mut self, module: &Module) {
        for item in &module.items {
            match &item.kind {
                ItemKind::Cfun(decl) => {
                    if let Err(diag) = SemanticContracts::validate_cfun(decl) {
                        self.diagnostics.push(diag);
                    }
                }
                ItemKind::Fun(decl) => {
                    if let Err(diag) = SemanticContracts::validate_test_modifiers(decl) {
                        self.diagnostics.push(diag);
                    }
                    if let Err(diag) = SemanticContracts::validate_thunk_decl(decl) {
                        self.diagnostics.push(diag);
                    }
                }
                ItemKind::Externc(block) => {
                    for ext_item in &block.items {
                        if let ExterncItem::Cfun(decl) = ext_item {
                            if let Err(diag) = SemanticContracts::validate_cfun(decl) {
                                self.diagnostics.push(diag);
                            }
                        }
                    }
                }
                ItemKind::Class(decl) => {
                    self.validate_class_interfaces(decl);
                    self.validate_class_inheritance(decl);
                }
                _ => {}
            }
        }
    }

    fn validate_class_interfaces(&mut self, decl: &ClassDecl) {
        for iface_id in &decl.interfaces {
            let iface_name = &iface_id.name;
            let Some(iface_def) = self.env.lookup_interface(iface_name) else {
                self.diagnostics.push(Diagnostic::error(
                    ErrorCode::E0305,
                    format!("cannot find interface `{iface_name}`"),
                    iface_id.span,
                ));
                continue;
            };

            for (m_name, iface_sig) in &iface_def.methods {
                match self.env.find_class_method(&decl.name.name, m_name) {
                    None => {
                        self.diagnostics.push(Diagnostic::error(
                            ErrorCode::E0408,
                            format!(
                                "class `{}` does not implement method `{m_name}` of interface `{iface_name}`",
                                decl.name.name
                            ),
                            decl.name.span,
                        ));
                    }
                    Some(class_sig) => {
                        let is_impl = |c: &str, i: &str| self.env.is_interface_impl(c, i);
                        let params_match = class_sig.params.len() == iface_sig.params.len()
                            && class_sig
                                .params
                                .iter()
                                .zip(&iface_sig.params)
                                .all(|(c_p, i_p)| {
                                    c_p.is_ref == i_p.is_ref
                                        && c_p.is_mut == i_p.is_mut
                                        && (c_p.ty == i_p.ty
                                            || i_p.ty.is_subtype_of(&c_p.ty, &is_impl))
                                });
                        let ret_match = class_sig.ret == iface_sig.ret
                            || class_sig.ret.is_subtype_of(&iface_sig.ret, &is_impl);

                        if !params_match || !ret_match {
                            self.diagnostics.push(Diagnostic::error(
                                ErrorCode::E0408,
                                format!(
                                    "method `{m_name}` in class `{}` has incompatible signature for interface `{iface_name}`",
                                    decl.name.name
                                ),
                                decl.name.span,
                            ));
                        }
                    }
                }
            }
        }
    }

    fn validate_class_inheritance(&mut self, decl: &ClassDecl) {
        if let Some(base_id) = &decl.base_class {
            let Some(base_def) = self.env.lookup_class(&base_id.name).cloned() else {
                self.diagnostics.push(Diagnostic::error(
                    ErrorCode::E0305,
                    format!("cannot find base class `{}`", base_id.name),
                    base_id.span,
                ));
                return;
            };

            if base_def.is_locked {
                self.diagnostics.push(Diagnostic::error(
                    ErrorCode::E0409,
                    format!("cannot inherit from locked class `{}`", base_id.name),
                    base_id.span,
                ));
            }
        }
    }

    fn resolve_type_node(&self, node: &TypeNode) -> Type {
        self.env.canonicalize_type(&ast_type_to_type(node))
    }

    fn register_module_items(&mut self, module: &Module) {
        for item in &module.items {
            match &item.kind {
                ItemKind::TypeAlias(decl) => {
                    let target_ty = ast_type_to_type(&decl.target);
                    self.env.register_type_alias(decl.name.name.clone(), target_ty);
                }
                ItemKind::Fun(decl) => self.register_fun(decl),
                ItemKind::Cfun(decl) => self.register_cfun(decl),
                ItemKind::Class(decl) => self.register_class(decl),
                ItemKind::Struct(decl) => self.register_struct(decl),
                ItemKind::Cstruct(decl) => self.register_cstruct(decl),
                ItemKind::Interface(decl) => self.register_interface(decl),
                ItemKind::Enum(decl) => self.register_enum(decl),
                ItemKind::Static(decl) => self.register_static(decl),
                ItemKind::Externc(block) => {
                    for ext_item in &block.items {
                        match ext_item {
                            ExterncItem::Cfun(decl) => self.register_cfun(decl),
                            ExterncItem::Cstruct(decl) => self.register_cstruct(decl),
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn check_module_bodies(&mut self, module: &Module) {
        for item in &module.items {
            match &item.kind {
                ItemKind::Fun(decl) => self.check_fun(decl),
                ItemKind::Cfun(decl) => {
                    if let Some(body) = &decl.body {
                        self.check_cfun_body(decl, body);
                    }
                }
                ItemKind::Class(decl) => self.check_class_bodies(decl),
                ItemKind::Static(decl) => self.check_static_item(decl),
                ItemKind::Externc(block) => {
                    for ext_item in &block.items {
                        if let ExterncItem::Cfun(decl) = ext_item {
                            if let Some(body) = &decl.body {
                                self.check_cfun_body(decl, body);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn register_fun(&mut self, decl: &FunDecl) {
        let params = decl
            .params
            .iter()
            .map(|p| {
                let mut ty = p.ty.as_ref().map_or(Type::Infer(0), ast_type_to_type);
                if p.is_variadic && !matches!(ty, Type::Array(_)) {
                    ty = Type::Array(Box::new(ty));
                }
                TypeParam::new(ty, p.is_ref, p.is_mut)
            })
            .collect();
        let ret = decl
            .return_type
            .as_ref()
            .map_or(Type::Void, ast_type_to_type);
        let is_thunk = decl.attrs.iter().any(|a| a.is("thunk"));
        self.env.register_fn(
            decl.name.name.clone(),
            FnSig {
                params,
                ret,
                is_cfun: is_thunk,
            },
        );
    }

    fn register_cfun(&mut self, decl: &CfunDecl) {
        let params = decl
            .params
            .iter()
            .map(|p| TypeParam::value(ast_type_to_type(&p.ty)))
            .collect();
        let ret = decl
            .return_type
            .as_ref()
            .map_or(Type::Void, ast_type_to_type);
        self.env.register_cfun(
            decl.name.name.clone(),
            FnSig {
                params,
                ret,
                is_cfun: true,
            },
        );
    }

    fn register_class(&mut self, decl: &ClassDecl) {
        let mut def = ClassDef::default();
        def.is_locked = decl.is_locked;
        if let Some(base_id) = &decl.base_class {
            def.superclass = Some(base_id.name.clone());
        }
        for iface in &decl.interfaces {
            def.interfaces.insert(iface.name.clone());
            let base = iface.name.split('<').next().unwrap_or(&iface.name);
            if base != iface.name {
                def.interfaces.insert(base.to_string());
            }
        }
        for field in &decl.fields {
            def.fields
                .insert(field.name.name.clone(), ast_type_to_type(&field.ty));
        }
        for method in &decl.methods {
            let params = method
                .params
                .iter()
                .map(|p| {
                    let mut ty = p.ty.as_ref().map_or(Type::Infer(0), ast_type_to_type);
                    if p.is_variadic && !matches!(ty, Type::Array(_)) {
                        ty = Type::Array(Box::new(ty));
                    }
                    TypeParam::new(ty, p.is_ref, p.is_mut)
                })
                .collect();
            let ret = method
                .return_type
                .as_ref()
                .map_or(Type::Void, ast_type_to_type);
            def.methods.insert(
                method.name.name.clone(),
                FnSig {
                    params,
                    ret,
                    is_cfun: false,
                },
            );
        }
        for accessor in &decl.accessors {
            let ty = ast_type_to_type(&accessor.ty);
            match &accessor.kind {
                AccessorKind::Get => {
                    def.getters.insert(accessor.name.name.clone(), ty);
                }
                AccessorKind::Set(_) => {
                    def.setters.insert(accessor.name.name.clone(), ty);
                }
            }
        }
        self.env.register_class(decl.name.name.clone(), def);
    }

    fn register_struct(&mut self, decl: &StructDecl) {
        let mut def = StructDef::default();
        for field in &decl.fields {
            if def.fields.insert(field.name.name.clone(), ast_type_to_type(&field.ty)).is_some() {
                self.diagnostics.push(Diagnostic::error(
                    ErrorCode::E0304,
                    format!("field '{}' declared multiple times in struct '{}'", field.name.name, decl.name.name),
                    field.name.span,
                ));
            } else {
                def.field_names.push(field.name.name.clone());
            }
        }
        for method in &decl.methods {
            let params = method
                .params
                .iter()
                .map(|p| {
                    let mut ty = p.ty.as_ref().map_or(Type::Infer(0), ast_type_to_type);
                    if p.is_variadic && !matches!(ty, Type::Array(_)) {
                        ty = Type::Array(Box::new(ty));
                    }
                    TypeParam::new(ty, p.is_ref, p.is_mut)
                })
                .collect();
            let ret = method
                .return_type
                .as_ref()
                .map_or(Type::Void, ast_type_to_type);
            if def.methods.insert(
                method.name.name.clone(),
                FnSig {
                    params,
                    ret,
                    is_cfun: false,
                },
            ).is_some() {
                self.diagnostics.push(Diagnostic::error(
                    ErrorCode::E0304,
                    format!("method '{}' declared multiple times in struct '{}'", method.name.name, decl.name.name),
                    method.name.span,
                ));
            }
        }
        self.env.register_struct(decl.name.name.clone(), def);
    }

    fn register_cstruct(&mut self, decl: &CstructDecl) {
        let mut def = StructDef::default();
        for field in &decl.fields {
            if def.fields.insert(field.name.name.clone(), ast_type_to_type(&field.ty)).is_some() {
                self.diagnostics.push(Diagnostic::error(
                    ErrorCode::E0304,
                    format!("field '{}' declared multiple times in struct '{}'", field.name.name, decl.name.name),
                    field.name.span,
                ));
            } else {
                def.field_names.push(field.name.name.clone());
            }
        }
        self.env.register_struct(decl.name.name.clone(), def);
    }

    fn register_interface(&mut self, decl: &InterfaceDecl) {
        let mut def = InterfaceDef::default();
        def.generics = decl.generics.iter().map(|g| g.name.name.clone()).collect();
        for method in &decl.methods {
            let params = method
                .params
                .iter()
                .map(|p| {
                    let mut ty = p.ty.as_ref().map_or(Type::Infer(0), ast_type_to_type);
                    if p.is_variadic && !matches!(ty, Type::Array(_)) {
                        ty = Type::Array(Box::new(ty));
                    }
                    TypeParam::new(ty, p.is_ref, p.is_mut)
                })
                .collect();
            let ret = method
                .return_type
                .as_ref()
                .map_or(Type::Void, ast_type_to_type);
            def.methods.insert(
                method.name.name.clone(),
                FnSig {
                    params,
                    ret,
                    is_cfun: false,
                },
            );
        }
        self.env.register_interface(decl.name.name.clone(), def);
    }

    fn register_enum(&mut self, decl: &EnumDecl) {
        let mut def = EnumDef::default();
        for variant in &decl.variants {
            match variant {
                EnumVariant::Simple(id) => {
                    def.variants.insert(id.name.clone(), None);
                }
                EnumVariant::WithValue(id, _) => {
                    def.variants.insert(id.name.clone(), Some(Type::Int64));
                }
                EnumVariant::Adt(id, fields) => {
                    let field_tys = fields.iter().map(|f| ast_type_to_type(&f.ty)).collect();
                    def.variants
                        .insert(id.name.clone(), Some(Type::Tuple(field_tys)));
                }
            }
        }
        self.env.register_enum(decl.name.name.clone(), def);
    }

    fn register_static(&mut self, decl: &StaticDecl) {
        let ty = if let Some(type_node) = &decl.ty {
            ast_type_to_type(type_node)
        } else {
            match self.infer_expr(&decl.init) {
                Ok(t) => t,
                Err(diag) => {
                    self.diagnostics.push(diag);
                    Type::Error
                }
            }
        };
        self.env.insert_root_static(decl.name.name.clone(), ty, decl.span);
    }

    fn check_static_item(&mut self, decl: &StaticDecl) {
        if let Some(type_node) = &decl.ty {
            if self.validate_fixed_array_size(type_node, &decl.init).is_err() {
                return;
            }
            let expected = self.resolve_type_node(type_node);
            if let Err(diag) = self.check_expr(&decl.init, &expected) {
                self.diagnostics.push(diag);
            }
        } else if let Err(diag) = self.infer_expr(&decl.init) {
            self.diagnostics.push(diag);
        }
    }

    fn check_fun(&mut self, decl: &FunDecl) {
        let prev_fn = self.current_fn_name.replace(decl.name.name.clone());
        let ret = decl
            .return_type
            .as_ref()
            .map_or(Type::Void, |t| self.resolve_type_node(t));
        self.current_return_type = Some(ret.clone());
        self.env.push_scope();

        for param in &decl.params {
            let mut param_ty = param
                .ty
                .as_ref()
                .map_or(Type::Infer(0), |t| self.resolve_type_node(t));
            if param.is_variadic && !matches!(param_ty, Type::Array(_)) {
                param_ty = Type::Array(Box::new(param_ty));
            }
            self.env
                .insert_var(param.name.name.clone(), param_ty, param.is_mut, param.span);
        }

        let _ = self.check_block_impl(
            &decl.body,
            Some(&ret),
            Some(("function", decl.name.name.as_str(), decl.name.span)),
        );
        self.env.pop_scope();
        self.current_return_type = None;
        self.current_fn_name = prev_fn;
    }

    fn check_cfun_body(&mut self, decl: &CfunDecl, body: &Block) {
        let prev_fn = self.current_fn_name.replace(decl.name.name.clone());
        let ret = decl
            .return_type
            .as_ref()
            .map_or(Type::Void, |t| self.resolve_type_node(t));
        self.current_return_type = Some(ret.clone());
        self.env.push_scope();

        for param in &decl.params {
            let param_ty = self.resolve_type_node(&param.ty);
            self.env
                .insert_var(param.name.name.clone(), param_ty, false, param.span);
        }

        let _ = self.check_block_impl(
            body,
            Some(&ret),
            Some(("function", decl.name.name.as_str(), decl.name.span)),
        );
        self.env.pop_scope();
        self.current_return_type = None;
        self.current_fn_name = prev_fn;
    }

    fn check_class_bodies(&mut self, decl: &ClassDecl) {
        self.current_class = Some(decl.name.name.clone());
        for method in &decl.methods {
            self.check_method(decl, method);
        }
        for accessor in &decl.accessors {
            self.check_accessor(decl, accessor);
        }
        self.current_class = None;
    }

    fn check_accessor(&mut self, decl: &ClassDecl, accessor: &PropertyAccessor) {
        let ty = self.resolve_type_node(&accessor.ty);
        let ret_ty = match &accessor.kind {
            AccessorKind::Get => ty.clone(),
            AccessorKind::Set(_) => Type::Void,
        };
        self.current_return_type = Some(ret_ty.clone());
        self.env.push_scope();

        self.env.insert_var(
            "this".to_string(),
            Type::Class(decl.name.name.clone()),
            false,
            accessor.span,
        );

        if let AccessorKind::Set(param) = &accessor.kind {
            self.env.insert_var(param.name.clone(), ty, false, param.span);
        }

        let _ = self.check_block_impl(
            &accessor.body,
            Some(&ret_ty),
            Some(("accessor", accessor.name.name.as_str(), accessor.name.span)),
        );
        self.env.pop_scope();
        self.current_return_type = None;
    }

    fn validate_operator_method(&mut self, method: &MethodDecl) {
        if !method.is_operator {
            return;
        }
        let name = method.name.as_str();
        let p_count = method.params.len();
        let is_valid = match name {
            "-" => p_count == 0 || p_count == 1,
            "!" | "~" => p_count == 0,
            "+" | "*" | "/" | "%" | "**" | "&" | "|" | "^" | "<<" | ">>"
            | "==" | "!=" | "<" | "<=" | ">" | ">=" => p_count == 1,
            "[]" => p_count >= 1,
            "[]=" => p_count >= 2,
            _ => true,
        };
        if !is_valid {
            let diag = Diagnostic::error(
                ErrorCode::E0428,
                format!("invalid signature for operator `{name}`: unexpected parameter count {p_count}"),
                method.span,
            );
            self.diagnostics.push(diag);
        }
    }

    fn check_method(&mut self, decl: &ClassDecl, method: &MethodDecl) {
        let prev_fn = self.current_fn_name.replace(method.name.name.clone());
        self.validate_operator_method(method);
        let ret = method
            .return_type
            .as_ref()
            .map_or(Type::Void, |t| self.resolve_type_node(t));
        self.current_return_type = Some(ret.clone());
        self.env.push_scope();

        if method.this_param != item::ThisParam::None {
            self.env.insert_var(
                "this".to_string(),
                Type::Class(decl.name.name.clone()),
                false,
                method.span,
            );
        }

        for param in &method.params {
            let mut param_ty = param
                .ty
                .as_ref()
                .map_or(Type::Infer(0), |t| self.resolve_type_node(t));
            if param.is_variadic && !matches!(param_ty, Type::Array(_)) {
                param_ty = Type::Array(Box::new(param_ty));
            }
            self.env
                .insert_var(param.name.name.clone(), param_ty, param.is_mut, param.span);
        }

        let _ = self.check_block_impl(
            &method.body,
            Some(&ret),
            Some(("method", method.name.name.as_str(), method.name.span)),
        );
        self.env.pop_scope();
        self.current_return_type = None;
        self.current_fn_name = prev_fn;
    }

    pub fn check_block(
        &mut self,
        block: &Block,
        expected: Option<&Type>,
    ) -> Result<Type, Diagnostic> {
        self.check_block_impl(block, expected, None)
    }

    // Checks block statements and reports E0410 if expected return value is missing.
    fn check_block_impl(
        &mut self,
        block: &Block,
        expected: Option<&Type>,
        ret_ctx: Option<(&'static str, &str, Span)>,
    ) -> Result<Type, Diagnostic> {
        self.env.push_scope();
        let mut last_type = Type::Void;
        let mut missing_return_reported = false;

        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == block.stmts.len() - 1;
            if is_last {
                if let (Some(expected_ty), StmtKind::Expr(expr)) = (expected, &stmt.kind) {
                    if *expected_ty == Type::Void {
                        let _ = self.infer_expr(expr)?;
                        last_type = Type::Void;
                        continue;
                    }
                    if let Some(ctx) = ret_ctx {
                        if !matches!(expected_ty, Type::Never | Type::Error) {
                            let inferred = self.infer_expr(expr)?;
                            if inferred == Type::Void {
                                self.report_missing_return(ctx, expected_ty);
                                missing_return_reported = true;
                                last_type = Type::Void;
                                continue;
                            }
                        }
                    }
                    last_type = self.check_expr(expr, expected_ty)?;
                    continue;
                }
            }
            self.check_stmt(stmt)?;
            if is_last {
                match &stmt.kind {
                    StmtKind::Expr(expr) => last_type = self.infer_expr(expr)?,
                    StmtKind::Return(_) | StmtKind::Recur(_) => last_type = Type::Never,
                    _ => {}
                }
            }
        }

        if let (Some(ctx), Some(expected_ty)) = (ret_ctx, expected) {
            if !missing_return_reported
                && last_type == Type::Void
                && !matches!(expected_ty, Type::Void | Type::Never | Type::Error)
            {
                self.report_missing_return(ctx, expected_ty);
            }
        }

        self.env.pop_scope();
        Ok(last_type)
    }

    fn report_missing_return(&mut self, ctx: (&'static str, &str, Span), ret_ty: &Type) {
        let (kind, name, span) = ctx;
        let diag = Diagnostic::error(
            ErrorCode::E0410,
            format!(
                "missing return: {kind} `{name}` declares return type `{ret_ty}` but its body does not end with a return value"
            ),
            span,
        )
        .with_note(format!(
            "add a trailing `return` with a value of type `{ret_ty}` (or end the body with an expression of that type) on every code path"
        ));
        self.diagnostics.push(diag);
    }

    fn validate_fixed_array_size(
        &mut self,
        type_node: &TypeNode,
        init: &Expr,
    ) -> Result<(), Diagnostic> {
        if let TypeNodeKind::Array(elem_ty, Some(expected_size)) = &type_node.kind {
            if let ExprKind::ArrayInit(elems) = &init.kind {
                if elems.len() != *expected_size {
                    let diag = Diagnostic::error(
                        ErrorCode::E0401,
                        format!(
                            "fixed-size array size mismatch: expected {} elements, found {}",
                            expected_size,
                            elems.len()
                        ),
                        init.span,
                    )
                    .with_note(format!(
                        "the array type `{elem_ty}[{expected_size}]` specifies exactly {expected_size} elements"
                    ));
                    self.diagnostics.push(diag.clone());
                    return Err(diag);
                }
            }
        }
        Ok(())
    }

    pub fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match &stmt.kind {
            StmtKind::Set { target, ty, init } => {
                let init_ty = if let Some(type_node) = ty {
                    self.validate_fixed_array_size(type_node, init)?;
                    let expected = self.resolve_type_node(type_node);
                    self.check_expr(init, &expected)?
                } else {
                    let inferred = self.infer_expr(init)?;
                    if inferred == Type::Void {
                        let var_name = match target {
                            BindingTarget::Ident(id) => format!(" `{}`", id.name),
                            _ => String::new(),
                        };
                        let diag = Diagnostic::error(
                            ErrorCode::E0402,
                            format!("cannot infer type for variable{var_name}"),
                            stmt.span,
                        )
                        .with_note("consider giving this variable an explicit type annotation, e.g. `set name: <type> = ...`");
                        self.diagnostics.push(diag.clone());
                        return Err(diag);
                    }
                    inferred
                };
                self.bind_target(target, init_ty, false, stmt.span);
                Ok(())
            }
            StmtKind::Static(decl) => {
                let init_ty = if let Some(type_node) = &decl.ty {
                    self.validate_fixed_array_size(type_node, &decl.init)?;
                    let expected = self.resolve_type_node(type_node);
                    self.check_expr(&decl.init, &expected)?
                } else {
                    let inferred = self.infer_expr(&decl.init)?;
                    if inferred == Type::Void {
                        let diag = Diagnostic::error(
                            ErrorCode::E0402,
                            format!("cannot infer type for variable `{}`", decl.name.name),
                            stmt.span,
                        )
                        .with_note("consider giving this static variable an explicit type annotation, e.g. `static name: <type> = ...`");
                        self.diagnostics.push(diag.clone());
                        return Err(diag);
                    }
                    inferred
                };
                self.env.insert_static_var(decl.name.name.clone(), init_ty, decl.span);
                Ok(())
            }
            StmtKind::Rewrite { target, value, .. } => {
                let target_ty = self.check_lvalue(target, stmt.span)?;
                self.check_expr(value, &target_ty)?;
                Ok(())
            }
            StmtKind::Return(expr_opt) => {
                if let Some(ret_ty) = self.current_return_type.clone() {
                    if let Some(expr) = expr_opt {
                        self.check_expr(expr, &ret_ty)?;
                    } else if ret_ty != Type::Void && ret_ty != Type::Never {
                        let diag = Diagnostic::error(
                            ErrorCode::E0401,
                            format!("expected return type `{ret_ty}`, found `void`"),
                            stmt.span,
                        );
                        self.diagnostics.push(diag.clone());
                        return Err(diag);
                    }
                } else {
                    let actual = if let Some(expr) = expr_opt {
                        self.infer_expr(expr)?
                    } else {
                        Type::Void
                    };
                    self.current_return_type = Some(actual);
                }
                Ok(())
            }
            StmtKind::Expr(expr) => {
                let _ = self.infer_expr(expr)?;
                Ok(())
            }
            StmtKind::Defer(block) => {
                self.check_block(block, None)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn bind_target(&mut self, target: &BindingTarget, ty: Type, is_mut: bool, span: Span) {
        match target {
            BindingTarget::Ident(id) => {
                self.env.insert_var(id.name.clone(), ty, is_mut, span);
            }
            BindingTarget::Tuple(targets) => {
                if let Type::Tuple(elem_tys) = ty {
                    for (t, elem_ty) in targets.iter().zip(elem_tys) {
                        self.bind_target(t, elem_ty, is_mut, span);
                    }
                }
            }
            _ => {}
        }
    }

    fn check_lvalue(&mut self, lvalue: &LValue, span: Span) -> Result<Type, Diagnostic> {
        match lvalue {
            LValue::Ident(id) => {
                if let Some(info) = self.env.lookup_var(&id.name) {
                    if info.is_static {
                        let diag = Diagnostic::error(
                            ErrorCode::E0503,
                            format!("cannot reassign static constant `{}`", id.name),
                            span,
                        )
                        .with_note("`static` variables are immutable and cannot be reassigned with `rewrite`");
                        self.diagnostics.push(diag.clone());
                        return Err(diag);
                    }
                    Ok(info.ty.clone())
                } else {
                    let diag = Diagnostic::error(
                        ErrorCode::E0402,
                        format!("undefined variable `{}`", id.name),
                        span,
                    );
                    self.diagnostics.push(diag.clone());
                    Err(diag)
                }
            }
            LValue::Field(target, field) => {
                let target_ty = self.infer_expr(target)?;
                let base_ty = match &target_ty {
                    Type::Ref { inner, .. } | Type::Shared(inner) | Type::Weak(inner) => inner.as_ref(),
                    other => other,
                };
                if let Type::Class(cname) = base_ty {
                    if let Some(setter_ty) = self.lookup_class_setter(cname, &field.name) {
                        return Ok(setter_ty);
                    }
                    if let Some(field_ty) = self.lookup_class_field(cname, &field.name) {
                        let has_real_field = {
                            let mut curr = Some(cname.as_str());
                            let mut found = false;
                            while let Some(c) = curr {
                                if let Some(def) = self.env.lookup_class(c) {
                                    if def.fields.contains_key(&field.name) {
                                        found = true;
                                        break;
                                    }
                                    curr = def.superclass.as_deref();
                                } else {
                                    break;
                                }
                            }
                            found
                        };
                        if !has_real_field {
                            let diag = Diagnostic::error(
                                ErrorCode::E0401,
                                format!("cannot assign to read-only property `{}`", field.name),
                                span,
                            );
                            self.diagnostics.push(diag.clone());
                            return Err(diag);
                        }
                        return Ok(field_ty);
                    }
                }
                self.lookup_field_type(&target_ty, &field.name, span)
            }
            _ => Ok(Type::Error),
        }
    }

    pub fn check_expr(&mut self, expr: &Expr, expected: &Type) -> Result<Type, Diagnostic> {
        if expected == &Type::Error {
            return Ok(Type::Error);
        }

        if let ExprKind::Literal(lit) = &expr.kind {
            if let Some(ty) = self.check_literal(lit, expected, expr.span)? {
                return Ok(ty);
            }
        }

        if let ExprKind::Unary(op, inner) = &expr.kind {
            if let Some(ty) = self.check_unary(*op, inner, expected, expr.span)? {
                return Ok(ty);
            }
        }

        if let ExprKind::Binary(op, left, right) = &expr.kind {
            if expected.is_float() && matches!(op, BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem | BinaryOp::Pow) {
                self.check_expr(left, expected)?;
                self.check_expr(right, expected)?;
                return Ok(expected.clone());
            }
        }

        if let (ExprKind::ArrayInit(elems), Type::Array(elem_ty)) = (&expr.kind, expected) {
            for elem in elems {
                self.check_expr(elem, elem_ty)?;
            }
            return Ok(expected.clone());
        }

        if let (ExprKind::ArrayAlloc(elem_ty, _count), Type::Array(expected_elem)) = (&expr.kind, expected) {
            let elem_t = self.resolve_type_node(elem_ty);
            if elem_t != **expected_elem {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    format!("array element type mismatch: expected `{expected_elem}`, found `{elem_t}`"),
                    expr.span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
            return Ok(expected.clone());
        }

        if let (ExprKind::TupleInit(elems), Type::Tuple(tys)) = (&expr.kind, expected) {
            if elems.len() == tys.len() {
                for (elem, ty) in elems.iter().zip(tys) {
                    self.check_expr(elem, ty)?;
                }
                return Ok(expected.clone());
            }
        }

        if let ExprKind::StructInit(name, fields) = &expr.kind {
            let actual = self.check_struct_init(name, fields, expr.span)?;
            let is_impl = |c: &str, i: &str| self.env.is_subclass_or_impl(c, i);
            if actual.is_subtype_of(expected, &is_impl) {
                return Ok(expected.clone());
            }
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                format!("type mismatch: expected `{expected}`, found `{actual}`"),
                expr.span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }

        if let (ExprKind::AnonymousFun(fun), Type::Fn { params: exp_params, ret: exp_ret, .. }) = (&expr.kind, expected) {
            return self.check_anonymous_fun(fun, Some(exp_params), Some(exp_ret), expr.span);
        }

        if let (ExprKind::Lambda(params, body), Type::Fn { params: exp_params, ret: exp_ret, .. }) = (&expr.kind, expected) {
            return self.check_lambda(params, body, Some(exp_params), Some(exp_ret), expr.span);
        }

        if let ExprKind::Block(b) | ExprKind::RegionBlock(b) = &expr.kind {
            return self.check_block(b, Some(expected));
        }

        let actual = self.infer_expr(expr)?;
        let is_impl = |c: &str, i: &str| self.env.is_subclass_or_impl(c, i);
        if actual.is_subtype_of(expected, &is_impl) {
            return Ok(expected.clone());
        }

        if let Type::Union(members) = expected {
            let diag_len = self.diagnostics.len();
            for member in members {
                if let Ok(ty) = self.check_expr(expr, member) {
                    self.diagnostics.truncate(diag_len);
                    return Ok(ty);
                }
                self.diagnostics.truncate(diag_len);
            }
        }

        let diag = Diagnostic::error(
            ErrorCode::E0401,
            format!("expected `{expected}`, found `{actual}`"),
            expr.span,
        );
        self.diagnostics.push(diag.clone());
        Err(diag)
    }

    fn check_unary(
        &mut self,
        op: UnaryOp,
        inner: &Expr,
        expected: &Type,
        span: Span,
    ) -> Result<Option<Type>, Diagnostic> {
        match op {
            UnaryOp::Neg => self.check_neg(inner, expected, span),
            UnaryOp::Not if expected == &Type::Bool => {
                self.check_expr(inner, &Type::Bool)?;
                Ok(Some(Type::Bool))
            }
            UnaryOp::BitNot if expected.is_integer() => {
                self.check_expr(inner, expected)?;
                Ok(Some(expected.clone()))
            }
            _ => Ok(None),
        }
    }

    fn check_neg(
        &mut self,
        inner: &Expr,
        expected: &Type,
        span: Span,
    ) -> Result<Option<Type>, Diagnostic> {
        if let ExprKind::Literal(LiteralValue::Int(n)) = &inner.kind {
            return self.check_literal(&LiteralValue::Int(n.wrapping_neg()), expected, span);
        }
        if let ExprKind::Literal(LiteralValue::Float(f)) = &inner.kind {
            return self.check_literal(&LiteralValue::Float(-f), expected, span);
        }
        if expected.is_numeric() {
            self.check_expr(inner, expected)?;
            return Ok(Some(expected.clone()));
        }
        Ok(None)
    }

    fn check_literal(
        &mut self,
        lit: &LiteralValue,
        expected: &Type,
        span: Span,
    ) -> Result<Option<Type>, Diagnostic> {
        let spec = self.target.spec();
        match (lit, expected) {
            (LiteralValue::Int(n), ty) if ty.is_integer() => {
                if ty.fits_i64_with_spec(*n, spec.data_model, spec.pointer_width) {
                    Ok(Some(expected.clone()))
                } else {
                    let diag = Diagnostic::error(
                        ErrorCode::E0401,
                        format!("literal `{n}` out of range for `{expected}`"),
                        span,
                    );
                    self.diagnostics.push(diag.clone());
                    Err(diag)
                }
            }
            (LiteralValue::UInt(n), ty) if ty.is_integer() => {
                if ty.fits_u64_with_spec(*n, spec.data_model, spec.pointer_width) {
                    Ok(Some(expected.clone()))
                } else {
                    let diag = Diagnostic::error(
                        ErrorCode::E0401,
                        format!("literal `{n}` out of range for `{expected}`"),
                        span,
                    );
                    self.diagnostics.push(diag.clone());
                    Err(diag)
                }
            }
            (LiteralValue::Float(_), ty) if ty.is_float() => Ok(Some(expected.clone())),
            (LiteralValue::Int(_) | LiteralValue::UInt(_), ty) if ty.is_float() => Ok(Some(expected.clone())),
            (LiteralValue::String(_), Type::String) => Ok(Some(Type::String)),
            (LiteralValue::Bool(_), Type::Bool) => Ok(Some(Type::Bool)),
            (LiteralValue::Char(_), Type::Char) => Ok(Some(Type::Char)),
            (LiteralValue::Null, ty) if ty.is_nullable() => Ok(Some(expected.clone())),
            _ => Ok(None),
        }
    }

    pub fn infer_expr(&mut self, expr: &Expr) -> Result<Type, Diagnostic> {
        match &expr.kind {
            ExprKind::Literal(lit) => Ok(Self::infer_literal(lit)),
            ExprKind::Ident(id) => self.infer_ident(id, expr.span),
            ExprKind::This => self.infer_this(expr.span),
            ExprKind::Super => self.infer_super(expr.span),
            ExprKind::Unary(op, inner) => self.infer_unary(*op, inner, expr.span),
            ExprKind::Binary(op, left, right) => self.infer_binary(*op, left, right, expr.span),
            ExprKind::Call(callee, args) => self.infer_call(callee, args, expr.span),
            ExprKind::MethodCall(target, method, args) => {
                self.infer_method_call(target, method, args, expr.span)
            }
            ExprKind::QualifiedMethodCall(target, module, method, args) => {
                self.infer_qualified_method_call(target, module, method, args, expr.span)
            }
            ExprKind::SafeCall(target, method, args) => {
                self.infer_safe_call(target, method, args, expr.span)
            }
            ExprKind::Switch { target, arms } => self.infer_switch(target, arms, expr.span),
            ExprKind::ArrayInit(elems) => self.infer_array(elems, expr.span),
            ExprKind::ArrayAlloc(elem_ty, _) => {
                let elem_t = self.resolve_type_node(elem_ty);
                if elem_t == Type::Void || elem_t == Type::Never {
                    let diag = Diagnostic::error(
                        ErrorCode::E0401,
                        format!("cannot allocate array of `{elem_t}`"),
                        expr.span,
                    );
                    self.diagnostics.push(diag.clone());
                    return Err(diag);
                }
                Ok(Type::Array(Box::new(elem_t)))
            }
            ExprKind::TupleInit(elems) => self.infer_tuple(elems),
            ExprKind::Block(b) | ExprKind::RegionBlock(b) => self.check_block(b, None),
            ExprKind::If {
                cond,
                then_branch,
                elif_branches,
                else_branch,
            } => self.infer_if(
                cond,
                then_branch,
                elif_branches,
                else_branch.as_ref(),
                expr.span,
            ),
            ExprKind::InterpolatedString(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.infer_expr(e)?;
                    }
                }
                Ok(Type::String)
            }
            ExprKind::Ref { mutable, expr } => {
                let inner_ty = self.infer_expr(expr)?;
                Ok(Type::Ref {
                    inner: Box::new(inner_ty),
                    mutable: *mutable,
                })
            }
            ExprKind::Is(target, ty_node) => {
                let _ = self.infer_expr(target)?;
                let _ = self.resolve_type_node(ty_node);
                Ok(Type::Bool)
            }
            ExprKind::Ternary(cond, then_e, else_e) => {
                self.check_expr(cond, &Type::Bool)?;
                let t1 = self.infer_expr(then_e)?;
                let t2 = self.infer_expr(else_e)?;
                Ok(Type::union([t1, t2]))
            }
            ExprKind::FieldAccess(target, field) => {
                self.infer_field_access(target, field, expr.span)
            }
            ExprKind::Sus {
                body,
                catches,
                finally_branch,
            } => self.infer_sus(body, catches, finally_branch.as_ref()),
            ExprKind::PtrOf(id) => {
                let inner_ty = self.infer_ident(id, expr.span)?;
                Ok(Type::Ptr(Box::new(inner_ty)))
            }
            ExprKind::PtrCast(inner, ty_node) => {
                let _ = self.infer_expr(inner)?;
                Ok(self.resolve_type_node(ty_node))
            }
            ExprKind::IndexAccess(target, index) => {
                self.infer_index_access(target, index, expr.span)
            }
            ExprKind::SliceAccess(target, start, end, _) => {
                self.infer_slice_access(target, start.as_deref(), end.as_deref(), expr.span)
            }
            ExprKind::StructInit(name, fields) => {
                self.check_struct_init(name, fields, expr.span)
            }
            ExprKind::ClassInit(callee, args) => {
                self.infer_class_init(callee, args, expr.span)
            }
            ExprKind::AnonymousFun(fun) => self.check_anonymous_fun(fun, None, None, expr.span),
            ExprKind::Lambda(params, body) => self.check_lambda(params, body, None, None, expr.span),
            ExprKind::Path(segments) => self.infer_path_expr(segments, expr.span),
            ExprKind::NamedArg(_, val) => self.infer_expr(val),
            ExprKind::While { cond, body, .. } => {
                self.check_expr(cond, &Type::Bool)?;
                self.check_block(body, None)?;
                Ok(Type::Void)
            }
            ExprKind::Loop { body, .. } => {
                self.check_block(body, None)?;
                Ok(Type::Void)
            }
            ExprKind::For {
                index_var,
                item_var,
                iter,
                body,
                ..
            } => {
                let iter_ty = self.infer_expr(iter)?;
                self.env.push_scope();
                if let Some(idx) = index_var {
                    self.env
                        .insert_var(idx.name.clone(), Type::Int64, false, idx.span);
                }
                let item_ty = match &iter_ty {
                    Type::Array(elem) => (**elem).clone(),
                    _ => Type::Int64,
                };
                self.env
                    .insert_var(item_var.name.clone(), item_ty, false, item_var.span);
                let res = self.check_block(body, None);
                self.env.pop_scope();
                res?;
                Ok(Type::Void)
            }
            ExprKind::Error => Ok(Type::Error),
            _ => Ok(Type::Void),
        }
    }

    fn check_struct_init(
        &mut self,
        name: &Ident,
        fields: &[FieldInit],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let struct_def = match self.env.lookup_struct(&name.name) {
            Some(def) => def.clone(),
            None => {
                let diag = Diagnostic::error(
                    ErrorCode::E0305,
                    format!("cannot find struct '{}' in this scope", name.name),
                    name.span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
        };

        let mut seen = std::collections::HashSet::new();
        let mut first_err: Option<Diagnostic> = None;

        for f in fields {
            if !seen.insert(f.name.name.clone()) {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    format!("field '{}' specified more than once in struct literal", f.name.name),
                    f.name.span,
                );
                self.diagnostics.push(diag.clone());
                if first_err.is_none() {
                    first_err = Some(diag);
                }
                continue;
            }

            match struct_def.fields.get(&f.name.name) {
                Some(expected_ty) => {
                    if let Err(mut diag) = self.check_expr(&f.value, expected_ty) {
                        let note = format!(
                            "field '{}' of struct '{}' expects type '{}'",
                            f.name.name, name.name, expected_ty
                        );
                        diag.notes.push(note.clone());
                        if let Some(last) = self.diagnostics.last_mut() {
                            if last.code.as_str() == "E0401" {
                                last.notes.push(note);
                            }
                        }
                        if first_err.is_none() {
                            first_err = Some(diag);
                        }
                    }
                }
                None => {
                    let diag = Diagnostic::error(
                        ErrorCode::E0401,
                        format!("struct '{}' has no field named '{}'", name.name, f.name.name),
                        f.name.span,
                    );
                    self.diagnostics.push(diag.clone());
                    if first_err.is_none() {
                        first_err = Some(diag);
                    }
                }
            }
        }

        let mut missing = Vec::new();
        for field_name in &struct_def.field_names {
            if !seen.contains(field_name) {
                missing.push(field_name.as_str());
            }
        }
        if !missing.is_empty() {
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                format!("missing field(s) in struct '{}': {}", name.name, missing.join(", ")),
                span,
            );
            self.diagnostics.push(diag.clone());
            if first_err.is_none() {
                first_err = Some(diag);
            }
        }

        if let Some(err) = first_err {
            return Err(err);
        }

        Ok(Type::Struct(name.name.clone()))
    }

    fn check_anonymous_fun(
        &mut self,
        fun: &yukic_ast::FunDecl,
        expected_params: Option<&[TypeParam]>,
        expected_ret: Option<&Type>,
        _span: Span,
    ) -> Result<Type, Diagnostic> {
        let mut param_types = Vec::with_capacity(fun.params.len());
        for (i, p) in fun.params.iter().enumerate() {
            let ty = if let Some(ty_node) = &p.ty {
                self.resolve_type_node(ty_node)
            } else if let Some(exp_params) = expected_params {
                if let Some(exp_p) = exp_params.get(i) {
                    exp_p.ty.clone()
                } else {
                    let diag = Diagnostic::error(
                        ErrorCode::E0402,
                        format!("parameter `{}` of anonymous function requires a type annotation", p.name.name),
                        p.span,
                    );
                    self.diagnostics.push(diag.clone());
                    return Err(diag);
                }
            } else {
                let diag = Diagnostic::error(
                    ErrorCode::E0402,
                    format!("parameter `{}` of anonymous function requires a type annotation", p.name.name),
                    p.span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            };
            param_types.push(TypeParam::new(ty, p.is_ref, p.is_mut));
        }

        let explicit_ret = fun.return_type.as_ref().map(|t| self.resolve_type_node(t));
        let expected_block_ret = explicit_ret.as_ref().or(expected_ret);

        self.env.push_scope();
        for (p, tp) in fun.params.iter().zip(&param_types) {
            self.env.insert_var(p.name.name.clone(), tp.ty.clone(), p.is_mut, p.span);
        }

        let old_return_type = self.current_return_type.take();
        self.current_return_type = expected_block_ret.cloned();

        let block_res = if explicit_ret.is_some() {
            self.check_block_impl(
                &fun.body,
                expected_block_ret,
                Some(("anonymous function", "<anonymous>", fun.span)),
            )
        } else {
            self.check_block(&fun.body, expected_block_ret)
        };
        let inferred_ret = self.current_return_type.clone();
        self.current_return_type = old_return_type;

        let block_ty = block_res?;
        self.env.pop_scope();

        let final_ret = explicit_ret
            .or_else(|| expected_ret.cloned())
            .or(inferred_ret)
            .unwrap_or(block_ty);

        Ok(Type::Fn {
            params: param_types,
            ret: Box::new(final_ret),
            is_cfun: false,
        })
    }

    fn check_lambda(
        &mut self,
        params: &[Ident],
        body: &Expr,
        expected_params: Option<&[TypeParam]>,
        expected_ret: Option<&Type>,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let ast_params = params
            .iter()
            .map(|id| yukic_ast::Param {
                name: id.clone(),
                is_ref: false,
                is_mut: false,
                is_variadic: false,
                ty: None,
                default_val: None,
                span: id.span,
            })
            .collect();
        let ret_stmt = yukic_ast::Stmt::new(
            yukic_ast::StmtKind::Return(Some(body.clone())),
            body.span,
        );
        let block = yukic_ast::Block {
            stmts: vec![ret_stmt],
            span: body.span,
        };
        let decl = yukic_ast::FunDecl {
            name: Ident::new("<anonymous>", span),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: ast_params,
            return_type: None,
            body: block,
            span,
        };
        self.check_anonymous_fun(&decl, expected_params, expected_ret, span)
    }

    fn infer_literal(lit: &LiteralValue) -> Type {
        match lit {
            LiteralValue::Int(_) => Type::Int64,
            LiteralValue::UInt(_) => Type::Uint64,
            LiteralValue::Float(_) => Type::Float64,
            LiteralValue::String(_) => Type::String,
            LiteralValue::Char(_) => Type::Char,
            LiteralValue::Bool(_) => Type::Bool,
            LiteralValue::Bytes(_) => Type::Bytes,
            LiteralValue::Null => Type::Null,
        }
    }

    fn infer_ident(&mut self, id: &Ident, span: Span) -> Result<Type, Diagnostic> {
        if let Some(variants) = self.env.get_ambiguous(&id.name) {
            let var_str = variants
                .iter()
                .map(|v| format!("`{v}::{}`", id.name))
                .collect::<Vec<_>>()
                .join(" or ");
            let diag = Diagnostic::error(
                ErrorCode::E0304,
                format!(
                    "ambiguous symbol `{}` (available in namespaces: {})",
                    id.name,
                    variants.iter().map(|v| format!("`{v}`")).collect::<Vec<_>>().join(", ")
                ),
                span,
            )
            .with_note(format!("use qualified path {var_str}, or specify an alias"));
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }
        if let Some(info) = self.env.lookup_var(&id.name) {
            return Ok(info.ty.clone());
        }
        if let Some(sig) = self.env.lookup_fn(&id.name) {
            return Ok(Type::Fn {
                params: sig.params.clone(),
                ret: Box::new(sig.ret.clone()),
                is_cfun: sig.is_cfun,
            });
        }
        let diag = Diagnostic::error(
            ErrorCode::E0402,
            format!("cannot infer type for `{}`: symbol not defined", id.name),
            span,
        );
        self.diagnostics.push(diag.clone());
        Err(diag)
    }

    fn infer_this(&mut self, span: Span) -> Result<Type, Diagnostic> {
        if let Some(ref class_name) = self.current_class {
            Ok(Type::Class(class_name.clone()))
        } else if let Some(var) = self.env.lookup_var("this") {
            Ok(var.ty.clone())
        } else {
            let diag = Diagnostic::error(
                ErrorCode::E0402,
                "cannot use `this` outside of a class method",
                span,
            );
            self.diagnostics.push(diag.clone());
            Err(diag)
        }
    }

    fn infer_super(&mut self, span: Span) -> Result<Type, Diagnostic> {
        if let Some(ref class_name) = self.current_class {
            if let Some(class_def) = self.env.lookup_class(class_name) {
                if let Some(ref super_name) = class_def.superclass {
                    Ok(Type::Class(super_name.clone()))
                } else {
                    let diag = Diagnostic::error(
                        ErrorCode::E0416,
                        format!("class '{class_name}' does not have a superclass"),
                        span,
                    );
                    self.diagnostics.push(diag.clone());
                    Err(diag)
                }
            } else {
                let diag = Diagnostic::error(ErrorCode::E0416, "invalid_super_call", span);
                self.diagnostics.push(diag.clone());
                Err(diag)
            }
        } else {
            let diag = Diagnostic::error(
                ErrorCode::E0416,
                "cannot use `super` outside of a class method",
                span,
            );
            self.diagnostics.push(diag.clone());
            Err(diag)
        }
    }

    fn infer_unary(&mut self, op: UnaryOp, inner: &Expr, span: Span) -> Result<Type, Diagnostic> {
        let ty = self.infer_expr(inner)?;
        let op_sym = match op {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
            UnaryOp::BitNot => "~",
        };
        if let Ok(sig) = MethodResolution::resolve_method(&self.env, &ty, op_sym, span) {
            let p_len = sig.params.len();
            if p_len == 0 || (p_len == 1 && matches!(sig.params[0].ty, Type::Class(_) | Type::Struct(_))) {
                return Ok(sig.ret);
            }
        }
        match op {
            UnaryOp::Neg => {
                if ty.is_numeric() {
                    Ok(ty)
                } else {
                    let diag = Diagnostic::error(
                        ErrorCode::E0401,
                        format!("cannot negate non-numeric type `{ty}`"),
                        span,
                    );
                    self.diagnostics.push(diag.clone());
                    Err(diag)
                }
            }
            UnaryOp::Not => {
                if ty == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    let diag = Diagnostic::error(
                        ErrorCode::E0401,
                        format!("cannot apply `!` to non-bool type `{ty}`"),
                        span,
                    );
                    self.diagnostics.push(diag.clone());
                    Err(diag)
                }
            }
            UnaryOp::BitNot => {
                if ty.is_integer() {
                    Ok(ty)
                } else {
                    let diag = Diagnostic::error(
                        ErrorCode::E0401,
                        format!("cannot apply `~` to non-integer type `{ty}`"),
                        span,
                    );
                    self.diagnostics.push(diag.clone());
                    Err(diag)
                }
            }
        }
    }

    fn coerce_literal_to_type(&self, lit: &LiteralValue, expected: &Type) -> bool {
        let spec = self.target.spec();
        match (lit, expected) {
            (LiteralValue::Int(n), ty) if ty.is_integer() => {
                ty.fits_i64_with_spec(*n, spec.data_model, spec.pointer_width)
            }
            (LiteralValue::UInt(n), ty) if ty.is_integer() => {
                ty.fits_u64_with_spec(*n, spec.data_model, spec.pointer_width)
            }
            (LiteralValue::Float(_), ty) if ty.is_float() => true,
            _ => false,
        }
    }

    fn check_binary_op_types(op: BinaryOp, t_left: &Type, t_right: &Type) -> Option<Type> {
        match op {
            BinaryOp::Add => {
                if *t_left == Type::String && *t_right == Type::String {
                    Some(Type::String)
                } else if t_left == t_right && t_left.is_numeric() {
                    Some(t_left.clone())
                } else {
                    None
                }
            }
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem | BinaryOp::Pow => {
                if t_left == t_right && t_left.is_numeric() {
                    Some(t_left.clone())
                } else {
                    None
                }
            }
            BinaryOp::Eq
            | BinaryOp::NotEq => {
                if t_left == t_right {
                    Some(Type::Bool)
                } else if (*t_right == Type::Null && (t_left.is_nullable() || matches!(t_left, Type::Ptr(_))))
                    || (*t_left == Type::Null && (t_right.is_nullable() || matches!(t_right, Type::Ptr(_))))
                {
                    Some(Type::Bool)
                } else if t_left.unwrap_nullable() == t_right.unwrap_nullable()
                    && (t_left.is_nullable() || t_right.is_nullable())
                {
                    Some(Type::Bool)
                } else {
                    None
                }
            }
            BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq => {
                if t_left == t_right {
                    Some(Type::Bool)
                } else {
                    None
                }
            }
            BinaryOp::And | BinaryOp::Or => {
                if (*t_left == Type::Bool || *t_left == Type::Never)
                    && (*t_right == Type::Bool || *t_right == Type::Never)
                {
                    if *t_left == Type::Never && *t_right == Type::Never {
                        Some(Type::Never)
                    } else {
                        Some(Type::Bool)
                    }
                } else {
                    None
                }
            }
            BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr => {
                if t_left == t_right && t_left.is_integer() {
                    Some(t_left.clone())
                } else {
                    None
                }
            }
            BinaryOp::NullCoalesce => {
                let unwrapped = t_left.unwrap_nullable();
                Some(Type::union([unwrapped, t_right.clone()]))
            }
            BinaryOp::Range | BinaryOp::RangeInclusive => {
                if t_left.is_integer() && t_right.is_integer() {
                    Some(Type::Int64)
                } else {
                    None
                }
            }
        }
    }

    fn infer_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let (t_left, t_right) = match (&left.kind, &right.kind) {
            (_, ExprKind::Literal(lit)) => {
                let tl = self.infer_expr(left)?;
                let expected = if op == BinaryOp::NullCoalesce {
                    tl.unwrap_nullable()
                } else {
                    tl.clone()
                };
                if self.coerce_literal_to_type(lit, &expected) {
                    (tl, expected)
                } else {
                    let tr = self.infer_expr(right)?;
                    (tl, tr)
                }
            }
            (ExprKind::Literal(lit), _) => {
                let tr = self.infer_expr(right)?;
                let expected = if op == BinaryOp::NullCoalesce {
                    tr.unwrap_nullable()
                } else {
                    tr.clone()
                };
                if self.coerce_literal_to_type(lit, &expected) {
                    (expected, tr)
                } else {
                    let tl = self.infer_expr(left)?;
                    (tl, tr)
                }
            }
            _ => (self.infer_expr(left)?, self.infer_expr(right)?),
        };

        if let Some(res_ty) = Self::check_binary_op_types(op, &t_left, &t_right) {
            return Ok(res_ty);
        }

        if let Some(res_ty) = self.try_infer_operator_binary(op, &t_left, right, span)? {
            return Ok(res_ty);
        }

        let diag = Diagnostic::error(
            ErrorCode::E0401,
            format!("mismatched types in binary operation: `{t_left}` and `{t_right}`"),
            span,
        );
        self.diagnostics.push(diag.clone());
        Err(diag)
    }

    fn try_infer_operator_binary(
        &mut self,
        op: BinaryOp,
        t_left: &Type,
        right: &Expr,
        span: Span,
    ) -> Result<Option<Type>, Diagnostic> {
        let op_sym = match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::Pow => "**",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
            BinaryOp::Eq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            _ => return Ok(None),
        };

        if let Ok(sig) = MethodResolution::resolve_method(&self.env, t_left, op_sym, span) {
            let offset = usize::from(sig.params.len() == 2);
            if let Some(param) = sig.params.get(offset) {
                self.check_arg_matches_param(right, param)?;
                return Ok(Some(sig.ret));
            }
        }

        if op == BinaryOp::NotEq {
            if let Ok(sig) = MethodResolution::resolve_method(&self.env, t_left, "==", span) {
                let offset = usize::from(sig.params.len() == 2);
                if let Some(param) = sig.params.get(offset) {
                    self.check_arg_matches_param(right, param)?;
                    return Ok(Some(Type::Bool));
                }
            }
        }

        Ok(None)
    }

    fn infer_super_constructor_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let target_ty = self.infer_expr(callee)?;
        match MethodResolution::resolve_method(&self.env, &target_ty, "init", span) {
            Ok(sig) => {
                let param_offset = usize::from(sig.params.len() == args.len() + 1);
                for (arg, param) in args.iter().zip(&sig.params[param_offset..]) {
                    self.check_expr(arg, &param.ty)?;
                }
                Ok(sig.ret)
            }
            Err(_) => {
                let diag = Diagnostic::error(
                    ErrorCode::E0416,
                    "invalid_super_call: constructor `init` not found in superclass",
                    span,
                );
                self.diagnostics.push(diag.clone());
                Err(diag)
            }
        }
    }

    fn find_class_init(&self, class_def: &ClassDef) -> Option<FnSig> {
        if let Some(sig) = class_def.methods.get("init") {
            return Some(sig.clone());
        }
        let mut curr = class_def.superclass.clone();
        while let Some(ancestor) = curr {
            let Some(def) = self.env.lookup_class(&ancestor) else { break };
            if let Some(sig) = def.methods.get("init") {
                return Some(sig.clone());
            }
            curr = def.superclass.clone();
        }
        None
    }

    fn check_arg_matches_param(
        &mut self,
        arg: &Expr,
        param: &TypeParam,
    ) -> Result<Type, Diagnostic> {
        let expected = if param.is_ref {
            Type::Ref {
                inner: Box::new(param.ty.clone()),
                mutable: param.is_mut,
            }
        } else {
            param.ty.clone()
        };
        self.check_expr(arg, &expected)
    }

    fn infer_class_init(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let class_name = match &callee.kind {
            ExprKind::Ident(id) => id.name.clone(),
            ExprKind::Path(idents) => idents.last().map_or_else(|| "<unknown>".to_string(), |id| id.name.clone()),
            _ => {
                let diag = Diagnostic::error(
                    ErrorCode::E0402,
                    "expected class name after 'init'",
                    span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
        };

        let Some(class_def) = self.env.lookup_class(&class_name).cloned() else {
            let diag = Diagnostic::error(
                ErrorCode::E0402,
                format!("cannot find class `{class_name}` to instantiate"),
                span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        };

        if let Some(sig) = self.find_class_init(&class_def) {
            let param_offset = usize::from(sig.params.len() == args.len() + 1);
            let expected_count = sig.params.len() - param_offset;
            if args.len() != expected_count {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    format!(
                        "constructor for class `{class_name}` expects {expected_count} argument(s), found {}",
                        args.len()
                    ),
                    span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
            for (arg, param) in args.iter().zip(&sig.params[param_offset..]) {
                self.check_arg_matches_param(arg, param)?;
            }
        } else if !args.is_empty() {
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                format!(
                    "class `{class_name}` has no constructor and expects 0 arguments, found {}",
                    args.len()
                ),
                span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }

        Ok(Type::Class(class_name))
    }

    fn infer_call(&mut self, callee: &Expr, args: &[Expr], span: Span) -> Result<Type, Diagnostic> {
        if matches!(&callee.kind, ExprKind::Super) {
            return self.infer_super_constructor_call(callee, args, span);
        }
        if let ExprKind::Ident(id) = &callee.kind {
            if id.name == "typeof" {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    "'typeof' is a compile-time macro; use 'typeof!(...)' instead",
                    span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
            if let Some(res) = self.infer_builtin_call(&id.name, args, span) {
                return res;
            }
            if self.env.lookup_class(&id.name).is_some() {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    format!("cannot instantiate class `{}` without 'init'; use 'init {}(...)' instead", id.name, id.name),
                    span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
        }
        if let ExprKind::Path(segments) = &callee.kind {
            if segments.len() == 2 && segments[0].name == "rt" && segments[1].name == "typeof" {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    "'typeof' is a compile-time macro; use 'typeof!(...)' or 'rt::typeof!(...)' instead",
                    span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
            let path_name = segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join("::");
            if let Some(res) = self.infer_builtin_call(&path_name, args, span) {
                return res;
            }
            if let Some(last) = segments.last() {
                if self.env.lookup_class(&last.name).is_some() {
                    let diag = Diagnostic::error(
                        ErrorCode::E0401,
                        format!("cannot instantiate class `{path_name}` without 'init'; use 'init {path_name}(...)' instead"),
                        span,
                    );
                    self.diagnostics.push(diag.clone());
                    return Err(diag);
                }
            }
        }

        let callee_ty = self.infer_expr(callee)?;
        let fn_ty = match &callee_ty {
            Type::Fn { .. } => Some(callee_ty.clone()),
            Type::Ptr(inner) => match &**inner {
                Type::Fn { .. } => Some((**inner).clone()),
                Type::Void => {
                    for arg in args {
                        self.infer_expr(arg)?;
                    }
                    return Ok(Type::Int64);
                }
                _ => None,
            },
            _ => None,
        };

        if let Some(Type::Fn { params, ret, .. }) = fn_ty {
            if args.len() != params.len() {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    format!(
                        "function expects {} argument(s), found {}",
                        params.len(),
                        args.len()
                    ),
                    span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
            for (arg, param) in args.iter().zip(&params) {
                self.check_arg_matches_param(arg, param)?;
            }
            return Ok(*ret);
        }

        let diag = Diagnostic::error(
            ErrorCode::E0401,
            format!("cannot call non-function type `{callee_ty}`"),
            span,
        );
        self.diagnostics.push(diag.clone());
        Err(diag)
    }

    fn infer_builtin_call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Option<Result<Type, Diagnostic>> {
        let bare = name.strip_prefix("rt::").unwrap_or(name);
        match bare {
            "__typeof_intrinsic__" | "typeof" if !args.is_empty() => Some(self.infer_expr(&args[0]).map(|_| Type::String)),
            "cast" if args.len() >= 2 => Some(self.infer_cast_call(&args[0], &args[1], span)),
            "exit" if !args.is_empty() => Some(self.infer_exit_call(&args[0])),
            "args" if args.is_empty() => Some(Ok(Type::Array(Box::new(Type::String)))),
            "raw_args" if args.is_empty() => Some(Ok(Type::String)),
            "panic" => Some(self.infer_panic_call(args, span)),
            "from_raw_parts" | "string::from_raw_parts" | "slice::from_raw_parts" => {
                Some(self.infer_from_raw_parts_call(bare, args, span))
            }
            "string::from_utf8" => Some(self.infer_from_utf8_call(args, span)),
            "assert" | "assert_eq" | "assert_ne" => Some(self.infer_assert_call(bare, args, span)),
            "ptr_of" if args.len() == 1 => {
                Some(self.infer_expr(&args[0]).map(|t| Type::Ptr(Box::new(t))))
            }
            _ => None,
        }
    }

    fn infer_exit_call(&mut self, code_expr: &Expr) -> Result<Type, Diagnostic> {
        self.check_expr(code_expr, &Type::Int32)?;
        Ok(Type::Never)
    }

    fn infer_panic_call(&mut self, args: &[Expr], span: Span) -> Result<Type, Diagnostic> {
        if args.len() != 1 {
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                format!("`panic` expects exactly 1 argument of struct type, found {}", args.len()),
                span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }
        let arg = &args[0];
        let ty = self.infer_expr(arg)?;
        if !matches!(ty, Type::Struct(_)) {
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                format!("`panic` expects a struct error payload, found `{}`", ty),
                arg.span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }
        Ok(Type::Never)
    }

    fn infer_index_access(
        &mut self,
        target: &Expr,
        index: &Expr,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let target_ty = self.infer_expr(target)?;
        if let Ok(sig) = MethodResolution::resolve_method(&self.env, &target_ty, "[]", span) {
            let offset = usize::from(sig.params.len() == 2);
            if let Some(param) = sig.params.get(offset) {
                self.check_arg_matches_param(index, param)?;
                return Ok(sig.ret);
            }
        }

        let index_ty = self.infer_expr(index)?;
        if !index_ty.is_integer() {
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                format!("index must be an integer, found `{index_ty}`"),
                index.span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }
        match target_ty {
            Type::Bytes | Type::String => Ok(Type::Uint8),
            Type::Array(elem) | Type::Ptr(elem) => Ok(*elem),
            Type::Map(k, v) => {
                self.check_expr(index, &k)?;
                Ok(*v)
            }
            _ => {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    format!("type `{target_ty}` cannot be indexed"),
                    span,
                );
                self.diagnostics.push(diag.clone());
                Err(diag)
            }
        }
    }

    fn infer_slice_access(
        &mut self,
        target: &Expr,
        start: Option<&Expr>,
        end: Option<&Expr>,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let target_ty = self.infer_expr(target)?;
        if let Some(s) = start {
            let s_ty = self.infer_expr(s)?;
            if !s_ty.is_integer() {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    format!("slice start bound must be integer, found `{s_ty}`"),
                    s.span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
        }
        if let Some(e) = end {
            let e_ty = self.infer_expr(e)?;
            if !e_ty.is_integer() {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    format!("slice end bound must be integer, found `{e_ty}`"),
                    e.span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
        }
        match target_ty {
            Type::Array(_) | Type::Bytes | Type::String => Ok(target_ty),
            _ => {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    format!("type `{target_ty}` cannot be sliced"),
                    span,
                );
                self.diagnostics.push(diag.clone());
                Err(diag)
            }
        }
    }

    fn infer_path_expr(&mut self, segments: &[Ident], span: Span) -> Result<Type, Diagnostic> {
        let path = segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join("::");

        if segments.len() == 2 && segments[0].name == "extern" {
            let c_name = &segments[1].name;
            if let Some(sig) = self.env.lookup_cfun(c_name) {
                return Ok(Type::Fn {
                    params: sig.params.clone(),
                    ret: Box::new(sig.ret.clone()),
                    is_cfun: true,
                });
            }
            if let Some(sig) = self.env.lookup_fn(c_name) {
                if sig.is_cfun {
                    return Ok(Type::Fn {
                        params: sig.params.clone(),
                        ret: Box::new(sig.ret.clone()),
                        is_cfun: true,
                    });
                }
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    format!("symbol `{c_name}` is not a C function; 'extern::' can only be used with C functions"),
                    span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
            let diag = Diagnostic::error(
                ErrorCode::E0402,
                format!("C function `{c_name}` not found"),
                span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }

        if path == "string::from_raw_parts" || path == "rt::from_raw_parts" || path == "rt::string::from_raw_parts" {
            return Ok(Type::Fn {
                params: vec![
                    TypeParam::new(Type::Ptr(Box::new(Type::Uint8)), false, false),
                    TypeParam::new(Type::Int64, false, false),
                ],
                ret: Box::new(Type::String),
                is_cfun: false,
            });
        }
        if path == "string::from_utf8" {
            return Ok(Type::Fn {
                params: vec![TypeParam::new(Type::Bytes, false, false)],
                ret: Box::new(Type::String),
                is_cfun: false,
            });
        }
        if let Some(info) = self.env.lookup_var(&path) {
            return Ok(info.ty.clone());
        }
        if let Some(sig) = self.env.lookup_fn(&path) {
            return Ok(Type::Fn {
                params: sig.params.clone(),
                ret: Box::new(sig.ret.clone()),
                is_cfun: sig.is_cfun,
            });
        }
        if segments.len() == 2 {
            let enum_name = &segments[0].name;
            let variant_name = &segments[1].name;
            if let Some(enum_def) = self.env.lookup_enum(enum_name).cloned() {
                if let Some(payload) = enum_def.variants.get(variant_name) {
                    return match payload {
                        None => Ok(Type::Enum(enum_name.clone())),
                        Some(Type::Tuple(fields)) => {
                            let params = fields
                                .iter()
                                .map(|ty| TypeParam::value(ty.clone()))
                                .collect();
                            Ok(Type::Fn {
                                params,
                                ret: Box::new(Type::Enum(enum_name.clone())),
                                is_cfun: false,
                            })
                        }
                        Some(single_ty) => {
                            Ok(Type::Fn {
                                params: vec![TypeParam::value(single_ty.clone())],
                                ret: Box::new(Type::Enum(enum_name.clone())),
                                is_cfun: false,
                            })
                        }
                    };
                }
            }
        }
        let diag = Diagnostic::error(ErrorCode::E0402, format!("symbol `{path}` not found"), span);
        self.diagnostics.push(diag.clone());
        Err(diag)
    }

    fn infer_from_raw_parts_call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        if args.len() < 2 || args.len() > 3 {
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                format!("`{name}` expects 2 or 3 arguments, found {}", args.len()),
                span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }
        let ptr_ty = self.infer_expr(&args[0])?;
        let len_ty = self.infer_expr(&args[1])?;
        if !len_ty.is_integer() {
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                format!("length must be an integer, found `{len_ty}`"),
                args[1].span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }
        if args.len() == 3 {
            let cap_ty = self.infer_expr(&args[2])?;
            if !cap_ty.is_integer() {
                let diag = Diagnostic::error(
                    ErrorCode::E0401,
                    format!("capacity must be an integer, found `{cap_ty}`"),
                    args[2].span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
        }
        if let Type::Ptr(inner) = ptr_ty {
            if *inner == Type::Uint8 && args.len() == 2 && (name == "from_raw_parts" || name.starts_with("string")) {
                Ok(Type::String)
            } else if *inner == Type::Uint8 && args.len() == 3 {
                Ok(Type::Bytes)
            } else {
                Ok(Type::Array(inner))
            }
        } else {
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                format!("first argument to `{name}` must be a pointer, found `{ptr_ty}`"),
                args[0].span,
            );
            self.diagnostics.push(diag.clone());
            Err(diag)
        }
    }

    fn infer_from_utf8_call(&mut self, args: &[Expr], span: Span) -> Result<Type, Diagnostic> {
        if args.len() != 1 {
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                format!("`from_utf8` expects 1 argument, found {}", args.len()),
                span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }
        self.check_expr(&args[0], &Type::Bytes)?;
        Ok(Type::String)
    }

    fn infer_assert_call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        if args.is_empty() {
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                format!("`{name}` expects at least 1 argument"),
                span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }
        match name {
            "assert" => {
                self.check_expr(&args[0], &Type::Bool)?;
                if args.len() > 1 {
                    self.check_expr(&args[1], &Type::String)?;
                }
            }
            "assert_eq" | "assert_ne" => {
                if args.len() < 2 {
                    let diag = Diagnostic::error(
                        ErrorCode::E0401,
                        format!("`{name}` expects at least 2 arguments"),
                        span,
                    );
                    self.diagnostics.push(diag.clone());
                    return Err(diag);
                }
                let t1 = self.infer_expr(&args[0])?;
                let t2 = self.infer_expr(&args[1])?;
                if t1 != t2
                    && !t1.is_subtype_of(&t2, &|_, _| false)
                    && !t2.is_subtype_of(&t1, &|_, _| false)
                {
                    let diag = Diagnostic::error(
                        ErrorCode::E0401,
                        format!("cannot compare `{t1}` and `{t2}` in `{name}`"),
                        span,
                    );
                    self.diagnostics.push(diag.clone());
                    return Err(diag);
                }
                if args.len() > 2 {
                    self.check_expr(&args[2], &Type::String)?;
                }
            }
            _ => {}
        }
        Ok(Type::Void)
    }

    fn infer_cast_call(
        &mut self,
        val_expr: &Expr,
        target_expr: &Expr,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let val_ty = self.infer_expr(val_expr)?;

        let target_ty = match &target_expr.kind {
            ExprKind::Literal(LiteralValue::String(name)) => {
                let parsed = crate::convert::parse_primitive_or_named(name);
                self.env.canonicalize_type(&parsed)
            }
            ExprKind::Call(callee, inner_args) => {
                if let ExprKind::Ident(id) = &callee.kind {
                    if (id.name == "typeof" || id.name == "__typeof_intrinsic__") && !inner_args.is_empty() {
                        self.infer_expr(&inner_args[0])?
                    } else {
                        let diag = Diagnostic::error(
                            ErrorCode::E0431,
                            "target type in `cast` must be a compile-time string literal or `typeof`",
                            target_expr.span,
                        );
                        self.diagnostics.push(diag.clone());
                        return Err(diag);
                    }
                } else {
                    let diag = Diagnostic::error(
                        ErrorCode::E0431,
                        "target type in `cast` must be a compile-time string literal or `typeof`",
                        target_expr.span,
                    );
                    self.diagnostics.push(diag.clone());
                    return Err(diag);
                }
            }
            _ => {
                let diag = Diagnostic::error(
                    ErrorCode::E0431,
                    "target type in `cast` must be a compile-time string literal or `typeof`",
                    target_expr.span,
                );
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
        };
        if let Err(diag) =
            SemanticContracts::validate_cast(val_expr, &val_ty, target_expr, &target_ty, span)
        {
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }

        Ok(target_ty)
    }

    fn infer_method_call(
        &mut self,
        target: &Expr,
        method: &Ident,
        args: &[Expr],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let target_ty = self.infer_expr(target)?;
        let is_super = matches!(&target.kind, ExprKind::Super);
        let sig = match MethodResolution::resolve_method(&self.env, &target_ty, &method.name, span)
        {
            Ok(s) => s,
            Err(diag) => {
                let mut final_diag = if is_super {
                    Diagnostic::error(
                        ErrorCode::E0416,
                        format!("invalid_super_call: method '{}' not found in superclass", method.name),
                        span,
                    )
                } else {
                    diag
                };
                if matches!(target_ty, Type::Fn { .. }) {
                    if let ExprKind::Ident(target_id) = &target.kind {
                        final_diag.notes.push(format!(
                            "`{}` is a function, not an object instance; function types have no methods",
                            target_id.name
                        ));
                        if self.current_fn_name.as_deref() == Some(target_id.name.as_str()) {
                            final_diag.notes.push(format!(
                                "`{}` refers to the enclosing function; did you mean to call `.{}` on a local instance instead?",
                                target_id.name, method.name
                            ));
                        }
                    }
                }
                self.diagnostics.push(final_diag.clone());
                return Err(final_diag);
            }
        };

        // check remaining parameters (first param was `this` for inherent/UFCS)
        let param_offset = usize::from(sig.params.len() == args.len() + 1);
        for (arg, param) in args.iter().zip(&sig.params[param_offset..]) {
            self.check_arg_matches_param(arg, param)?;
        }

        Ok(sig.ret)
    }

    fn infer_qualified_method_call(
        &mut self,
        target: &Expr,
        module: &Ident,
        method: &Ident,
        args: &[Expr],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let target_ty = self.infer_expr(target)?;
        let sig = match MethodResolution::resolve_qualified_method(
            &self.env,
            &module.name,
            &method.name,
            &target_ty,
            span,
        ) {
            Ok(s) => s,
            Err(diag) => {
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
        };

        let param_offset = usize::from(sig.params.len() == args.len() + 1);
        for (arg, param) in args.iter().zip(&sig.params[param_offset..]) {
            self.check_arg_matches_param(arg, param)?;
        }

        Ok(sig.ret)
    }

    fn infer_safe_call(
        &mut self,
        target: &Expr,
        method: &Ident,
        args: &[Expr],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let target_ty = self.infer_expr(target)?;
        let unwrapped = target_ty.unwrap_nullable();

        if args.is_empty() {
            if let Some(field_ty) = self.find_field_type(&unwrapped, &method.name) {
                return Ok(Type::nullable(field_ty));
            }
        }

        let sig = match MethodResolution::resolve_method(&self.env, &unwrapped, &method.name, span)
        {
            Ok(s) => s,
            Err(diag) => {
                self.diagnostics.push(diag.clone());
                return Err(diag);
            }
        };

        let param_offset = usize::from(sig.params.len() == args.len() + 1);
        for (arg, param) in args.iter().zip(&sig.params[param_offset..]) {
            self.check_arg_matches_param(arg, param)?;
        }

        Ok(Type::nullable(sig.ret))
    }

    fn infer_switch(
        &mut self,
        target: &Expr,
        arms: &[SwitchArm],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let target_ty = self.infer_expr(target)?;
        if let Err(diag) = ExhaustivenessChecker::check_switch(&self.env, &target_ty, arms, span) {
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }

        let mut body_types = Vec::new();
        for arm in arms {
            self.env.push_scope();
            self.bind_pattern_vars(&arm.pattern, &target_ty);
            let arm_ty = self.check_block(&arm.body, None)?;
            self.env.pop_scope();
            body_types.push(arm_ty);
        }

        Ok(Type::union(body_types))
    }

    fn bind_pattern_vars(&mut self, pattern: &Pattern, target_ty: &Type) {
        match &pattern.kind {
            PatternKind::Ident(id) if id.name != "else" => {
                self.env
                    .insert_var(id.name.clone(), target_ty.clone(), false, id.span);
            }
            PatternKind::Variant(opt_enum, var_id, sub_pats) => {
                let enum_name = opt_enum.as_ref().map(|id| id.name.as_str()).unwrap_or_else(|| {
                    if let Type::Enum(n) = target_ty {
                        n.as_str()
                    } else {
                        ""
                    }
                });
                if let Some(enum_def) = self.env.lookup_enum(enum_name).cloned() {
                    if let Some(payload) = enum_def.variants.get(&var_id.name) {
                        match payload {
                            Some(Type::Tuple(elem_tys)) => {
                                for (sub_pat, ty) in sub_pats.iter().zip(elem_tys) {
                                    self.bind_pattern_vars(sub_pat, ty);
                                }
                            }
                            Some(single_ty) => {
                                if let Some(sub_pat) = sub_pats.first() {
                                    self.bind_pattern_vars(sub_pat, single_ty);
                                }
                            }
                            None => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn infer_array(&mut self, elems: &[Expr], span: Span) -> Result<Type, Diagnostic> {
        if elems.is_empty() {
            let diag = Diagnostic::error(
                ErrorCode::E0402,
                "cannot infer type of empty array literal without context",
                span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }

        let mut types = Vec::new();
        for elem in elems {
            types.push(self.infer_expr(elem)?);
        }
        let unified = Type::union(types);
        Ok(Type::Array(Box::new(unified)))
    }

    fn infer_tuple(&mut self, elems: &[Expr]) -> Result<Type, Diagnostic> {
        let mut types = Vec::new();
        for elem in elems {
            types.push(self.infer_expr(elem)?);
        }
        Ok(Type::Tuple(types))
    }

    fn infer_if(
        &mut self,
        cond: &Expr,
        then_branch: &Block,
        elif_branches: &[(Expr, Block)],
        else_branch: Option<&Block>,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        if elif_branches.len() >= 2 {
            if let Some(var_name) = Self::detect_same_var_chain(cond, elif_branches) {
                self.diagnostics.push(Diagnostic::warning(
                    WarningCode::W0415,
                    format!(
                        "chained 'if-elif' comparisons on '{var_name}' can be simplified with 'switch'"
                    ),
                    span,
                ));
            }
        }
        self.check_expr(cond, &Type::Bool)?;
        let mut types = vec![self.check_block(then_branch, None)?];
        for (elif_cond, elif_body) in elif_branches {
            self.check_expr(elif_cond, &Type::Bool)?;
            types.push(self.check_block(elif_body, None)?);
        }
        if let Some(else_b) = else_branch {
            types.push(self.check_block(else_b, None)?);
            Ok(Type::union(types))
        } else {
            Ok(Type::Void)
        }
    }

    fn detect_same_var_chain(cond: &Expr, elif_branches: &[(Expr, Block)]) -> Option<String> {
        let get_lhs_ident = |e: &Expr| -> Option<String> {
            if let ExprKind::Binary(BinaryOp::Eq, lhs, _) = &e.kind {
                if let ExprKind::Ident(id) = &lhs.kind {
                    return Some(id.name.clone());
                }
            }
            None
        };
        let var_name = get_lhs_ident(cond)?;
        for (elif_cond, _) in elif_branches {
            let elif_var = get_lhs_ident(elif_cond)?;
            if elif_var != var_name {
                return None;
            }
        }
        Some(var_name)
    }

    fn infer_field_access(
        &mut self,
        target: &Expr,
        field: &Ident,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let target_ty = self.infer_expr(target)?;
        let is_super = matches!(&target.kind, ExprKind::Super);
        match self.lookup_field_type(&target_ty, &field.name, span) {
            Ok(ty) => Ok(ty),
            Err(diag) => {
                if is_super {
                    let final_diag = Diagnostic::error(
                        ErrorCode::E0416,
                        format!("invalid_super_call: field '{}' not found in superclass", field.name),
                        span,
                    );
                    self.diagnostics.push(final_diag.clone());
                    Err(final_diag)
                } else {
                    Err(diag)
                }
            }
        }
    }

    fn find_field_type(&self, target_ty: &Type, field_name: &str) -> Option<Type> {
        match target_ty {
            Type::Struct(name) => {
                if let Some(def) = self.env.lookup_struct(name) {
                    if let Some(ty) = def.fields.get(field_name) {
                        return Some(ty.clone());
                    }
                }
                self.lookup_class_field(name, field_name)
            }
            Type::Class(name) => self.lookup_class_field(name, field_name),
            Type::Tuple(elem_tys) => {
                if let Ok(idx) = field_name.parse::<usize>() {
                    elem_tys.get(idx).cloned()
                } else {
                    None
                }
            }
            Type::Ref { inner, .. } | Type::Shared(inner) | Type::Weak(inner) => {
                self.find_field_type(inner, field_name)
            }
            _ => None,
        }
    }

    fn lookup_field_type(
        &mut self,
        target_ty: &Type,
        field_name: &str,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        if let Some(ty) = self.find_field_type(target_ty, field_name) {
            return Ok(ty);
        }

        let diag = Diagnostic::error(
            ErrorCode::E0403,
            format!("no field `{field_name}` on type `{target_ty}`"),
            span,
        );
        self.diagnostics.push(diag.clone());
        Err(diag)
    }

    fn lookup_class_field(&self, class_name: &str, field_name: &str) -> Option<Type> {
        let mut curr = Some(class_name);
        while let Some(name) = curr {
            let def = self.env.lookup_class(name)?;
            if let Some(ty) = def.fields.get(field_name) {
                return Some(ty.clone());
            }
            if let Some(ty) = def.getters.get(field_name) {
                return Some(ty.clone());
            }
            curr = def.superclass.as_deref();
        }
        None
    }

    fn lookup_class_setter(&self, class_name: &str, field_name: &str) -> Option<Type> {
        let mut curr = Some(class_name);
        while let Some(name) = curr {
            let def = self.env.lookup_class(name)?;
            if let Some(ty) = def.setters.get(field_name) {
                return Some(ty.clone());
            }
            curr = def.superclass.as_deref();
        }
        None
    }

    fn validate_catch_error_type(
        &mut self,
        tn: &TypeNode,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let name = match &tn.kind {
            TypeNodeKind::Ident(id) => id.name.as_str(),
            TypeNodeKind::Path(segments) => segments.last().map(|id| id.name.as_str()).unwrap_or(""),
            _ => "",
        };
        if name.is_empty() {
            let diag = Diagnostic::error(
                ErrorCode::E0401,
                "catch type must name an error struct",
                tn.span,
            );
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }
        if self.env.lookup_struct(name).is_none() {
            let diag = Diagnostic::error(
                ErrorCode::E0402,
                format!("error struct `{name}` is not defined in any loaded module"),
                span,
            )
            .with_note("check the spelling; error structs of loaded modules are available in `catch` without an explicit import");
            self.diagnostics.push(diag.clone());
            return Err(diag);
        }
        Ok(())
    }

    fn infer_sus(
        &mut self,
        body: &Block,
        catches: &[CatchClause],
        finally_branch: Option<&Block>,
    ) -> Result<Type, Diagnostic> {
        let body_ty = self.check_block(body, None)?;
        let mut result_types = vec![body_ty];

        for catch in catches {
            self.env.push_scope();
            self.env.insert_var("errkind".to_string(), Type::String, false, catch.span);
            if let Some(type_node) = &catch.error_type {
                self.validate_catch_error_type(type_node, catch.span)?;
                if let Some(err_var) = &catch.error_var {
                    let err_ty = self.resolve_type_node(type_node);
                    self.env
                        .insert_var(err_var.name.clone(), err_ty, false, err_var.span);
                }
            }
            let catch_ty = self.check_block(&catch.body, None)?;
            self.env.pop_scope();
            result_types.push(catch_ty);
        }

        if let Some(fin) = finally_branch {
            let _ = self.check_block(fin, None)?;
        }

        Ok(Type::union(result_types))
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
