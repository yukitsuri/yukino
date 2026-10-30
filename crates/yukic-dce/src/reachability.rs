#![deny(unsafe_code)]
#![allow(clippy::wildcard_imports, clippy::match_same_arms)]

use std::collections::{HashMap, HashSet};
use yukic_ast::*;

pub struct ReachabilityAnalysis<'a> {
    by_id: HashMap<ItemId, &'a Item>,
    by_name: HashMap<String, Vec<ItemId>>,
    file_structs: HashMap<yukic_span::FileId, Vec<ItemId>>,
    name_file: HashMap<String, yukic_span::FileId>,
}

impl<'a> ReachabilityAnalysis<'a> {
    pub fn new(module: &'a Module) -> Self {
        let mut by_id = HashMap::new();
        let mut by_name: HashMap<String, Vec<ItemId>> = HashMap::new();
        let mut file_structs: HashMap<yukic_span::FileId, Vec<ItemId>> = HashMap::new();
        let mut name_file: HashMap<String, yukic_span::FileId> = HashMap::new();

        for item in &module.items {
            by_id.insert(item.id, item);
            let fid = item.span.file_id;
            let is_import = matches!(item.kind, ItemKind::Import(_));
            for name in item_exported_names(item) {
                by_name.entry(name.clone()).or_default().push(item.id);
                if !is_import {
                    name_file.insert(name, fid);
                }
            }
            if let ItemKind::Class(c) = &item.kind {
                for m in &c.methods {
                    name_file.insert(m.name.as_str().to_string(), fid);
                }
            }
            if matches!(item.kind, ItemKind::Struct(_) | ItemKind::Cstruct(_)) {
                file_structs.entry(fid).or_default().push(item.id);
            }
        }

        Self {
            by_id,
            by_name,
            file_structs,
            name_file,
        }
    }

    pub fn compute_live_items(&self, roots: &HashSet<ItemId>) -> HashSet<ItemId> {
        let mut live = HashSet::new();
        let mut worklist: Vec<ItemId> = roots.iter().copied().collect();

        while let Some(item_id) = worklist.pop() {
            if !live.insert(item_id) { continue; }

            if let Some(item) = self.by_id.get(&item_id) {
                let refs = Self::collect_item_references(item);
                for ref_name in refs {
                    if let Some(callee) = ref_name.strip_prefix("@catchall:") {
                        if let Some(&fid) = self.name_file.get(callee) {
                            if let Some(struct_ids) = self.file_structs.get(&fid) {
                                for &sid in struct_ids {
                                    if !live.contains(&sid) {
                                        worklist.push(sid);
                                    }
                                }
                            }
                        }
                    } else if let Some(target_ids) = self.by_name.get(&ref_name) {
                        for &target_id in target_ids {
                            if !live.contains(&target_id) {
                                worklist.push(target_id);
                            }
                        }
                    }
                }
            }
        }

        live
    }

    fn collect_item_references(item: &Item) -> Vec<String> {
        let mut refs = Vec::new();
        match &item.kind {
            ItemKind::Fun(decl) => collect_fun_refs(decl, &mut refs),
            ItemKind::Cfun(decl) => collect_cfun_refs(decl, &mut refs),
            ItemKind::Class(decl) => collect_class_refs(decl, &mut refs),
            ItemKind::Struct(decl) => collect_struct_refs(decl, &mut refs),
            ItemKind::Cstruct(decl) => collect_cstruct_refs(decl, &mut refs),
            ItemKind::Interface(decl) => collect_interface_refs(decl, &mut refs),
            ItemKind::Enum(decl) => collect_enum_refs(decl, &mut refs),
            ItemKind::TypeAlias(decl) => collect_type_alias_refs(decl, &mut refs),
            ItemKind::Static(decl) => collect_static_refs(decl, &mut refs),
            ItemKind::Externc(decl) => collect_externc_refs(decl, &mut refs),
            ItemKind::Macro(decl) => collect_macro_refs(decl, &mut refs),
            ItemKind::Import(decl) => collect_import_refs(decl, &mut refs),
            ItemKind::Alias(decl) => collect_alias_refs(decl, &mut refs),
            ItemKind::Error => {}
        }
        refs
    }
}

fn item_exported_names(item: &Item) -> Vec<String> {
    match &item.kind {
        ItemKind::Fun(d) => vec![d.name.as_str().to_string()],
        ItemKind::Cfun(d) => vec![d.name.as_str().to_string()],
        ItemKind::Class(d) => vec![d.name.as_str().to_string()],
        ItemKind::Struct(d) => vec![d.name.as_str().to_string()],
        ItemKind::Cstruct(d) => vec![d.name.as_str().to_string()],
        ItemKind::Interface(d) => vec![d.name.as_str().to_string()],
        ItemKind::Enum(d) => vec![d.name.as_str().to_string()],
        ItemKind::TypeAlias(d) => vec![d.name.as_str().to_string()],
        ItemKind::Static(d) => vec![d.name.as_str().to_string()],
        ItemKind::Macro(d) => vec![d.name.as_str().to_string()],
        ItemKind::Import(d) => collect_import_exported_names(d),
        ItemKind::Alias(d) => vec![d.alias.as_str().to_string()],
        _ => Vec::new(),
    }
}

fn collect_import_exported_names(decl: &ImportDecl) -> Vec<String> {
    let Some(clause) = &decl.symbols else { return Vec::new(); };
    
    clause
        .items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            if let Some(alias) = clause.aliases.get(idx) {
                Some(alias.as_str().to_string())
            } 

            else if let ImportPattern::Exact(id) = &item.pattern {
                Some(id.as_str().to_string())
            } 

            else { None }
        })
        .collect()
}

fn collect_fun_refs(decl: &FunDecl, refs: &mut Vec<String>) {
    for param in &decl.params {
        if let Some(ty) = &param.ty { collect_type_refs(ty, refs); }
        
        if let Some(default_val) = &param.default_val {
            collect_expr_refs(default_val, refs);
        }
    }
    if let Some(ret_ty) = &decl.return_type {
        collect_type_refs(ret_ty, refs);
    }
    collect_block_refs(&decl.body, refs);
}

fn collect_cfun_refs(decl: &CfunDecl, refs: &mut Vec<String>) {
    for param in &decl.params { collect_type_refs(&param.ty, refs); }
    
    if let Some(ret_ty) = &decl.return_type {
        collect_type_refs(ret_ty, refs);
    }
    if let Some(body) = &decl.body {
        collect_block_refs(body, refs);
    }
}

fn collect_class_refs(decl: &ClassDecl, refs: &mut Vec<String>) {
    if let Some(base) = &decl.base_class {
        refs.push(base.as_str().to_string());
    }
    for iface in &decl.interfaces {
        let base = iface.as_str().split('<').next().unwrap_or(iface.as_str());
        refs.push(base.to_string());
    }
    for field in &decl.fields {
        collect_type_refs(&field.ty, refs);
        if let Some(init) = &field.default_val {
            collect_expr_refs(init, refs);
        }
    }
    for method in &decl.methods {
        collect_method_refs(method, refs);
    }
    for acc in &decl.accessors {
        collect_type_refs(&acc.ty, refs);
        collect_block_refs(&acc.body, refs);
    }
}

fn collect_struct_refs(decl: &StructDecl, refs: &mut Vec<String>) {
    for field in &decl.fields {
        collect_type_refs(&field.ty, refs);
        if let Some(init) = &field.default_val {
            collect_expr_refs(init, refs);
        }
    }
    for method in &decl.methods {
        collect_method_refs(method, refs);
    }
    for static_method in &decl.static_methods {
        for param in &static_method.params {
            if let Some(ty) = &param.ty {
                collect_type_refs(ty, refs);
            }
        }
        if let Some(ret_ty) = &static_method.return_type {
            collect_type_refs(ret_ty, refs);
        }
        collect_block_refs(&static_method.body, refs);
    }
}

fn collect_method_refs(method: &MethodDecl, refs: &mut Vec<String>) {
    for param in &method.params {
        if let Some(ty) = &param.ty {
            collect_type_refs(ty, refs);
        }
    }
    if let Some(ret_ty) = &method.return_type {
        collect_type_refs(ret_ty, refs);
    }
    collect_block_refs(&method.body, refs);
}

fn collect_cstruct_refs(decl: &CstructDecl, refs: &mut Vec<String>) {
    for field in &decl.fields {
        collect_type_refs(&field.ty, refs);
    }
}

fn collect_interface_refs(decl: &InterfaceDecl, refs: &mut Vec<String>) {
    for method in &decl.methods {
        for param in &method.params {
            if let Some(ty) = &param.ty {
                collect_type_refs(ty, refs);
            }
        }
        if let Some(ret_ty) = &method.return_type {
            collect_type_refs(ret_ty, refs);
        }
        if let Some(body) = &method.default_body {
            collect_block_refs(body, refs);
        }
    }
}

fn collect_enum_refs(decl: &EnumDecl, refs: &mut Vec<String>) {
    for variant in &decl.variants {
        match variant {
            EnumVariant::Simple(_) => {}
            EnumVariant::WithValue(_, expr) => collect_expr_refs(expr, refs),
            EnumVariant::Adt(_, fields) => {
                for field in fields {
                    collect_type_refs(&field.ty, refs);
                }
            }
        }
    }
}

fn collect_type_alias_refs(decl: &TypeAliasDecl, refs: &mut Vec<String>) {
    collect_type_refs(&decl.target, refs);
}

fn collect_static_refs(decl: &StaticDecl, refs: &mut Vec<String>) {
    if let Some(ty) = &decl.ty {
        collect_type_refs(ty, refs);
    }
    collect_expr_refs(&decl.init, refs);
}

fn collect_externc_refs(block: &ExterncBlock, refs: &mut Vec<String>) {
    for item in &block.items {
        match item {
            ExterncItem::Cfun(decl) => collect_cfun_refs(decl, refs),
            ExterncItem::Cstruct(decl) => collect_cstruct_refs(decl, refs),
        }
    }
}

fn collect_macro_refs(decl: &MacroDefDecl, refs: &mut Vec<String>) {
    match &decl.body {
        MacroBody::Block(block) => collect_block_refs(block, refs),
        MacroBody::Expr(expr) => collect_expr_refs(expr, refs),
    }
}

fn collect_import_refs(decl: &ImportDecl, refs: &mut Vec<String>) {
    if let PathPrefix::Package(pkg) = &decl.path.prefix {
        refs.push(pkg.as_str().to_string());
    }
    for seg in &decl.path.segments {
        if let ImportPattern::Exact(id) = &seg.pattern {
            refs.push(id.as_str().to_string());
        }
    }
}

fn collect_alias_refs(decl: &AliasDecl, refs: &mut Vec<String>) {
    match &decl.kind {
        AliasKind::Direct(id) => refs.push(id.as_str().to_string()),
        AliasKind::Symbol { symbol, .. } => refs.push(symbol.as_str().to_string()),
        AliasKind::Namespace(idents) => {
            if let Some(last) = idents.last() {
                refs.push(last.as_str().to_string());
            }
        }
    }
}

pub fn collect_type_refs(ty: &TypeNode, refs: &mut Vec<String>) {
    match &ty.kind {
        TypeNodeKind::Ident(ident) => refs.push(ident.as_str().to_string()),
        TypeNodeKind::Path(segments) => {
            if let Some(last) = segments.last() {
                refs.push(last.as_str().to_string());
            }
        }
        TypeNodeKind::Pointer(inner)
        | TypeNodeKind::Ref(inner)
        | TypeNodeKind::Array(inner, _)
        | TypeNodeKind::Optional(inner) => collect_type_refs(inner, refs),
        TypeNodeKind::Map(k, v) | TypeNodeKind::Result(k, v) => {
            collect_type_refs(k, refs);
            collect_type_refs(v, refs);
        }
        TypeNodeKind::Tuple(types) | TypeNodeKind::Union(types) => {
            for t in types {
                collect_type_refs(t, refs);
            }
        }
        TypeNodeKind::Fun(params, ret) | TypeNodeKind::CFun(params, ret) => {
            for p in params {
                collect_type_refs(p, refs);
            }
            collect_type_refs(ret, refs);
        }
        TypeNodeKind::Generic(ident, args) => {
            refs.push(ident.as_str().to_string());
            for arg in args {
                collect_type_refs(arg, refs);
            }
        }
    }
}

pub fn collect_block_refs(block: &Block, refs: &mut Vec<String>) {
    for stmt in &block.stmts {
        collect_stmt_refs(stmt, refs);
    }
}

pub fn collect_stmt_refs(stmt: &Stmt, refs: &mut Vec<String>) {
    match &stmt.kind {
        StmtKind::Set { ty, init, .. } => {
            if let Some(t) = ty {
                collect_type_refs(t, refs);
            }
            collect_expr_refs(init, refs);
        }
        StmtKind::Rewrite { target, value, .. } => {
            collect_lvalue_refs(target, refs);
            collect_expr_refs(value, refs);
        }
        StmtKind::Static(decl) => collect_static_refs(decl, refs),
        StmtKind::Expr(expr) => collect_expr_refs(expr, refs),
        StmtKind::Return(Some(expr)) => collect_expr_refs(expr, refs),
        StmtKind::Recur(exprs) => {
            for e in exprs {
                collect_expr_refs(e, refs);
            }
        }
        StmtKind::Defer(block) => collect_block_refs(block, refs),
        StmtKind::Return(None)
        | StmtKind::Break(_)
        | StmtKind::Continue(_)
        | StmtKind::MacroDef(_)
        | StmtKind::MacroCall(_)
        | StmtKind::Space
        | StmtKind::Error => {}
    }
}

fn collect_lvalue_refs(target: &LValue, refs: &mut Vec<String>) {
    match target {
        LValue::Ident(_) => {}
        LValue::Field(expr, _) => collect_expr_refs(expr, refs),
        LValue::Index(arr, idx) => {
            collect_expr_refs(arr, refs);
            collect_expr_refs(idx, refs);
        }
    }
}

pub fn collect_expr_refs(expr: &Expr, refs: &mut Vec<String>) {
    match &expr.kind {
        ExprKind::Ident(ident) => refs.push(ident.as_str().to_string()),
        ExprKind::Path(segments) => {
            if segments.len() == 2 && (segments[0].name == "extern" || segments[0].name == "rt") {
                refs.push(segments[1].name.clone());
            } else {
                if let Some(first) = segments.first() {
                    refs.push(first.as_str().to_string());
                }
                if let Some(last) = segments.last() {
                    refs.push(last.as_str().to_string());
                }
            }
        }
        ExprKind::Unary(_, inner) => collect_expr_refs(inner, refs),
        ExprKind::Binary(_, lhs, rhs) => {
            collect_expr_refs(lhs, refs);
            collect_expr_refs(rhs, refs);
        }
        ExprKind::Ternary(cond, then_b, else_b) => {
            collect_expr_refs(cond, refs);
            collect_expr_refs(then_b, refs);
            collect_expr_refs(else_b, refs);
        }
        ExprKind::Call(callee, args) | ExprKind::ClassInit(callee, args) => {
            collect_expr_refs(callee, refs);
            for arg in args {
                collect_expr_refs(arg, refs);
            }
        }
        ExprKind::MethodCall(target, method, args) => {
            collect_expr_refs(target, refs);
            refs.push(method.as_str().to_string());
            for arg in args {
                collect_expr_refs(arg, refs);
            }
        }
        ExprKind::QualifiedMethodCall(target, ty_name, method, args) => {
            collect_expr_refs(target, refs);
            refs.push(ty_name.as_str().to_string());
            refs.push(method.as_str().to_string());
            for arg in args {
                collect_expr_refs(arg, refs);
            }
        }
        ExprKind::SafeCall(target, method, args) => {
            collect_expr_refs(target, refs);
            refs.push(method.as_str().to_string());
            for arg in args {
                collect_expr_refs(arg, refs);
            }
        }
        ExprKind::FieldAccess(target, _) => collect_expr_refs(target, refs),
        ExprKind::IndexAccess(arr, idx) => {
            collect_expr_refs(arr, refs);
            collect_expr_refs(idx, refs);
        }
        ExprKind::SliceAccess(arr, start, end, _) => {
            collect_expr_refs(arr, refs);
            if let Some(s) = start {
                collect_expr_refs(s, refs);
            }
            if let Some(e) = end {
                collect_expr_refs(e, refs);
            }
        }
        ExprKind::StructInit(name, fields) => {
            refs.push(name.as_str().to_string());
            for f in fields {
                collect_expr_refs(&f.value, refs);
            }
        }
        ExprKind::ArrayInit(items) | ExprKind::TupleInit(items) => {
            for item in items {
                collect_expr_refs(item, refs);
            }
        }
        ExprKind::ArrayAlloc(elem_ty, _) => {
            collect_type_refs(elem_ty, refs);
        }
        ExprKind::MapInit(pairs) => {
            for (k, v) in pairs {
                collect_expr_refs(k, refs);
                collect_expr_refs(v, refs);
            }
        }
        ExprKind::Block(block) | ExprKind::RegionBlock(block) => collect_block_refs(block, refs),
        ExprKind::If {
            cond,
            then_branch,
            elif_branches,
            else_branch,
        } => {
            collect_expr_refs(cond, refs);
            collect_block_refs(then_branch, refs);
            for (c, b) in elif_branches {
                collect_expr_refs(c, refs);
                collect_block_refs(b, refs);
            }
            if let Some(b) = else_branch {
                collect_block_refs(b, refs);
            }
        }
        ExprKind::Switch { target, arms } => {
            collect_expr_refs(target, refs);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_expr_refs(guard, refs);
                }
                collect_block_refs(&arm.body, refs);
            }
        }
        ExprKind::While { cond, body, .. } => {
            collect_expr_refs(cond, refs);
            collect_block_refs(body, refs);
        }
        ExprKind::For { iter, body, .. } => {
            collect_expr_refs(iter, refs);
            collect_block_refs(body, refs);
        }
        ExprKind::Loop { body, .. } => collect_block_refs(body, refs),
        ExprKind::Sus {
            body,
            catches,
            finally_branch,
        } => {
            let mut sus_body_refs = Vec::new();
            collect_block_refs(body, &mut sus_body_refs);
            refs.extend(sus_body_refs.clone());
            if catches.iter().any(|c| c.error_type.is_none()) {
                for r in sus_body_refs {
                    refs.push(format!("@catchall:{r}"));
                }
            }
            for c in catches {
                if let Some(ty) = &c.error_type {
                    collect_type_refs(ty, refs);
                }
                collect_block_refs(&c.body, refs);
            }
            if let Some(f) = finally_branch {
                collect_block_refs(f, refs);
            }
        }
        ExprKind::Pipe(lhs, rhs) => {
            collect_expr_refs(lhs, refs);
            collect_expr_refs(rhs, refs);
        }
        ExprKind::Lambda(_, body) => collect_expr_refs(body, refs),
        ExprKind::AnonymousFun(decl) => collect_fun_refs(decl, refs),
        ExprKind::PtrCast(inner, ty) => {
            collect_expr_refs(inner, refs);
            collect_type_refs(ty, refs);
        }
        ExprKind::PtrOf(ident) => refs.push(ident.as_str().to_string()),
        ExprKind::Ref { expr: inner, .. } => collect_expr_refs(inner, refs),
        ExprKind::Is(inner, ty) => {
            collect_expr_refs(inner, refs);
            collect_type_refs(ty, refs);
        }
        ExprKind::Spread(inner) | ExprKind::NamedArg(_, inner) => collect_expr_refs(inner, refs),
        ExprKind::InterpolatedString(parts) => {
            for part in parts {
                if let StringPart::Expr(e) = part {
                    collect_expr_refs(e, refs);
                }
            }
        }
        ExprKind::Literal(_)
        | ExprKind::This
        | ExprKind::Super
        | ExprKind::MacroCall(_)
        | ExprKind::Error => {}
    }
}
