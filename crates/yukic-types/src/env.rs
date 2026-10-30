#![deny(unsafe_code)]

use std::collections::{BTreeSet, HashMap};
use yukic_span::Span;

use crate::types::{Type, TypeParam};

#[derive(Clone, Debug)]
pub struct VarInfo {
    pub ty: Type,
    pub is_mut: bool,
    pub is_static: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FnSig {
    pub params: Vec<TypeParam>,
    pub ret: Type,
    pub is_cfun: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ClassDef {
    pub fields: HashMap<String, Type>,
    pub methods: HashMap<String, FnSig>,
    pub getters: HashMap<String, Type>,
    pub setters: HashMap<String, Type>,
    pub superclass: Option<String>,
    pub interfaces: BTreeSet<String>,
    pub is_locked: bool,
}

#[derive(Clone, Debug, Default)]
pub struct StructDef {
    pub fields: HashMap<String, Type>,
    pub field_names: Vec<String>,
    pub methods: HashMap<String, FnSig>,
}

#[derive(Clone, Debug, Default)]
pub struct InterfaceDef {
    pub generics: Vec<String>,
    pub methods: HashMap<String, FnSig>,
}

#[derive(Clone, Debug, Default)]
pub struct EnumDef {
    pub variants: HashMap<String, Option<Type>>,
}

#[derive(Clone, Debug)]
pub struct TypeEnv {
    scopes: Vec<HashMap<String, VarInfo>>,
    functions: HashMap<String, FnSig>,
    c_functions: HashMap<String, FnSig>,
    classes: HashMap<String, ClassDef>,
    structs: HashMap<String, StructDef>,
    interfaces: HashMap<String, InterfaceDef>,
    enums: HashMap<String, EnumDef>,
    type_aliases: HashMap<String, Type>,
    pub ambiguous_symbols: HashMap<String, Vec<String>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            c_functions: HashMap::new(),
            classes: HashMap::new(),
            structs: HashMap::new(),
            interfaces: HashMap::new(),
            enums: HashMap::new(),
            type_aliases: HashMap::new(),
            ambiguous_symbols: HashMap::new(),
        };
        env.register_builtins();
        env
    }

    fn register_builtins(&mut self) {
        self.functions.insert(
            "exit".to_string(),
            FnSig {
                params: vec![TypeParam::value(Type::Int32)],
                ret: Type::Never,
                is_cfun: false,
            },
        );
        self.functions.insert(
            "args".to_string(),
            FnSig {
                params: Vec::new(),
                ret: Type::Array(Box::new(Type::String)),
                is_cfun: false,
            },
        );
        self.functions.insert(
            "raw_args".to_string(),
            FnSig {
                params: Vec::new(),
                ret: Type::String,
                is_cfun: false,
            },
        );
        self.functions.insert(
            "panic".to_string(),
            FnSig {
                params: vec![TypeParam::value(Type::String)],
                ret: Type::Never,
                is_cfun: false,
            },
        );
        let mut error_fields = HashMap::new();
        error_fields.insert("message".to_string(), Type::String);
        error_fields.insert("kind".to_string(), Type::String);
        error_fields.insert("stack".to_string(), Type::Array(Box::new(Type::String)));
        self.classes.insert(
            "error".to_string(),
            ClassDef {
                fields: error_fields.clone(),
                methods: HashMap::new(),
                superclass: None,
                interfaces: BTreeSet::new(),
                ..Default::default()
            },
        );
        self.classes.insert(
            "Error".to_string(),
            ClassDef {
                fields: error_fields.clone(),
                methods: HashMap::new(),
                superclass: None,
                interfaces: BTreeSet::new(),
                ..Default::default()
            },
        );
        let error_names = vec!["message".to_string(), "code".to_string()];
        self.structs.insert(
            "error".to_string(),
            StructDef {
                fields: error_fields.clone(),
                field_names: error_names.clone(),
                methods: HashMap::new(),
            },
        );
        self.structs.insert(
            "Error".to_string(),
            StructDef {
                fields: error_fields,
                field_names: error_names,
                methods: HashMap::new(),
            },
        );
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn insert_var(&mut self, name: String, ty: Type, is_mut: bool, span: Span) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name,
                VarInfo {
                    ty,
                    is_mut,
                    is_static: false,
                    span,
                },
            );
        }
    }

    pub fn insert_static_var(&mut self, name: String, ty: Type, span: Span) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name,
                VarInfo {
                    ty,
                    is_mut: false,
                    is_static: true,
                    span,
                },
            );
        }
    }

    pub fn insert_root_static(&mut self, name: String, ty: Type, span: Span) {
        if let Some(scope) = self.scopes.first_mut() {
            scope.insert(
                name,
                VarInfo {
                    ty,
                    is_mut: false,
                    is_static: true,
                    span,
                },
            );
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<&VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info);
            }
        }
        None
    }

    pub fn register_fn(&mut self, name: String, sig: FnSig) {
        self.functions.insert(name, sig);
    }

    pub fn register_cfun(&mut self, name: String, sig: FnSig) {
        self.c_functions.insert(name.clone(), sig.clone());
        self.functions.insert(name, sig);
    }

    pub fn lookup_fn(&self, name: &str) -> Option<&FnSig> {
        self.functions.get(name)
    }

    pub fn lookup_cfun(&self, name: &str) -> Option<&FnSig> {
        self.c_functions.get(name)
    }

    pub fn register_ambiguous(&mut self, name: String, namespaces: Vec<String>) {
        self.ambiguous_symbols.insert(name, namespaces);
    }

    pub fn get_ambiguous(&self, name: &str) -> Option<&[String]> {
        self.ambiguous_symbols.get(name).map(|v| v.as_slice())
    }

    pub fn register_class(&mut self, name: String, class: ClassDef) {
        self.classes.insert(name, class);
    }

    pub fn lookup_class(&self, name: &str) -> Option<&ClassDef> {
        self.classes.get(name)
    }

    pub fn register_struct(&mut self, name: String, def: StructDef) {
        self.structs.insert(name, def);
    }

    pub fn lookup_struct(&self, name: &str) -> Option<&StructDef> {
        self.structs.get(name)
    }

    pub fn register_interface(&mut self, name: String, iface: InterfaceDef) {
        self.interfaces.insert(name, iface);
    }

    pub fn lookup_interface(&self, name: &str) -> Option<InterfaceDef> {
        let (base, args) = crate::convert::parse_type_generic_args(name);
        let iface = self.interfaces.get(&base)?;
        if args.is_empty() || iface.generics.is_empty() {
            return Some(iface.clone());
        }
        let mapping: HashMap<String, Type> = iface
            .generics
            .iter()
            .cloned()
            .zip(args)
            .collect();
        let mut instantiated = iface.clone();
        for sig in instantiated.methods.values_mut() {
            for p in &mut sig.params {
                p.ty = p.ty.substitute(&mapping);
            }
            sig.ret = sig.ret.substitute(&mapping);
        }
        Some(instantiated)
    }

    pub fn register_enum(&mut self, name: String, def: EnumDef) {
        self.enums.insert(name, def);
    }

    pub fn lookup_enum(&self, name: &str) -> Option<&EnumDef> {
        self.enums.get(name)
    }

    pub fn is_interface_impl(&self, class: &str, iface: &str) -> bool {
        let mut curr = Some(class);
        while let Some(c) = curr {
            if let Some(def) = self.classes.get(c) {
                if def.interfaces.contains(iface) {
                    return true;
                }
                if !iface.contains('<') {
                    if def.interfaces.iter().any(|i| i.split('<').next().unwrap_or(i) == iface) {
                        return true;
                    }
                }
                curr = def.superclass.as_deref();
            } else {
                break;
            }
        }
        false
    }

    pub fn is_subclass_of(&self, class: &str, base: &str) -> bool {
        if class == base {
            return true;
        }
        let mut curr = self.classes.get(class).and_then(|d| d.superclass.as_deref());
        while let Some(c) = curr {
            if c == base {
                return true;
            }
            curr = self.classes.get(c).and_then(|d| d.superclass.as_deref());
        }
        false
    }

    pub fn is_subclass_or_impl(&self, class: &str, target: &str) -> bool {
        self.is_subclass_of(class, target) || self.is_interface_impl(class, target)
    }

    pub fn find_class_method(&self, class_name: &str, method: &str) -> Option<FnSig> {
        let mut curr = Some(class_name);
        while let Some(c) = curr {
            if let Some(def) = self.lookup_class(c) {
                if let Some(sig) = def.methods.get(method) {
                    return Some(sig.clone());
                }
                curr = def.superclass.as_deref();
            } else {
                break;
            }
        }
        None
    }

    pub fn register_type_alias(&mut self, name: String, target: Type) {
        self.type_aliases.insert(name, target);
    }

    pub fn lookup_type_alias(&self, name: &str) -> Option<&Type> {
        self.type_aliases.get(name)
    }

    pub fn type_aliases(&self) -> &HashMap<String, Type> {
        &self.type_aliases
    }

    pub fn canonicalize_type(&self, ty: &Type) -> Type {
        let classes: std::collections::HashSet<String> = self.classes.keys().cloned().collect();
        let ifaces: std::collections::HashSet<String> = self.interfaces.keys().cloned().collect();
        let enums: std::collections::HashSet<String> = self.enums.keys().cloned().collect();
        Self::canonicalize_type_with_names(ty, &classes, &ifaces, &enums, &self.type_aliases)
    }

    pub fn canonicalize_type_with_names(
        ty: &Type,
        classes: &std::collections::HashSet<String>,
        ifaces: &std::collections::HashSet<String>,
        enums: &std::collections::HashSet<String>,
        aliases: &std::collections::HashMap<String, Type>,
    ) -> Type {
        let mut visited = std::collections::HashSet::new();
        Self::canonicalize_type_internal(ty, classes, ifaces, enums, aliases, &mut visited)
    }

    fn canonicalize_type_internal(
        ty: &Type,
        classes: &std::collections::HashSet<String>,
        ifaces: &std::collections::HashSet<String>,
        enums: &std::collections::HashSet<String>,
        aliases: &std::collections::HashMap<String, Type>,
        visited: &mut std::collections::HashSet<String>,
    ) -> Type {
        match ty {
            Type::Struct(name) => {
                if let Some(aliased) = aliases.get(name) {
                    if visited.insert(name.clone()) {
                        let res = Self::canonicalize_type_internal(aliased, classes, ifaces, enums, aliases, visited);
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
                    } else {
                        ty.clone()
                    }
                }
            }
            Type::Array(inner) => {
                Type::Array(Box::new(Self::canonicalize_type_internal(inner, classes, ifaces, enums, aliases, visited)))
            }
            Type::Tuple(members) => Type::Tuple(
                members
                    .iter()
                    .map(|m| Self::canonicalize_type_internal(m, classes, ifaces, enums, aliases, visited))
                    .collect(),
            ),
            Type::Union(members) => Type::union(
                members
                    .iter()
                    .map(|m| Self::canonicalize_type_internal(m, classes, ifaces, enums, aliases, visited)),
            ),
            Type::Ref { inner, mutable } => Type::Ref {
                inner: Box::new(Self::canonicalize_type_internal(inner, classes, ifaces, enums, aliases, visited)),
                mutable: *mutable,
            },
            Type::Ptr(inner) => {
                Type::Ptr(Box::new(Self::canonicalize_type_internal(inner, classes, ifaces, enums, aliases, visited)))
            }
            Type::Shared(inner) => {
                Type::Shared(Box::new(Self::canonicalize_type_internal(inner, classes, ifaces, enums, aliases, visited)))
            }
            Type::Weak(inner) => {
                Type::Weak(Box::new(Self::canonicalize_type_internal(inner, classes, ifaces, enums, aliases, visited)))
            }
            Type::Map(k, v) => Type::Map(
                Box::new(Self::canonicalize_type_internal(k, classes, ifaces, enums, aliases, visited)),
                Box::new(Self::canonicalize_type_internal(v, classes, ifaces, enums, aliases, visited)),
            ),
            Type::Fn { params, ret, is_cfun } => Type::Fn {
                params: params
                    .iter()
                    .map(|p| TypeParam::new(Self::canonicalize_type_internal(&p.ty, classes, ifaces, enums, aliases, visited), p.is_ref, p.is_mut))
                    .collect(),
                ret: Box::new(Self::canonicalize_type_internal(ret, classes, ifaces, enums, aliases, visited)),
                is_cfun: *is_cfun,
            },
            _ => ty.clone(),
        }
    }

    pub fn canonicalize_all_types(&mut self) {
        let classes: std::collections::HashSet<String> = self.classes.keys().cloned().collect();
        let ifaces: std::collections::HashSet<String> = self.interfaces.keys().cloned().collect();
        let enums: std::collections::HashSet<String> = self.enums.keys().cloned().collect();

        let mut changed = true;
        let mut passes = 0;
        while changed && passes < 100 {
            changed = false;
            passes += 1;
            let current_aliases = self.type_aliases.clone();
            for target in self.type_aliases.values_mut() {
                let canonical = Self::canonicalize_type_with_names(target, &classes, &ifaces, &enums, &current_aliases);
                if &canonical != target {
                    *target = canonical;
                    changed = true;
                }
            }
        }

        self.canonicalize_classes(&classes, &ifaces, &enums);
        self.canonicalize_interfaces(&classes, &ifaces, &enums);
        self.canonicalize_structs(&classes, &ifaces, &enums);
        self.canonicalize_functions(&classes, &ifaces, &enums);
        self.canonicalize_enums(&classes, &ifaces, &enums);
        self.canonicalize_scopes(&classes, &ifaces, &enums);
    }

    fn canonicalize_classes(
        &mut self,
        classes: &std::collections::HashSet<String>,
        ifaces: &std::collections::HashSet<String>,
        enums: &std::collections::HashSet<String>,
    ) {
        let keys: Vec<String> = self.classes.keys().cloned().collect();
        for k in keys {
            if let Some(mut def) = self.classes.remove(&k) {
                for ty in def.fields.values_mut() {
                    *ty = Self::canonicalize_type_with_names(ty, classes, ifaces, enums, &self.type_aliases);
                }
                for ty in def.getters.values_mut() {
                    *ty = Self::canonicalize_type_with_names(ty, classes, ifaces, enums, &self.type_aliases);
                }
                for ty in def.setters.values_mut() {
                    *ty = Self::canonicalize_type_with_names(ty, classes, ifaces, enums, &self.type_aliases);
                }
                for sig in def.methods.values_mut() {
                    sig.params.iter_mut().for_each(|p| p.ty = Self::canonicalize_type_with_names(&p.ty, classes, ifaces, enums, &self.type_aliases));
                    sig.ret = Self::canonicalize_type_with_names(&sig.ret, classes, ifaces, enums, &self.type_aliases);
                }
                self.classes.insert(k, def);
            }
        }
    }

    fn canonicalize_interfaces(
        &mut self,
        classes: &std::collections::HashSet<String>,
        ifaces: &std::collections::HashSet<String>,
        enums: &std::collections::HashSet<String>,
    ) {
        let keys: Vec<String> = self.interfaces.keys().cloned().collect();
        for k in keys {
            if let Some(mut def) = self.interfaces.remove(&k) {
                for sig in def.methods.values_mut() {
                    sig.params.iter_mut().for_each(|p| p.ty = Self::canonicalize_type_with_names(&p.ty, classes, ifaces, enums, &self.type_aliases));
                    sig.ret = Self::canonicalize_type_with_names(&sig.ret, classes, ifaces, enums, &self.type_aliases);
                }
                self.interfaces.insert(k, def);
            }
        }
    }

    fn canonicalize_structs(
        &mut self,
        classes: &std::collections::HashSet<String>,
        ifaces: &std::collections::HashSet<String>,
        enums: &std::collections::HashSet<String>,
    ) {
        let keys: Vec<String> = self.structs.keys().cloned().collect();
        for k in keys {
            if let Some(mut def) = self.structs.remove(&k) {
                for ty in def.fields.values_mut() {
                    *ty = Self::canonicalize_type_with_names(ty, classes, ifaces, enums, &self.type_aliases);
                }
                for sig in def.methods.values_mut() {
                    sig.params.iter_mut().for_each(|p| p.ty = Self::canonicalize_type_with_names(&p.ty, classes, ifaces, enums, &self.type_aliases));
                    sig.ret = Self::canonicalize_type_with_names(&sig.ret, classes, ifaces, enums, &self.type_aliases);
                }
                self.structs.insert(k, def);
            }
        }
    }

    fn canonicalize_functions(
        &mut self,
        classes: &std::collections::HashSet<String>,
        ifaces: &std::collections::HashSet<String>,
        enums: &std::collections::HashSet<String>,
    ) {
        let keys: Vec<String> = self.functions.keys().cloned().collect();
        for k in keys {
            if let Some(mut sig) = self.functions.remove(&k) {
                sig.params.iter_mut().for_each(|p| p.ty = Self::canonicalize_type_with_names(&p.ty, classes, ifaces, enums, &self.type_aliases));
                sig.ret = Self::canonicalize_type_with_names(&sig.ret, classes, ifaces, enums, &self.type_aliases);
                self.functions.insert(k, sig);
            }
        }
    }

    fn canonicalize_enums(
        &mut self,
        classes: &std::collections::HashSet<String>,
        ifaces: &std::collections::HashSet<String>,
        enums: &std::collections::HashSet<String>,
    ) {
        for def in self.enums.values_mut() {
            for payload in def.variants.values_mut() {
                if let Some(ty) = payload {
                    *ty = Self::canonicalize_type_with_names(ty, classes, ifaces, enums, &self.type_aliases);
                }
            }
        }
    }

    fn canonicalize_scopes(
        &mut self,
        classes: &std::collections::HashSet<String>,
        ifaces: &std::collections::HashSet<String>,
        enums: &std::collections::HashSet<String>,
    ) {
        for scope in &mut self.scopes {
            for var in scope.values_mut() {
                var.ty = Self::canonicalize_type_with_names(&var.ty, classes, ifaces, enums, &self.type_aliases);
            }
        }
    }
}

impl Default for TypeEnv { fn default() -> Self { Self::new() } }
