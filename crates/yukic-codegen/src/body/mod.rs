#![deny(unsafe_code)]

pub mod call;
pub mod expr;
pub mod stmt;

use std::collections::HashMap;
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module as LlvmModule;
use inkwell::types::{BasicType, BasicTypeEnum, PointerType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue, StructValue};
use yukic_ast::expr::{BinaryOp, Expr, ExprKind, LiteralValue};
use yukic_ast::item::{CParam, Param};
use yukic_ast::stmt::{Block, StmtKind};
use yukic_target::TargetTriple;
use yukic_types::Type;
use yukic_types::convert::ast_type_to_type;

use crate::cabi::CAbiManager;
use crate::exception::{CleanupAction, ScopeKind, ScopeStack};
use crate::intrinsic::IntrinsicManager;
use crate::region::RegionManager;
use crate::registry::TypeRegistry;
use crate::rodata::RoDataManager;
use crate::vtable::VTableManager;

#[derive(Clone, Debug)]
pub struct LocalVar<'ctx> {
    pub ptr: PointerValue<'ctx>,
    pub ty: BasicTypeEnum<'ctx>,
    pub semantic_ty: Option<Type>,
}

#[derive(Clone, Copy, Debug)]
pub struct LoopContext<'ctx> {
    pub continue_bb: BasicBlock<'ctx>,
    pub break_bb: BasicBlock<'ctx>,
}

pub struct BodyCompiler<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a LlvmModule<'ctx>,
    pub rodata: &'a mut RoDataManager<'ctx>,
    pub intrinsic: &'a IntrinsicManager<'ctx>,
    pub region: &'a RegionManager<'ctx>,
    pub vtable: &'a VTableManager<'ctx>,
    pub cabi: &'a CAbiManager<'ctx>,
    pub registry: &'a TypeRegistry,
    pub target: TargetTriple,
    pub region_ptr: Option<PointerValue<'ctx>>,
    pub dest_region: Option<PointerValue<'ctx>>,
    pub fn_val: FunctionValue<'ctx>,
    pub return_type: Option<BasicTypeEnum<'ctx>>,
    pub is_main: bool,
    pub scopes: Vec<HashMap<String, LocalVar<'ctx>>>,
    pub cleanups_stack: ScopeStack<'ctx>,
    pub loop_stack: Vec<LoopContext<'ctx>>,
    pub unwind_dest: Option<BasicBlock<'ctx>>,
    pub current_cleanup_bb: Option<BasicBlock<'ctx>>,
    pub in_cleanup: bool,
    pub current_class: Option<String>,
    pub panic_strategy: String,
}

impl<'a, 'ctx> BodyCompiler<'a, 'ctx> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        context: &'ctx Context,
        builder: &'a Builder<'ctx>,
        module: &'a LlvmModule<'ctx>,
        rodata: &'a mut RoDataManager<'ctx>,
        intrinsic: &'a IntrinsicManager<'ctx>,
        region: &'a RegionManager<'ctx>,
        vtable: &'a VTableManager<'ctx>,
        cabi: &'a CAbiManager<'ctx>,
        registry: &'a TypeRegistry,
        target: TargetTriple,
        region_ptr: Option<PointerValue<'ctx>>,
        dest_region: Option<PointerValue<'ctx>>,
        fn_val: FunctionValue<'ctx>,
        return_type: Option<BasicTypeEnum<'ctx>>,
        is_main: bool,
    ) -> Self {
        let mut cleanups_stack = ScopeStack::new();
        cleanups_stack.push_scope(ScopeKind::Function);

        Self {
            context,
            builder,
            module,
            rodata,
            intrinsic,
            region,
            vtable,
            cabi,
            registry,
            target,
            region_ptr,
            dest_region,
            fn_val,
            return_type,
            is_main,
            scopes: vec![HashMap::new()],
            cleanups_stack,
            loop_stack: Vec::new(),
            unwind_dest: None,
            current_cleanup_bb: None,
            in_cleanup: false,
            current_class: None,
            panic_strategy: "unwind".to_string(),
        }
    }

    pub fn with_panic_strategy(mut self, strategy: impl Into<String>) -> Self {
        self.panic_strategy = strategy.into();
        self
    }

    pub fn is_abort_strategy(&self) -> bool {
        self.panic_strategy == "abort"
    }

    pub fn lower_type(&self, ty: &Type) -> Result<BasicTypeEnum<'ctx>, String> {
        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer =
            crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
        lowerer.lower_type(ty)
    }

    pub fn ptr_type(&self) -> PointerType<'ctx> {
        self.context.ptr_type(AddressSpace::default())
    }

    pub fn get_or_declare_personality(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("yukino_personality") {
            return f;
        }
        let fn_ty = self.context.i32_type().fn_type(&[], true);
        self.module.add_function("yukino_personality", fn_ty, None)
    }

    pub fn get_or_declare_panic_handled(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("yukino_panic_handled") {
            return f;
        }
        let fn_ty = self.context.void_type().fn_type(&[], false);
        let f = self.module.add_function("yukino_panic_handled", fn_ty, None);
        f.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
        f
    }

    pub fn get_or_declare_get_message(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("yukino_exception_get_message") {
            return f;
        }
        let str_ty = self.context.struct_type(
            &[self.ptr_type().into(), self.context.i64_type().into()],
            false,
        );
        let fn_ty = str_ty.fn_type(&[self.ptr_type().into()], false);
        let f = self.module.add_function("yukino_exception_get_message", fn_ty, None);
        f.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
        f
    }

    pub fn get_or_declare_get_kind(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("yukino_exception_get_kind") {
            return f;
        }
        let str_ty = self.context.struct_type(
            &[self.ptr_type().into(), self.context.i64_type().into()],
            false,
        );
        let fn_ty = str_ty.fn_type(&[self.ptr_type().into()], false);
        let f = self.module.add_function("yukino_exception_get_kind", fn_ty, None);
        f.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
        f
    }

    pub fn get_or_declare_has_code(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("yukino_exception_has_code") {
            return f;
        }
        let fn_ty = self.context.i32_type().fn_type(&[self.ptr_type().into()], false);
        let f = self.module.add_function("yukino_exception_has_code", fn_ty, None);
        f.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
        f
    }

    pub fn get_or_declare_get_code(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("yukino_exception_get_code") {
            return f;
        }
        let fn_ty = self.context.i32_type().fn_type(&[self.ptr_type().into()], false);
        let f = self.module.add_function("yukino_exception_get_code", fn_ty, None);
        f.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
        f
    }

    pub fn get_or_declare_get_payload(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("yukino_exception_get_payload") {
            return f;
        }
        let fn_ty = self.ptr_type().fn_type(&[self.ptr_type().into()], false);
        let f = self.module.add_function("yukino_exception_get_payload", fn_ty, None);
        f.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
        f
    }

    pub fn get_or_declare_panic_abort_fn(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("yukino_panic_abort") {
            return f;
        }
        let ptr_t = self.ptr_type();
        let i64_t = self.context.i64_type();
        let i32_t = self.context.i32_type();
        let void_t = self.context.void_type();
        let fn_type = void_t.fn_type(
            &[
                ptr_t.into(),
                i64_t.into(),
                ptr_t.into(),
                i64_t.into(),
                i32_t.into(),
                i32_t.into(),
            ],
            false,
        );
        let f = self.module.add_function("yukino_panic_abort", fn_type, None);
        f.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
        f
    }

    pub fn get_or_declare_eh_typeid(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("llvm.eh.typeid.for.p0") {
            return f;
        }
        let fn_ty = self
            .context
            .i32_type()
            .fn_type(&[self.ptr_type().into()], false);
        self.module.add_function("llvm.eh.typeid.for.p0", fn_ty, None)
    }

    pub fn get_or_create_typeinfo(&self, name: &str) -> inkwell::values::GlobalValue<'ctx> {
        let sym_name = format!("_Y_TYPEINFO_{name}");
        if let Some(g) = self.module.get_global(&sym_name) {
            return g;
        }
        let global = self.module.add_global(self.ptr_type(), Some(AddressSpace::default()), &sym_name);
        global.set_initializer(&self.ptr_type().const_null());
        global.set_constant(true);
        global
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn add_local_with_ty(
        &mut self,
        name: String,
        ptr: PointerValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
        semantic_ty: Option<Type>,
    ) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, LocalVar { ptr, ty, semantic_ty });
        }
    }

    pub fn add_local(&mut self, name: String, ptr: PointerValue<'ctx>, ty: BasicTypeEnum<'ctx>) {
        self.add_local_with_ty(name, ptr, ty, None);
    }

    pub fn lookup_local(&self, name: &str) -> Option<&LocalVar<'ctx>> {
        for scope in self.scopes.iter().rev() {
            
            if let Some(var) = scope.get(name) { return Some(var); }
            
        }
        
        None
    }

    pub fn expr_semantic_ty(&self, expr: &Expr) -> Option<Type> {
        match &expr.kind {
            ExprKind::Ident(id) => {
                if let Some(l) = self.lookup_local(&id.name) {
                    if let Some(sem) = &l.semantic_ty { return Some(sem.clone()); }
                }
                self.registry.fn_types.get(&id.name).cloned()
            }
            ExprKind::This => self.current_class.as_ref().map(|c| Type::Class(c.clone())),
            ExprKind::Super => self.current_class.as_ref().and_then(|c| {
                self.registry.classes.get(c).and_then(|l| {
                    l.superclass.as_ref().map(|s| Type::Class(s.clone()))
                })
            }),
            ExprKind::Call(callee, args) => {
                let name = match &callee.kind {
                    ExprKind::Ident(id) => Some(id.name.as_str()),
                    ExprKind::Path(idents) => {
                        if idents.len() == 2 && idents[0].name == "rt" {
                            Some(idents[1].name.as_str())
                        } else {
                            idents.last().map(|id| id.name.as_str())
                        }
                    }
                    _ => None,
                };
                if let Some(id_name) = name {
                    if id_name == "cast" && args.len() >= 2 {
                        return self.cast_call_semantic_ty(&args[1]);
                    }
                    if id_name == "from_raw_parts" || id_name == "string::from_raw_parts" {
                        if args.len() == 2 {
                            return Some(Type::String);
                        } else if args.len() >= 3 {
                            if let Some(Type::Ptr(inner)) = self.expr_semantic_ty(&args[0]) {
                                if *inner == Type::Uint8 && id_name == "from_raw_parts" {
                                    return Some(Type::Bytes);
                                }
                                return Some(Type::Array(inner));
                            }
                            return Some(Type::Bytes);
                        }
                    }
                }
                self.callee_semantic_ty(callee, args.len())
            }
            ExprKind::ClassInit(callee, _) => {
                if let ExprKind::Ident(id) = &callee.kind {
                    Some(Type::Class(id.name.clone()))
                } else if let ExprKind::Path(idents) = &callee.kind {
                    idents.last().map(|id| Type::Class(id.name.clone()))
                } else {
                    None
                }
            }
            ExprKind::PtrCast(_, ty_node) => Some(ast_type_to_type(ty_node)),
            ExprKind::MethodCall(target, method, _) => {
                self.method_semantic_ty(target, &method.name)
            }
            ExprKind::SafeCall(target, method, args) => {
                if args.is_empty() {
                    if let Some(f_ty) = self.field_semantic_ty(target, &method.name) {
                        return Some(Type::nullable(f_ty));
                    }
                }
                self.method_semantic_ty(target, &method.name).map(Type::nullable)
            }
            ExprKind::FieldAccess(target, field) => {
                self.field_semantic_ty(target, &field.name)
            }
            ExprKind::Binary(op, left, _) => {
                if *op == BinaryOp::NullCoalesce {
                    self.expr_semantic_ty(left).map(|ty| ty.unwrap_nullable())
                } else {
                    let op_symbol = crate::mangle::binary_op_to_symbol(*op);
                    self.method_semantic_ty(left, op_symbol)
                }
            }
            ExprKind::Unary(op, operand) => {
                let op_symbol = crate::mangle::unary_op_to_symbol(*op);
                self.method_semantic_ty(operand, op_symbol)
            }
            ExprKind::IndexAccess(target, _) => {
                let actual = match self.expr_semantic_ty(target) {
                    Some(Type::Ref { inner, .. }) => Some(*inner),
                    other => other,
                };
                match actual {
                    Some(Type::Array(elem)) => Some(*elem),
                    _ => self.method_semantic_ty(target, "[]"),
                }
            }
            ExprKind::SliceAccess(target, _, _, _) => self.expr_semantic_ty(target),
            ExprKind::InterpolatedString(_) => Some(Type::String),
            ExprKind::StructInit(name, _) => Some(Type::Struct(name.name.clone())),
            ExprKind::ArrayInit(elems) => {
                let elem_ty = elems.first().and_then(|e| self.expr_semantic_ty(e)).unwrap_or(Type::Int32);
                Some(Type::Array(Box::new(elem_ty)))
            }
            ExprKind::ArrayAlloc(elem_ty, _) => {
                Some(Type::Array(Box::new(yukic_types::ast_type_to_type(elem_ty))))
            }
            ExprKind::Is(_, _) => Some(Type::Bool),
            ExprKind::Literal(lit) => match lit {
                LiteralValue::Int(_) => Some(Type::Int32),
                LiteralValue::UInt(_) => Some(Type::Uint32),
                LiteralValue::Float(_) => Some(Type::Float64),
                LiteralValue::Bool(_) => Some(Type::Bool),
                LiteralValue::String(_) => Some(Type::String),
                LiteralValue::Bytes(_) => Some(Type::Bytes),
                LiteralValue::Char(_) => Some(Type::Char),
                LiteralValue::Null => Some(Type::Null),
            },
            ExprKind::If { then_branch, .. } => {
                then_branch.stmts.last().and_then(|s| match &s.kind {
                    StmtKind::Expr(e) => self.expr_semantic_ty(e),
                    _ => None,
                })
            }
            ExprKind::Block(b) | ExprKind::RegionBlock(b) => {
                b.stmts.last().and_then(|s| match &s.kind {
                    StmtKind::Expr(e) => self.expr_semantic_ty(e),
                    _ => None,
                })
            }
            ExprKind::AnonymousFun(fun) => {
                let params = fun
                    .params
                    .iter()
                    .map(|p| {
                        let ty = p
                            .ty
                            .as_ref()
                            .map(yukic_types::ast_type_to_type)
                            .unwrap_or(Type::Int64);
                        yukic_types::TypeParam::new(ty, p.is_ref, p.is_mut)
                    })
                    .collect();
                let ret = fun
                    .return_type
                    .as_ref()
                    .map(yukic_types::ast_type_to_type)
                    .or_else(|| expr::deduce_closure_return_type(fun, &[], self))
                    .unwrap_or(Type::Void);
                Some(Type::Fn {
                    params,
                    ret: Box::new(ret),
                    is_cfun: false,
                })
            }
            ExprKind::Lambda(params, body) => {
                let ps = params
                    .iter()
                    .map(|_| yukic_types::TypeParam::value(Type::Int64))
                    .collect();
                let ret = self.expr_semantic_ty(body).unwrap_or(Type::Int64);
                Some(Type::Fn {
                    params: ps,
                    ret: Box::new(ret),
                    is_cfun: false,
                })
            }
            _ => None,
        }
    }

    fn cast_call_semantic_ty(&self, target_arg: &Expr) -> Option<Type> {
        match &target_arg.kind {
            ExprKind::Literal(LiteralValue::String(s)) => {
                Some(yukic_types::convert::parse_primitive_or_named(s))
            }
            ExprKind::Ident(target_id) => {
                Some(yukic_types::convert::parse_primitive_or_named(&target_id.name))
            }
            ExprKind::Call(inner_callee, inner_args) => {
                if let ExprKind::Ident(inner_id) = &inner_callee.kind {
                    if (inner_id.name == "typeof" || inner_id.name == "__typeof_intrinsic__") && !inner_args.is_empty() {
                        return self.expr_semantic_ty(&inner_args[0]);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn callee_semantic_ty(&self, callee: &Expr, arg_count: usize) -> Option<Type> {
        if let ExprKind::Ident(id) = &callee.kind {
            if self.registry.classes.contains_key(&id.name) {
                return Some(Type::Class(id.name.clone()));
            }
            if let Some(local) = self.lookup_local(&id.name) {
                if let Some(Type::Fn { ret, .. }) = &local.semantic_ty {
                    return Some((**ret).clone());
                }
            }
            if let Some(mangled) = self.registry.resolve_fun_symbol(&id.name, arg_count) {
                if let Some(ret) = self.registry.functions.get(&mangled) {
                    return Some(ret.clone());
                }
            }
            if let Some(ret) = self.registry.functions.get(&id.name) {
                return Some(ret.clone());
            }
            return None;
        }
        None
    }

    fn method_semantic_ty(&self, target: &Expr, method_name: &str) -> Option<Type> {
        if let Some(target_ty) = self.expr_semantic_ty(target) {
            let actual = match &target_ty {
                Type::Ref { inner, .. } => inner.as_ref(),
                other => other,
            };
            if matches!(actual, Type::String | Type::Bytes | Type::Array(_)) {
                match method_name {
                    "len" | "cap" => return Some(Type::Int64),
                    "is_empty" => return Some(Type::Bool),
                    "as_ptr" => {
                        return Some(match actual {
                            Type::Array(inner) => Type::Ptr(inner.clone()),
                            _ => Type::Ptr(Box::new(Type::Uint8)),
                        });
                    }
                    "cp" => return Some(actual.clone()),
                    "push" | "del" | "clear" | "reserve" => return Some(Type::Void),
                    "pop" => {
                        return Some(match actual {
                            Type::Array(inner) => Type::nullable((**inner).clone()),
                            Type::Bytes => Type::nullable(Type::Uint8),
                            _ => Type::Null,
                        });
                    }
                    _ => {}
                }
            }
            let unwrapped = target_ty.unwrap_nullable();
            let base_name = match unwrapped {
                Type::Class(name) | Type::Struct(name) | Type::Interface(name) => Some(name),
                _ => None,
            };
            if let Some(name) = base_name {
                let qualified = format!("{name}::{method_name}");
                if let Some(ret) = self.registry.functions.get(&qualified) {
                    return Some(ret.clone());
                }
                let mangled = crate::mangle::mangle_method_name(method_name, 1);
                let qualified_mangled = format!("{name}::{mangled}");
                if let Some(ret) = self.registry.functions.get(&qualified_mangled) {
                    return Some(ret.clone());
                }
            }
        }
        self.registry.functions.get(method_name).cloned()
    }

    pub fn find_class_method(
        &self,
        class_name: &str,
        method_name: &str,
        param_count: usize,
    ) -> Option<String> {
        let mangled = crate::mangle::mangle_method_name(method_name, param_count);
        let mut curr = Some(class_name.to_string());
        while let Some(ancestor) = curr {
            let candidate = crate::mangle::method_symbol_name(&ancestor, method_name, param_count);
            let qualified = format!("{ancestor}::{method_name}");
            let qualified_mangled = format!("{ancestor}::{mangled}");
            if self.module.get_function(&candidate).is_some()
                || self.registry.functions.contains_key(&qualified)
                || self.registry.functions.contains_key(&qualified_mangled)
            {
                return Some(candidate);
            }
            curr = self
                .registry
                .classes
                .get(&ancestor)
                .and_then(|c| c.superclass.clone());
        }
        None
    }

    fn field_semantic_ty(&self, target: &Expr, field_name: &str) -> Option<Type> {
        let target_ty = self.expr_semantic_ty(target)?;
        let unwrapped = target_ty.unwrap_nullable();
        match unwrapped {
            Type::Struct(name) => {
                if let Some(layout) = self.registry.structs.get(&name) {
                    return layout.fields.iter().find(|(f, _)| f == field_name).map(|(_, ty)| ty.clone());
                }
                // Uncanonicalized cstruct annotations arrive as Struct.
                let layout = self.registry.cstructs.get(&name)?;
                layout.fields.iter().find(|(f, _)| f == field_name).map(|(_, ty)| ty.clone())
            }
            Type::Cstruct(name) => {
                let layout = self.registry.cstructs.get(&name)?;
                layout.fields.iter().find(|(f, _)| f == field_name).map(|(_, ty)| ty.clone())
            }
            Type::Class(name) => {
                let layout = self.registry.classes.get(&name)?;
                layout.fields.iter().find(|(f, _)| f == field_name).map(|(_, ty)| ty.clone())
            }
            _ => None,
        }
    }

    pub fn is_terminated(&self) -> bool {
        self.builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_terminator)
            .is_some()
    }

    pub fn setup_params(&mut self, params: &[Param], has_dest: bool) -> Result<(), String> {
        let offset = usize::from(has_dest);
        for (i, param) in params.iter().enumerate() {
            let arg_idx = u32::try_from(offset + i).map_err(|e| e.to_string())?;
            let Some(param_val) = self.fn_val.get_nth_param(arg_idx) else {
                continue;
            };
            let mut sem_ty = param
                .ty
                .as_ref()
                .map(|t| self.registry.canonicalize_type(&ast_type_to_type(t)));
            if param.is_variadic {
                if let Some(t) = sem_ty {
                    sem_ty = Some(if matches!(t, Type::Array(_)) {
                        t
                    } else {
                        Type::Array(Box::new(t))
                    });
                }
            }

            if param.is_ref {
                let ptr = param_val.into_pointer_value();
                let inner_ty = if let Some(ref t) = sem_ty {
                    self.lower_type(t)?
                } else {
                    param_val.get_type()
                };
                self.add_local_with_ty(param.name.name.clone(), ptr, inner_ty, sem_ty);
                continue;
            }

            let param_ty = param_val.get_type();
            let alloca = self
                .builder
                .build_alloca(param_ty, &param.name.name)
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(alloca, param_val)
                .map_err(|e| e.to_string())?;
            self.add_local_with_ty(param.name.name.clone(), alloca, param_ty, sem_ty);
        }
        Ok(())
    }

    pub fn setup_method_params(
        &mut self,
        params: &[Param],
        has_this: bool,
        has_dest: bool,
    ) -> Result<(), String> {
        let mut arg_idx = u32::from(has_dest);
        if has_this {
            if let Some(this_val) = self.fn_val.get_nth_param(arg_idx) {
                let this_ty = this_val.get_type();
                let alloca = self
                    .builder
                    .build_alloca(this_ty, "this")
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(alloca, this_val)
                    .map_err(|e| e.to_string())?;
                let sem_ty = self.current_class.as_ref().map(|c| Type::Class(c.clone()));
                self.add_local_with_ty("this".to_string(), alloca, this_ty, sem_ty);
            }
            arg_idx += 1;
        }

        for param in params {
            let Some(param_val) = self.fn_val.get_nth_param(arg_idx) else {
                continue;
            };
            let mut sem_ty = param
                .ty
                .as_ref()
                .map(|t| self.registry.canonicalize_type(&ast_type_to_type(t)));
            if param.is_variadic {
                if let Some(t) = sem_ty {
                    sem_ty = Some(if matches!(t, Type::Array(_)) {
                        t
                    } else {
                        Type::Array(Box::new(t))
                    });
                }
            }

            if param.is_ref {
                let ptr = param_val.into_pointer_value();
                let inner_ty = if let Some(ref t) = sem_ty {
                    self.lower_type(t)?
                } else {
                    param_val.get_type()
                };
                self.add_local_with_ty(param.name.name.clone(), ptr, inner_ty, sem_ty);
                arg_idx += 1;
                continue;
            }

            let param_ty = param_val.get_type();
            let alloca = self
                .builder
                .build_alloca(param_ty, &param.name.name)
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(alloca, param_val)
                .map_err(|e| e.to_string())?;
            self.add_local_with_ty(param.name.name.clone(), alloca, param_ty, sem_ty);
            arg_idx += 1;
        }
        Ok(())
    }

    pub fn setup_cparams(&mut self, params: &[CParam]) -> Result<(), String> {
        for (i, param) in params.iter().enumerate() {
            let arg_idx = u32::try_from(i).map_err(|e| e.to_string())?;
            let Some(param_val) = self.fn_val.get_nth_param(arg_idx) else {
                continue;
            };
            let param_ty = param_val.get_type();
            let alloca = self
                .builder
                .build_alloca(param_ty, &param.name.name)
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(alloca, param_val)
                .map_err(|e| e.to_string())?;
            let sem_ty = ast_type_to_type(&param.ty);
            self.add_local_with_ty(param.name.name.clone(), alloca, param_ty, Some(sem_ty));
        }
        Ok(())
    }

    pub fn to_bool(&self, val: BasicValueEnum<'ctx>) -> Result<IntValue<'ctx>, String> {
        match val {
            BasicValueEnum::IntValue(iv) => {
                if iv.get_type().get_bit_width() == 1 {
                    Ok(iv)
                } else {
                    let zero = iv.get_type().const_zero();
                    self.builder
                        .build_int_compare(inkwell::IntPredicate::NE, iv, zero, "tobool")
                        .map_err(|e| e.to_string())
                }
            }
            BasicValueEnum::PointerValue(pv) => {
                let null = self.ptr_type().const_null();
                self.builder
                    .build_int_compare(inkwell::IntPredicate::NE, pv, null, "tobool")
                    .map_err(|e| e.to_string())
            }
            _ => Err("cannot convert value to boolean".to_string()),
        }
    }

    pub fn coerce_to_interface(
        &self,
        instance_ptr: PointerValue<'ctx>,
        iface_name: &str,
    ) -> Result<StructValue<'ctx>, String> {
        let fat_ty = self.vtable.interface_fat_type();
        let null_ptr = self.ptr_type().const_null();

        let is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, instance_ptr, null_ptr, "fat.is_null")
            .map_err(|e| e.to_string())?;

        let curr_bb = self.builder.get_insert_block().unwrap();
        let non_null_bb = self.context.append_basic_block(self.fn_val, "fat.non_null");
        let merge_bb = self.context.append_basic_block(self.fn_val, "fat.merge");

        self.builder
            .build_conditional_branch(is_null, merge_bb, non_null_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(non_null_bb);

        let vt_ptr = self
            .builder
            .build_load(self.ptr_type(), instance_ptr, "fat.vt_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();
        let actual_tid = self
            .builder
            .build_load(self.context.i64_type(), vt_ptr, "fat.tid")
            .map_err(|e| e.to_string())?
            .into_int_value();

        let mut impl_classes = Vec::new();
        for (cname, _) in &self.registry.classes {
            if self.registry.class_implements_interface(cname, iface_name) {
                if let Some(tid) = self.vtable.get_class_type_id(cname) {
                    let itable_name = crate::mangle::itable_symbol_name(cname, iface_name);
                    impl_classes.push((cname.clone(), tid, itable_name));
                }
            }
        }

        let selected_itable = if impl_classes.is_empty() {
            null_ptr
        } else if impl_classes.len() == 1 {
            let (_, _, it_name) = &impl_classes[0];
            self.module
                .get_global(it_name)
                .map(|g| g.as_pointer_value())
                .unwrap_or(null_ptr)
        } else {
            let mut curr_sel = null_ptr;
            for (_, tid, it_name) in &impl_classes {
                if let Some(it_global) = self.module.get_global(it_name) {
                    let expected_tid = self.context.i64_type().const_int(*tid, false);
                    let is_match = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, actual_tid, expected_tid, "tid.match")
                        .map_err(|e| e.to_string())?;
                    let sel = self
                        .builder
                        .build_select(is_match, it_global.as_pointer_value(), curr_sel, "it.sel")
                        .map_err(|e| e.to_string())?
                        .into_pointer_value();
                    curr_sel = sel;
                }
            }
            curr_sel
        };

        let undef = fat_ty.get_undef();
        let f1 = self
            .builder
            .build_insert_value(undef, instance_ptr, 0, "fat.inst")
            .map_err(|e| e.to_string())?;
        let f2 = self
            .builder
            .build_insert_value(f1, selected_itable, 1, "fat.itable")
            .map_err(|e| e.to_string())?
            .into_struct_value();

        let non_null_end_bb = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(fat_ty, "fat.res")
            .map_err(|e| e.to_string())?;

        let null_struct: BasicValueEnum<'ctx> = fat_ty.const_zero().into();
        let non_null_struct: BasicValueEnum<'ctx> = f2.into();

        phi.add_incoming(&[
            (&null_struct, curr_bb),
            (&non_null_struct, non_null_end_bb),
        ]);

        Ok(phi.as_basic_value().into_struct_value())
    }

    pub fn coerce_type(
        &self,
        val: BasicValueEnum<'ctx>,
        target: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if val.get_type() == target {
            return Ok(val);
        }

        if target.is_pointer_type() && !val.is_pointer_value() {
            let alloca = self
                .builder
                .build_alloca(val.get_type(), "arg.spill")
                .map_err(|e| e.to_string())?;
            self.builder.build_store(alloca, val).map_err(|e| e.to_string())?;
            return Ok(alloca.into());
        }

        if let BasicTypeEnum::StructType(target_st) = target {
            if target_st.count_fields() == 2
                && target_st.get_field_type_at_index(0).map_or(false, |t| t.is_pointer_type())
                && target_st.get_field_type_at_index(1).map_or(false, |t| t.is_pointer_type())
                && val.is_pointer_value()
            {
                let raw_ptr = val.into_pointer_value();
                let actual_fn_ptr = self
                    .get_or_create_thunk_for_fn_ptr(raw_ptr)
                    .unwrap_or(raw_ptr);
                let undef = target_st.get_undef();
                let s1 = self
                    .builder
                    .build_insert_value(undef, actual_fn_ptr, 0, "closure.fn")
                    .map_err(|e| e.to_string())?;
                let null_env = self.ptr_type().const_null();
                let s2 = self
                    .builder
                    .build_insert_value(s1, null_env, 1, "closure.env")
                    .map_err(|e| e.to_string())?
                    .into_struct_value();
                return Ok(s2.into());
            }

            if target_st.count_fields() == 2
                && target_st.get_field_type_at_index(0) == Some(self.context.bool_type().into())
            {
                let inner_ty = target_st.get_field_type_at_index(1).unwrap();
                if val.is_pointer_value() && val.into_pointer_value().is_null() {
                    return Ok(target_st.const_zero().into());
                }

                if let BasicValueEnum::StructValue(src_sv) = val {
                    if src_sv.get_type().count_fields() == 2
                        && src_sv.get_type().get_field_type_at_index(0)
                            == Some(self.context.bool_type().into())
                    {
                        let tag = self
                            .builder
                            .build_extract_value(src_sv, 0, "opt.tag")
                            .map_err(|e| e.to_string())?;
                        let src_inner = self
                            .builder
                            .build_extract_value(src_sv, 1, "opt.val")
                            .map_err(|e| e.to_string())?;
                        let coerced_inner = self.coerce_type(src_inner, inner_ty)?;
                        let undef = target_st.get_undef();
                        let s1 = self
                            .builder
                            .build_insert_value(undef, tag, 0, "opt.tag")
                            .map_err(|e| e.to_string())?;
                        let s2 = self
                            .builder
                            .build_insert_value(s1, coerced_inner, 1, "opt.val")
                            .map_err(|e| e.to_string())?
                            .into_struct_value();
                        return Ok(s2.into());
                    }
                }

                let coerced_inner = self.coerce_type(val, inner_ty)?;
                let true_flag = self.context.bool_type().const_int(1, false);
                let undef = target_st.get_undef();
                let s1 = self
                    .builder
                    .build_insert_value(undef, true_flag, 0, "opt.some.tag")
                    .map_err(|e| e.to_string())?;
                let s2 = self
                    .builder
                    .build_insert_value(s1, coerced_inner, 1, "opt.some.val")
                    .map_err(|e| e.to_string())?
                    .into_struct_value();
                return Ok(s2.into());
            }
        }

        match (val, target) {
            (BasicValueEnum::FloatValue(fv), BasicTypeEnum::FloatType(ft)) => {
                let src_ty = fv.get_type();
                if src_ty == ft {
                    Ok(val)
                } else if src_ty == self.context.f64_type() && ft == self.context.f32_type() {
                    let tr = self
                        .builder
                        .build_float_trunc(fv, ft, "fptrunc")
                        .map_err(|e| e.to_string())?;
                    Ok(tr.into())
                } else if src_ty == self.context.f32_type() && ft == self.context.f64_type() {
                    let ext = self
                        .builder
                        .build_float_ext(fv, ft, "fpext")
                        .map_err(|e| e.to_string())?;
                    Ok(ext.into())
                } else {
                    Ok(val)
                }
            }
            (BasicValueEnum::IntValue(iv), BasicTypeEnum::IntType(it)) => {
                let src_w = iv.get_type().get_bit_width();
                let dst_w = it.get_bit_width();
                match src_w.cmp(&dst_w) {
                    std::cmp::Ordering::Less => {
                        let ext = self
                            .builder
                            .build_int_s_extend(iv, it, "sext")
                            .map_err(|e| e.to_string())?;
                        Ok(ext.into())
                    }
                    std::cmp::Ordering::Greater => {
                        let tr = self
                            .builder
                            .build_int_truncate(iv, it, "trunc")
                            .map_err(|e| e.to_string())?;
                        Ok(tr.into())
                    }
                    std::cmp::Ordering::Equal => Ok(val),
                }
            }
            (BasicValueEnum::IntValue(iv), BasicTypeEnum::PointerType(_)) => {
                let ptr = self
                    .builder
                    .build_int_to_ptr(iv, self.ptr_type(), "inttoptr")
                    .map_err(|e| e.to_string())?;
                Ok(ptr.into())
            }
            (BasicValueEnum::PointerValue(pv), BasicTypeEnum::IntType(it)) => {
                let i = self
                    .builder
                    .build_ptr_to_int(pv, it, "ptrtoint")
                    .map_err(|e| e.to_string())?;
                Ok(i.into())
            }
            _ => Ok(val),
        }
    }

    fn get_or_create_thunk_for_fn_ptr(&self, raw_ptr: PointerValue<'ctx>) -> Option<PointerValue<'ctx>> {
        let name_str = raw_ptr.get_name().to_str().ok()?;
        if name_str.is_empty()
            || name_str.starts_with("__yuki_closure_")
            || name_str.starts_with("__yuki_thunk_")
        {
            return Some(raw_ptr);
        }
        let orig_fn = self.module.get_function(name_str)?;
        let thunk_name = format!("__yuki_thunk_{name_str}");
        if let Some(thunk) = self.module.get_function(&thunk_name) {
            return Some(thunk.as_global_value().as_pointer_value());
        }

        let orig_ty = orig_fn.get_type();
        let orig_params = orig_ty.get_param_types();
        let has_dest = orig_params.first().map_or(false, |p| {
            p.is_pointer_type()
                && orig_fn
                    .get_first_param()
                    .map_or(false, |fp| fp.get_name().to_str().unwrap_or("").starts_with("_dest"))
        });

        let mut thunk_params: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> =
            Vec::with_capacity(orig_params.len() + 1);
        if has_dest {
            thunk_params.push(orig_params[0].into());
        }
        thunk_params.push(self.ptr_type().into()); // env
        let start_idx = if has_dest { 1 } else { 0 };
        for p in &orig_params[start_idx..] {
            thunk_params.push((*p).into());
        }

        let ret_ty = orig_ty.get_return_type();
        let thunk_fn_ty = match ret_ty {
            Some(rt) => rt.fn_type(&thunk_params, false),
            None => self.context.void_type().fn_type(&thunk_params, false),
        };

        let thunk_fn = self.module.add_function(&thunk_name, thunk_fn_ty, None);
        thunk_fn.set_linkage(inkwell::module::Linkage::Internal);
        thunk_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);

        let old_bb = self.builder.get_insert_block();
        let thunk_bb = self.context.append_basic_block(thunk_fn, "entry");
        self.builder.position_at_end(thunk_bb);

        let mut forward_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
            Vec::with_capacity(orig_params.len());
        if has_dest {
            forward_args.push(thunk_fn.get_first_param().unwrap().into());
        }
        let user_start = if has_dest { 2 } else { 1 };
        for i in 0..(orig_params.len() - if has_dest { 1 } else { 0 }) {
            forward_args.push(
                thunk_fn
                    .get_nth_param((user_start + i) as u32)
                    .unwrap()
                    .into(),
            );
        }

        let call_res = self
            .builder
            .build_call(orig_fn, &forward_args, "call_orig")
            .ok()?;
        call_res.set_call_convention(orig_fn.get_call_conventions());
        if ret_ty.is_some() {
            let ret_val = call_res.try_as_basic_value().basic()?;
            self.builder.build_return(Some(&ret_val)).ok()?;
        } else {
            self.builder.build_return(None).ok()?;
        }

        if let Some(bb) = old_bb {
            self.builder.position_at_end(bb);
        }

        Some(thunk_fn.as_global_value().as_pointer_value())
    }

    pub fn compile_block(
        &mut self,
        block: &Block,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        self.cleanups_stack.push_scope(ScopeKind::Block);
        self.current_cleanup_bb = None;
        self.push_scope();

        let mut last_val = None;
        for stmt in &block.stmts {
            if self.is_terminated() {
                break;
            }
            last_val = self.compile_stmt(stmt)?;
        }

        if !self.is_terminated() {
            let cleanups = self.cleanups_stack.current_scope_cleanups_lifo();
            for cleanup in &cleanups {
                self.execute_cleanup(cleanup)?;
            }
        }

        self.cleanups_stack.pop_scope();
        self.current_cleanup_bb = None;
        self.pop_scope();
        Ok(last_val)
    }

    pub fn compile_region_block(
        &mut self,
        block: &Block,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        self.cleanups_stack.push_scope(ScopeKind::Block);
        self.current_cleanup_bb = None;
        self.push_scope();

        if let Some(reg) = self.region_ptr {
            let mark = self.region.emit_loop_save_mark(self.module, self.builder, reg)?;
            self.cleanups_stack
                .add_cleanup(CleanupAction::RegionRestoreMark { region: reg, mark })?;
        }

        let mut last_val = None;
        for stmt in &block.stmts {
            if self.is_terminated() {
                break;
            }
            last_val = self.compile_stmt(stmt)?;
        }

        if !self.is_terminated() {
            let cleanups = self.cleanups_stack.current_scope_cleanups_lifo();
            for cleanup in &cleanups {
                self.execute_cleanup(cleanup)?;
            }
        }

        self.cleanups_stack.pop_scope();
        self.current_cleanup_bb = None;
        self.pop_scope();
        Ok(last_val)
    }

    pub fn has_active_cleanups(&self) -> bool {
        if self.in_cleanup {
            return false;
        }
        self.cleanups_stack.has_any_cleanups()
    }

    pub fn execute_cleanup(&mut self, cleanup: &CleanupAction<'ctx>) -> Result<(), String> {
        let old_in_cleanup = self.in_cleanup;
        self.in_cleanup = true;
        let res = match cleanup {
            CleanupAction::Defer(block) => {
                let old_unwind = self.unwind_dest;
                self.unwind_dest = None;
                let val = self.compile_block(block).map(|_| ());
                self.unwind_dest = old_unwind;
                val
            }
            CleanupAction::RegionEpilogue(reg) => {
                self.region.emit_region_epilogue(self.module, self.builder, *reg)
            }
            CleanupAction::RegionRestoreMark { region, mark } => {
                self.region.emit_loop_restore_mark(self.module, self.builder, *region, *mark)
            }
            CleanupAction::Custom(f) => {
                f(self.builder)
            }
        };
        self.in_cleanup = old_in_cleanup;
        res
    }

    pub fn get_or_create_cleanup_unwind_bb(&mut self) -> Result<BasicBlock<'ctx>, String> {
        if let Some(bb) = self.current_cleanup_bb {
            return Ok(bb);
        }

        let personality = self.get_or_declare_personality();
        self.fn_val.set_personality_function(personality);

        let cleanup_bb = self.context.append_basic_block(self.fn_val, "defer.landing");
        let current_bb = self.builder.get_insert_block();

        self.builder.position_at_end(cleanup_bb);

        let pad_type = self.context.struct_type(
            &[self.ptr_type().into(), self.context.i32_type().into()],
            false,
        );
        let pad_val = self
            .builder
            .build_landing_pad(pad_type, personality, &[], true, "eh_defer")
            .map_err(|e| e.to_string())?;

        let cleanups = self.cleanups_stack.collect_return_cleanups();

        let old_in_cleanup = self.in_cleanup;
        self.in_cleanup = true;

        for cleanup in &cleanups {
            self.execute_cleanup(cleanup)?;
        }

        if let Some(reg) = self.region_ptr {
            self.region.emit_region_epilogue(self.module, self.builder, reg)?;
        }

        self.in_cleanup = old_in_cleanup;

        self.builder
            .build_resume(pad_val)
            .map_err(|e| e.to_string())?;

        if let Some(bb) = current_bb {
            self.builder.position_at_end(bb);
        }

        self.current_cleanup_bb = Some(cleanup_bb);
        Ok(cleanup_bb)
    }
}

#[cfg(test)]
mod tests {
    use inkwell::context::Context;
    use yukic_ast::expr::{BinaryOp, Expr, ExprKind, LiteralValue};
    use yukic_ast::ident::Ident;
    use yukic_ast::item::{FunDecl, Item, ItemId, ItemKind, Param, Visibility};
    use yukic_ast::stmt::{BindingTarget, Block, Stmt, StmtKind};
    use yukic_ast::types::{TypeNode, TypeNodeKind};
    use yukic_ast::Module as AstModule;
    use yukic_span::{BytePos, FileId, Span};
    use yukic_target::TargetTriple;

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(0))
    }

    #[test]
    fn test_compile_fun_with_set_and_arithmetic() {
        let sp = dummy_span();
        let stmts = vec![
            Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(Ident::new("x", sp)),
                    ty: None,
                    init: Expr::new(ExprKind::Literal(LiteralValue::Int(10)), sp),
                },
                sp,
            ),
            Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(Ident::new("y", sp)),
                    ty: None,
                    init: Expr::new(ExprKind::Literal(LiteralValue::Int(20)), sp),
                },
                sp,
            ),
            Stmt::new(
                StmtKind::Return(Some(Expr::new(
                    ExprKind::Binary(
                        BinaryOp::Add,
                        Box::new(Expr::new(ExprKind::Ident(Ident::new("x", sp)), sp)),
                        Box::new(Expr::new(ExprKind::Ident(Ident::new("y", sp)), sp)),
                    ),
                    sp,
                ))),
                sp,
            ),
        ];

        let fun = FunDecl {
            name: Ident::new("add_vars", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: Some(yukic_ast::TypeNode::new(
                yukic_ast::TypeNodeKind::Ident(Ident::new("int64", sp)),
                sp,
            )),
            body: Block::new(stmts, sp),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(
            ItemId(1),
            Visibility::Public,
            ItemKind::Fun(fun),
            sp,
        ));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(&context, "test_body", TargetTriple::host());
        codegen.compile_module(&ast).expect("compile module failed");

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("add i64"));
        assert!(ir.contains("ret"));
    }

    #[test]
    fn test_compile_main_calling_println() {
        let sp = dummy_span();
        let call_stmt = Stmt::new(
            StmtKind::Expr(Expr::new(
                ExprKind::Call(
                    Box::new(Expr::new(ExprKind::Ident(Ident::new("println", sp)), sp)),
                    vec![
                        Expr::new(ExprKind::Literal(LiteralValue::String("hello".into())), sp),
                        Expr::new(ExprKind::Literal(LiteralValue::Bool(true)), sp),
                    ],
                ),
                sp,
            )),
            sp,
        );

        let fun = FunDecl {
            name: Ident::new("main", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: Block::new(vec![call_stmt], sp),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(
            ItemId(1),
            Visibility::Public,
            ItemKind::Fun(fun),
            sp,
        ));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(&context, "test_main", TargetTriple::host());
        assert!(codegen.compile_module(&ast).is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("call fastcc void @println"));
        assert!(ir.contains("ret i32 0"));
    }

    #[test]
    fn test_compile_nullable_coalesce_and_null_checks() {
        let sp = dummy_span();

        // fun test_coal(x: int64?) -> int64 { return x ?? 100 }
        let coal_stmt = Stmt::new(
            StmtKind::Return(Some(Expr::new(
                ExprKind::Binary(
                    BinaryOp::NullCoalesce,
                    Box::new(Expr::new(ExprKind::Ident(Ident::new("x", sp)), sp)),
                    Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(100)), sp)),
                ),
                sp,
            ))),
            sp,
        );

        let fun1 = FunDecl {
            name: Ident::new("test_coal", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![Param {
                name: Ident::new("x", sp),
                ty: Some(TypeNode::new(
                    TypeNodeKind::Optional(Box::new(TypeNode::new(
                        TypeNodeKind::Ident(Ident::new("int64", sp)),
                        sp,
                    ))),
                    sp,
                )),
                default_val: None,
                is_mut: false,
                is_ref: false,
                is_variadic: false,
                span: sp,
            }],
            return_type: Some(TypeNode::new(
                TypeNodeKind::Ident(Ident::new("int64", sp)),
                sp,
            )),
            body: Block::new(vec![coal_stmt], sp),
            span: sp,
        };

        // fun test_null_eq(x: int64?) -> bool { return x == null }
        let eq_stmt = Stmt::new(
            StmtKind::Return(Some(Expr::new(
                ExprKind::Binary(
                    BinaryOp::Eq,
                    Box::new(Expr::new(ExprKind::Ident(Ident::new("x", sp)), sp)),
                    Box::new(Expr::new(ExprKind::Literal(LiteralValue::Null), sp)),
                ),
                sp,
            ))),
            sp,
        );

        let fun2 = FunDecl {
            name: Ident::new("test_null_eq", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![Param {
                name: Ident::new("x", sp),
                ty: Some(TypeNode::new(
                    TypeNodeKind::Optional(Box::new(TypeNode::new(
                        TypeNodeKind::Ident(Ident::new("int64", sp)),
                        sp,
                    ))),
                    sp,
                )),
                default_val: None,
                is_mut: false,
                is_ref: false,
                is_variadic: false,
                span: sp,
            }],
            return_type: Some(TypeNode::new(
                TypeNodeKind::Ident(Ident::new("bool", sp)),
                sp,
            )),
            body: Block::new(vec![eq_stmt], sp),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Fun(fun1), sp));
        ast.items.push(Item::new(ItemId(2), Visibility::Public, ItemKind::Fun(fun2), sp));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(&context, "test_null", TargetTriple::host());
        assert!(codegen.compile_module(&ast).is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("coal.rhs"));
        assert!(ir.contains("coal.merge"));
        assert!(ir.contains("coal.res = phi i64"));
        assert!(ir.contains("is_null"));
    }

    #[test]
    fn test_compile_fun_with_defer_lifo() {
        let sp = dummy_span();
        let stmts = vec![
            Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(Ident::new("x", sp)),
                    ty: None,
                    init: Expr::new(ExprKind::Literal(LiteralValue::Int(1)), sp),
                },
                sp,
            ),
            Stmt::new(
                StmtKind::Defer(Block::new(
                    vec![Stmt::new(
                        StmtKind::Rewrite {
                            target: yukic_ast::stmt::LValue::Ident(Ident::new("x", sp)),
                            op: yukic_ast::stmt::AssignOp::Eq,
                            value: Expr::new(ExprKind::Literal(LiteralValue::Int(10)), sp),
                        },
                        sp,
                    )],
                    sp,
                )),
                sp,
            ),
            Stmt::new(
                StmtKind::Defer(Block::new(
                    vec![Stmt::new(
                        StmtKind::Rewrite {
                            target: yukic_ast::stmt::LValue::Ident(Ident::new("x", sp)),
                            op: yukic_ast::stmt::AssignOp::Eq,
                            value: Expr::new(ExprKind::Literal(LiteralValue::Int(20)), sp),
                        },
                        sp,
                    )],
                    sp,
                )),
                sp,
            ),
            Stmt::new(
                StmtKind::Return(Some(Expr::new(
                    ExprKind::Ident(Ident::new("x", sp)),
                    sp,
                ))),
                sp,
            ),
        ];

        let fun = FunDecl {
            name: Ident::new("test_defer", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: Some(TypeNode::new(
                TypeNodeKind::Ident(Ident::new("int64", sp)),
                sp,
            )),
            body: Block::new(stmts, sp),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Fun(fun), sp));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(&context, "test_defer_mod", TargetTriple::host());
        assert!(codegen.compile_module(&ast).is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("store i64 20"));
        assert!(ir.contains("store i64 10"));
        assert!(ir.contains("personality ptr @yukino_personality"));
        let pos20 = ir.find("store i64 20").unwrap();
        let pos10 = ir.find("store i64 10").unwrap();
        assert!(pos20 < pos10, "LIFO: defer 20 should execute before defer 10");
    }

    #[test]
    fn test_compile_fun_with_defer_and_invoke() {
        let sp = dummy_span();
        let stmts = vec![
            Stmt::new(
                StmtKind::Defer(Block::new(
                    vec![Stmt::new(
                        StmtKind::Expr(Expr::new(
                            ExprKind::Call(
                                Box::new(Expr::new(ExprKind::Ident(Ident::new("cleanup_func", sp)), sp)),
                                vec![],
                            ),
                            sp,
                        )),
                        sp,
                    )],
                    sp,
                )),
                sp,
            ),
            Stmt::new(
                StmtKind::Expr(Expr::new(
                    ExprKind::Call(
                        Box::new(Expr::new(ExprKind::Ident(Ident::new("risky_call", sp)), sp)),
                        vec![],
                    ),
                    sp,
                )),
                sp,
            ),
        ];

        let fun = FunDecl {
            name: Ident::new("test_defer_unwind", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: Block::new(stmts, sp),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Fun(fun), sp));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(&context, "test_defer_unwind_mod", TargetTriple::host());
        assert!(codegen.compile_module(&ast).is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("personality ptr @yukino_personality"));
        assert!(ir.contains("invoke fastcc void @risky_call()"));
        assert!(ir.contains("landingpad { ptr, i32 }"));
        assert!(ir.contains("cleanup"));
        assert!(ir.contains("call fastcc void @cleanup_func()"));
        assert!(ir.contains("resume { ptr, i32 }"));
    }

    #[test]
    fn test_compile_defer_in_loop_break() {
        let sp = dummy_span();
        let loop_body = Block::new(
            vec![
                Stmt::new(
                    StmtKind::Defer(Block::new(
                        vec![Stmt::new(
                            StmtKind::Expr(Expr::new(
                                ExprKind::Call(
                                    Box::new(Expr::new(ExprKind::Ident(Ident::new("cleanup_loop", sp)), sp)),
                                    vec![],
                                ),
                                sp,
                            )),
                            sp,
                        )],
                        sp,
                    )),
                    sp,
                ),
                Stmt::new(StmtKind::Break(None), sp),
            ],
            sp,
        );

        let fun = FunDecl {
            name: Ident::new("test_loop_defer", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: Block::new(
                vec![Stmt::new(
                    StmtKind::Expr(Expr::new(
                        ExprKind::Loop {
                            label: None,
                            body: loop_body,
                        },
                        sp,
                    )),
                    sp,
                )],
                sp,
            ),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Fun(fun), sp));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(&context, "test_loop_defer_mod", TargetTriple::host());
        assert!(codegen.compile_module(&ast).is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("call fastcc void @cleanup_loop()"));
        assert!(ir.contains("br label %loop.exit"));
    }

    #[test]
    fn test_compile_labeled_loop_break_and_continue_cleanups() {
        let sp = dummy_span();
        let inner_loop = Expr::new(
            ExprKind::Loop {
                label: Some(Ident::new("inner", sp)),
                body: Block::new(
                    vec![
                        Stmt::new(
                            StmtKind::Defer(Block::new(
                                vec![Stmt::new(
                                    StmtKind::Expr(Expr::new(
                                        ExprKind::Call(
                                            Box::new(Expr::new(ExprKind::Ident(Ident::new("cleanup_inner", sp)), sp)),
                                            vec![],
                                        ),
                                        sp,
                                    )),
                                    sp,
                                )],
                                sp,
                            )),
                            sp,
                        ),
                        Stmt::new(StmtKind::Break(Some(Ident::new("outer", sp))), sp),
                    ],
                    sp,
                ),
            },
            sp,
        );

        let outer_loop = Expr::new(
            ExprKind::Loop {
                label: Some(Ident::new("outer", sp)),
                body: Block::new(
                    vec![
                        Stmt::new(
                            StmtKind::Defer(Block::new(
                                vec![Stmt::new(
                                    StmtKind::Expr(Expr::new(
                                        ExprKind::Call(
                                            Box::new(Expr::new(ExprKind::Ident(Ident::new("cleanup_outer", sp)), sp)),
                                            vec![],
                                        ),
                                        sp,
                                    )),
                                    sp,
                                )],
                                sp,
                            )),
                            sp,
                        ),
                        Stmt::new(StmtKind::Expr(inner_loop), sp),
                    ],
                    sp,
                ),
            },
            sp,
        );

        let fun = FunDecl {
            name: Ident::new("test_nested_labeled_break", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: Block::new(vec![Stmt::new(StmtKind::Expr(outer_loop), sp)], sp),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Fun(fun), sp));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(&context, "test_labeled_break_mod", TargetTriple::host());
        assert!(codegen.compile_module(&ast).is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("call fastcc void @cleanup_inner()"));
        assert!(ir.contains("call fastcc void @cleanup_outer()"));
        // Both cleanups executed in order before break to outer loop exit
        let idx_inner = ir.find("call fastcc void @cleanup_inner()").unwrap();
        let idx_outer = ir.find("call fastcc void @cleanup_outer()").unwrap();
        assert!(idx_inner < idx_outer);
    }

    #[test]
    fn test_compile_defer_in_sus() {
        let sp = dummy_span();
        let sus_body = Block::new(
            vec![
                Stmt::new(
                    StmtKind::Defer(Block::new(
                        vec![Stmt::new(
                            StmtKind::Expr(Expr::new(
                                ExprKind::Call(
                                    Box::new(Expr::new(ExprKind::Ident(Ident::new("cleanup_sus", sp)), sp)),
                                    vec![],
                                ),
                                sp,
                            )),
                            sp,
                        )],
                        sp,
                    )),
                    sp,
                ),
                Stmt::new(
                    StmtKind::Expr(Expr::new(
                        ExprKind::Call(
                            Box::new(Expr::new(ExprKind::Ident(Ident::new("risky_call", sp)), sp)),
                            vec![],
                        ),
                        sp,
                    )),
                    sp,
                ),
            ],
            sp,
        );

        let catches = vec![
            yukic_ast::expr::CatchClause {
                error_var: Some(Ident::new("e", sp)),
                error_type: None,
                body: Block::new(
                    vec![Stmt::new(
                        StmtKind::Expr(Expr::new(
                            ExprKind::Call(
                                Box::new(Expr::new(ExprKind::Ident(Ident::new("handle_err", sp)), sp)),
                                vec![],
                            ),
                            sp,
                        )),
                        sp,
                    )],
                    sp,
                ),
                span: sp,
            },
        ];

        let finally_b = Block::new(
            vec![Stmt::new(
                StmtKind::Expr(Expr::new(
                    ExprKind::Call(
                        Box::new(Expr::new(ExprKind::Ident(Ident::new("finally_block", sp)), sp)),
                        vec![],
                    ),
                    sp,
                )),
                sp,
            )],
            sp,
        );

        let fun = FunDecl {
            name: Ident::new("test_sus_defer", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: Block::new(
                vec![Stmt::new(
                    StmtKind::Expr(Expr::new(
                        ExprKind::Sus {
                            body: sus_body,
                            catches,
                            finally_branch: Some(finally_b),
                        },
                        sp,
                    )),
                    sp,
                )],
                sp,
            ),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Fun(fun), sp));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(&context, "test_sus_defer_mod", TargetTriple::host());
        assert!(codegen.compile_module(&ast).is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("call fastcc void @cleanup_sus()"));
        assert!(ir.contains("call fastcc void @finally_block()"));
        assert!(ir.contains("landingpad { ptr, i32 }"));
        assert!(ir.contains("cleanup"));
    }

    #[test]
    fn test_super_codegen_direct_call_and_field_access() {
        let sp = dummy_span();

        // 1. class Base { val: int64; fun greet() {} }
        let base_class = yukic_ast::item::ClassDecl {
            name: Ident::new("Base", sp),
            is_locked: false,
            base_class: None,
            interfaces: Vec::new(),
            fields: vec![yukic_ast::item::ClassField {
                name: Ident::new("val", sp),
                attrs: Vec::new(),
                ty: TypeNode::new(TypeNodeKind::Ident(Ident::new("int64", sp)), sp),
                is_pub: true,
                default_val: None,
                span: sp,
            }],
            methods: vec![yukic_ast::item::MethodDecl {
                name: Ident::new("greet", sp),
                attrs: Vec::new(),
                this_param: yukic_ast::item::ThisParam::Value,
                params: Vec::new(),
                return_type: None,
                body: Block::new(Vec::new(), sp),
                is_operator: false,
                is_override: false,
                span: sp,
            }],
            accessors: Vec::new(),
            span: sp,
        };

        // 2. class Derived extends Base { fun greet() { super.greet(); set x = super.val; } }
        let derived_class = yukic_ast::item::ClassDecl {
            name: Ident::new("Derived", sp),
            is_locked: false,
            base_class: Some(Ident::new("Base", sp)),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: vec![yukic_ast::item::MethodDecl {
                name: Ident::new("greet", sp),
                attrs: Vec::new(),
                this_param: yukic_ast::item::ThisParam::Value,
                params: Vec::new(),
                return_type: None,
                body: Block::new(
                    vec![
                        // super.greet()
                        Stmt::new(
                            StmtKind::Expr(Expr::new(
                                ExprKind::MethodCall(
                                    Box::new(Expr::new(ExprKind::Super, sp)),
                                    Ident::new("greet", sp),
                                    vec![],
                                ),
                                sp,
                            )),
                            sp,
                        ),
                        // set x = super.val
                        Stmt::new(
                            StmtKind::Set {
                                target: BindingTarget::Ident(Ident::new("x", sp)),
                                ty: None,
                                init: Expr::new(
                                    ExprKind::FieldAccess(
                                        Box::new(Expr::new(ExprKind::Super, sp)),
                                        Ident::new("val", sp),
                                    ),
                                    sp,
                                ),
                            },
                            sp,
                        ),
                    ],
                    sp,
                ),
                is_operator: false,
                is_override: true,
                span: sp,
            }],
            accessors: Vec::new(),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Class(base_class), sp));
        ast.items.push(Item::new(ItemId(2), Visibility::Public, ItemKind::Class(derived_class), sp));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(
            &context,
            "test_super_mod",
            TargetTriple::host(),
        );
        assert!(codegen.compile_module(&ast).is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        // Derived.greet must perform direct super call to Base.greet
        assert!(ir.contains("call fastcc void @_Y_Base_greet(ptr"));
        // Derived.greet accesses field val
        assert!(ir.contains("%class.Base, ptr"));
        assert!(ir.contains("load i64, ptr %val"));
    }

    #[test]
    fn test_operator_overload_codegen() {
        let sp = dummy_span();

        let vec2_class = yukic_ast::item::ClassDecl {
            name: Ident::new("Vec2", sp),
            is_locked: false,
            base_class: None,
            interfaces: Vec::new(),
            fields: vec![
                yukic_ast::item::ClassField {
                    name: Ident::new("x", sp),
                    attrs: Vec::new(),
                    ty: TypeNode::new(TypeNodeKind::Ident(Ident::new("int64", sp)), sp),
                    is_pub: true,
                    default_val: None,
                    span: sp,
                },
            ],
            methods: vec![
                yukic_ast::item::MethodDecl {
                    name: Ident::new("+", sp),
                    attrs: Vec::new(),
                    this_param: yukic_ast::item::ThisParam::Value,
                    params: vec![yukic_ast::item::Param {
                        name: Ident::new("other", sp),
                        ty: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("Vec2", sp)), sp)),
                        is_mut: false,
                        default_val: None,
                        is_variadic: false,
                        is_ref: false,
                        span: sp,
                    }],
                    return_type: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("Vec2", sp)), sp)),
                    body: Block::new(Vec::new(), sp),
                    is_operator: true,
                    is_override: false,
                    span: sp,
                },
                yukic_ast::item::MethodDecl {
                    name: Ident::new("-", sp),
                    attrs: Vec::new(),
                    this_param: yukic_ast::item::ThisParam::Value,
                    params: Vec::new(),
                    return_type: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("Vec2", sp)), sp)),
                    body: Block::new(Vec::new(), sp),
                    is_operator: true,
                    is_override: false,
                    span: sp,
                },
                yukic_ast::item::MethodDecl {
                    name: Ident::new("==", sp),
                    attrs: Vec::new(),
                    this_param: yukic_ast::item::ThisParam::Value,
                    params: vec![yukic_ast::item::Param {
                        name: Ident::new("other", sp),
                        ty: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("Vec2", sp)), sp)),
                        is_mut: false,
                        default_val: None,
                        is_variadic: false,
                        is_ref: false,
                        span: sp,
                    }],
                    return_type: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("bool", sp)), sp)),
                    body: Block::new(Vec::new(), sp),
                    is_operator: true,
                    is_override: false,
                    span: sp,
                },
            ],
            accessors: Vec::new(),
            span: sp,
        };

        let test_fn = yukic_ast::item::FunDecl {
            name: Ident::new("test_ops", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![
                yukic_ast::item::Param {
                    name: Ident::new("a", sp),
                    ty: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("Vec2", sp)), sp)),
                    is_mut: false,
                    default_val: None,
                    is_variadic: false,
                    is_ref: false,
                    span: sp,
                },
                yukic_ast::item::Param {
                    name: Ident::new("b", sp),
                    ty: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("Vec2", sp)), sp)),
                    is_mut: false,
                    default_val: None,
                    is_variadic: false,
                    is_ref: false,
                    span: sp,
                },
            ],
            return_type: None,
            body: Block::new(
                vec![
                    // set s = a + b
                    Stmt::new(
                        StmtKind::Set {
                            target: BindingTarget::Ident(Ident::new("s", sp)),
                            ty: None,
                            init: Expr::new(
                                ExprKind::Binary(
                                    yukic_ast::expr::BinaryOp::Add,
                                    Box::new(Expr::new(ExprKind::Ident(Ident::new("a", sp)), sp)),
                                    Box::new(Expr::new(ExprKind::Ident(Ident::new("b", sp)), sp)),
                                ),
                                sp,
                            ),
                        },
                        sp,
                    ),
                    // set n = -a
                    Stmt::new(
                        StmtKind::Set {
                            target: BindingTarget::Ident(Ident::new("n", sp)),
                            ty: None,
                            init: Expr::new(
                                ExprKind::Unary(
                                    yukic_ast::expr::UnaryOp::Neg,
                                    Box::new(Expr::new(ExprKind::Ident(Ident::new("a", sp)), sp)),
                                ),
                                sp,
                            ),
                        },
                        sp,
                    ),
                    // set eq = a == b
                    Stmt::new(
                        StmtKind::Set {
                            target: BindingTarget::Ident(Ident::new("eq", sp)),
                            ty: None,
                            init: Expr::new(
                                ExprKind::Binary(
                                    yukic_ast::expr::BinaryOp::Eq,
                                    Box::new(Expr::new(ExprKind::Ident(Ident::new("a", sp)), sp)),
                                    Box::new(Expr::new(ExprKind::Ident(Ident::new("b", sp)), sp)),
                                ),
                                sp,
                            ),
                        },
                        sp,
                    ),
                    // set ne = a != b
                    Stmt::new(
                        StmtKind::Set {
                            target: BindingTarget::Ident(Ident::new("ne", sp)),
                            ty: None,
                            init: Expr::new(
                                ExprKind::Binary(
                                    yukic_ast::expr::BinaryOp::NotEq,
                                    Box::new(Expr::new(ExprKind::Ident(Ident::new("a", sp)), sp)),
                                    Box::new(Expr::new(ExprKind::Ident(Ident::new("b", sp)), sp)),
                                ),
                                sp,
                            ),
                        },
                        sp,
                    ),
                ],
                sp,
            ),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Class(vec2_class), sp));
        ast.items.push(Item::new(ItemId(2), Visibility::Public, ItemKind::Fun(test_fn), sp));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(
            &context,
            "test_operator_mod",
            TargetTriple::host(),
        );
        codegen.compile_module(&ast).expect("compile module failed");

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("@_Y_Vec2___op_add"));
        assert!(ir.contains("@_Y_Vec2___op_neg"));
        assert!(ir.contains("@_Y_Vec2___op_eq"));
        assert!(ir.contains("xor i1 %eq_val, true") || ir.contains("xor i1"));
    }

    #[test]
    fn test_compile_slice_access_codegen() {
        let sp = dummy_span();

        let stmts = vec![
            Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(Ident::new("s", sp)),
                    ty: None,
                    init: Expr::new(ExprKind::Literal(LiteralValue::String("hello world".into())), sp),
                },
                sp,
            ),
            Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(Ident::new("sub", sp)),
                    ty: None,
                    init: Expr::new(
                        ExprKind::SliceAccess(
                            Box::new(Expr::new(ExprKind::Ident(Ident::new("s", sp)), sp)),
                            Some(Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(0)), sp))),
                            Some(Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(5)), sp))),
                            false,
                        ),
                        sp,
                    ),
                },
                sp,
            ),
            Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(Ident::new("arr", sp)),
                    ty: None,
                    init: Expr::new(
                        ExprKind::ArrayInit(vec![
                            Expr::new(ExprKind::Literal(LiteralValue::Int(10)), sp),
                            Expr::new(ExprKind::Literal(LiteralValue::Int(20)), sp),
                            Expr::new(ExprKind::Literal(LiteralValue::Int(30)), sp),
                        ]),
                        sp,
                    ),
                },
                sp,
            ),
            Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(Ident::new("subarr", sp)),
                    ty: None,
                    init: Expr::new(
                        ExprKind::SliceAccess(
                            Box::new(Expr::new(ExprKind::Ident(Ident::new("arr", sp)), sp)),
                            Some(Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(1)), sp))),
                            Some(Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(2)), sp))),
                            true,
                        ),
                        sp,
                    ),
                },
                sp,
            ),
        ];

        let fun = FunDecl {
            name: Ident::new("test_slicing", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: Block::new(stmts, sp),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Fun(fun), sp));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(
            &context,
            "test_slice_mod",
            TargetTriple::host(),
        );
        codegen.compile_module(&ast).expect("compile module with slicing failed");

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("%str_len = insertvalue { ptr, i64 } %str_ptr, i64 5, 1"));
        assert!(ir.contains("getelementptr inbounds i8, ptr"));
        assert!(ir.contains("i64 2, 1"));
    }

    #[test]
    fn test_compile_anonymous_function_and_closure() {
        let sp = dummy_span();

        // 1. Uncaptured lambda: set f = fun(x: int64) => x * 2; return f(21);
        let lambda_param = Param {
            name: Ident::new("x", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("int64", sp)), sp)),
            default_val: None,
            span: sp,
        };
        let lambda_body = Expr::new(
            ExprKind::Binary(
                BinaryOp::Mul,
                Box::new(Expr::new(ExprKind::Ident(Ident::new("x", sp)), sp)),
                Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(2)), sp)),
            ),
            sp,
        );
        let lambda_decl = FunDecl {
            name: Ident::new("<anonymous>", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![lambda_param],
            return_type: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("int64", sp)), sp)),
            body: Block::new(vec![Stmt::new(StmtKind::Return(Some(lambda_body)), sp)], sp),
            span: sp,
        };

        let set_f = Stmt::new(
            StmtKind::Set {
                target: BindingTarget::Ident(Ident::new("f", sp)),
                ty: None,
                init: Expr::new(ExprKind::AnonymousFun(Box::new(lambda_decl)), sp),
            },
            sp,
        );

        let call_f = Stmt::new(
            StmtKind::Return(Some(Expr::new(
                ExprKind::Call(
                    Box::new(Expr::new(ExprKind::Ident(Ident::new("f", sp)), sp)),
                    vec![Expr::new(ExprKind::Literal(LiteralValue::Int(21)), sp)],
                ),
                sp,
            ))),
            sp,
        );

        let fun = FunDecl {
            name: Ident::new("test_closure_invoke", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("int64", sp)), sp)),
            body: Block::new(vec![set_f, call_f], sp),
            span: sp,
        };

        let mut ast = AstModule::default();
        ast.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Fun(fun), sp));

        let context = Context::create();
        let mut codegen = crate::generator::CodeGenerator::new(
            &context,
            "test_closure_mod",
            TargetTriple::host(),
        );
        codegen.compile_module(&ast).expect("compile module with closure failed");

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("@__yuki_closure_0"));
        assert!(ir.contains("call fastcc"));

        // 2. Closure with variable capture:
        // fun test_capture() -> int64 { set base = 100; set f = fun(x: int64) => base + x; return f(5); }
        let base_stmt = Stmt::new(
            StmtKind::Set {
                target: BindingTarget::Ident(Ident::new("base", sp)),
                ty: None,
                init: Expr::new(ExprKind::Literal(LiteralValue::Int(100)), sp),
            },
            sp,
        );

        let cap_param = Param {
            name: Ident::new("x", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("int64", sp)), sp)),
            default_val: None,
            span: sp,
        };
        let cap_body = Expr::new(
            ExprKind::Binary(
                BinaryOp::Add,
                Box::new(Expr::new(ExprKind::Ident(Ident::new("base", sp)), sp)),
                Box::new(Expr::new(ExprKind::Ident(Ident::new("x", sp)), sp)),
            ),
            sp,
        );
        let cap_decl = FunDecl {
            name: Ident::new("<anonymous>", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![cap_param],
            return_type: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("int64", sp)), sp)),
            body: Block::new(vec![Stmt::new(StmtKind::Return(Some(cap_body)), sp)], sp),
            span: sp,
        };

        let set_f_cap = Stmt::new(
            StmtKind::Set {
                target: BindingTarget::Ident(Ident::new("f", sp)),
                ty: None,
                init: Expr::new(ExprKind::AnonymousFun(Box::new(cap_decl)), sp),
            },
            sp,
        );

        let call_f_cap = Stmt::new(
            StmtKind::Return(Some(Expr::new(
                ExprKind::Call(
                    Box::new(Expr::new(ExprKind::Ident(Ident::new("f", sp)), sp)),
                    vec![Expr::new(ExprKind::Literal(LiteralValue::Int(5)), sp)],
                ),
                sp,
            ))),
            sp,
        );

        let cap_fun = FunDecl {
            name: Ident::new("test_closure_capture", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("int64", sp)), sp)),
            body: Block::new(vec![base_stmt, set_f_cap, call_f_cap], sp),
            span: sp,
        };

        let mut ast2 = AstModule::default();
        ast2.items.push(Item::new(ItemId(2), Visibility::Public, ItemKind::Fun(cap_fun), sp));

        let mut codegen2 = crate::generator::CodeGenerator::new(
            &context,
            "test_closure_cap_mod",
            TargetTriple::host(),
        );
        codegen2.compile_module(&ast2).expect("compile module with captured closure failed");
        let ir2 = codegen2.emit_llvm_ir_to_string();
        assert!(ir2.contains("struct.env___yuki_closure_"));
        assert!(ir2.contains("yukino_region_alloc"));
    }
}


