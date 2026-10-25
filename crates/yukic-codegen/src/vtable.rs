#![deny(unsafe_code)]

use inkwell::AddressSpace;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module as LlvmModule;
use inkwell::types::{FunctionType, PointerType, StructType};
use inkwell::values::{
    BasicMetadataValueEnum, CallSiteValue, FunctionValue, GlobalValue, PointerValue, StructValue,
};
use std::collections::HashMap;
use yukic_ast::item::{ClassDecl, InterfaceDecl};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VTableSlot {
    pub method_name: String,
    pub slot_index: u32,
    pub mangled_symbol: String,
}

#[derive(Clone, Debug)]
pub struct VTableLayout {
    pub class_name: String,
    pub type_id: u64,
    pub base_class: Option<String>,
    pub slots: Vec<VTableSlot>,
}

#[derive(Clone, Debug)]
pub struct ITableLayout {
    pub class_name: String,
    pub interface_name: String,
    pub slots: Vec<VTableSlot>,
}

pub struct VTableManager<'ctx> {
    context: &'ctx Context,
    layouts: HashMap<String, VTableLayout>,
    itables: HashMap<(String, String), ITableLayout>,
    interfaces: HashMap<String, Vec<String>>,
    interface_decls: HashMap<String, InterfaceDecl>,
    global_vtables: HashMap<String, GlobalValue<'ctx>>,
    global_itables: HashMap<(String, String), GlobalValue<'ctx>>,
}

impl<'ctx> VTableManager<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self {
            context,
            layouts: HashMap::new(),
            itables: HashMap::new(),
            interfaces: HashMap::new(),
            interface_decls: HashMap::new(),
            global_vtables: HashMap::new(),
            global_itables: HashMap::new(),
        }
    }

    pub fn ptr_type(&self) -> PointerType<'ctx> {
        self.context.ptr_type(AddressSpace::default())
    }

    pub fn interface_fat_type(&self) -> StructType<'ctx> {
        let ptr = self.ptr_type();
        self.context.struct_type(&[ptr.into(), ptr.into()], false)
    }

    pub fn register_class(&mut self, decl: &ClassDecl, type_id: u64) {
        let base_name = decl.base_class.as_ref().map(|id| id.name.clone());
        let mut slots = Vec::new();

        // poly
        if let Some(ref base) = base_name
            && let Some(base_layout) = self.layouts.get(base)
        {
            slots.clone_from(&base_layout.slots);
        }
        // ovr
        for method in &decl.methods {
            let m_name = crate::mangle::mangle_method_name(&method.name.name, method.params.len());
            let class_name = &decl.name.name;
            let mangled = crate::mangle::method_symbol_name(class_name, &method.name.name, method.params.len());

            if let Some(slot) = slots.iter_mut().find(|s| s.method_name == m_name) {
                slot.mangled_symbol = mangled;
            } else {
                let slot_index = u32::try_from(slots.len()).unwrap_or(u32::MAX);
                slots.push(VTableSlot {
                    method_name: m_name.to_string(),
                    slot_index,
                    mangled_symbol: mangled,
                });
            }
        }

        let layout = VTableLayout {
            class_name: decl.name.name.clone(),
            type_id,
            base_class: base_name,
            slots,
        };
        self.layouts.insert(decl.name.name.clone(), layout);
    }

    pub fn register_interface(&mut self, iface: &InterfaceDecl) {
        self.interface_decls
            .insert(iface.name.name.clone(), iface.clone());
        self.interfaces
            .entry(iface.name.name.clone())
            .or_insert_with(|| {
                iface
                    .methods
                    .iter()
                    .map(|m| crate::mangle::mangle_method_name(&m.name.name, m.params.len()).to_string())
                    .collect()
            });
    }

    pub fn register_interface_implementation(&mut self, class_name: &str, iface: &InterfaceDecl) {
        self.register_interface_implementation_with_name(class_name, &iface.name.name, iface);
    }

    pub fn register_interface_implementation_with_name(
        &mut self,
        class_name: &str,
        iface_name: &str,
        iface: &InterfaceDecl,
    ) {
        self.register_interface(iface);

        let mut slots = Vec::new();
        for (idx, method) in iface.methods.iter().enumerate() {
            let m_name = crate::mangle::mangle_method_name(&method.name.name, method.params.len());
            let mangled = self
                .layouts
                .get(class_name)
                .and_then(|layout| layout.slots.iter().find(|s| s.method_name == m_name))
                .map(|s| s.mangled_symbol.clone())
                .unwrap_or_else(|| {
                    crate::mangle::method_symbol_name(class_name, &method.name.name, method.params.len())
                });
            let slot_index = u32::try_from(idx).unwrap_or(u32::MAX);
            slots.push(VTableSlot {
                method_name: m_name.to_string(),
                slot_index,
                mangled_symbol: mangled,
            });
        }

        let itable = ITableLayout {
            class_name: class_name.to_string(),
            interface_name: iface_name.to_string(),
            slots,
        };
        self.itables
            .insert((class_name.to_string(), iface_name.to_string()), itable);
    }

    pub fn get_layout(&self, class_name: &str) -> Option<&VTableLayout> {
        self.layouts.get(class_name)
    }

    pub fn get_class_type_id(&self, class_name: &str) -> Option<u64> {
        self.layouts.get(class_name).map(|l| l.type_id)
    }

    pub fn get_itable_layout(&self, class: &str, iface: &str) -> Option<&ITableLayout> {
        self.itables.get(&(class.to_string(), iface.to_string()))
    }

    pub fn get_interface_slot(&self, iface_name: &str, method_name: &str) -> Option<u32> {
        let methods = self.interfaces.get(iface_name)?;
        methods
            .iter()
            .position(|m| m == method_name)
            .and_then(|idx| u32::try_from(idx).ok())
    }

    pub fn get_interface_decl(&self, iface_name: &str) -> Option<&InterfaceDecl> {
        self.interface_decls.get(iface_name)
    }

    pub fn vtable_struct_type(&self, layout: &VTableLayout) -> StructType<'ctx> {
        let mut field_types = Vec::with_capacity(layout.slots.len() + 2);
        field_types.push(self.context.i64_type().into()); // type_id
        field_types.push(self.ptr_type().into()); // destructor
        for _ in &layout.slots {
            field_types.push(self.ptr_type().into());
        }
        self.context.struct_type(&field_types, false)
    }

    pub fn itable_struct_type(&self, itable: &ITableLayout) -> StructType<'ctx> {
        let field_types = vec![self.ptr_type().into(); itable.slots.len()];
        self.context.struct_type(&field_types, false)
    }

    pub fn emit_class_vtable(
        &mut self,
        module: &LlvmModule<'ctx>,
        class_name: &str,
    ) -> Result<GlobalValue<'ctx>, String> {
        self.emit_class_vtable_with_linkage(module, class_name, inkwell::module::Linkage::Internal)
    }

    pub fn emit_class_vtable_with_linkage(
        &mut self,
        module: &LlvmModule<'ctx>,
        class_name: &str,
        linkage: inkwell::module::Linkage,
    ) -> Result<GlobalValue<'ctx>, String> {
        let global = if let Some(g) = self.global_vtables.get(class_name) {
            if g.get_initializer().is_some() {
                return Ok(*g);
            }
            *g
        } else {
            let layout = self
                .layouts
                .get(class_name)
                .ok_or_else(|| format!("unknown class layout: {class_name}"))?
                .clone();

            let vtable_ty = self.vtable_struct_type(&layout);
            let global_name = format!("_Y_VTABLE_{class_name}");

            let global = module.add_global(vtable_ty, Some(AddressSpace::default()), &global_name);
            global.set_constant(true);
            global.set_alignment(8);
            self.global_vtables.insert(class_name.to_string(), global);
            global
        };

        let layout = self
            .layouts
            .get(class_name)
            .ok_or_else(|| format!("unknown class layout: {class_name}"))?
            .clone();

        let vtable_ty = self.vtable_struct_type(&layout);

        global.set_linkage(linkage);

        let type_id_val = self.context.i64_type().const_int(layout.type_id, false);
        let dtor_val = self.ptr_type().const_null();

        let mut field_vals = Vec::with_capacity(layout.slots.len() + 2);
        field_vals.push(type_id_val.into());
        field_vals.push(dtor_val.into());

        for slot in &layout.slots {
            let fn_val = self.get_or_declare_method_stub(module, &slot.mangled_symbol);
            field_vals.push(fn_val.as_global_value().as_pointer_value().into());
        }

        let const_struct = vtable_ty.const_named_struct(&field_vals);
        global.set_initializer(&const_struct);

        Ok(global)
    }

    pub fn declare_external_vtable(
        &mut self,
        module: &LlvmModule<'ctx>,
        class_name: &str,
    ) -> Result<GlobalValue<'ctx>, String> {
        if let Some(g) = self.global_vtables.get(class_name) {
            return Ok(*g);
        }

        let layout = self
            .layouts
            .get(class_name)
            .ok_or_else(|| format!("unknown class layout: {class_name}"))?
            .clone();

        let vtable_ty = self.vtable_struct_type(&layout);
        let global_name = format!("_Y_VTABLE_{class_name}");

        let global = module.add_global(vtable_ty, Some(AddressSpace::default()), &global_name);
        global.set_constant(true);
        global.set_linkage(inkwell::module::Linkage::External);
        global.set_alignment(8);

        self.global_vtables.insert(class_name.to_string(), global);
        Ok(global)
    }

    pub fn emit_interface_itable(
        &mut self,
        module: &LlvmModule<'ctx>,
        class_name: &str,
        iface_name: &str,
    ) -> Result<GlobalValue<'ctx>, String> {
        self.emit_interface_itable_with_linkage(
            module,
            class_name,
            iface_name,
            inkwell::module::Linkage::Internal,
        )
    }

    pub fn emit_interface_itable_with_linkage(
        &mut self,
        module: &LlvmModule<'ctx>,
        class_name: &str,
        iface_name: &str,
        linkage: inkwell::module::Linkage,
    ) -> Result<GlobalValue<'ctx>, String> {
        let key = (class_name.to_string(), iface_name.to_string());
        let global = if let Some(g) = self.global_itables.get(&key) {
            if g.get_initializer().is_some() {
                return Ok(*g);
            }
            *g
        } else {
            let itable = self
                .itables
                .get(&key)
                .ok_or_else(|| format!("no itable for ({class_name}, {iface_name})"))?
                .clone();

            let itable_ty = self.itable_struct_type(&itable);
            let global_name = crate::mangle::itable_symbol_name(class_name, iface_name);

            let global = module.add_global(itable_ty, Some(AddressSpace::default()), &global_name);
            global.set_constant(true);
            global.set_alignment(8);
            self.global_itables.insert(key.clone(), global);
            global
        };

        let itable = self
            .itables
            .get(&key)
            .ok_or_else(|| format!("no itable for ({class_name}, {iface_name})"))?
            .clone();

        let itable_ty = self.itable_struct_type(&itable);

        global.set_linkage(linkage);

        let mut field_vals = Vec::with_capacity(itable.slots.len());
        for slot in &itable.slots {
            let fn_val = self.get_or_declare_method_stub(module, &slot.mangled_symbol);
            field_vals.push(fn_val.as_global_value().as_pointer_value().into());
        }

        let const_struct = itable_ty.const_named_struct(&field_vals);
        global.set_initializer(&const_struct);

        Ok(global)
    }

    pub fn declare_external_itable(
        &mut self,
        module: &LlvmModule<'ctx>,
        class_name: &str,
        iface_name: &str,
    ) -> Result<GlobalValue<'ctx>, String> {
        let key = (class_name.to_string(), iface_name.to_string());
        if let Some(g) = self.global_itables.get(&key) {
            return Ok(*g);
        }

        let itable = self
            .itables
            .get(&key)
            .ok_or_else(|| format!("no itable for ({class_name}, {iface_name})"))?
            .clone();

        let itable_ty = self.itable_struct_type(&itable);
        let global_name = crate::mangle::itable_symbol_name(class_name, iface_name);

        let global = module.add_global(itable_ty, Some(AddressSpace::default()), &global_name);
        global.set_constant(true);
        global.set_linkage(inkwell::module::Linkage::External);
        global.set_alignment(8);

        self.global_itables.insert(key, global);
        Ok(global)
    }

    fn get_or_declare_method_stub(
        &self,
        module: &LlvmModule<'ctx>,
        symbol: &str,
    ) -> FunctionValue<'ctx> {
        if let Some(f) = module.get_function(symbol) {
            return f;
        }
        let fn_ty = self
            .context
            .void_type()
            .fn_type(&[self.ptr_type().into()], false);
        module.add_function(symbol, fn_ty, None)
    }

    pub fn emit_virtual_call(
        &self,
        builder: &Builder<'ctx>,
        class_name: &str,
        method_name: &str,
        instance_ptr: PointerValue<'ctx>,
        fn_type: FunctionType<'ctx>,
        extra_args: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<CallSiteValue<'ctx>, String> {
        let layout = self
            .layouts
            .get(class_name)
            .ok_or_else(|| format!("unknown class: {class_name}"))?;

        let slot = layout
            .slots
            .iter()
            .find(|s| s.method_name == method_name)
            .ok_or_else(|| format!("method {method_name} not found in {class_name}"))?;

        let vtable_ty = self.vtable_struct_type(layout);
        let vt_ptr = builder
            .build_load(self.ptr_type(), instance_ptr, "vtable_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        // slot index 0 = type_id, 1 = destructor, 2.. = virtual methods
        let slot_gep = builder
            .build_struct_gep(vtable_ty, vt_ptr, slot.slot_index + 2, "vslot_gep")
            .map_err(|e| e.to_string())?;

        let fn_ptr = builder
            .build_load(self.ptr_type(), slot_gep, "method_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        let mut args = Vec::with_capacity(extra_args.len() + 1);
        args.push(instance_ptr.into());
        args.extend_from_slice(extra_args);

        builder
            .build_indirect_call(fn_type, fn_ptr, &args, "vcall")
            .map_err(|e| e.to_string())
    }

    pub fn emit_super_call(
        &self,
        builder: &Builder<'ctx>,
        base_fn: FunctionValue<'ctx>,
        instance_ptr: PointerValue<'ctx>,
        extra_args: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<CallSiteValue<'ctx>, String> {
        let mut args = Vec::with_capacity(extra_args.len() + 1);
        args.push(instance_ptr.into());
        args.extend_from_slice(extra_args);

        let call = builder
            .build_call(base_fn, &args, "super_call")
            .map_err(|e| e.to_string())?;
        call.set_call_convention(base_fn.get_call_conventions());
        Ok(call)
    }

    pub fn emit_interface_call(
        &self,
        builder: &Builder<'ctx>,
        iface_fat_val: StructValue<'ctx>,
        slot_index: u32,
        fn_type: FunctionType<'ctx>,
        extra_args: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<CallSiteValue<'ctx>, String> {
        let obj_ptr = builder
            .build_extract_value(iface_fat_val, 0, "obj_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();
        let itable_ptr = builder
            .build_extract_value(iface_fat_val, 1, "itable_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        let field_tys = vec![self.ptr_type().into(); (slot_index as usize) + 1];
        let itable_ty = self.context.struct_type(&field_tys, false);
        let slot_gep = builder
            .build_struct_gep(itable_ty, itable_ptr, slot_index, "islot_gep")
            .map_err(|e| e.to_string())?;
        let fn_ptr = builder
            .build_load(self.ptr_type(), slot_gep, "imethod_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        let mut args = Vec::with_capacity(extra_args.len() + 1);
        args.push(obj_ptr.into());
        args.extend_from_slice(extra_args);

        builder
            .build_indirect_call(fn_type, fn_ptr, &args, "icall")
            .map_err(|e| e.to_string())
    }

    pub fn make_interface_fat_pointer(
        &self,
        builder: &Builder<'ctx>,
        instance_ptr: PointerValue<'ctx>,
        itable_global: GlobalValue<'ctx>,
    ) -> Result<StructValue<'ctx>, String> {
        let fat_ty = self.interface_fat_type();
        let itable_ptr = itable_global.as_pointer_value();

        let fat1 = builder
            .build_insert_value(fat_ty.const_zero(), instance_ptr, 0, "fat_obj")
            .map_err(|e| e.to_string())?
            .into_struct_value();

        let fat2 = builder
            .build_insert_value(fat1, itable_ptr, 1, "fat_itable")
            .map_err(|e| e.to_string())?
            .into_struct_value();

        Ok(fat2)
    }

    pub fn init_instance_vtable(
        &self,
        builder: &Builder<'ctx>,
        instance_ptr: PointerValue<'ctx>,
        vtable_global: GlobalValue<'ctx>,
    ) -> Result<(), String> {
        let vt_ptr = vtable_global.as_pointer_value();
        builder
            .build_store(instance_ptr, vt_ptr)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
