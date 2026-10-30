#![deny(unsafe_code)]

use inkwell::AddressSpace;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::builder::{Builder, BuilderError};
use inkwell::context::Context;
use inkwell::module::Module as LlvmModule;

use inkwell::targets::{TargetData, TargetTriple as InkwellTargetTriple};
use inkwell::types::{BasicType, BasicTypeEnum, FunctionType, PointerType, StructType};
use inkwell::values::{
    ArrayValue, BasicMetadataValueEnum, BasicValueEnum, CallSiteValue, FloatValue, FunctionValue, GlobalValue,
    InstructionValue, IntValue, PointerValue, StructValue,
};
use std::collections::HashSet;
use std::path::Path;
use yukic_ast::expr::{Expr, ExprKind, LiteralValue};
use yukic_ast::item::ItemKind;
use yukic_ast::{ItemId, Module as AstModule};
use yukic_target::TargetTriple;
use yukic_types::Type;

use crate::cabi::CAbiManager;
use crate::cgu::CodegenUnit;
use crate::closure::{CapturedVar, ClosureEnvLayout, ClosureManager};
use crate::context::init_llvm_targets;
use crate::debug::DebugInfoManager;
use crate::exception::{CleanupAction, ExceptionManager, Scope, ScopeKind, ScopeStack};
use crate::intrinsic::{IntrinsicManager, SliceElemLayout};
use crate::region::{DEFAULT_APP_REGION_CAPACITY, DEFAULT_LOCAL_REGION_CAPACITY, RegionManager};
use crate::registry::TypeRegistry;
use crate::rodata::RoDataManager;
use crate::types::TypeLowerer;
use crate::vtable::VTableManager;
use yukic_borrow::EscapeKind;
use yukic_span::{SourceMap, Span};
use yukic_types::ast_type_to_type;

pub struct CodeGenerator<'ctx> {
    pub context: &'ctx Context,
    pub module: LlvmModule<'ctx>,
    pub builder: Builder<'ctx>,
    pub target: TargetTriple,
    pub target_data: TargetData,
    pub registry: TypeRegistry,
    pub rodata: RoDataManager<'ctx>,
    pub region: RegionManager<'ctx>,
    pub vtable: VTableManager<'ctx>,
    pub closure: ClosureManager<'ctx>,
    pub exception: ExceptionManager<'ctx>,
    pub scopes: ScopeStack<'ctx>,
    pub cabi: CAbiManager<'ctx>,
    pub intrinsic: IntrinsicManager<'ctx>,
    pub debug: DebugInfoManager<'ctx>,
    pub panic_strategy: String,
    pub function_sections: bool,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str, target: TargetTriple) -> Self {
        init_llvm_targets();

        let module = context.create_module(module_name);
        let llvm_triple = InkwellTargetTriple::create(target.spec().triple);
        module.set_triple(&llvm_triple);

        let target_data = TargetData::create(target.spec().data_layout);
        module.set_data_layout(&target_data.get_data_layout());

        embed_artifact_version_metadata(&module, context);

        let builder = context.create_builder();
        let region = RegionManager::new(context);
        region.declare_intrinsics(&module);
        let vtable = VTableManager::new(context);
        let closure = ClosureManager::new(context);
        let exception = ExceptionManager::new(context);
        let scopes = ScopeStack::new();
        let cabi = CAbiManager::new(context);
        let intrinsic = IntrinsicManager::new(context);
        intrinsic.declare_intrinsics(&module);
        let debug = DebugInfoManager::new(context, target);

        Self {
            context,
            module,
            builder,
            target,
            target_data,
            registry: TypeRegistry::new(),
            rodata: RoDataManager::new(context),
            region,
            vtable,
            closure,
            exception,
            scopes,
            cabi,
            intrinsic,
            debug,
            panic_strategy: "unwind".to_string(),
            function_sections: false,
        }
    }

    pub fn with_panic_strategy(mut self, strategy: impl Into<String>) -> Self {
        self.panic_strategy = strategy.into();
        self
    }

    pub fn with_function_sections(mut self, enabled: bool) -> Self {
        self.function_sections = enabled;
        self
    }

    pub(crate) fn set_function_section(&self, fn_val: FunctionValue<'ctx>, symbol_name: &str) {
        if self.function_sections {
            fn_val.set_section(Some(&format!(".text.{symbol_name}")));
        }
    }

    pub fn ptr_type(&self) -> PointerType<'ctx> {
        self.context.ptr_type(AddressSpace::default())
    }

    pub fn build_load<T: BasicType<'ctx>>(
        &self,
        pointee_ty: T,
        ptr: PointerValue<'ctx>,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, BuilderError> {
        self.builder.build_load(pointee_ty, ptr, name)
    }

    pub fn build_struct_gep<T: BasicType<'ctx>>(
        &self,
        struct_ty: T,
        ptr: PointerValue<'ctx>,
        index: u32,
        name: &str,
    ) -> Result<PointerValue<'ctx>, BuilderError> {
        self.builder.build_struct_gep(struct_ty, ptr, index, name)
    }

    pub fn build_indirect_call(
        &self,
        fn_type: FunctionType<'ctx>,
        fn_ptr: PointerValue<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        self.builder
            .build_indirect_call(fn_type, fn_ptr, args, name)
    }

    pub fn build_call(
        &self,
        fn_val: FunctionValue<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        self.builder.build_call(fn_val, args, name)
    }

    pub fn lowerer<'a>(&'a self) -> TypeLowerer<'a, 'ctx> {
        TypeLowerer::new(self.context, &self.target_data, self.target, &self.registry)
    }

    pub fn llvm_type(&self, ty: &Type) -> Result<BasicTypeEnum<'ctx>, String> {
        self.lowerer().lower_type(ty)
    }

    pub fn llvm_return_type(&self, ty: &Type) -> Result<Option<BasicTypeEnum<'ctx>>, String> {
        self.lowerer().lower_return_type(ty)
    }

    pub fn lower_named_cstruct(&self, name: &str) -> Result<StructType<'ctx>, String> {
        self.lowerer().lower_named_cstruct(name)
    }

    pub fn register_ast_types(&mut self, module: &AstModule) {
        self.registry.register_module(module);
    }

    pub fn ast_type_to_canonical_type(&self, node: &yukic_ast::TypeNode) -> Type {
        self.registry.canonicalize_type(&ast_type_to_type(node))
    }

    pub fn symbol_for_fun(&self, fun: &yukic_ast::item::FunDecl) -> String {
        if !crate::mangle::should_mangle_fun(&fun.name.name, &fun.attrs) {
            return fun.name.name.clone();
        }
        crate::mangle::mangle_fun_decl(fun, &self.registry)
    }

    pub fn symbol_for_cfun(&self, cfun: &yukic_ast::item::CfunDecl) -> String {
        if let Some(ref sym) = cfun.symbol_override {
            return sym.name.clone();
        }
        if !crate::mangle::should_mangle_cfun(&cfun.name.name, &cfun.yukino_attrs) {
            return cfun.name.name.clone();
        }
        crate::mangle::mangle_cfun_decl(cfun, &self.registry)
    }

    pub fn string_type(&self) -> StructType<'ctx> {
        self.lowerer().string_type()
    }

    pub fn slice_type(&self) -> StructType<'ctx> {
        self.lowerer().slice_type()
    }

    pub fn interface_type(&self) -> StructType<'ctx> {
        self.lowerer().interface_type()
    }

    pub fn closure_type(&self) -> StructType<'ctx> {
        self.lowerer().closure_type()
    }

    pub fn weak_type(&self) -> StructType<'ctx> {
        self.lowerer().weak_type()
    }

    pub fn nullable_type(&self, inner: &Type) -> Result<BasicTypeEnum<'ctx>, String> {
        self.lowerer()
            .lower_union(&[inner.clone(), Type::Null].into_iter().collect())
    }

    pub fn abi_size_of(&self, ty: &Type) -> Result<u64, String> {
        self.lowerer().abi_size_of(ty)
    }

    pub fn abi_align_of(&self, ty: &Type) -> Result<u32, String> {
        self.lowerer().abi_align_of(ty)
    }

    pub fn emit_string_literal(&mut self, text: &str) -> StructValue<'ctx> {
        self.rodata.emit_string(&self.module, text)
    }

    pub fn emit_bytes_literal(&mut self, raw_bytes: &[u8]) -> StructValue<'ctx> {
        self.rodata.emit_bytes(&self.module, raw_bytes)
    }

    pub fn emit_typeof(&mut self, ty: &Type) -> StructValue<'ctx> {
        self.rodata.emit_typeof(&self.module, ty)
    }

    pub fn emit_literal(&mut self, lit: &LiteralValue) -> Result<BasicValueEnum<'ctx>, String> {
        match lit {
            LiteralValue::Int(v) => Ok(self
                .context
                .i64_type()
                .const_int((*v).cast_unsigned(), true)
                .into()),
            LiteralValue::UInt(v) => Ok(self.context.i64_type().const_int(*v, false).into()),
            LiteralValue::Float(v) => Ok(self.context.f64_type().const_float(*v).into()),
            LiteralValue::Bool(v) => {
                let int_val = u64::from(*v);
                Ok(self.context.bool_type().const_int(int_val, false).into())
            }
            LiteralValue::Char(v) => Ok(self
                .context
                .i32_type()
                .const_int(u64::from(*v), false)
                .into()),
            LiteralValue::Null => Ok(self.ptr_type().const_null().into()),
            LiteralValue::String(s) => Ok(self.emit_string_literal(s).into()),
            LiteralValue::Bytes(b) => Ok(self.emit_bytes_literal(b).into()),
        }
    }

    pub fn emit_llvm_ir_to_string(&self) -> String {
        self.debug.finalize();
        self.module.print_to_string().to_string()
    }

    pub fn emit_llvm_ir_to_file(&self, path: &Path) -> Result<(), String> {
        self.debug.finalize();
        self.module.print_to_file(path).map_err(|e| e.to_string())
    }

    pub fn enable_debug_info(
        &mut self,
        main_filename: &str,
        directory: &str,
        is_optimized: bool,
    ) -> Option<inkwell::debug_info::DIFile<'ctx>> {
        self.debug
            .enable_debug_info(&self.module, main_filename, directory, is_optimized);
        self.debug
            .get_or_create_file(std::path::Path::new(main_filename))
    }

    pub fn set_debug_location(&mut self, span: Span, source_map: Option<&SourceMap>) {
        self.debug
            .set_location_from_span(&self.builder, span, source_map);
    }

    pub fn set_debug_location_coords(&self, line: u32, column: u32) {
        self.debug.set_location(&self.builder, line, column);
    }

    pub fn clear_debug_location(&self) {
        self.debug.clear_location(&self.builder);
    }

    pub fn finalize_debug_info(&self) {
        self.debug.finalize();
    }

    pub fn compile_module(&mut self, ast: &AstModule) -> Result<(), String> {
        self.register_ast_types(ast);
        self.register_vtable_layouts(ast);
        self.register_defaults(ast);

        for item in &ast.items {
            match &item.kind {
                ItemKind::Fun(fun) => self.declare_foreign_fun(fun)?,
                ItemKind::Static(decl) => self.compile_static_decl(decl)?,
                ItemKind::Class(decl) => self.pre_declare_class_methods(decl)?,
                _ => {}
            }
        }

        for item in &ast.items {
            self.compile_item(item)?;
        }
        self.emit_all_vtables(ast)?;

        self.module.verify().map_err(|e| e.to_string())
    }

    pub fn compile_cgu(&mut self, ast: &AstModule, unit: &CodegenUnit) -> Result<(), String> {
        self.register_ast_types(ast);
        self.register_vtable_layouts(ast);
        self.register_defaults(ast);

        let owned_ids: HashSet<ItemId> = unit.item_ids.iter().copied().collect();

        for item in &ast.items {
            if owned_ids.contains(&item.id) {
                match &item.kind {
                    ItemKind::Fun(fun) => self.declare_foreign_fun(fun)?,
                    ItemKind::Static(decl) => self.compile_static_decl(decl)?,
                    ItemKind::Class(decl) => self.pre_declare_class_methods(decl)?,
                    _ => {}
                }
            } else {
                self.declare_foreign_item(item)?;
            }
        }

        // compile items owned by this cgu
        for item in &ast.items {
            if owned_ids.contains(&item.id) {
                self.compile_item(item)?;
                if let ItemKind::Class(decl) = &item.kind {
                    self.vtable.emit_class_vtable_with_linkage(
                        &self.module,
                        &decl.name.name,
                        inkwell::module::Linkage::External,
                    )?;
                    for iface_id in &decl.interfaces {
                        self.vtable.emit_interface_itable_with_linkage(
                            &self.module,
                            &decl.name.name,
                            &iface_id.name,
                            inkwell::module::Linkage::External,
                        )?;
                    }
                }
            }
        }

        self.module.verify().map_err(|e| e.to_string())
    }

    pub(crate) fn declare_foreign_item(&mut self, item: &yukic_ast::Item) -> Result<(), String> {
        let is_public = item.visibility == yukic_ast::item::Visibility::Public;
        match &item.kind {
            ItemKind::Fun(fun) => self.declare_foreign_fun(fun)?,
            ItemKind::Cfun(cfun) => self.declare_foreign_cfun(cfun, is_public)?,
            ItemKind::Class(decl) => self.declare_foreign_class(decl)?,
            ItemKind::Static(decl) => self.declare_foreign_static(decl),
            ItemKind::Externc(block) => self.compile_externc_block(block, is_public)?,
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn declare_foreign_fun(&mut self, fun: &yukic_ast::item::FunDecl) -> Result<(), String> {
        let sym_name = self.symbol_for_fun(fun);
        if sym_name == "main" || self.module.get_function(&sym_name).is_some() {
            return Ok(());
        }

        let ret_ty = fun
            .return_type
            .as_ref()
            .map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
        let llvm_ret = self.llvm_return_type(&ret_ty)?;

        let mut param_types = Vec::new();
        for param in &fun.params {
            if param.is_ref {
                param_types.push(self.ptr_type().into());
            } else {
                let mut pty = param
                    .ty
                    .as_ref()
                    .map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
                if param.is_variadic && !matches!(pty, Type::Array(_)) {
                    pty = Type::Array(Box::new(pty));
                }
                param_types.push(self.llvm_type(&pty)?);
            }
        }

        let is_thunk = fun.attrs.iter().any(|a| a.is("thunk"));
        let has_regional_ref = fun.params.iter().any(|p| {
            if p.is_ref {
                let pty = p.ty.as_ref().map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
                RegionManager::is_regional_type(&pty)
            } else {
                false
            }
        });
        let (fn_type, has_dest) = self.transform_function_type_with_dest(&param_types, &ret_ty, llvm_ret, is_thunk, has_regional_ref);
        let fn_val = self.module.add_function(
            &sym_name,
            fn_type,
            Some(inkwell::module::Linkage::External),
        );
        if has_dest {
            if let Some(p) = fn_val.get_first_param() {
                p.set_name("_dest_region");
            }
        }
        let call_conv = if is_thunk {
            inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32
        } else {
            inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32
        };
        fn_val.set_call_conventions(call_conv);
        self.apply_fun_attributes(fn_val, &fun.attrs);
        Ok(())
    }

    pub(crate) fn declare_foreign_cfun(
        &mut self,
        cfun: &yukic_ast::item::CfunDecl,
        is_public: bool,
    ) -> Result<(), String> {
        let sym_name = self.symbol_for_cfun(cfun);
        if self.module.get_function(&sym_name).is_some() {
            return Ok(());
        }

        let ret_ty = cfun
            .return_type
            .as_ref()
            .map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
        let llvm_ret = self.llvm_return_type(&ret_ty)?;

        let mut param_types = Vec::new();
        for param in &cfun.params {
            let pty = self.ast_type_to_canonical_type(&param.ty);
            param_types.push(self.llvm_type(&pty)?);
        }

        let (fn_type, _) = self.transform_function_type(&param_types, &ret_ty, llvm_ret, true);
        let fn_val =
            self.module
                .add_function(&sym_name, fn_type, Some(inkwell::module::Linkage::External));
        self.cabi
            .configure_cfun(fn_val, &cfun.c_attrs, is_public, false)?;
        Ok(())
    }

    pub(crate) fn pre_declare_class_methods(
        &mut self,
        decl: &yukic_ast::item::ClassDecl,
    ) -> Result<(), String> {
        for method in &decl.methods {
            let sym_name = crate::mangle::method_symbol_name(
                &decl.name.name,
                &method.name.name,
                method.params.len(),
            );


            if self.module.get_function(&sym_name).is_some() {
                continue;
            }

            let ret_ty = method
                .return_type
                .as_ref()
                .map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
            let llvm_ret = self.llvm_return_type(&ret_ty)?;

            let mut param_types = Vec::new();
            if method.this_param != yukic_ast::item::ThisParam::None {
                param_types.push(self.ptr_type().into());
            }
            for param in &method.params {
                if param.is_ref {
                    param_types.push(self.ptr_type().into());
                } else {
                    let mut pty = param
                        .ty
                        .as_ref()
                        .map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
                    if param.is_variadic && !matches!(pty, Type::Array(_)) {
                        pty = Type::Array(Box::new(pty));
                    }
                    param_types.push(self.llvm_type(&pty)?);
                }
            }

            let is_init = method.name.name == "init";
            let (fn_type, has_dest) = if is_init {
                let mut final_params = Vec::with_capacity(param_types.len() + 1);
                final_params.push(self.ptr_type().into());
                for pt in &param_types {
                    final_params.push((*pt).into());
                }
                let fn_ty = match llvm_ret {
                    Some(ret) => ret.fn_type(&final_params, false),
                    None => self.context.void_type().fn_type(&final_params, false),
                };
                (fn_ty, true)
            } else {
                let has_regional_ref = method.params.iter().any(|p| {
                    if p.is_ref {
                        let pty = p.ty.as_ref().map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
                        RegionManager::is_regional_type(&pty)
                    } else {
                        false
                    }
                });
                self.transform_function_type_with_dest(&param_types, &ret_ty, llvm_ret, false, has_regional_ref)
            };
            let fn_val = self.module.add_function(
                &sym_name,
                fn_type,
                Some(inkwell::module::Linkage::External),
            );
            if has_dest {
                if let Some(p) = fn_val.get_first_param() {
                    p.set_name("_dest_region");
                }
            }
            fn_val.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);
        }
        self.pre_declare_class_accessors(decl)
    }

    fn pre_declare_class_accessors(
        &mut self,
        decl: &yukic_ast::item::ClassDecl,
    ) -> Result<(), String> {
        for acc in &decl.accessors {
            let (sym_name, ret_ty, param_types) = match &acc.kind {
                yukic_ast::item::AccessorKind::Get => {
                    let sym = format!("_Y_{}___get_{}", decl.name.name, acc.name.name);
                    let ret = self.ast_type_to_canonical_type(&acc.ty);
                    (sym, ret, vec![self.ptr_type().into()])
                }
                yukic_ast::item::AccessorKind::Set(_) => {
                    let sym = format!("_Y_{}___set_{}", decl.name.name, acc.name.name);
                    let pty = self.ast_type_to_canonical_type(&acc.ty);
                    (sym, Type::Void, vec![self.ptr_type().into(), self.llvm_type(&pty)?])
                }
            };
            if self.module.get_function(&sym_name).is_some() {
                continue;
            }
            let llvm_ret = self.llvm_return_type(&ret_ty)?;
            let (fn_type, has_dest) = self.transform_function_type(&param_types, &ret_ty, llvm_ret, false);
            let fn_val = self.module.add_function(
                &sym_name,
                fn_type,
                Some(inkwell::module::Linkage::External),
            );
            if has_dest {
                if let Some(p) = fn_val.get_first_param() {
                    p.set_name("_dest_region");
                }
            }
            fn_val.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);
        }
        Ok(())
    }

    pub(crate) fn declare_foreign_class(
        &mut self,
        decl: &yukic_ast::item::ClassDecl,
    ) -> Result<(), String> {
        self.pre_declare_class_methods(decl)?;
        let _ = self
            .vtable
            .declare_external_vtable(&self.module, &decl.name.name);
        for iface_id in &decl.interfaces {
            let _ =
                self.vtable
                    .declare_external_itable(&self.module, &decl.name.name, &iface_id.name);
            let base_name = iface_id.name.split('<').next().unwrap_or(&iface_id.name);
            if base_name != iface_id.name {
                let _ = self.vtable.declare_external_itable(&self.module, &decl.name.name, base_name);
            }
        }
        Ok(())
    }

    pub(crate) fn declare_foreign_static(&mut self, decl: &yukic_ast::item::StaticDecl) {
        if self.module.get_global(&decl.name.name).is_none() {
            let ty = if let Some(type_node) = &decl.ty {
                let ast_ty = ast_type_to_type(type_node);
                self.llvm_type(&ast_ty).unwrap_or_else(|_| self.ptr_type().into())
            } else if let ExprKind::Literal(lit) = &decl.init.kind {
                self.emit_literal(lit)
                    .map_or_else(|_| self.ptr_type().into(), |v| v.get_type())
            } else {
                self.ptr_type().into()
            };
            let global = self
                .module
                .add_global(ty, Some(AddressSpace::default()), &decl.name.name);
            global.set_linkage(inkwell::module::Linkage::External);
        }
    }

    pub fn emit_all_vtables(&mut self, ast: &AstModule) -> Result<(), String> {
        for item in &ast.items {
            if let ItemKind::Class(decl) = &item.kind {
                self.vtable
                    .emit_class_vtable(&self.module, &decl.name.name)?;
                for iface_id in &decl.interfaces {
                    self.vtable.emit_interface_itable(
                        &self.module,
                        &decl.name.name,
                        &iface_id.name,
                    )?;
                    let base_name = iface_id.name.split('<').next().unwrap_or(&iface_id.name);
                    if base_name != iface_id.name {
                        let _ = self.vtable.emit_interface_itable(
                            &self.module,
                            &decl.name.name,
                            base_name,
                        );
                    }
                }
            }
        }
        Ok(())
    }


    pub(crate) fn register_vtable_layouts(&mut self, ast: &AstModule) {
        let mut pending: Vec<(&yukic_ast::item::ClassDecl, u64)> = Vec::new();
        let mut class_type_id = 1001u64;
        for item in &ast.items {
            if let ItemKind::Class(decl) = &item.kind {
                pending.push((decl, class_type_id));
                class_type_id += 1;
            }
        }
        let mut changed = true;
        while changed && !pending.is_empty() {
            changed = false;
            let mut remaining = Vec::new();
            for (decl, tid) in pending {
                let ready = match &decl.base_class {
                    None => true,
                    Some(base) => self.vtable.get_layout(&base.name).is_some(),
                };
                if ready {
                    self.vtable.register_class(decl, tid);
                    changed = true;
                } else {
                    remaining.push((decl, tid));
                }
            }
            pending = remaining;
        }
        for (decl, tid) in pending {
            self.vtable.register_class(decl, tid);
        }
        self.register_itable_layouts(ast);
    }

    fn register_itable_layouts(&mut self, ast: &AstModule) {
        for item in &ast.items {
            if let ItemKind::Interface(iface) = &item.kind {
                self.vtable.register_interface(iface);
            }
        }
        for item in &ast.items {
            if let ItemKind::Class(decl) = &item.kind {
                for iface_id in &decl.interfaces {
                    let base_name = iface_id.name.split('<').next().unwrap_or(&iface_id.name);
                    if let Some(iface_decl) = self
                        .vtable
                        .get_interface_decl(base_name)
                        .cloned()
                        .or_else(|| {
                            ast.items.iter().find_map(|it| match &it.kind {
                                ItemKind::Interface(iface) if iface.name.name == base_name => {
                                    Some(iface.clone())
                                }
                                _ => None,
                            })
                        })
                    {
                        self.vtable
                            .register_interface_implementation(&decl.name.name, &iface_decl);
                        if iface_id.name != base_name {
                            self.vtable.register_interface_implementation_with_name(
                                &decl.name.name,
                                &iface_id.name,
                                &iface_decl,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn region_type(&self) -> StructType<'ctx> {
        self.region.region_type()
    }

    pub fn is_regional_type(ty: &Type) -> bool {
        RegionManager::is_regional_type(ty)
    }

    pub fn transform_function_type(
        &self,
        param_types: &[BasicTypeEnum<'ctx>],
        return_type: &Type,
        llvm_ret: Option<BasicTypeEnum<'ctx>>,
        is_cfun: bool,
    ) -> (FunctionType<'ctx>, bool) {
        self.region
            .transform_function_type(param_types, return_type, llvm_ret, is_cfun)
    }

    pub fn transform_function_type_with_dest(
        &self,
        param_types: &[BasicTypeEnum<'ctx>],
        return_type: &Type,
        llvm_ret: Option<BasicTypeEnum<'ctx>>,
        is_cfun: bool,
        has_regional_ref: bool,
    ) -> (FunctionType<'ctx>, bool) {
        self.region
            .transform_function_type_with_dest(param_types, return_type, llvm_ret, is_cfun, has_regional_ref)
    }

    pub fn emit_region_prologue(
        &self,
        initial_capacity: u64,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        self.region
            .emit_region_prologue(&self.module, &self.builder, initial_capacity, name)
    }

    pub fn emit_region_epilogue(&self, region_ptr: PointerValue<'ctx>) -> Result<(), String> {
        self.region
            .emit_region_epilogue(&self.module, &self.builder, region_ptr)
    }

    pub fn emit_fast_path_alloc(
        &self,
        current_fn: FunctionValue<'ctx>,
        region_ptr: PointerValue<'ctx>,
        size: u64,
        align: u64,
    ) -> Result<PointerValue<'ctx>, String> {
        self.region.emit_fast_path_alloc(
            &self.module,
            &self.builder,
            current_fn,
            region_ptr,
            size,
            align,
        )
    }

    pub fn emit_loop_save_mark(
        &self,
        region_ptr: PointerValue<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        self.region
            .emit_loop_save_mark(&self.module, &self.builder, region_ptr)
    }

    pub fn emit_loop_restore_mark(
        &self,
        region_ptr: PointerValue<'ctx>,
        mark: PointerValue<'ctx>,
    ) -> Result<(), String> {
        self.region
            .emit_loop_restore_mark(&self.module, &self.builder, region_ptr, mark)
    }

    pub fn select_region_for_escape(
        &self,
        escape_kind: EscapeKind,
        local_region: PointerValue<'ctx>,
        dest_region: Option<PointerValue<'ctx>>,
    ) -> PointerValue<'ctx> {
        self.region
            .select_region_for_escape(escape_kind, local_region, dest_region)
    }

    // collect method default params for codegen-side filling
    fn register_defaults(&mut self, ast: &AstModule) {
        yukic_ast::defaults::collect_module_defaults(ast, None, &mut self.registry.defaults);
    }

    fn compile_item(&mut self, item: &yukic_ast::Item) -> Result<(), String> {
        let is_public = item.visibility == yukic_ast::item::Visibility::Public;
        match &item.kind {
            ItemKind::Static(decl) => self.compile_static_decl(decl)?,
            ItemKind::Fun(fun) => self.compile_fun_decl(fun)?,
            ItemKind::Cfun(cfun) => {
                self.compile_cfun_decl(cfun, is_public)?;
            }
            ItemKind::Class(decl) => self.compile_class_decl(decl)?,
            ItemKind::Externc(block) => self.compile_externc_block(block, is_public)?,
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn compile_externc_block(
        &mut self,
        block: &yukic_ast::item::ExterncBlock,
        is_public: bool,
    ) -> Result<(), String> {
        for item in &block.items {
            if let yukic_ast::item::ExterncItem::Cfun(cfun) = item {
                self.compile_cfun_decl(cfun, is_public)?;
            }
        }
        Ok(())
    }

    fn compile_class_decl(&mut self, decl: &yukic_ast::item::ClassDecl) -> Result<(), String> {
        for method in &decl.methods {
            self.compile_method_decl(&decl.name.name, method)?;
        }
        for accessor in &decl.accessors {
            self.compile_accessor_decl(&decl.name.name, accessor)?;
        }
        Ok(())
    }

    fn compile_accessor_decl(
        &mut self,
        class_name: &str,
        acc: &yukic_ast::item::PropertyAccessor,
    ) -> Result<(), String> {
        let (method_name, params, return_type) = match &acc.kind {
            yukic_ast::item::AccessorKind::Get => (
                format!("__get_{}", acc.name.name),
                Vec::new(),
                Some(acc.ty.clone()),
            ),
            yukic_ast::item::AccessorKind::Set(param_name) => (
                format!("__set_{}", acc.name.name),
                vec![yukic_ast::item::Param {
                    name: param_name.clone(),
                    is_ref: false,
                    is_mut: false,
                    is_variadic: false,
                    ty: Some(acc.ty.clone()),
                    default_val: None,
                    span: acc.span,
                }],
                None,
            ),
        };
        let synthetic_method = yukic_ast::item::MethodDecl {
            name: yukic_ast::ident::Ident::new(method_name, acc.name.span),
            attrs: acc.attrs.clone(),
            this_param: yukic_ast::item::ThisParam::Value,
            params,
            return_type,
            body: acc.body.clone(),
            is_operator: false,
            is_override: false,
            span: acc.span,
        };
        self.compile_method_decl(class_name, &synthetic_method)
    }

    fn compile_method_decl(
        &mut self,
        class_name: &str,
        method: &yukic_ast::item::MethodDecl,
    ) -> Result<(), String> {
        let ret_ty = method
            .return_type
            .as_ref()
            .map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
        let llvm_ret = self.llvm_return_type(&ret_ty)?;

        let mut param_types = Vec::new();
        if method.this_param != yukic_ast::item::ThisParam::None {
            param_types.push(self.ptr_type().into());
        }
        for param in &method.params {
            if param.is_ref {
                param_types.push(self.ptr_type().into());
            } else {
                let mut pty = param
                    .ty
                    .as_ref()
                    .map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
                if param.is_variadic && !matches!(pty, Type::Array(_)) {
                    pty = Type::Array(Box::new(pty));
                }
                param_types.push(self.llvm_type(&pty)?);
            }
        }

        let is_init = method.name.name == "init";
        let (fn_type, has_dest) = if is_init {
            let mut final_params = Vec::with_capacity(param_types.len() + 1);
            final_params.push(self.ptr_type().into());
            for pt in &param_types {
                final_params.push((*pt).into());
            }
            let fn_ty = match llvm_ret {
                Some(ret) => ret.fn_type(&final_params, false),
                None => self.context.void_type().fn_type(&final_params, false),
            };
            (fn_ty, true)
        } else {
            let has_regional_ref = method.params.iter().any(|p| {
                if p.is_ref {
                    let pty = p.ty.as_ref().map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
                    RegionManager::is_regional_type(&pty)
                } else {
                    false
                }
            });
            self.transform_function_type_with_dest(&param_types, &ret_ty, llvm_ret, false, has_regional_ref)
        };
        let sym_name = crate::mangle::method_symbol_name(
            class_name,
            &method.name.name,
            method.params.len(),
        );
        let fn_val = self
            .module
            .get_function(&sym_name)
            .unwrap_or_else(|| self.module.add_function(&sym_name, fn_type, None));
        fn_val.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);
        self.apply_fun_attributes(fn_val, &method.attrs);
        self.set_function_section(fn_val, &sym_name);

        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        let local_reg = if crate::region_elision::method_needs_local_region(method, has_dest, &self.registry) {
            Some(self.emit_region_prologue(DEFAULT_LOCAL_REGION_CAPACITY, "_local_region")?)
        } else {
            None
        };

        let dest_region = if has_dest {
            let dest_arg = fn_val.get_first_param().map(|p| p.into_pointer_value());
            if let Some(dest_p) = dest_arg {
                dest_p.set_name("_dest_region");
            }
            dest_arg
        } else {
            None
        };

        let mut body_compiler = crate::body::BodyCompiler::new(
            self.context,
            &self.builder,
            &self.module,
            &mut self.rodata,
            &self.intrinsic,
            &self.region,
            &self.vtable,
            &self.cabi,
            &self.registry,
            self.target,
            local_reg,
            dest_region,
            fn_val,
            llvm_ret,
            false,
        )
        .with_panic_strategy(&self.panic_strategy);
        body_compiler.current_class = Some(class_name.to_string());
        body_compiler.setup_method_params(
            &method.params,
            method.this_param != yukic_ast::item::ThisParam::None,
            has_dest,
        )?;
        body_compiler.compile_block(&method.body)?;

        if !body_compiler.is_terminated() {
            if let Some(reg) = local_reg {
                self.emit_region_epilogue(reg)?;
            }
            match llvm_ret {
                Some(ret) => {
                    let zero = ret.const_zero();
                    self.builder
                        .build_return(Some(&zero))
                        .map_err(|e| e.to_string())?;
                }
                None => {
                    self.builder.build_return(None).map_err(|e| e.to_string())?;
                }
            }
        }

        Ok(())
    }

    fn compile_static_decl(&mut self, decl: &yukic_ast::item::StaticDecl) -> Result<(), String> {
        if let ExprKind::ArrayInit(elements) = &decl.init.kind {
            let slice_ty = self.slice_type();
            let ptr_ty = self.ptr_type();
            let i64_ty = self.context.i64_type();
            let count = elements.len() as u64;

            let slice_val = if elements.is_empty() {
                let null_ptr = ptr_ty.const_null();
                let zero = i64_ty.const_zero();
                slice_ty.const_named_struct(&[null_ptr.into(), zero.into(), zero.into()])
            } else {
                let elem_target_ty = decl.ty.as_ref().and_then(|t| {
                    let canon = self.ast_type_to_canonical_type(t);
                    match canon {
                        Type::Array(elem) => Some(*elem),
                        _ => None,
                    }
                });

                let mut vals = Vec::with_capacity(elements.len());
                for elem in elements {
                    let lit = eval_const_literal(elem)
                        .ok_or_else(|| "elements of static array must be constant literals".to_string())?;
                    let val = if let Some(ref elem_ty) = elem_target_ty {
                        self.emit_typed_const_literal(&lit, elem_ty)?
                    } else {
                        self.emit_literal(&lit)?
                    };
                    vals.push(val);
                }

                let elem_ty = vals[0].get_type();
                let arr_ty = elem_ty.array_type(elements.len() as u32);
                let arr_val = make_const_array(elem_ty, &vals)?;
                let arr_name = format!(".arr.{}", decl.name.name);
                let arr_global = self.module.add_global(arr_ty, Some(AddressSpace::default()), &arr_name);
                arr_global.set_initializer(&arr_val);
                arr_global.set_constant(true);
                arr_global.set_linkage(inkwell::module::Linkage::Internal);

                let count_val = i64_ty.const_int(count, false);
                slice_ty.const_named_struct(&[arr_global.as_pointer_value().into(), count_val.into(), count_val.into()])
            };

            let global = self.module.get_global(&decl.name.name).unwrap_or_else(|| {
                self.module.add_global(
                    slice_val.get_type(),
                    Some(AddressSpace::default()),
                    &decl.name.name,
                )
            });
            global.set_initializer(&slice_val);
            global.set_constant(true);
            if self.function_sections {
                global.set_section(Some(&format!(".rodata.{}", decl.name.name)));
            }
            return Ok(());
        }

        if let Some(lit) = eval_const_literal(&decl.init) {
            let target_ty = decl.ty.as_ref().map(|t| self.ast_type_to_canonical_type(t));
            let const_val = if let Some(target_ty) = target_ty {
                self.emit_typed_const_literal(&lit, &target_ty)?
            } else {
                self.emit_literal(&lit)?
            };
            let global = self.module.get_global(&decl.name.name).unwrap_or_else(|| {
                self.module.add_global(
                    const_val.get_type(),
                    Some(AddressSpace::default()),
                    &decl.name.name,
                )
            });
            global.set_initializer(&const_val);
            global.set_constant(true);
            if self.function_sections {
                global.set_section(Some(&format!(".rodata.{}", decl.name.name)));
            }
        }
        Ok(())
    }

    fn emit_typed_const_literal(
        &mut self,
        lit: &LiteralValue,
        ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let llvm_ty = self.llvm_type(ty)?;
        match (lit, llvm_ty) {
            (LiteralValue::Int(v), BasicTypeEnum::IntType(it)) => {
                Ok(it.const_int((*v).cast_unsigned(), true).into())
            }
            (LiteralValue::UInt(v), BasicTypeEnum::IntType(it)) => {
                Ok(it.const_int(*v, false).into())
            }
            (LiteralValue::Float(v), BasicTypeEnum::FloatType(ft)) => {
                Ok(ft.const_float(*v).into())
            }
            (LiteralValue::Int(v), BasicTypeEnum::FloatType(ft)) => {
                Ok(ft.const_float(*v as f64).into())
            }
            (LiteralValue::UInt(v), BasicTypeEnum::FloatType(ft)) => {
                Ok(ft.const_float(*v as f64).into())
            }
            _ => self.emit_literal(lit),
        }
    }

    pub(crate) fn compile_fun_decl(&mut self, fun: &yukic_ast::item::FunDecl) -> Result<(), String> {
        if fun.name.name == "main" {
            return self.compile_main_fun(fun);
        }

        let ret_ty = fun
            .return_type
            .as_ref()
            .map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
        let llvm_ret = self.llvm_return_type(&ret_ty)?;

        let mut param_types = Vec::new();
        for param in &fun.params {
            if param.is_ref {
                param_types.push(self.ptr_type().into());
            } else {
                let mut pty = param
                    .ty
                    .as_ref()
                    .map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
                if param.is_variadic && !matches!(pty, Type::Array(_)) {
                    pty = Type::Array(Box::new(pty));
                }
                param_types.push(self.llvm_type(&pty)?);
            }
        }

        let is_thunk = fun.attrs.iter().any(|a| a.is("thunk"));
        let has_regional_ref = fun.params.iter().any(|p| {
            if p.is_ref {
                let pty = p.ty.as_ref().map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
                RegionManager::is_regional_type(&pty)
            } else {
                false
            }
        });
        let (fn_type, has_dest) =
            self.transform_function_type_with_dest(&param_types, &ret_ty, llvm_ret, is_thunk, has_regional_ref);
        let sym_name = self.symbol_for_fun(fun);
        let fn_val = self
            .module
            .get_function(&sym_name)
            .unwrap_or_else(|| self.module.add_function(&sym_name, fn_type, None));
        let call_conv = if is_thunk {
            inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32
        } else {
            inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32
        };
        fn_val.set_call_conventions(call_conv);
        self.apply_fun_attributes(fn_val, &fun.attrs);
        self.set_function_section(fn_val, &sym_name);

        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        let local_reg = if crate::region_elision::fun_needs_local_region(fun, has_dest, &self.registry) {
            Some(self.emit_region_prologue(DEFAULT_LOCAL_REGION_CAPACITY, "_local_region")?)
        } else {
            None
        };

        let dest_region = if has_dest {
            let dest_arg = fn_val.get_first_param().map(|p| p.into_pointer_value());
            if let Some(dest_p) = dest_arg {
                dest_p.set_name("_dest_region");
            }
            dest_arg
        } else {
            None
        };

        let mut body_compiler = crate::body::BodyCompiler::new(
            self.context,
            &self.builder,
            &self.module,
            &mut self.rodata,
            &self.intrinsic,
            &self.region,
            &self.vtable,
            &self.cabi,
            &self.registry,
            self.target,
            local_reg,
            dest_region,
            fn_val,
            llvm_ret,
            false,
        )
        .with_panic_strategy(&self.panic_strategy);
        body_compiler.setup_params(&fun.params, has_dest)?;
        body_compiler.compile_block(&fun.body)?;

        if !body_compiler.is_terminated() {
            if let Some(reg) = local_reg {
                self.emit_region_epilogue(reg)?;
            }
            match llvm_ret {
                Some(ret) => {
                    let zero = ret.const_zero();
                    self.builder
                        .build_return(Some(&zero))
                        .map_err(|e| e.to_string())?;
                }
                None => {
                    self.builder.build_return(None).map_err(|e| e.to_string())?;
                }
            }
        }

        Ok(())
    }

    fn compile_main_fun(&mut self, fun: &yukic_ast::item::FunDecl) -> Result<(), String> {
        let i32_type = self.context.i32_type();
        let ptr_type = self.ptr_type();
        let main_fn_type = i32_type.fn_type(&[i32_type.into(), ptr_type.into()], false);

        let fn_val = self.module.add_function("main", main_fn_type, None);
        fn_val.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
        self.apply_fun_attributes(fn_val, &fun.attrs);
        self.set_function_section(fn_val, "main");

        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        if let (Some(argc_arg), Some(argv_arg)) =
            (fn_val.get_first_param(), fn_val.get_params().get(1))
        {
            let _ = self.intrinsic.setup_main_args(
                &self.module,
                &self.builder,
                argc_arg.into_int_value(),
                argv_arg.into_pointer_value(),
            );
        }

        let app_reg = self.emit_region_prologue(DEFAULT_APP_REGION_CAPACITY, "_app_region")?;

        let mut body_compiler = crate::body::BodyCompiler::new(
            self.context,
            &self.builder,
            &self.module,
            &mut self.rodata,
            &self.intrinsic,
            &self.region,
            &self.vtable,
            &self.cabi,
            &self.registry,
            self.target,
            Some(app_reg),
            None,
            fn_val,
            Some(i32_type.into()),
            true,
        )
        .with_panic_strategy(&self.panic_strategy);

        body_compiler.compile_block(&fun.body)?;

        if !body_compiler.is_terminated() {
            self.emit_region_epilogue(app_reg)?;
            self.builder
                .build_return(Some(&i32_type.const_int(0, false)))
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub fn compile_cfun_decl(
        &mut self,
        cfun: &yukic_ast::item::CfunDecl,
        is_public: bool,
    ) -> Result<FunctionValue<'ctx>, String> {
        let ret_ty = cfun
            .return_type
            .as_ref()
            .map_or(Type::Void, |t| self.ast_type_to_canonical_type(t));
        let llvm_ret = self.llvm_return_type(&ret_ty)?;

        let mut param_types = Vec::new();
        for param in &cfun.params {
            let pty = self.ast_type_to_canonical_type(&param.ty);
            param_types.push(self.llvm_type(&pty)?);
        }

        let (fn_type, _) = self.transform_function_type(&param_types, &ret_ty, llvm_ret, true);
        let sym_name = self.symbol_for_cfun(cfun);

        let fn_val = self
            .module
            .get_function(&sym_name)
            .unwrap_or_else(|| self.module.add_function(&sym_name, fn_type, None));

        self.cabi
            .configure_cfun(fn_val, &cfun.c_attrs, is_public, cfun.body.is_some())?;
        self.apply_fun_attributes(fn_val, &cfun.yukino_attrs);
        self.set_function_section(fn_val, &sym_name);

        if let Some(body) = &cfun.body {
            let entry = self.context.append_basic_block(fn_val, "entry");
            self.builder.position_at_end(entry);

            let local_reg = if crate::region_elision::block_requires_region(body, &self.registry) {
                Some(self.emit_region_prologue(DEFAULT_LOCAL_REGION_CAPACITY, "_local_region")?)
            } else {
                None
            };

            let mut body_compiler = crate::body::BodyCompiler::new(
                self.context,
                &self.builder,
                &self.module,
                &mut self.rodata,
                &self.intrinsic,
                &self.region,
                &self.vtable,
                &self.cabi,
                &self.registry,
                self.target,
                local_reg,
                None,
                fn_val,
                llvm_ret,
                false,
            )
            .with_panic_strategy(&self.panic_strategy);
            body_compiler.setup_cparams(&cfun.params)?;
            body_compiler.compile_block(body)?;

            if !body_compiler.is_terminated() {
                if let Some(reg) = local_reg {
                    self.emit_region_epilogue(reg)?;
                }
                match llvm_ret {
                    Some(ret) => {
                        let zero = ret.const_zero();
                        self.builder
                            .build_return(Some(&zero))
                            .map_err(|e| e.to_string())?;
                    }
                    None => {
                        self.builder.build_return(None).map_err(|e| e.to_string())?;
                    }
                }
            }
        }

        Ok(fn_val)
    }

    pub fn emit_class_vtable(&mut self, class_name: &str) -> Result<GlobalValue<'ctx>, String> {
        self.vtable.emit_class_vtable(&self.module, class_name)
    }

    pub fn emit_interface_itable(
        &mut self,
        class_name: &str,
        iface_name: &str,
    ) -> Result<GlobalValue<'ctx>, String> {
        self.vtable
            .emit_interface_itable(&self.module, class_name, iface_name)
    }

    pub fn init_instance_vtable(
        &self,
        instance_ptr: PointerValue<'ctx>,
        vtable_global: GlobalValue<'ctx>,
    ) -> Result<(), String> {
        self.vtable
            .init_instance_vtable(&self.builder, instance_ptr, vtable_global)
    }

    pub fn emit_virtual_call(
        &self,
        class_name: &str,
        method_name: &str,
        instance_ptr: PointerValue<'ctx>,
        fn_type: FunctionType<'ctx>,
        extra_args: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<CallSiteValue<'ctx>, String> {
        self.vtable.emit_virtual_call(
            &self.builder,
            class_name,
            method_name,
            instance_ptr,
            fn_type,
            extra_args,
        )
    }

    pub fn emit_super_call(
        &self,
        base_fn: FunctionValue<'ctx>,
        instance_ptr: PointerValue<'ctx>,
        extra_args: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<CallSiteValue<'ctx>, String> {
        self.vtable
            .emit_super_call(&self.builder, base_fn, instance_ptr, extra_args)
    }

    pub fn emit_interface_call(
        &self,
        iface_fat_val: StructValue<'ctx>,
        slot_index: u32,
        fn_type: FunctionType<'ctx>,
        extra_args: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<CallSiteValue<'ctx>, String> {
        self.vtable.emit_interface_call(
            &self.builder,
            iface_fat_val,
            slot_index,
            fn_type,
            extra_args,
        )
    }

    pub fn get_interface_slot(&self, iface_name: &str, method_name: &str) -> Option<u32> {
        self.vtable.get_interface_slot(iface_name, method_name)
    }

    pub fn make_interface_fat_pointer(
        &self,
        instance_ptr: PointerValue<'ctx>,
        itable_global: GlobalValue<'ctx>,
    ) -> Result<StructValue<'ctx>, String> {
        self.vtable
            .make_interface_fat_pointer(&self.builder, instance_ptr, itable_global)
    }

    pub fn create_closure_env_layout(
        &self,
        name: &str,
        captures: &[CapturedVar],
    ) -> Result<ClosureEnvLayout<'ctx>, String> {
        self.closure
            .create_env_layout(name, captures, &self.lowerer())
    }

    pub fn allocate_and_init_closure_env(
        &self,
        current_fn: FunctionValue<'ctx>,
        region_ptr: PointerValue<'ctx>,
        layout: &ClosureEnvLayout<'ctx>,
        values: &[BasicValueEnum<'ctx>],
    ) -> Result<PointerValue<'ctx>, String> {
        self.closure.allocate_and_init_env(
            &self.module,
            &self.builder,
            current_fn,
            &self.region,
            region_ptr,
            layout,
            values,
        )
    }

    pub fn make_closure_fat_pointer(
        &self,
        fn_ptr: PointerValue<'ctx>,
        env_ptr: PointerValue<'ctx>,
        name: &str,
    ) -> Result<StructValue<'ctx>, String> {
        self.closure
            .make_closure_fat_pointer(&self.builder, fn_ptr, env_ptr, name)
    }

    pub fn unpack_closure_environment(
        &self,
        env_ptr: PointerValue<'ctx>,
        layout: &ClosureEnvLayout<'ctx>,
    ) -> Result<Vec<(String, BasicValueEnum<'ctx>)>, String> {
        self.closure
            .unpack_environment(&self.builder, env_ptr, layout)
    }

    pub fn emit_closure_call(
        &self,
        closure_fat: StructValue<'ctx>,
        fn_type: FunctionType<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
        dest_region: Option<PointerValue<'ctx>>,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        self.closure
            .emit_closure_call(&self.builder, closure_fat, fn_type, args, dest_region, name)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn emit_closure_invoke(
        &self,
        closure_fat: StructValue<'ctx>,
        fn_type: FunctionType<'ctx>,
        args: &[BasicValueEnum<'ctx>],
        then_block: inkwell::basic_block::BasicBlock<'ctx>,
        unwind_block: inkwell::basic_block::BasicBlock<'ctx>,
        dest_region: Option<PointerValue<'ctx>>,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        self.closure.emit_closure_invoke(
            &self.builder,
            closure_fat,
            fn_type,
            args,
            then_block,
            unwind_block,
            dest_region,
            name,
        )
    }

    pub fn push_scope(&mut self, kind: ScopeKind<'ctx>) {
        self.scopes.push_scope(kind);
    }

    pub fn pop_scope(&mut self) -> Option<Scope<'ctx>> {
        self.scopes.pop_scope()
    }

    pub fn add_cleanup(&mut self, action: CleanupAction<'ctx>) -> Result<(), String> {
        self.scopes.add_cleanup(action)
    }

    pub fn collect_return_cleanups(&self) -> Vec<CleanupAction<'ctx>> {
        self.scopes.collect_return_cleanups()
    }

    pub fn collect_loop_cleanups(
        &self,
        is_break: bool,
    ) -> Result<
        (
            Vec<CleanupAction<'ctx>>,
            inkwell::basic_block::BasicBlock<'ctx>,
        ),
        String,
    > {
        self.scopes.collect_loop_cleanups(is_break, None)
    }

    pub fn collect_loop_cleanups_with_label(
        &self,
        is_break: bool,
        target_label: Option<&str>,
    ) -> Result<
        (
            Vec<CleanupAction<'ctx>>,
            inkwell::basic_block::BasicBlock<'ctx>,
        ),
        String,
    > {
        self.scopes.collect_loop_cleanups(is_break, target_label)
    }

    pub fn setup_personality_function(&self, fn_val: FunctionValue<'ctx>) -> FunctionValue<'ctx> {
        let personality = self.exception.get_or_declare_personality(&self.module);
        self.exception.set_personality_function(fn_val, personality);
        personality
    }

    pub fn emit_panic(
        &self,
        msg_ptr: PointerValue<'ctx>,
        msg_len: inkwell::values::IntValue<'ctx>,
    ) -> Result<(), String> {
        self.exception
            .emit_panic(&self.module, &self.builder, msg_ptr, msg_len)
    }

    pub fn emit_invoke(
        &self,
        callee: FunctionValue<'ctx>,
        args: &[BasicValueEnum<'ctx>],
        then_block: inkwell::basic_block::BasicBlock<'ctx>,
        unwind_block: inkwell::basic_block::BasicBlock<'ctx>,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        self.exception
            .emit_invoke(&self.builder, callee, args, then_block, unwind_block, name)
    }

    pub fn emit_indirect_invoke(
        &self,
        fn_type: FunctionType<'ctx>,
        fn_ptr: PointerValue<'ctx>,
        args: &[BasicValueEnum<'ctx>],
        then_block: inkwell::basic_block::BasicBlock<'ctx>,
        unwind_block: inkwell::basic_block::BasicBlock<'ctx>,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        self.exception.emit_indirect_invoke(
            &self.builder,
            fn_type,
            fn_ptr,
            args,
            then_block,
            unwind_block,
            name,
        )
    }

    pub fn emit_landing_pad(
        &self,
        personality_fn: FunctionValue<'ctx>,
        typeinfo_clauses: &[GlobalValue<'ctx>],
        has_catch_all: bool,
        has_cleanup: bool,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, BuilderError> {
        self.exception.emit_landing_pad(
            &self.builder,
            personality_fn,
            typeinfo_clauses,
            has_catch_all,
            has_cleanup,
            name,
        )
    }

    pub fn emit_resume(&self, eh_val: BasicValueEnum<'ctx>) -> Result<(), BuilderError> {
        self.exception.emit_resume(&self.builder, eh_val)
    }

    pub fn extract_exception_pointer(
        &self,
        eh_val: BasicValueEnum<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, BuilderError> {
        self.exception
            .extract_exception_pointer(&self.builder, eh_val, name)
    }

    pub fn extract_exception_selector(
        &self,
        eh_val: BasicValueEnum<'ctx>,
        name: &str,
    ) -> Result<inkwell::values::IntValue<'ctx>, BuilderError> {
        self.exception
            .extract_exception_selector(&self.builder, eh_val, name)
    }

    pub fn get_or_create_typeinfo(&self, error_type_name: &str) -> GlobalValue<'ctx> {
        self.exception
            .get_or_create_typeinfo(&self.module, error_type_name)
    }

    pub fn emit_typeid_for(
        &self,
        typeinfo_global: GlobalValue<'ctx>,
        name: &str,
    ) -> Result<inkwell::values::IntValue<'ctx>, BuilderError> {
        let eh_typeid_fn = self.exception.get_or_declare_eh_typeid(&self.module);
        let val = self.builder.build_call(
            eh_typeid_fn,
            &[typeinfo_global.as_pointer_value().into()],
            name,
        )?;
        Ok(val.try_as_basic_value().unwrap_basic().into_int_value())
    }

    pub fn cabi(&self) -> &CAbiManager<'ctx> {
        &self.cabi
    }

    pub fn emit_cfun_call(
        &self,
        fn_val: FunctionValue<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
        call_conv: u32,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        self.cabi
            .emit_cfun_call(&self.builder, fn_val, args, call_conv, name)
    }

    pub fn emit_indirect_cfun_call(
        &self,
        fn_type: FunctionType<'ctx>,
        fn_ptr: PointerValue<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
        call_conv: u32,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        self.cabi
            .emit_indirect_cfun_call(&self.builder, fn_type, fn_ptr, args, call_conv, name)
    }

    pub fn get_cfun_pointer(&self, fn_val: FunctionValue<'ctx>) -> PointerValue<'ctx> {
        self.cabi.as_fn_ptr(fn_val)
    }

    pub fn emit_ptr_deref(
        &self,
        elem_type: BasicTypeEnum<'ctx>,
        ptr: PointerValue<'ctx>,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, BuilderError> {
        self.cabi
            .emit_ptr_deref(&self.builder, elem_type, ptr, name)
    }

    pub fn emit_ptr_deref_set(
        &self,
        ptr: PointerValue<'ctx>,
        val: BasicValueEnum<'ctx>,
    ) -> Result<InstructionValue<'ctx>, BuilderError> {
        self.cabi.emit_ptr_deref_set(&self.builder, ptr, val)
    }

    pub fn emit_ptr_offset(
        &self,
        ptr: PointerValue<'ctx>,
        byte_offset: IntValue<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, BuilderError> {
        self.cabi
            .emit_ptr_offset(&self.builder, ptr, byte_offset, name)
    }

    pub fn emit_ptr_is_null(
        &self,
        ptr: PointerValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, BuilderError> {
        self.cabi.emit_ptr_is_null(&self.builder, ptr, name)
    }

    pub fn emit_ptr_to_int(
        &self,
        ptr: PointerValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, BuilderError> {
        self.cabi.emit_ptr_to_int(&self.builder, ptr, name)
    }

    pub fn emit_int_to_ptr(
        &self,
        int_val: IntValue<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, BuilderError> {
        self.cabi.emit_int_to_ptr(&self.builder, int_val, name)
    }

    pub fn emit_null_ptr(&self) -> PointerValue<'ctx> {
        self.cabi.emit_null_ptr()
    }

    pub fn emit_cstruct_alloca(
        &self,
        struct_type: StructType<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, BuilderError> {
        let alloca = self
            .cabi
            .emit_cstruct_alloca(&self.builder, struct_type, name)?;
        let struct_name = struct_type.get_name().and_then(|n| n.to_str().ok());
        if let Some(raw_name) = struct_name.and_then(|n| n.strip_prefix("cstruct.")) {
            if let Some(layout) = self.registry.cstructs.get(raw_name) {
                if let Some(align) = layout.align {
                    if let Some(inst) = alloca.as_instruction() {
                        let _ = inst.set_alignment(align);
                    }
                }
            }
        }
        Ok(alloca)
    }

    pub fn emit_cstruct_field_gep(
        &self,
        struct_type: StructType<'ctx>,
        ptr: PointerValue<'ctx>,
        field_idx: u32,
        name: &str,
    ) -> Result<PointerValue<'ctx>, BuilderError> {
        self.cabi
            .emit_cstruct_field_gep(&self.builder, struct_type, ptr, field_idx, name)
    }

    pub fn emit_cstruct_load_field(
        &self,
        struct_type: StructType<'ctx>,
        ptr: PointerValue<'ctx>,
        field_idx: u32,
        field_type: BasicTypeEnum<'ctx>,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, BuilderError> {
        self.cabi.emit_cstruct_load_field(
            &self.builder,
            struct_type,
            ptr,
            field_idx,
            field_type,
            name,
        )
    }

    pub fn emit_cstruct_store_field(
        &self,
        struct_type: StructType<'ctx>,
        ptr: PointerValue<'ctx>,
        field_idx: u32,
        val: BasicValueEnum<'ctx>,
    ) -> Result<InstructionValue<'ctx>, BuilderError> {
        self.cabi
            .emit_cstruct_store_field(&self.builder, struct_type, ptr, field_idx, val)
    }

    pub fn emit_to_cstr(
        &self,
        current_fn: FunctionValue<'ctx>,
        region_ptr: PointerValue<'ctx>,
        str_val: StructValue<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        self.cabi.emit_to_cstr(
            &self.module,
            &self.builder,
            current_fn,
            &self.region,
            region_ptr,
            str_val,
            name,
        )
    }

    pub fn emit_from_cstr(
        &self,
        cstr_ptr: PointerValue<'ctx>,
        name: &str,
    ) -> Result<StructValue<'ctx>, String> {
        self.cabi
            .emit_from_cstr(&self.module, &self.builder, cstr_ptr, name)
    }

    pub fn emit_from_cstr_n(
        &self,
        cstr_ptr: PointerValue<'ctx>,
        len: IntValue<'ctx>,
        name: &str,
    ) -> Result<StructValue<'ctx>, BuilderError> {
        self.cabi
            .emit_from_cstr_n(&self.builder, cstr_ptr, len, name)
    }

    pub fn emit_exit(&self, code: IntValue<'ctx>) -> Result<InstructionValue<'ctx>, String> {
        self.intrinsic.emit_exit(&self.module, &self.builder, code)
    }

    pub fn emit_cast(
        &self,
        val: BasicValueEnum<'ctx>,
        src_ty: &Type,
        target_ty: &Type,
        region_ptr: Option<PointerValue<'ctx>>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.intrinsic.emit_cast(
            &self.module,
            &self.builder,
            val,
            src_ty,
            target_ty,
            region_ptr,
        )
    }

    pub fn emit_args(
        &self,
        region_ptr: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.intrinsic
            .emit_args(&self.module, &self.builder, region_ptr)
    }

    pub fn emit_raw_args(
        &self,
        region_ptr: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.intrinsic
            .emit_raw_args(&self.module, &self.builder, region_ptr)
    }

    pub fn emit_slice_len(
        &self,
        slice_or_str: BasicValueEnum<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        self.intrinsic.emit_slice_len(&self.builder, slice_or_str)
    }

    pub fn emit_slice_cap(
        &self,
        slice_val: BasicValueEnum<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        self.intrinsic.emit_slice_cap(&self.builder, slice_val)
    }

    pub fn emit_slice_as_ptr(
        &self,
        slice_or_str: BasicValueEnum<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        self.intrinsic
            .emit_slice_as_ptr(&self.builder, slice_or_str)
    }

    pub fn emit_slice_push(
        &self,
        region_ptr: PointerValue<'ctx>,
        slice_ptr: PointerValue<'ctx>,
        elem_ptr: PointerValue<'ctx>,
        layout: SliceElemLayout<'ctx>,
    ) -> Result<(), String> {
        self.intrinsic.emit_slice_push(
            &self.module,
            &self.builder,
            region_ptr,
            slice_ptr,
            elem_ptr,
            layout,
        )
    }

    pub fn emit_slice_del(
        &self,
        slice_ptr: PointerValue<'ctx>,
        index: IntValue<'ctx>,
        elem_size: IntValue<'ctx>,
    ) -> Result<(), String> {
        self.intrinsic
            .emit_slice_del(&self.module, &self.builder, slice_ptr, index, elem_size)
    }

    pub fn emit_slice_reserve(
        &self,
        region_ptr: PointerValue<'ctx>,
        slice_ptr: PointerValue<'ctx>,
        additional: IntValue<'ctx>,
        layout: SliceElemLayout<'ctx>,
    ) -> Result<(), String> {
        self.intrinsic.emit_slice_reserve(
            &self.module,
            &self.builder,
            region_ptr,
            slice_ptr,
            additional,
            layout,
        )
    }

    pub fn emit_string_from_raw_parts(
        &self,
        ptr_val: PointerValue<'ctx>,
        len_val: IntValue<'ctx>,
    ) -> Result<StructValue<'ctx>, String> {
        self.intrinsic
            .emit_string_from_raw_parts(&self.builder, ptr_val, len_val)
    }

    pub fn emit_slice_from_raw_parts(
        &self,
        ptr_val: PointerValue<'ctx>,
        len_val: IntValue<'ctx>,
        cap_val: IntValue<'ctx>,
    ) -> Result<StructValue<'ctx>, String> {
        self.intrinsic
            .emit_slice_from_raw_parts(&self.builder, ptr_val, len_val, cap_val)
    }

    pub fn emit_slice_elem_ptr(
        &self,
        slice_or_str: BasicValueEnum<'ctx>,
        index: IntValue<'ctx>,
        elem_ty: BasicTypeEnum<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        self.intrinsic
            .emit_slice_elem_ptr(&self.builder, slice_or_str, index, elem_ty)
    }

    pub fn emit_subslice(
        &self,
        slice_or_str: BasicValueEnum<'ctx>,
        start: IntValue<'ctx>,
        end: IntValue<'ctx>,
        elem_size: IntValue<'ctx>,
        is_string: bool,
    ) -> Result<StructValue<'ctx>, String> {
        self.intrinsic.emit_subslice(
            &self.builder,
            slice_or_str,
            start,
            end,
            elem_size,
            is_string,
        )
    }

    pub fn compile_codegen_unit(
        &mut self,
        _ast: &AstModule,
        _cgu: &CodegenUnit,
    ) -> Result<(), String> {
        self.module.verify().map_err(|e| e.to_string())
    }

    pub fn llvm_module(&self) -> &LlvmModule<'ctx> {
        &self.module
    }

    pub fn context(&self) -> &'ctx Context {
        self.context
    }

    pub fn builder(&self) -> &Builder<'ctx> {
        &self.builder
    }

    pub fn target(&self) -> TargetTriple {
        self.target
    }

    fn apply_fun_attributes(
        &self,
        fn_val: FunctionValue<'ctx>,
        attrs: &[yukic_ast::AttributeNode],
    ) {
        let mut nounwind = false;
        for attr in attrs {
            match attr.path_str().as_str() {
                "stable" => {
                    self.add_fn_attribute(fn_val, "nounwind");
                    nounwind = true;
                    self.add_fn_attribute(fn_val, "willreturn");
                    let mem_id = Attribute::get_named_enum_kind_id("memory");
                    if mem_id != 0 {
                        let a = self.context.create_enum_attribute(mem_id, 0);
                        fn_val.add_attribute(AttributeLoc::Function, a);
                    } else {
                        self.add_fn_attribute(fn_val, "readnone");
                    }
                }
                "nounwind" => {
                    self.add_fn_attribute(fn_val, "nounwind");
                    nounwind = true;
                }
                "inline" => {
                    if let Some(arg) = Self::extract_string_attr_arg(&attr.args) {
                        if arg == "always" {
                            self.add_fn_attribute(fn_val, "alwaysinline");
                        } else if arg == "never" {
                            self.add_fn_attribute(fn_val, "noinline");
                        } else {
                            self.add_fn_attribute(fn_val, "inlinehint");
                        }
                    } else {
                        self.add_fn_attribute(fn_val, "inlinehint");
                    }
                }
                "no_inline" | "inline_never" => self.add_fn_attribute(fn_val, "noinline"),
                "inline_always" => self.add_fn_attribute(fn_val, "alwaysinline"),
                "cold" => self.add_fn_attribute(fn_val, "cold"),
                "noreturn" => self.add_fn_attribute(fn_val, "noreturn"),
                _ => {}
            }
        }
        if !nounwind {
            self.add_fn_attribute_val(fn_val, "uwtable", 2);
        }
    }

    fn add_fn_attribute(&self, fn_val: FunctionValue<'ctx>, name: &str) {
        self.add_fn_attribute_val(fn_val, name, 0);
    }

    fn add_fn_attribute_val(&self, fn_val: FunctionValue<'ctx>, name: &str, val: u64) {
        let kind_id = Attribute::get_named_enum_kind_id(name);
        if kind_id != 0 {
            let attr = self.context.create_enum_attribute(kind_id, val);
            fn_val.add_attribute(AttributeLoc::Function, attr);
        }
    }

    fn extract_string_attr_arg(args: &yukic_ast::attr::AttributeArgs) -> Option<String> {
        match args {
            yukic_ast::attr::AttributeArgs::Positional(exprs) => {
                if let Some(expr) = exprs.first()
                    && let ExprKind::Literal(LiteralValue::String(s)) = &expr.kind
                {
                    Some(s.clone())
                } else {
                    None
                }
            }
            yukic_ast::attr::AttributeArgs::Named(pairs) => {
                if let Some((_, expr)) = pairs.first()
                    && let ExprKind::Literal(LiteralValue::String(s)) = &expr.kind
                {
                    Some(s.clone())
                } else {
                    None
                }
            }
            yukic_ast::attr::AttributeArgs::None => None,
        }
    }
}

pub fn embed_artifact_version_metadata<'ctx>(module: &LlvmModule<'ctx>, context: &'ctx Context) {
    let ident_str = format!("yukic v{}", env!("CARGO_PKG_VERSION"));
    let md_str = context.metadata_string(&ident_str);
    let md_node = context.metadata_node(&[md_str.into()]);
    let _ = module.add_global_metadata("llvm.ident", &md_node);
}

fn eval_const_literal(expr: &Expr) -> Option<LiteralValue> {
    match &expr.kind {
        ExprKind::Literal(lit) => Some(lit.clone()),
        ExprKind::Unary(yukic_ast::expr::UnaryOp::Neg, inner) => match eval_const_literal(inner)? {
            LiteralValue::Int(v) => Some(LiteralValue::Int(v.wrapping_neg())),
            LiteralValue::Float(v) => Some(LiteralValue::Float(-v)),
            _ => None,
        },
        ExprKind::Unary(yukic_ast::expr::UnaryOp::Not, inner) => match eval_const_literal(inner)? {
            LiteralValue::Bool(b) => Some(LiteralValue::Bool(!b)),
            _ => None,
        },
        ExprKind::Unary(yukic_ast::expr::UnaryOp::BitNot, inner) => match eval_const_literal(inner)? {
            LiteralValue::Int(v) => Some(LiteralValue::Int(!v)),
            LiteralValue::UInt(v) => Some(LiteralValue::UInt(!v)),
            _ => None,
        },
        _ => None,
    }
}

fn make_const_array<'ctx>(
    elem_ty: BasicTypeEnum<'ctx>,
    vals: &[BasicValueEnum<'ctx>],
) -> Result<ArrayValue<'ctx>, String> {
    match elem_ty {
        BasicTypeEnum::IntType(it) => {
            let ivs: Vec<IntValue<'ctx>> = vals.iter().map(|v| v.into_int_value()).collect();
            Ok(it.const_array(&ivs))
        }
        BasicTypeEnum::FloatType(ft) => {
            let fvs: Vec<FloatValue<'ctx>> = vals.iter().map(|v| v.into_float_value()).collect();
            Ok(ft.const_array(&fvs))
        }
        BasicTypeEnum::StructType(st) => {
            let svs: Vec<StructValue<'ctx>> = vals.iter().map(|v| v.into_struct_value()).collect();
            Ok(st.const_array(&svs))
        }
        BasicTypeEnum::PointerType(pt) => {
            let pvs: Vec<PointerValue<'ctx>> = vals.iter().map(|v| v.into_pointer_value()).collect();
            Ok(pt.const_array(&pvs))
        }
        BasicTypeEnum::ArrayType(at) => {
            let avs: Vec<ArrayValue<'ctx>> = vals.iter().map(|v| v.into_array_value()).collect();
            Ok(at.const_array(&avs))
        }
        _ => Err("unsupported element type for static const array".to_string()),
    }
}
