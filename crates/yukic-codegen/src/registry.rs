#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};
use yukic_ast::Module as AstModule;
use yukic_ast::item::{
    ClassDecl, CstructDecl, EnumDecl, EnumVariant, ItemKind, StructDecl,
};
use yukic_types::{Type, TypeParam};
use yukic_types::convert::ast_type_to_type;

#[derive(Clone, Debug, PartialEq)]
pub struct StructLayout {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassLayout {
    pub name: String,
    pub superclass: Option<String>,
    pub interfaces: Vec<String>,
    pub fields: Vec<(String, Type)>,
    pub getters: HashSet<String>,
    pub setters: HashSet<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumVariantLayout {
    pub name: String,
    pub payload: Vec<Type>,
    pub tag: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumLayout {
    pub name: String,
    pub variants: Vec<EnumVariantLayout>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CstructLayout {
    pub name: String,
    pub fields: Vec<(String, Type)>,
    pub is_packed: bool,
    pub align: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MangledFunSig {
    pub name: String,
    pub param_types: Vec<Type>,
    pub ret_type: Type,
    pub mangled_symbol: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TypeRegistry {
    pub structs: HashMap<String, StructLayout>,
    pub cstructs: HashMap<String, CstructLayout>,
    pub classes: HashMap<String, ClassLayout>,
    pub enums: HashMap<String, EnumLayout>,
    pub interfaces: HashSet<String>,
    pub functions: HashMap<String, Type>,
    pub fn_types: HashMap<String, Type>,
    pub cfun_types: HashMap<String, Type>,
    pub c_symbols: HashMap<String, String>,
    pub type_aliases: HashMap<String, Type>,
    pub symbol_overrides: HashMap<String, String>,
    pub fn_candidates: HashMap<String, Vec<MangledFunSig>>,
    // method defaults keyed as "Class::method" (bare names collide)
    pub defaults: yukic_ast::defaults::DefaultArgsMap,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_module(&mut self, module: &AstModule) {
        for item in &module.items {
            self.register_item(&item.kind);
        }
        self.resolve_inherited_class_fields();
        self.canonicalize_registered_types();
        self.register_mangled_symbols(module);
    }

    pub fn canonicalize_registered_types(&mut self) {
        let classes: HashSet<String> = self.classes.keys().cloned().collect();
        let ifaces: HashSet<String> = self.interfaces.clone();
        let enums: HashSet<String> = self.enums.keys().cloned().collect();
        let cstructs: HashSet<String> = self.cstructs.keys().cloned().collect();

        let mut changed = true;
        let mut passes = 0;
        while changed && passes < 100 {
            changed = false;
            passes += 1;
            let current_aliases = self.type_aliases.clone();
            for target in self.type_aliases.values_mut() {
                let canonical = Self::canon_type(target, &classes, &ifaces, &enums, &cstructs, &current_aliases);
                if &canonical != target {
                    *target = canonical;
                    changed = true;
                }
            }
        }

        for ret in self.functions.values_mut() {
            *ret = Self::canon_type(ret, &classes, &ifaces, &enums, &cstructs, &self.type_aliases);
        }
        for fn_ty in self.fn_types.values_mut() {
            *fn_ty = Self::canon_type(fn_ty, &classes, &ifaces, &enums, &cstructs, &self.type_aliases);
        }
        for fn_ty in self.cfun_types.values_mut() {
            *fn_ty = Self::canon_type(fn_ty, &classes, &ifaces, &enums, &cstructs, &self.type_aliases);
        }
        for layout in self.classes.values_mut() {
            for (_, field_ty) in &mut layout.fields {
                *field_ty = Self::canon_type(field_ty, &classes, &ifaces, &enums, &cstructs, &self.type_aliases);
            }
        }
        for layout in self.structs.values_mut() {
            for (_, field_ty) in &mut layout.fields {
                *field_ty = Self::canon_type(field_ty, &classes, &ifaces, &enums, &cstructs, &self.type_aliases);
            }
        }
        for layout in self.cstructs.values_mut() {
            for (_, field_ty) in &mut layout.fields {
                *field_ty = Self::canon_type(field_ty, &classes, &ifaces, &enums, &cstructs, &self.type_aliases);
            }
        }
    }

    fn canon_type(
        ty: &Type,
        classes: &HashSet<String>,
        ifaces: &HashSet<String>,
        enums: &HashSet<String>,
        cstructs: &HashSet<String>,
        aliases: &HashMap<String, Type>,
    ) -> Type {
        let mut visited = HashSet::new();
        Self::canon_type_internal(ty, classes, ifaces, enums, cstructs, aliases, &mut visited)
    }

    fn canon_type_internal(
        ty: &Type,
        classes: &HashSet<String>,
        ifaces: &HashSet<String>,
        enums: &HashSet<String>,
        cstructs: &HashSet<String>,
        aliases: &HashMap<String, Type>,
        visited: &mut HashSet<String>,
    ) -> Type {
        match ty {
            Type::Struct(name) => {
                if let Some(aliased) = aliases.get(name) {
                    if visited.insert(name.clone()) {
                        let res = Self::canon_type_internal(aliased, classes, ifaces, enums, cstructs, aliases, visited);
                        visited.remove(name);
                        res
                    } else {
                        Type::Error
                    }
                } else {
                    let base_name = name.split('<').next().unwrap_or(name);
                    if ifaces.contains(base_name) {
                        Type::Interface(name.clone())
                    } else if classes.contains(base_name) {
                        Type::Class(base_name.to_string())
                    } else if enums.contains(base_name) {
                        Type::Enum(base_name.to_string())
                    } else if cstructs.contains(base_name) {
                        Type::Cstruct(base_name.to_string())
                    } else {
                        ty.clone()
                    }
                }
            }
            Type::Array(inner) => Type::Array(Box::new(Self::canon_type_internal(inner, classes, ifaces, enums, cstructs, aliases, visited))),
            Type::Tuple(members) => {
                Type::Tuple(members.iter().map(|m| Self::canon_type_internal(m, classes, ifaces, enums, cstructs, aliases, visited)).collect())
            }
            Type::Union(members) => {
                Type::union(members.iter().map(|m| Self::canon_type_internal(m, classes, ifaces, enums, cstructs, aliases, visited)))
            }
            Type::Ref { inner, mutable } => Type::Ref {
                inner: Box::new(Self::canon_type_internal(inner, classes, ifaces, enums, cstructs, aliases, visited)),
                mutable: *mutable,
            },
            Type::Ptr(inner) => Type::Ptr(Box::new(Self::canon_type_internal(inner, classes, ifaces, enums, cstructs, aliases, visited))),
            Type::Shared(inner) => Type::Shared(Box::new(Self::canon_type_internal(inner, classes, ifaces, enums, cstructs, aliases, visited))),
            Type::Weak(inner) => Type::Weak(Box::new(Self::canon_type_internal(inner, classes, ifaces, enums, cstructs, aliases, visited))),
            Type::Map(k, v) => Type::Map(
                Box::new(Self::canon_type_internal(k, classes, ifaces, enums, cstructs, aliases, visited)),
                Box::new(Self::canon_type_internal(v, classes, ifaces, enums, cstructs, aliases, visited)),
            ),
            Type::Fn { params, ret, is_cfun } => Type::Fn {
                params: params
                    .iter()
                    .map(|p| TypeParam {
                        ty: Self::canon_type_internal(&p.ty, classes, ifaces, enums, cstructs, aliases, visited),
                        is_ref: p.is_ref,
                        is_mut: p.is_mut,
                    })
                    .collect(),
                ret: Box::new(Self::canon_type_internal(ret, classes, ifaces, enums, cstructs, aliases, visited)),
                is_cfun: *is_cfun,
            },
            _ => ty.clone(),
        }
    }

    fn register_item(&mut self, kind: &ItemKind) {
        match kind {
            ItemKind::TypeAlias(decl) => {
                let target = ast_type_to_type(&decl.target);
                self.type_aliases.insert(decl.name.name.clone(), target);
            }
            ItemKind::Struct(decl) => self.register_struct(decl),
            ItemKind::Cstruct(decl) => self.register_cstruct(decl),
            ItemKind::Interface(decl) => {
                self.interfaces.insert(decl.name.name.clone());
                for method in &decl.methods {
                    let ret = method.return_type.as_ref().map_or(Type::Void, ast_type_to_type);
                    let mangled = crate::mangle::mangle_method_name(&method.name.name, method.params.len());
                    self.functions.insert(format!("{}::{}", decl.name.name, method.name.name), ret.clone());
                    self.functions.insert(format!("{}::{}", decl.name.name, mangled), ret);
                }
            }
            ItemKind::Class(decl) => {
                self.register_class(decl);
                for method in &decl.methods {
                    let ret = method.return_type.as_ref().map_or(Type::Void, ast_type_to_type);
                    let mangled = crate::mangle::mangle_method_name(&method.name.name, method.params.len());
                    self.functions.insert(format!("{}::{}", decl.name.name, method.name.name), ret.clone());
                    self.functions.insert(format!("{}::{}", decl.name.name, mangled), ret);
                }
            }
            ItemKind::Enum(decl) => self.register_enum(decl),
            ItemKind::Fun(decl) => {
                let ret = decl.return_type.as_ref().map_or(Type::Void, ast_type_to_type);
                self.functions.insert(decl.name.name.clone(), ret.clone());
                let params = decl
                    .params
                    .iter()
                    .map(|p| {
                        let ty = p.ty.as_ref().map_or(Type::Infer(0), ast_type_to_type);
                        TypeParam::value(ty)
                    })
                    .collect();
                let is_thunk = decl.attrs.iter().any(|a| a.is("thunk"));
                self.fn_types.insert(
                    decl.name.name.clone(),
                    Type::Fn {
                        params,
                        ret: Box::new(ret),
                        is_cfun: is_thunk,
                    },
                );
            }
            ItemKind::Cfun(decl) => {
                let ret = decl.return_type.as_ref().map_or(Type::Void, ast_type_to_type);
                self.functions.insert(decl.name.name.clone(), ret.clone());
                let sym_override = decl.symbol_override.as_ref().map_or_else(|| decl.name.name.clone(), |sym| sym.name.clone());
                self.c_symbols.insert(decl.name.name.clone(), sym_override);
                if let Some(ref sym) = decl.symbol_override {
                    self.symbol_overrides.insert(decl.name.name.clone(), sym.name.clone());
                }
                let params = decl
                    .params
                    .iter()
                    .map(|p| TypeParam::value(ast_type_to_type(&p.ty)))
                    .collect();
                let c_fn_ty = Type::Fn {
                    params,
                    ret: Box::new(ret),
                    is_cfun: true,
                };
                self.cfun_types.insert(decl.name.name.clone(), c_fn_ty.clone());
                self.fn_types.insert(decl.name.name.clone(), c_fn_ty);
            }
            ItemKind::Externc(block) => {
                for item in &block.items {
                    match item {
                        yukic_ast::item::ExterncItem::Cstruct(cstruct_decl) => {
                            self.register_cstruct(cstruct_decl);
                        }
                        yukic_ast::item::ExterncItem::Cfun(fn_decl) => {
                            let ret = fn_decl.return_type.as_ref().map_or(Type::Void, ast_type_to_type);
                            self.functions.insert(fn_decl.name.name.clone(), ret.clone());
                            let sym_override = fn_decl.symbol_override.as_ref().map_or_else(|| fn_decl.name.name.clone(), |sym| sym.name.clone());
                            self.c_symbols.insert(fn_decl.name.name.clone(), sym_override);
                            if let Some(ref sym) = fn_decl.symbol_override {
                                self.symbol_overrides.insert(fn_decl.name.name.clone(), sym.name.clone());
                            }
                            let params = fn_decl
                                .params
                                .iter()
                                .map(|p| TypeParam::value(ast_type_to_type(&p.ty)))
                                .collect();
                            let c_fn_ty = Type::Fn {
                                params,
                                ret: Box::new(ret),
                                is_cfun: true,
                            };
                            self.cfun_types.insert(fn_decl.name.name.clone(), c_fn_ty.clone());
                            self.fn_types.insert(fn_decl.name.name.clone(), c_fn_ty);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn register_struct(&mut self, decl: &StructDecl) {
        let fields = decl
            .fields
            .iter()
            .map(|f| (f.name.name.clone(), ast_type_to_type(&f.ty)))
            .collect();
        self.structs.insert(
            decl.name.name.clone(),
            StructLayout {
                name: decl.name.name.clone(),
                fields,
            },
        );
    }

    pub fn register_cstruct(&mut self, decl: &CstructDecl) {
        let fields = decl
            .fields
            .iter()
            .map(|f| (f.name.name.clone(), ast_type_to_type(&f.ty)))
            .collect();
        let mut is_packed = false;
        let mut align = None;
        for attr in &decl.c_attrs {
            if attr.name.name == "packed" {
                is_packed = true;
            } else if attr.name.name == "align" {
                align = attr.arg.as_ref().and_then(|arg| arg.parse::<u32>().ok());
            }
        }
        self.cstructs.insert(
            decl.name.name.clone(),
            CstructLayout {
                name: decl.name.name.clone(),
                fields,
                is_packed,
                align,
            },
        );
    }

    pub fn register_class(&mut self, decl: &ClassDecl) {
        let fields = decl
            .fields
            .iter()
            .map(|f| (f.name.name.clone(), ast_type_to_type(&f.ty)))
            .collect();
        let mut getters = HashSet::new();
        let mut setters = HashSet::new();
        for acc in &decl.accessors {
            match acc.kind {
                yukic_ast::item::AccessorKind::Get => {
                    getters.insert(acc.name.name.clone());
                }
                yukic_ast::item::AccessorKind::Set(_) => {
                    setters.insert(acc.name.name.clone());
                }
            }
        }
        let superclass = decl.base_class.as_ref().map(|b| b.name.clone());
        let mut interfaces = Vec::new();
        for i in &decl.interfaces {
            interfaces.push(i.name.clone());
            let base = i.name.split('<').next().unwrap_or(&i.name);
            if base != i.name {
                interfaces.push(base.to_string());
            }
        }
        self.classes.insert(
            decl.name.name.clone(),
            ClassLayout {
                name: decl.name.name.clone(),
                superclass,
                interfaces,
                fields,
                getters,
                setters,
            },
        );
    }

    pub fn resolve_inherited_class_fields(&mut self) {
        let class_names: Vec<String> = self.classes.keys().cloned().collect();
        for name in class_names {
            let resolved = self.collect_all_class_fields(&name, &mut Vec::new());
            if let Some(layout) = self.classes.get_mut(&name) {
                layout.fields = resolved;
            }
        }
    }

    fn collect_all_class_fields(
        &self,
        class_name: &str,
        visited: &mut Vec<String>,
    ) -> Vec<(String, Type)> {
        if visited.iter().any(|v| v == class_name) {
            return Vec::new();
        }
        visited.push(class_name.to_string());

        let Some(layout) = self.classes.get(class_name) else {
            return Vec::new();
        };

        let mut fields = Vec::new();
        if let Some(ref super_name) = layout.superclass {
            fields.extend(self.collect_all_class_fields(super_name, visited));
        }
        for (fname, fty) in &layout.fields {
            if !fields.iter().any(|(existing, _)| existing == fname) {
                fields.push((fname.clone(), fty.clone()));
            }
        }
        fields
    }

    pub fn register_enum(&mut self, decl: &EnumDecl) {
        let variants = decl
            .variants
            .iter()
            .enumerate()
            .map(|(idx, v)| {
                let tag = u32::try_from(idx).unwrap_or(u32::MAX);
                Self::build_variant_layout(tag, v)
            })
            .collect();
        self.enums.insert(
            decl.name.name.clone(),
            EnumLayout {
                name: decl.name.name.clone(),
                variants,
            },
        );
    }

    fn build_variant_layout(tag: u32, variant: &EnumVariant) -> EnumVariantLayout {
        match variant {
            EnumVariant::Simple(name) | EnumVariant::WithValue(name, _) => EnumVariantLayout {
                name: name.name.clone(),
                payload: Vec::new(),
                tag,
            },
            EnumVariant::Adt(name, fields) => {
                let payload = fields.iter().map(|f| ast_type_to_type(&f.ty)).collect();
                EnumVariantLayout {
                    name: name.name.clone(),
                    payload,
                    tag,
                }
            }
        }
    }

    pub fn add_struct(&mut self, name: impl Into<String>, fields: Vec<(String, Type)>) {
        let name = name.into();
        self.structs
            .insert(name.clone(), StructLayout { name, fields });
    }

    pub fn add_cstruct(
        &mut self,
        name: impl Into<String>,
        fields: Vec<(String, Type)>,
        is_packed: bool,
        align: Option<u32>,
    ) {
        let name = name.into();
        self.cstructs.insert(
            name.clone(),
            CstructLayout {
                name,
                fields,
                is_packed,
                align,
            },
        );
    }

    pub fn add_class(
        &mut self,
        name: impl Into<String>,
        superclass: Option<String>,
        fields: Vec<(String, Type)>,
    ) {
        let name = name.into();
        self.classes.insert(
            name.clone(),
            ClassLayout {
                name,
                superclass,
                interfaces: Vec::new(),
                fields,
                getters: HashSet::new(),
                setters: HashSet::new(),
            },
        );
    }

    pub fn has_getter(&self, class_name: &str, prop: &str) -> bool {
        let mut curr = Some(class_name);
        while let Some(name) = curr {
            if let Some(layout) = self.classes.get(name) {
                if layout.getters.contains(prop) {
                    return true;
                }
                curr = layout.superclass.as_deref();
            } else {
                break;
            }
        }
        false
    }

    pub fn has_setter(&self, class_name: &str, prop: &str) -> bool {
        let mut curr = Some(class_name);
        while let Some(name) = curr {
            if let Some(layout) = self.classes.get(name) {
                if layout.setters.contains(prop) { return true; }
                curr = layout.superclass.as_deref();
            } 


            else { break; }
        }
        false
    }

    pub fn add_enum(&mut self, name: impl Into<String>, variants: Vec<EnumVariantLayout>) {
        let name = name.into();
        self.enums
            .insert(name.clone(), EnumLayout { name, variants });
    }

    pub fn class_implements_interface(&self, class_name: &str, iface_name: &str) -> bool {
        let mut curr = Some(class_name);
        while let Some(c) = curr {
            if let Some(layout) = self.classes.get(c) {
                if layout.interfaces.iter().any(|i| i == iface_name) {
                    return true;
                }
                curr = layout.superclass.as_deref();
            } 

            else { break; }
        }


        false
    }

    pub fn class_is_subclass_of(&self, class_name: &str, base_name: &str) -> bool {
        if class_name == base_name {
            return true;
        }
        let mut curr = self.classes.get(class_name).and_then(|l| l.superclass.as_deref());
        while let Some(c) = curr {
            if c == base_name { return true; }
            curr = self.classes.get(c).and_then(|l| l.superclass.as_deref());
        }
        false
    }

    pub fn canonicalize_type(&self, ty: &Type) -> Type {
        let classes: HashSet<String> = self.classes.keys().cloned().collect();
        let ifaces: HashSet<String> = self.interfaces.clone();
        let enums: HashSet<String> = self.enums.keys().cloned().collect();
        let cstructs: HashSet<String> = self.cstructs.keys().cloned().collect();
        Self::canon_type(ty, &classes, &ifaces, &enums, &cstructs, &self.type_aliases)
    }

    pub fn register_mangled_symbols(&mut self, module: &AstModule) {
        for item in &module.items {
            match &item.kind {
                ItemKind::Fun(decl) => {
                    if crate::mangle::should_mangle_fun(&decl.name.name, &decl.attrs) {
                        let params = crate::mangle::fun_param_types(decl, self);
                        let ret = crate::mangle::fun_return_type(decl, self);
                        let mangled = crate::mangle::mangle_fun_symbol(&decl.name.name, &params, &ret);
                        self.symbol_overrides.insert(decl.name.name.clone(), mangled.clone());
                        let sig = MangledFunSig {
                            name: decl.name.name.clone(),
                            param_types: params.clone(),
                            ret_type: ret.clone(),
                            mangled_symbol: mangled.clone(),
                        };
                        self.fn_candidates
                            .entry(decl.name.name.clone())
                            .or_default()
                            .push(sig);
                        self.functions.insert(mangled.clone(), ret.clone());
                        let fn_params = params.into_iter().map(TypeParam::value).collect();
                        let is_thunk = decl.attrs.iter().any(|a| a.is("thunk"));
                        self.fn_types.insert(
                            mangled,
                            Type::Fn {
                                params: fn_params,
                                ret: Box::new(ret),
                                is_cfun: is_thunk,
                            },
                        );
                    }
                }
                ItemKind::Cfun(decl) => {
                    if decl.symbol_override.is_none()
                        && crate::mangle::should_mangle_cfun(&decl.name.name, &decl.yukino_attrs)
                    {
                        let params = crate::mangle::cfun_param_types(decl, self);
                        let ret = crate::mangle::cfun_return_type(decl, self);
                        let mangled = crate::mangle::mangle_fun_symbol(&decl.name.name, &params, &ret);
                        self.symbol_overrides.insert(decl.name.name.clone(), mangled.clone());
                        let sig = MangledFunSig {
                            name: decl.name.name.clone(),
                            param_types: params.clone(),
                            ret_type: ret.clone(),
                            mangled_symbol: mangled.clone(),
                        };
                        self.fn_candidates
                            .entry(decl.name.name.clone())
                            .or_default()
                            .push(sig);
                        self.functions.insert(mangled.clone(), ret.clone());
                        let fn_params = params.into_iter().map(TypeParam::value).collect();
                        self.fn_types.insert(
                            mangled,
                            Type::Fn {
                                params: fn_params,
                                ret: Box::new(ret),
                                is_cfun: true,
                            },
                        );
                    }
                }
                ItemKind::Externc(block) => {
                    for extern_item in &block.items {
                        if let yukic_ast::item::ExterncItem::Cfun(decl) = extern_item {
                            if decl.symbol_override.is_none()
                                && crate::mangle::should_mangle_cfun(&decl.name.name, &decl.yukino_attrs)
                            {
                                let params = crate::mangle::cfun_param_types(decl, self);
                                let ret = crate::mangle::cfun_return_type(decl, self);
                                let mangled =
                                    crate::mangle::mangle_fun_symbol(&decl.name.name, &params, &ret);
                                self.symbol_overrides.insert(decl.name.name.clone(), mangled.clone());
                                let sig = MangledFunSig {
                                    name: decl.name.name.clone(),
                                    param_types: params.clone(),
                                    ret_type: ret.clone(),
                                    mangled_symbol: mangled.clone(),
                                };
                                self.fn_candidates
                                    .entry(decl.name.name.clone())
                                    .or_default()
                                    .push(sig);
                                self.functions.insert(mangled.clone(), ret.clone());
                                let fn_params = params.into_iter().map(TypeParam::value).collect();
                                self.fn_types.insert(
                                    mangled,
                                    Type::Fn {
                                        params: fn_params,
                                        ret: Box::new(ret),
                                        is_cfun: true,
                                    },
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn resolve_fun_symbol(&self, name: &str, arity: usize) -> Option<String> {
        if let Some(sym) = self.symbol_overrides.get(name) {
            return Some(sym.clone());
        }
        if let Some(candidates) = self.fn_candidates.get(name) {
            if candidates.len() == 1 {
                return Some(candidates[0].mangled_symbol.clone());
            }
            let matching: Vec<&MangledFunSig> = candidates
                .iter()
                .filter(|c| c.param_types.len() == arity)
                .collect();
            if matching.len() == 1 {
                return Some(matching[0].mangled_symbol.clone());
            }
        }
        self.symbol_overrides.get(name).cloned()
    }
}
