#![deny(unsafe_code)]

use inkwell::AddressSpace;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::debug_info::{
    AsDIScope, DIBasicType, DICompileUnit, DICompositeType, DIDerivedType, DIFile, DIFlags,
    DIFlagsConstants, DILocalVariable, DILocation, DIScope, DISubprogram, DISubroutineType, DIType,
    DWARFEmissionKind, DWARFSourceLanguage, DebugInfoBuilder,
};
use inkwell::module::{FlagBehavior, Module as LlvmModule};
use inkwell::values::FunctionValue;
use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use yukic_span::{FileId, SourceMap, Span};
use yukic_target::TargetTriple;
use yukic_types::Type;

const DW_ATE_BOOLEAN: u32 = 0x02;
const DW_ATE_FLOAT: u32 = 0x04;
const DW_ATE_SIGNED: u32 = 0x05;
const DW_ATE_UNSIGNED: u32 = 0x07;
const DW_ATE_UTF: u32 = 0x10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugFormat {
    Dwarf,
    CodeView,
}

pub struct DebugInfoManager<'ctx> {
    context: &'ctx Context,
    target: TargetTriple,
    format: DebugFormat,
    inner: Option<DebugInfoInner<'ctx>>,
}

struct DebugInfoInner<'ctx> {
    builder: DebugInfoBuilder<'ctx>,
    compile_unit: DICompileUnit<'ctx>,
    file_cache: HashMap<PathBuf, DIFile<'ctx>>,
    file_id_cache: HashMap<FileId, DIFile<'ctx>>,
    type_cache: HashMap<String, DIType<'ctx>>,
    scope_stack: Vec<DIScope<'ctx>>,
    is_finalized: Cell<bool>,
}

impl<'ctx> DebugInfoManager<'ctx> {
    pub fn new(context: &'ctx Context, target: TargetTriple) -> Self {
        let is_windows = target.spec().os == yukic_target::OperatingSystem::Windows;

        let format = if is_windows { DebugFormat::CodeView} else { DebugFormat::Dwarf };
        
        Self {
            context,
            target,
            format,
            inner: None,
        }
    }

    pub fn format(&self) -> DebugFormat { self.format }
    pub fn is_enabled(&self) -> bool { self.inner.is_some() }
    pub fn target(&self) -> TargetTriple { self.target }

    pub fn set_format(&mut self, format: DebugFormat) {
        self.format = format;
    }

    pub fn compile_unit(&self) -> Option<DICompileUnit<'ctx>> {
        self.inner.as_ref().map(|inr| inr.compile_unit)
    }

    pub fn enable_debug_info(
        &mut self,
        module: &LlvmModule<'ctx>,
        main_filename: &str,
        directory: &str,
        is_optimized: bool,
    ) {
        if self.inner.is_some() { return; }

        self.emit_module_flags(module);

        let producer = format!("yukic v{}", env!("CARGO_PKG_VERSION"));
        let (builder, compile_unit) = module.create_debug_info_builder(
            true,
            DWARFSourceLanguage::C,
            main_filename,
            directory,
            &producer,
            is_optimized,
            "",
            0,
            "",
            DWARFEmissionKind::Full,
            0,
            false,
            false,
            "",
            "",
        );

        let initial_scope = compile_unit.as_debug_info_scope();
        let mut file_cache = HashMap::new();
        file_cache.insert(PathBuf::from(main_filename), compile_unit.get_file());

        self.inner = Some(DebugInfoInner {
            builder,
            compile_unit,
            file_cache,
            file_id_cache: HashMap::new(),
            type_cache: HashMap::new(),
            scope_stack: vec![initial_scope],
            is_finalized: Cell::new(false),
        });
    }

    fn emit_module_flags(&self, module: &LlvmModule<'ctx>) {
        let i32_type = self.context.i32_type();
        let debug_info_version = i32_type.const_int(3, false);
        module.add_basic_value_flag(
            "Debug Info Version",
            FlagBehavior::Warning,
            debug_info_version,
        );

        match self.format {
            DebugFormat::CodeView => {
                let cv_flag = i32_type.const_int(1, false);
                module.add_basic_value_flag("CodeView", FlagBehavior::Warning, cv_flag);
            }
            DebugFormat::Dwarf => {
                let dwarf_flag = i32_type.const_int(4, false);
                module.add_basic_value_flag("Dwarf Version", FlagBehavior::Warning, dwarf_flag);
            }
        }
    }

    pub fn get_or_create_file(&mut self, path: &Path) -> Option<DIFile<'ctx>> {
        let inner = self.inner.as_mut()?;
        if let Some(&file) = inner.file_cache.get(path) { return Some(file); }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.ykn");
        let directory = path
            .parent()
            .and_then(|p| p.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(".");

        let di_file = inner.builder.create_file(filename, directory);
        inner.file_cache.insert(path.to_path_buf(), di_file);
        Some(di_file)
    }

    pub fn get_file_for_span(
        &mut self,
        span: Span,
        source_map: &SourceMap,
    ) -> Option<DIFile<'ctx>> {
        let inner = self.inner.as_mut()?;
        if let Some(&file) = inner.file_id_cache.get(&span.file_id) {
            return Some(file);
        }

        let source_file = source_map.get_file(span.file_id)?;
        let path = &source_file.path;
        let di_file = self.get_or_create_file(path)?;
        if let Some(inr) = self.inner.as_mut() {
            inr.file_id_cache.insert(span.file_id, di_file);
        }
        Some(di_file)
    }

    pub fn current_scope(&self) -> Option<DIScope<'ctx>> {
        self.inner
            .as_ref()
            .and_then(|inr| inr.scope_stack.last().copied())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn emit_function(
        &mut self,
        fn_val: FunctionValue<'ctx>,
        name: &str,
        linkage_name: Option<&str>,
        file: DIFile<'ctx>,
        line: u32,
        subroutine_type: DISubroutineType<'ctx>,
        is_definition: bool,
        is_optimized: bool,
    ) -> Option<DISubprogram<'ctx>> {
        let inner = self.inner.as_mut()?;
        let scope = inner.scope_stack.first().copied()?;

        let subprogram = inner.builder.create_function(
            scope,
            name,
            linkage_name,
            file,
            line,
            subroutine_type,
            true,
            is_definition,
            line,
            DIFlags::PUBLIC,
            is_optimized,
        );

        fn_val.set_subprogram(subprogram);
        inner.scope_stack.push(subprogram.as_debug_info_scope());
        Some(subprogram)
    }

    pub fn push_lexical_block(
        &mut self,
        file: DIFile<'ctx>,
        line: u32,
        column: u32,
    ) -> Option<DIScope<'ctx>> {
        let inner = self.inner.as_mut()?;
        let parent_scope = inner.scope_stack.last().copied()?;
        let block = inner
            .builder
            .create_lexical_block(parent_scope, file, line, column);
        let block_scope = block.as_debug_info_scope();
        inner.scope_stack.push(block_scope);
        Some(block_scope)
    }

    pub fn pop_scope(&mut self) {
        if let Some(inner) = self.inner.as_mut().filter(|inr| inr.scope_stack.len() > 1) {
            inner.scope_stack.pop();
        }
    }

    pub fn set_location(
        &self,
        builder: &Builder<'ctx>,
        line: u32,
        column: u32,
    ) -> Option<DILocation<'ctx>> {
        let inner = self.inner.as_ref()?;
        let scope = inner.scope_stack.last().copied()?;

        let loc = inner
            .builder
            .create_debug_location(self.context, line, column, scope, None);
        builder.set_current_debug_location(loc);
        Some(loc)
    }

    pub fn set_location_from_span(
        &mut self,
        builder: &Builder<'ctx>,
        span: Span,
        source_map: Option<&SourceMap>,
    ) -> Option<DILocation<'ctx>> {
        if span.is_dummy() || self.inner.is_none() {
            return None;
        }

        let (line, col) = source_map.and_then(|sm| sm.lookup_line_column(span))?;
        let line = u32::try_from(line).unwrap_or(1);
        let col = u32::try_from(col).unwrap_or(1);
        self.set_location(builder, line, col)
    }

    pub fn clear_location(&self, builder: &Builder<'ctx>) {
        if self.inner.is_some() {
            builder.unset_current_debug_location();
        }
    }

    pub fn create_subroutine_type(
        &self,
        file: DIFile<'ctx>,
        ret_ty: Option<DIType<'ctx>>,
        param_types: &[DIType<'ctx>],
    ) -> Option<DISubroutineType<'ctx>> {
        let inner = self.inner.as_ref()?;
        Some(
            inner
                .builder
                .create_subroutine_type(file, ret_ty, param_types, DIFlags::PUBLIC),
        )
    }

    pub fn create_basic_type(
        &mut self,
        name: &str,
        bits: u64,
        encoding: u32,
    ) -> Option<DIBasicType<'ctx>> {
        let inner = self.inner.as_mut()?;
        inner
            .builder
            .create_basic_type(name, bits, encoding, DIFlags::PUBLIC)
            .ok()
    }

    pub fn create_pointer_type(
        &self,
        name: &str,
        pointee: DIType<'ctx>,
    ) -> Option<DIDerivedType<'ctx>> {
        let inner = self.inner.as_ref()?;
        Some(
            inner
                .builder
                .create_pointer_type(name, pointee, 64, 64, AddressSpace::default()),
        )
    }

    pub fn create_array_type(
        &self,
        element_type: DIType<'ctx>,
        element_count: u64,
        element_size_bits: u64,
        align_bits: u32,
    ) -> Option<DICompositeType<'ctx>> {
        let inner = self.inner.as_ref()?;
        let total_size = element_count.saturating_mul(element_size_bits);
        let subscript: Range<i64> = 0..i64::try_from(element_count).unwrap_or(i64::MAX);
        Some(
            inner
                .builder
                .create_array_type(element_type, total_size, align_bits, &[subscript]),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_struct_type(
        &mut self,
        scope: DIScope<'ctx>,
        name: &str,
        file: DIFile<'ctx>,
        line: u32,
        size_bits: u64,
        align_bits: u32,
        elements: &[DIType<'ctx>],
    ) -> Option<DICompositeType<'ctx>> {
        let inner = self.inner.as_mut()?;
        Some(inner.builder.create_struct_type(
            scope,
            name,
            file,
            line,
            size_bits,
            align_bits,
            DIFlags::PUBLIC,
            None,
            elements,
            0,
            None,
            name,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_member_type(
        &self,
        scope: DIScope<'ctx>,
        name: &str,
        file: DIFile<'ctx>,
        line: u32,
        size_bits: u64,
        align_bits: u32,
        offset_bits: u64,
        ty: DIType<'ctx>,
    ) -> Option<DIDerivedType<'ctx>> {
        let inner = self.inner.as_ref()?;
        Some(inner.builder.create_member_type(
            scope,
            name,
            file,
            line,
            size_bits,
            align_bits,
            offset_bits,
            DIFlags::PUBLIC,
            ty,
        ))
    }

    pub fn create_auto_variable(
        &mut self,
        name: &str,
        file: DIFile<'ctx>,
        line: u32,
        ty: DIType<'ctx>,
    ) -> Option<DILocalVariable<'ctx>> {
        let inner = self.inner.as_mut()?;
        let scope = inner.scope_stack.last().copied()?;

        Some(inner.builder.create_auto_variable(
            scope,
            name,
            file,
            line,
            ty,
            true,
            DIFlags::PUBLIC,
            64,
        ))
    }

    pub fn create_parameter_variable(
        &mut self,
        name: &str,
        arg_no: u32,
        file: DIFile<'ctx>,
        line: u32,
        ty: DIType<'ctx>,
    ) -> Option<DILocalVariable<'ctx>> {
        let inner = self.inner.as_mut()?;
        let scope = inner.scope_stack.last().copied()?;

        Some(inner.builder.create_parameter_variable(
            scope,
            name,
            arg_no,
            file,
            line,
            ty,
            true,
            DIFlags::PUBLIC,
        ))
    }

    pub fn get_or_create_type(&mut self, ty: &Type, file: DIFile<'ctx>) -> Option<DIType<'ctx>> {
        let key = ty.display_name();
        if let Some(&cached) = self.inner.as_ref().and_then(|inr| inr.type_cache.get(&key)) {
            return Some(cached);
        }

        let di_ty = self.lower_debug_type(ty, file)?;
        if let Some(inner) = self.inner.as_mut() {
            inner.type_cache.insert(key, di_ty);
        }
        Some(di_ty)
    }

    fn lower_debug_type(&mut self, ty: &Type, file: DIFile<'ctx>) -> Option<DIType<'ctx>> {
        match ty {
            Type::Bool => self
                .create_basic_type("bool", 1, DW_ATE_BOOLEAN)
                .map(|t| t.as_type()),
            Type::Int8 => self
                .create_basic_type("int8", 8, DW_ATE_SIGNED)
                .map(|t| t.as_type()),
            Type::Int16 => self
                .create_basic_type("int16", 16, DW_ATE_SIGNED)
                .map(|t| t.as_type()),
            Type::Int32 => self
                .create_basic_type("int32", 32, DW_ATE_SIGNED)
                .map(|t| t.as_type()),
            Type::Int64 => self
                .create_basic_type("int64", 64, DW_ATE_SIGNED)
                .map(|t| t.as_type()),
            Type::Uint8 => self
                .create_basic_type("uint8", 8, DW_ATE_UNSIGNED)
                .map(|t| t.as_type()),
            Type::Uint16 => self
                .create_basic_type("uint16", 16, DW_ATE_UNSIGNED)
                .map(|t| t.as_type()),
            Type::Uint32 => self
                .create_basic_type("uint32", 32, DW_ATE_UNSIGNED)
                .map(|t| t.as_type()),
            Type::Uint64 => self
                .create_basic_type("uint64", 64, DW_ATE_UNSIGNED)
                .map(|t| t.as_type()),
            Type::Float32 => self
                .create_basic_type("float32", 32, DW_ATE_FLOAT)
                .map(|t| t.as_type()),
            Type::Float64 => self
                .create_basic_type("float64", 64, DW_ATE_FLOAT)
                .map(|t| t.as_type()),
            Type::Char => self
                .create_basic_type("char", 32, DW_ATE_UTF)
                .map(|t| t.as_type()),
            Type::Ptr(inner) => {
                let inner_di = self.get_or_create_type(inner, file)?;
                self.create_pointer_type("ptr", inner_di)
                    .map(|t| t.as_type())
            }
            Type::String => self.lower_string_debug_type(file),
            Type::Bytes => self.lower_bytes_debug_type(file),
            Type::Array(inner) => {
                let inner_di = self.get_or_create_type(inner, file)?;
                self.create_array_type(inner_di, 0, 64, 64)
                    .map(|t| t.as_type())
            }
            Type::Tuple(elements) => self.lower_tuple_debug_type(elements, file),
            _ => self
                .create_basic_type("opaque", 64, DW_ATE_UNSIGNED)
                .map(|t| t.as_type()),
        }
    }

    fn lower_string_debug_type(&mut self, file: DIFile<'ctx>) -> Option<DIType<'ctx>> {
        let u8_di = self.get_or_create_type(&Type::Uint8, file)?;
        let ptr_di = self.create_pointer_type("ptr", u8_di)?.as_type();
        let i64_di = self.get_or_create_type(&Type::Int64, file)?;

        let scope = self.current_scope()?;
        let m0 = self
            .create_member_type(scope, "ptr", file, 0, 64, 64, 0, ptr_di)?
            .as_type();
        let m1 = self
            .create_member_type(scope, "len", file, 0, 64, 64, 64, i64_di)?
            .as_type();

        self.create_struct_type(scope, "string", file, 0, 128, 64, &[m0, m1])
            .map(|t| t.as_type())
    }

    fn lower_bytes_debug_type(&mut self, file: DIFile<'ctx>) -> Option<DIType<'ctx>> {
        let u8_di = self.get_or_create_type(&Type::Uint8, file)?;
        let ptr_di = self.create_pointer_type("ptr", u8_di)?.as_type();
        let i64_di = self.get_or_create_type(&Type::Int64, file)?;

        let scope = self.current_scope()?;
        let m0 = self
            .create_member_type(scope, "ptr", file, 0, 64, 64, 0, ptr_di)?
            .as_type();
        let m1 = self
            .create_member_type(scope, "len", file, 0, 64, 64, 64, i64_di)?
            .as_type();
        let m2 = self
            .create_member_type(scope, "cap", file, 0, 64, 64, 128, i64_di)?
            .as_type();

        self.create_struct_type(scope, "bytes", file, 0, 192, 64, &[m0, m1, m2])
            .map(|t| t.as_type())
    }

    fn lower_tuple_debug_type(
        &mut self,
        elements: &[Type],
        file: DIFile<'ctx>,
    ) -> Option<DIType<'ctx>> {
        let scope = self.current_scope()?;
        let mut members = Vec::with_capacity(elements.len());
        let mut offset = 0u64;

        for (i, elem_ty) in elements.iter().enumerate() {
            let di_ty = self.get_or_create_type(elem_ty, file)?;
            let size = u64::from(elem_ty.bit_width().unwrap_or(64));
            let field_name = format!("item_{i}");
            let member =
                self.create_member_type(scope, &field_name, file, 0, size, 64, offset, di_ty)?;
            members.push(member.as_type());
            offset += size;
        }

        self.create_struct_type(scope, "tuple", file, 0, offset, 64, &members)
            .map(|t| t.as_type())
    }

    pub fn finalize(&self) {
        if let Some(inner) = self.inner.as_ref().filter(|inr| !inr.is_finalized.get()) {
            inner.builder.finalize();
            inner.is_finalized.set(true);
        }
    }
}
