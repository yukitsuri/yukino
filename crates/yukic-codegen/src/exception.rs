#![deny(unsafe_code)]

use inkwell::AddressSpace;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::{Builder, BuilderError};
use inkwell::context::Context;
use inkwell::module::Module as LlvmModule;
use inkwell::types::{FunctionType, PointerType, StructType};
use inkwell::values::{
    BasicValueEnum, CallSiteValue, FunctionValue, GlobalValue, IntValue, PointerValue,
};

#[derive(Clone, Debug, PartialEq)]
pub enum ScopeKind<'ctx> {
    Function,
    Block,
    Loop {
        label: Option<String>,
        break_target: BasicBlock<'ctx>,
        continue_target: BasicBlock<'ctx>,
    },
    Sus,
}

#[derive(Clone, Debug)]
pub enum CleanupAction<'ctx> {
    Defer(yukic_ast::Block),
    RegionEpilogue(PointerValue<'ctx>),
    RegionRestoreMark {
        region: PointerValue<'ctx>,
        mark: PointerValue<'ctx>,
    },
    Custom(fn(&Builder<'ctx>) -> Result<(), String>),
}

#[derive(Clone, Debug)]
pub struct Scope<'ctx> {
    pub kind: ScopeKind<'ctx>,
    pub cleanups: Vec<CleanupAction<'ctx>>,
}

impl<'ctx> Scope<'ctx> {
    pub fn new(kind: ScopeKind<'ctx>) -> Self {
        Self {
            kind,
            cleanups: Vec::new(),
        }
    }
}

#[derive(Default)]
pub struct ScopeStack<'ctx> {
    scopes: Vec<Scope<'ctx>>,
}

impl<'ctx> ScopeStack<'ctx> {
    pub fn new() -> Self { Self { scopes: Vec::new() } }

    pub fn push_scope(&mut self, kind: ScopeKind<'ctx>) {
        self.scopes.push(Scope::new(kind));
    }

    pub fn pop_scope(&mut self) -> Option<Scope<'ctx>> {
        self.scopes.pop()
    }

    pub fn current_scope(&self) -> Option<&Scope<'ctx>> {
        self.scopes.last()
    }

    pub fn current_scope_mut(&mut self) -> Option<&mut Scope<'ctx>> {
        self.scopes.last_mut()
    }

    pub fn depth(&self) -> usize { self.scopes.len() }
    pub fn is_empty(&self) -> bool { self.scopes.is_empty() }

    pub fn add_cleanup(&mut self, action: CleanupAction<'ctx>) -> Result<(), String> {
        let is_sus_top = self.scopes.last().is_some_and(|s| matches!(s.kind, ScopeKind::Sus));
        match self.scopes.last_mut() {
            Some(scope) => { scope.cleanups.push(action.clone()); }
            None => return Err("cannot add cleanup: scope stack is empty".to_string()),
        }
        if !is_sus_top {
            for scope in self.scopes.iter_mut().rev() {
                if matches!(scope.kind, ScopeKind::Sus) {
                    scope.cleanups.push(action);
                    break;
                }
            }
        }
        
        Ok(())
    }

    pub fn collect_return_cleanups(&self) -> Vec<CleanupAction<'ctx>> {
        let mut result = Vec::new();
        for scope in self.scopes.iter().rev() {
            for cleanup in scope.cleanups.iter().rev() {
                result.push(cleanup.clone());
            }
        }
        result
    }

    pub fn collect_loop_cleanups(
        &self,
        is_break: bool,
        target_label: Option<&str>,
    ) -> Result<(Vec<CleanupAction<'ctx>>, BasicBlock<'ctx>), String> {
        let mut result = Vec::new();
        for scope in self.scopes.iter().rev() {
            for cleanup in scope.cleanups.iter().rev() {
                result.push(cleanup.clone());
            }
            if let ScopeKind::Loop {
                label,
                break_target,
                continue_target,
            } = &scope.kind
            {
                let is_match = match target_label {
                    Some(tl) => label.as_deref() == Some(tl),
                    None => true,
                };
                if is_match {
                    let target = if is_break {
                        *break_target
                    } else {
                        *continue_target
                    };
                    return Ok((result, target));
                }
            }
        }
        if let Some(tl) = target_label {
            Err(format!("loop with label '{tl}' not found in scope stack"))
        } else {
            Err("break/continue target loop not found in scope stack".to_string())
        }
    }

    pub fn collect_sus_cleanups(&self) -> Vec<CleanupAction<'ctx>> {
        let mut result = Vec::new();
        for scope in self.scopes.iter().rev() {
            for cleanup in scope.cleanups.iter().rev() {
                result.push(cleanup.clone());
            }

            if matches!(scope.kind, ScopeKind::Sus) { break; }
            
        } // for


        
        result
    }

    pub fn has_any_cleanups(&self) -> bool {
        self.scopes.iter().any(|s| !s.cleanups.is_empty())
    }

    pub fn current_scope_cleanups_lifo(&self) -> Vec<CleanupAction<'ctx>> {
        self.scopes
            .last()
            .map(|s| s.cleanups.iter().rev().cloned().collect())
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy)]
pub struct ExceptionManager<'ctx> {
    context: &'ctx Context,
}

impl<'ctx> ExceptionManager<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self { context }
    }

    pub fn ptr_type(&self) -> PointerType<'ctx> {
        self.context.ptr_type(AddressSpace::default())
    }

    pub fn landing_pad_type(&self) -> StructType<'ctx> {
        self.context.struct_type(
            &[self.ptr_type().into(), self.context.i32_type().into()],
            false,
        )
    }

    pub fn get_or_declare_personality(&self, module: &LlvmModule<'ctx>) -> FunctionValue<'ctx> {
        if let Some(f) = module.get_function("yukino_personality") {
            return f;
        }
        let fn_ty = self.context.i32_type().fn_type(&[], true);
        module.add_function("yukino_personality", fn_ty, None)
    }

    pub fn set_personality_function(
        &self,
        fn_val: FunctionValue<'ctx>,
        personality_fn: FunctionValue<'ctx>,
    ) {
        fn_val.set_personality_function(personality_fn);
    }

    pub fn get_or_declare_panic(&self, module: &LlvmModule<'ctx>) -> FunctionValue<'ctx> {
        if let Some(f) = module.get_function("yukino_raise_panic") {
            return f;
        }
        let fn_ty = self.context.void_type().fn_type(
            &[self.ptr_type().into(), self.context.i64_type().into()],
            false,
        );
        let fn_val = module.add_function("yukino_raise_panic", fn_ty, None);
        fn_val.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
        for attr_name in &["cold", "noreturn"] {
            let kind_id = inkwell::attributes::Attribute::get_named_enum_kind_id(attr_name);
            if kind_id != 0 {
                let attr = self.context.create_enum_attribute(kind_id, 0);
                fn_val.add_attribute(inkwell::attributes::AttributeLoc::Function, attr);
            }
        }
        fn_val
    }

    pub fn get_or_declare_eh_typeid(&self, module: &LlvmModule<'ctx>) -> FunctionValue<'ctx> {
        let name = "llvm.eh.typeid.for.p0";
        if let Some(f) = module.get_function(name) {
            return f;
        }
        let fn_ty = self
            .context
            .i32_type()
            .fn_type(&[self.ptr_type().into()], false);
        module.add_function(name, fn_ty, None)
    }

    pub fn get_or_create_typeinfo(
        &self,
        module: &LlvmModule<'ctx>,
        error_type_name: &str,
    ) -> GlobalValue<'ctx> {
        let sym_name = format!("_Y_TYPEINFO_{error_type_name}");
        if let Some(g) = module.get_global(&sym_name) {
            return g;
        }
        let global = module.add_global(self.ptr_type(), Some(AddressSpace::default()), &sym_name);
        global.set_initializer(&self.ptr_type().const_null());
        global.set_constant(true);
        global
    }

    pub fn emit_panic(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        msg_ptr: PointerValue<'ctx>,
        msg_len: IntValue<'ctx>,
    ) -> Result<(), String> {
        let panic_fn = self.get_or_declare_panic(module);
        builder
            .build_call(panic_fn, &[msg_ptr.into(), msg_len.into()], "panic_call")
            .map_err(|e| e.to_string())?;
        builder.build_unreachable().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn emit_invoke(
        &self,
        builder: &Builder<'ctx>,
        callee: FunctionValue<'ctx>,
        args: &[BasicValueEnum<'ctx>],
        then_block: BasicBlock<'ctx>,
        unwind_block: BasicBlock<'ctx>,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        builder.build_invoke(callee, args, then_block, unwind_block, name)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn emit_indirect_invoke(
        &self,
        builder: &Builder<'ctx>,
        fn_type: FunctionType<'ctx>,
        fn_ptr: PointerValue<'ctx>,
        args: &[BasicValueEnum<'ctx>],
        then_block: BasicBlock<'ctx>,
        unwind_block: BasicBlock<'ctx>,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        builder.build_indirect_invoke(fn_type, fn_ptr, args, then_block, unwind_block, name)
    }

    pub fn emit_landing_pad(
        &self,
        builder: &Builder<'ctx>,
        personality_fn: FunctionValue<'ctx>,
        typeinfo_clauses: &[GlobalValue<'ctx>],
        has_catch_all: bool,
        has_cleanup: bool,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, BuilderError> {
        let pad_type = self.landing_pad_type();
        let mut clauses: Vec<BasicValueEnum<'ctx>> = Vec::with_capacity(typeinfo_clauses.len() + 1);

        for g in typeinfo_clauses {
            clauses.push(g.as_pointer_value().into());
        }
        if has_catch_all {
            clauses.push(self.ptr_type().const_null().into());
        }

        builder.build_landing_pad(pad_type, personality_fn, &clauses, has_cleanup, name)
    }

    pub fn emit_resume(
        &self,
        builder: &Builder<'ctx>,
        eh_val: BasicValueEnum<'ctx>,
    ) -> Result<(), BuilderError> {
        builder.build_resume(eh_val).map(|_| ())
    }

    pub fn extract_exception_pointer(
        &self,
        builder: &Builder<'ctx>,
        eh_val: BasicValueEnum<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, BuilderError> {
        let struct_val = eh_val.into_struct_value();
        let ptr_val = builder.build_extract_value(struct_val, 0, name)?;
        Ok(ptr_val.into_pointer_value())
    }

    pub fn extract_exception_selector(
        &self,
        builder: &Builder<'ctx>,
        eh_val: BasicValueEnum<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, BuilderError> {
        let struct_val = eh_val.into_struct_value();
        let sel_val = builder.build_extract_value(struct_val, 1, name)?;
        Ok(sel_val.into_int_value())
    }
}
