#![deny(unsafe_code)]

use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module as LlvmModule};
use inkwell::types::StructType;
use inkwell::values::{GlobalValue, StructValue, UnnamedAddress};
use std::collections::HashMap;
use yukic_types::Type;

pub struct RoDataManager<'ctx> {
    pub context: &'ctx Context,
    strings: HashMap<String, (GlobalValue<'ctx>, StructValue<'ctx>)>,
    bytes: HashMap<Vec<u8>, (GlobalValue<'ctx>, StructValue<'ctx>)>,
    typeof_strings: HashMap<String, (GlobalValue<'ctx>, StructValue<'ctx>)>,
    string_counter: usize,
    bytes_counter: usize,
    typeof_counter: usize,
}

impl<'ctx> RoDataManager<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self {
            context,
            strings: HashMap::new(),
            bytes: HashMap::new(),
            typeof_strings: HashMap::new(),
            string_counter: 0,
            bytes_counter: 0,
            typeof_counter: 0,
        }
    }

    pub fn string_type(&self) -> StructType<'ctx> {
        self.context.struct_type(
            &[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.i64_type().into(),
            ],
            false,
        )
    }

    pub fn slice_type(&self) -> StructType<'ctx> {
        self.context.struct_type(
            &[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.i64_type().into(),
                self.context.i64_type().into(),
            ],
            false,
        )
    }

    pub fn emit_string(&mut self, module: &LlvmModule<'ctx>, text: &str) -> StructValue<'ctx> {
        if let Some((_, struct_val)) = self.strings.get(text) {
            return *struct_val;
        }

        let name = format!(".str.anon.{}", self.string_counter);
        self.string_counter += 1;

        let const_str = self.context.const_string(text.as_bytes(), true);
        let global = module.add_global(const_str.get_type(), Some(AddressSpace::default()), &name);
        global.set_initializer(&const_str);
        global.set_constant(true);
        global.set_linkage(Linkage::Private);
        global.set_unnamed_address(UnnamedAddress::Global);
        global.set_alignment(1);

        let ptr_val = global.as_pointer_value();
        let len_val = self.context.i64_type().const_int(text.len() as u64, false);
        let struct_val = self
            .string_type()
            .const_named_struct(&[ptr_val.into(), len_val.into()]);

        self.strings.insert(text.to_string(), (global, struct_val));
        struct_val
    }

    pub fn emit_bytes(&mut self, module: &LlvmModule<'ctx>, raw_bytes: &[u8]) -> StructValue<'ctx> {
        if let Some((_, struct_val)) = self.bytes.get(raw_bytes) {
            return *struct_val;
        }

        let name = format!(".bytes.anon.{}", self.bytes_counter);
        self.bytes_counter += 1;

        let const_bytes = self.context.const_string(raw_bytes, false);
        let global =
            module.add_global(const_bytes.get_type(), Some(AddressSpace::default()), &name);
        global.set_initializer(&const_bytes);
        global.set_constant(true);
        global.set_linkage(Linkage::Private);
        global.set_unnamed_address(UnnamedAddress::Global);
        global.set_alignment(1);

        let ptr_val = global.as_pointer_value();
        let len_val = self
            .context
            .i64_type()
            .const_int(raw_bytes.len() as u64, false);
        let cap_val = len_val;
        let struct_val =
            self.slice_type()
                .const_named_struct(&[ptr_val.into(), len_val.into(), cap_val.into()]);

        self.bytes.insert(raw_bytes.to_vec(), (global, struct_val));
        struct_val
    }

    pub fn emit_typeof(&mut self, module: &LlvmModule<'ctx>, ty: &Type) -> StructValue<'ctx> {
        let display = ty.display_name();
        if let Some((_, struct_val)) = self.typeof_strings.get(&display) {
            return *struct_val;
        }

        let name = format!(".str.type.{}", self.typeof_counter);
        self.typeof_counter += 1;

        let const_str = self.context.const_string(display.as_bytes(), true);
        let global = module.add_global(const_str.get_type(), Some(AddressSpace::default()), &name);
        global.set_initializer(&const_str);
        global.set_constant(true);
        global.set_linkage(Linkage::Private);
        global.set_unnamed_address(UnnamedAddress::Global);
        global.set_alignment(1);

        let ptr_val = global.as_pointer_value();
        let len_val = self
            .context
            .i64_type()
            .const_int(display.len() as u64, false);
        let struct_val = self
            .string_type()
            .const_named_struct(&[ptr_val.into(), len_val.into()]);

        self.typeof_strings.insert(display, (global, struct_val));
        struct_val
    }

    pub fn string_count(&self) -> usize { self.strings.len() } // str len
    pub fn bytes_count(&self) -> usize { self.bytes.len() } // bytes len
    pub fn typeof_count(&self) -> usize { self.typeof_strings.len() } // typeof len
}
