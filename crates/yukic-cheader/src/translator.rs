#![deny(unsafe_code)]
#![allow(clippy::collapsible_if)]

use std::collections::{BTreeSet, HashMap, HashSet};
use tree_sitter::{Node, Parser};
use yukic_ast::attr::CAttributeNode;
use yukic_ast::expr::{Expr, ExprKind, FloatSuffix, IntSuffix, LiteralValue};
use yukic_ast::ident::Ident;
use yukic_ast::item::{
    CParam, CfunDecl, CstructDecl, CvarDecl, ExterncBlock, ExterncItem, FieldDef, Item, ItemId,
    ItemKind, StaticDecl, TypeAliasDecl, Visibility,
};
use yukic_ast::types::{TypeNode, TypeNodeKind};
use yukic_span::Span;
use yukic_target::TargetTriple;

use crate::const_eval::eval_c_int_expr;

#[derive(Clone, Debug)]
pub struct TranslateOutput {
    pub items: Vec<Item>,
    pub symbols: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum CType {
    Named(String),
    Ptr(Box<CType>),
    FixedArray(Box<CType>, usize),
    IncompleteArray(Box<CType>),
    FnType {
        params: Vec<(Option<String>, CType)>,
        variadic: bool,
        ret: Box<CType>,
    },
}

struct Translator<'a> {
    src: &'a str,
    target: TargetTriple,
    span: Span,
    visibility: Visibility,
    externc_items: Vec<ExterncItem>,
    extra_items: Vec<Item>,
    emitted_cstructs: HashMap<String, usize>,
    cstruct_has_body: HashSet<String>,
    emitted_cfuns: HashSet<String>,
    emitted_cvars: HashSet<String>,
    emitted_aliases: HashSet<String>,
    emitted_statics: HashSet<String>,
    symbols: BTreeSet<String>,
    int_consts: HashMap<String, i128>,
    pack_stack: Vec<Option<u32>>,
    anon_counter: usize,
    next_id: u32,
}

pub fn translate_c_source(
    source: &str,
    defines: &HashMap<String, String>,
    header_display: &str,
    namespace: &str,
    target: TargetTriple,
    visibility: Visibility,
    span: Span,
) -> Result<TranslateOutput, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::LANGUAGE.into())
        .map_err(|e| format!("failed to load tree-sitter-c language: {e}"))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "tree-sitter-c failed to parse C header".to_string())?;

    let mut tr = Translator {
        src: source,
        target,
        span,
        visibility,
        externc_items: Vec::new(),
        extra_items: Vec::new(),
        emitted_cstructs: HashMap::new(),
        cstruct_has_body: HashSet::new(),
        emitted_cfuns: HashSet::new(),
        emitted_cvars: HashSet::new(),
        emitted_aliases: HashSet::new(),
        emitted_statics: HashSet::new(),
        symbols: BTreeSet::new(),
        int_consts: HashMap::new(),
        pack_stack: Vec::new(),
        anon_counter: 0,
        next_id: 0,
    };

    tr.walk_unit(tree.root_node());
    tr.emit_macro_constants(defines);

    let mut items = Vec::new();
    if !tr.externc_items.is_empty() {
        let block = ExterncBlock {
            header: Some(header_display.to_string()),
            namespace: Some(namespace.to_string()),
            items: std::mem::take(&mut tr.externc_items),
            span,
        };
        let id = tr.alloc_id();
        items.push(Item::new(id, visibility, ItemKind::Externc(block), span));
    }
    items.extend(tr.extra_items);

    Ok(TranslateOutput {
        items,
        symbols: tr.symbols,
    })
}

impl<'a> Translator<'a> {
    fn alloc_id(&mut self) -> ItemId {
        let id = ItemId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    fn ident(&self, name: &str) -> Ident {
        Ident::new(name.to_string(), self.span)
    }

    fn text(&self, node: Node<'a>) -> &'a str {
        &self.src[node.start_byte()..node.end_byte()]
    }

    fn current_pack_align(&self) -> Option<u32> {
        self.pack_stack.last().copied().flatten()
    }

    fn walk_unit(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.visit_top_level(child);
        }
    }

    fn visit_top_level(&mut self, node: Node<'a>) {
        match node.kind() {
            "preproc_call" => self.handle_pragma(node),
            "linkage_specification" => {
                if let Some(body) = node.child_by_field_name("body") {
                    self.walk_unit(body);
                } else {
                    self.walk_unit(node);
                }
            }
            "declaration" => self.handle_declaration(node),
            "type_definition" => self.handle_typedef(node),
            "struct_specifier" => {
                let _ = self.handle_aggregate_specifier(node, false, &[], None);
            }
            "union_specifier" => {
                let _ = self.handle_aggregate_specifier(node, true, &[], None);
            }
            "enum_specifier" => {
                let _ = self.handle_enum_specifier(node, None);
            }
            "function_definition" | "expression_statement" | "ERROR" => {}
            _ => {}
        }
    }

    fn handle_pragma(&mut self, node: Node<'a>) {
        let dir = node
            .child_by_field_name("directive")
            .map(|d| self.text(d).trim())
            .unwrap_or("");
        if dir != "#pragma" {
            return;
        }
        let arg = node
            .child_by_field_name("argument")
            .map(|a| self.text(a).trim())
            .unwrap_or("");
        if let Some(inner) = arg
            .strip_prefix("pack")
            .map(str::trim)
            .and_then(|s| s.strip_prefix('('))
            .and_then(|s| s.strip_suffix(')'))
        {
            let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
            match parts.as_slice() {
                [""] => {
                    self.pack_stack.push(None);
                }
                ["pop"] => {
                    let _ = self.pack_stack.pop();
                }
                ["push"] => {
                    let cur = self.current_pack_align();
                    self.pack_stack.push(cur);
                }
                ["push", n] => {
                    let val = n.parse::<u32>().ok();
                    self.pack_stack.push(val);
                }
                [n] => {
                    if let Ok(val) = n.parse::<u32>() {
                        self.pack_stack.push(Some(val));
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_declaration(&mut self, node: Node<'a>) {
        let storage = self.collect_storage_classes(node);
        let is_static = storage.iter().any(|s| *s == "static");
        let is_extern = storage.iter().any(|s| *s == "extern");
        let mut c_attrs = self.collect_c_attrs(node);

        let Some(type_node) = node.child_by_field_name("type") else {
            return;
        };

        let mut declarator_nodes = Vec::new();
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if is_declarator_field_child(node, child) {
                declarator_nodes.push(child);
            }
        }

        if declarator_nodes.is_empty() {
            match type_node.kind() {
                "struct_specifier" => {
                    let _ = self.handle_aggregate_specifier(type_node, false, &c_attrs, None);
                }
                "union_specifier" => {
                    let _ = self.handle_aggregate_specifier(type_node, true, &c_attrs, None);
                }
                "enum_specifier" => {
                    let _ = self.handle_enum_specifier(type_node, None);
                }
                _ => {}
            }
            return;
        }

        let base_ty = self.resolve_type_specifier(type_node, None, &c_attrs);
        for raw_decl in declarator_nodes {
            let decl = if raw_decl.kind() == "init_declarator" {
                raw_decl.child_by_field_name("declarator").unwrap_or(raw_decl)
            } else {
                raw_decl
            };
            let mut decl_attrs = c_attrs.clone();
            decl_attrs.extend(self.collect_c_attrs(decl));
            let (name_opt, mut ctype) = self.unwrap_declarator(base_ty.clone(), decl);
            let Some(name) = name_opt else {
                continue;
            };
            if is_static {
                continue;
            }
            
            let is_direct_fn = matches!(ctype, CType::FnType { .. }) && is_fn_declarator(decl);
            if is_direct_fn {
                if let CType::FnType {
                    params,
                    variadic,
                    ret,
                } = ctype
                {
                    self.emit_cfun(name, params, variadic, *ret, decl_attrs);
                }
            } else {
                if let CType::IncompleteArray(inner) = ctype {
                    ctype = CType::Ptr(inner);
                }
                if is_extern || !matches!(ctype, CType::FnType { .. }) {
                    self.emit_cvar(name, ctype, decl_attrs);
                }
            }
        }
        c_attrs.clear();
    }

    fn handle_typedef(&mut self, node: Node<'a>) {
        let c_attrs = self.collect_c_attrs(node);
        let Some(type_node) = node.child_by_field_name("type") else {
            return;
        };

        let mut decl_nodes = Vec::new();
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.kind() != "attribute_specifier"
                && child.kind() != "type_qualifier"
                && child.id() != type_node.id()
                && !is_storage_or_attr_kind(child.kind())
            {
                decl_nodes.push(child);
            }
        }

        // typedef struct { ... } Name;
        if decl_nodes.len() == 1 {
            let decl = decl_nodes[0];
            if decl.kind() == "type_identifier" || decl.kind() == "identifier" {
                let alias_name = self.text(decl).to_string();
                match type_node.kind() {
                    "struct_specifier" => {
                        let actual = self.handle_aggregate_specifier(
                            type_node,
                            false,
                            &c_attrs,
                            Some(&alias_name),
                        );
                        if actual != alias_name {
                            self.emit_type_alias(&alias_name, CType::Named(actual));
                        }
                        return;
                    }
                    "union_specifier" => {
                        let actual = self.handle_aggregate_specifier(
                            type_node,
                            true,
                            &c_attrs,
                            Some(&alias_name),
                        );
                        if actual != alias_name {
                            self.emit_type_alias(&alias_name, CType::Named(actual));
                        }
                        return;
                    }
                    "enum_specifier" => {
                        let actual = self.handle_enum_specifier(type_node, Some(&alias_name));
                        if actual != alias_name {
                            self.emit_type_alias(&alias_name, CType::Named(actual));
                        }
                        return;
                    }
                    _ => {}
                }
            }
        }

        let base_ty = self.resolve_type_specifier(type_node, None, &c_attrs);
        for decl in decl_nodes {
            let (name_opt, ctype) = self.unwrap_declarator(base_ty.clone(), decl);
            let Some(alias_name) = name_opt else {
                continue;
            };
            self.emit_type_alias(&alias_name, ctype);
        }
    }

    fn resolve_type_specifier(
        &mut self,
        node: Node<'a>,
        hint_name: Option<&str>,
        outer_attrs: &[CAttributeNode],
    ) -> CType {
        match node.kind() {
            "struct_specifier" => {
                let name = self.handle_aggregate_specifier(node, false, outer_attrs, hint_name);
                CType::Named(name)
            }
            "union_specifier" => {
                let name = self.handle_aggregate_specifier(node, true, outer_attrs, hint_name);
                CType::Named(name)
            }
            "enum_specifier" => {
                let name = self.handle_enum_specifier(node, hint_name);
                CType::Named(name)
            }
            "primitive_type" | "sized_type_specifier" => {
                CType::Named(map_primitive_specifier(self.text(node)))
            }
            "type_identifier" => {
                CType::Named(map_type_identifier(self.text(node)))
            }
            "macro_type_specifier" => {
                CType::Named("c_int".to_string())
            }
            _ => {
                let txt = self.text(node).trim();
                CType::Named(map_type_identifier(txt))
            }
        }
    }

    fn handle_aggregate_specifier(
        &mut self,
        node: Node<'a>,
        is_union: bool,
        outer_attrs: &[CAttributeNode],
        preferred_name: Option<&str>,
    ) -> String {
        let tag_name = node
            .child_by_field_name("name")
            .map(|n| self.text(n).trim().to_string());
        let name = tag_name
            .or_else(|| preferred_name.map(ToString::to_string))
            .unwrap_or_else(|| {
                let id = self.anon_counter;
                self.anon_counter += 1;
                if is_union {
                    format!("__c_anon_union_{id}")
                } else {
                    format!("__c_anon_struct_{id}")
                }
            });

        let body = node.child_by_field_name("body");
        let mut attrs = outer_attrs.to_vec();
        attrs.extend(self.collect_c_attrs(node));
        if self.current_pack_align() == Some(1)
            && !attrs.iter().any(|a| a.name.name == "packed")
        {
            attrs.push(CAttributeNode::new(self.ident("packed"), None, self.span));
        }

        if let Some(field_list) = body {
            let fields = self.parse_field_list(field_list, &name, is_union);
            let decl = CstructDecl {
                name: self.ident(&name),
                c_attrs: attrs,
                fields,
                is_union,
                span: self.span,
            };
            self.register_cstruct(decl, true);
        } else if !self.emitted_cstructs.contains_key(&name) {
            let decl = CstructDecl {
                name: self.ident(&name),
                c_attrs: attrs,
                fields: Vec::new(),
                is_union,
                span: self.span,
            };
            self.register_cstruct(decl, false);
        }

        name
    }

    fn register_cstruct(&mut self, decl: CstructDecl, has_body: bool) {
        let name = decl.name.name.clone();
        self.symbols.insert(name.clone());
        if let Some(&idx) = self.emitted_cstructs.get(&name) {
            if has_body && !self.cstruct_has_body.contains(&name) {
                self.externc_items[idx] = ExterncItem::Cstruct(decl);
                self.cstruct_has_body.insert(name);
            }
            return;
        }
        let idx = self.externc_items.len();
        self.externc_items.push(ExterncItem::Cstruct(decl));
        self.emitted_cstructs.insert(name.clone(), idx);
        if has_body {
            self.cstruct_has_body.insert(name);
        }
    }

    fn parse_field_list(
        &mut self,
        list_node: Node<'a>,
        parent_name: &str,
        parent_is_union: bool,
    ) -> Vec<FieldDef> {
        let mut fields = Vec::new();
        let mut pending_bf: Option<(String, u32, u32)> = None;
        let mut bf_counter = 0usize;
        let mut cursor = list_node.walk();

        for child in list_node.named_children(&mut cursor) {
            if child.kind() != "field_declaration" {
                continue;
            }
            let Some(type_node) = child.child_by_field_name("type") else {
                continue;
            };
            let bf_bits = self.find_bitfield_bits(child);
            let field_attrs = self.collect_c_attrs(child);

            let mut decls = Vec::new();
            let mut fcursor = child.walk();
            for sub in child.named_children(&mut fcursor) {
                if is_declarator_field_child(child, sub) {
                    decls.push(sub);
                }
            }

            if let Some(bits) = bf_bits {
                let unit_ty = match self.resolve_type_specifier(type_node, None, &field_attrs) {
                    CType::Named(n) => n,
                    _ => "c_uint".to_string(),
                };
                let unit_bits = bitfield_storage_bits(&unit_ty, self.target);
                if bits == 0 {
                    flush_bitfield(&mut pending_bf, &mut bf_counter, &mut fields, self.span);
                    continue;
                }
                match &mut pending_bf {
                    Some((cur_ty, cur_unit_bits, used))
                        if *cur_ty == unit_ty && *used + bits <= *cur_unit_bits =>
                    {
                        *used += bits;
                    }
                    _ => {
                        flush_bitfield(&mut pending_bf, &mut bf_counter, &mut fields, self.span);
                        pending_bf = Some((unit_ty, unit_bits, bits));
                    }
                }
                continue;
            }

            flush_bitfield(&mut pending_bf, &mut bf_counter, &mut fields, self.span);

            // anon without decl
            if decls.is_empty() {
                if !parent_is_union && type_node.kind() == "struct_specifier" {
                    if let Some(inner_list) = type_node.child_by_field_name("body") {
                        let nested = self.parse_field_list(inner_list, parent_name, false);
                        fields.extend(nested);
                        continue;
                    }
                }
                let anon_field = {
                    let id = self.anon_counter;
                    self.anon_counter += 1;
                    format!("{parent_name}__anon_{id}")
                };
                let ty = self.resolve_type_specifier(type_node, Some(&anon_field), &field_attrs);
                let field_ty_node = self.ctype_to_type_node(&ty, false);
                fields.push(FieldDef {
                    name: self.ident(&anon_field),
                    attrs: Vec::new(),
                    ty: field_ty_node,
                    default_val: None,
                    span: self.span,
                });
                continue;
            }

            let first_name_hint = decls
                .first()
                .and_then(|d| self.extract_declarator_name(*d))
                .map(|n| format!("{parent_name}__{n}"));
            let base_ty =
                self.resolve_type_specifier(type_node, first_name_hint.as_deref(), &field_attrs);

            for decl in decls {
                let (name_opt, mut ctype) = self.unwrap_declarator(base_ty.clone(), decl);
                let Some(fname) = name_opt else {
                    continue;
                };
                if let CType::IncompleteArray(inner) = ctype {
                    ctype = CType::FixedArray(inner, 0);
                }
                let ty_node = self.ctype_to_type_node(&ctype, false);
                fields.push(FieldDef {
                    name: self.ident(&fname),
                    attrs: Vec::new(),
                    ty: ty_node,
                    default_val: None,
                    span: self.span,
                });
            }
        }

        flush_bitfield(&mut pending_bf, &mut bf_counter, &mut fields, self.span);
        fields
    }

    fn find_bitfield_bits(&self, field_decl: Node<'a>) -> Option<u32> {
        let mut cursor = field_decl.walk();
        for child in field_decl.named_children(&mut cursor) {
            if child.kind() == "bitfield_clause" {
                let raw = self.text(child).trim().trim_start_matches(':').trim();
                let val = eval_c_int_expr(raw, &self.int_consts, self.target)?;
                return u32::try_from(val).ok();
            }
        }
        None
    }

    fn handle_enum_specifier(&mut self, node: Node<'a>, hint_name: Option<&str>) -> String {
        let tag = node
            .child_by_field_name("name")
            .map(|n| self.text(n).trim().to_string());
        let name = tag
            .clone()
            .or_else(|| hint_name.map(ToString::to_string))
            .unwrap_or_else(|| {
                let id = self.anon_counter;
                self.anon_counter += 1;
                format!("__c_anon_enum_{id}")
            });

        let underlying = node
            .child_by_field_name("underlying_type")
            .map(|u| map_type_identifier(self.text(u).trim()))
            .unwrap_or_else(|| "c_int".to_string());

        if let Some(body) = node.child_by_field_name("body") {
            let mut next_val: i128 = 0;
            let mut cursor = body.walk();
            for child in body.named_children(&mut cursor) {
                if child.kind() != "enumerator" {
                    continue;
                }
                let Some(ename_node) = child.child_by_field_name("name") else {
                    continue;
                };
                let ename = self.text(ename_node).trim().to_string();
                let val = if let Some(val_node) = child.child_by_field_name("value") {
                    eval_c_int_expr(self.text(val_node), &self.int_consts, self.target)
                        .unwrap_or(next_val)
                } else {
                    next_val
                };
                next_val = val.saturating_add(1);
                self.int_consts.insert(ename.clone(), val);
                self.emit_int_static(&ename, val, &underlying);
            }
        }

        if tag.is_some() || hint_name.is_some() {
            self.emit_type_alias(&name, CType::Named(underlying));
        }
        name
    }

    fn emit_cfun(
        &mut self,
        name: String,
        params: Vec<(Option<String>, CType)>,
        variadic: bool,
        ret: CType,
        c_attrs: Vec<CAttributeNode>,
    ) {
        self.symbols.insert(name.clone());
        if !self.emitted_cfuns.insert(name.clone()) {
            return;
        }
        let cparams = params
            .into_iter()
            .enumerate()
            .map(|(idx, (pname, pty))| {
                let decayed = match pty {
                    CType::FixedArray(inner, _) | CType::IncompleteArray(inner) => CType::Ptr(inner),
                    other => other,
                };
                let clean_name = pname
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| format!("arg{idx}"));
                CParam {
                    name: self.ident(&clean_name),
                    ty: self.ctype_to_type_node(&decayed, true),
                    span: self.span,
                }
            })
            .collect();
        let ret_node = match &ret {
            CType::Named(n) if n == "c_void" || n == "void" => None,
            _ => Some(self.ctype_to_type_node(&ret, true)),
        };
        self.externc_items.push(ExterncItem::Cfun(CfunDecl {
            name: self.ident(&name),
            c_attrs,
            yukino_attrs: Vec::new(),
            params: cparams,
            is_variadic: variadic,
            return_type: ret_node,
            symbol_override: None,
            body: None,
            span: self.span,
        }));
    }

    fn emit_cvar(&mut self, name: String, ctype: CType, c_attrs: Vec<CAttributeNode>) {
        self.symbols.insert(name.clone());
        if !self.emitted_cvars.insert(name.clone()) {
            return;
        }
        let ty = self.ctype_to_type_node(&ctype, false);
        self.externc_items.push(ExterncItem::Cvar(CvarDecl {
            name: self.ident(&name),
            c_attrs,
            ty,
            span: self.span,
        }));
    }

    fn emit_type_alias(&mut self, alias_name: &str, target: CType) {
        if let CType::Named(ref tname) = target
            && tname == alias_name
        {
            return;
        }
        if is_yukino_keyword(alias_name) {
            return;
        }
        self.symbols.insert(alias_name.to_string());
        if !self.emitted_aliases.insert(alias_name.to_string()) {
            return;
        }
        let id = self.alloc_id();
        let decl = TypeAliasDecl {
            name: self.ident(alias_name),
            generics: Vec::new(),
            target: self.ctype_to_type_node(&target, false),
            span: self.span,
        };
        self.extra_items
            .push(Item::new(id, self.visibility, ItemKind::TypeAlias(decl), self.span));
    }

    fn emit_int_static(&mut self, name: &str, val: i128, ty_name: &str) {
        if is_yukino_keyword(name) {
            return;
        }
        self.symbols.insert(name.to_string());
        if !self.emitted_statics.insert(name.to_string()) {
            return;
        }
        let (effective_ty, init_lit) = if let Ok(v32) = i32::try_from(val) {
            (ty_name.to_string(), LiteralValue::Int(i64::from(v32)))
        } else if let Ok(v64) = i64::try_from(val) {
            let t = if ty_name == "c_int" {
                "int64".to_string()
            } else {
                ty_name.to_string()
            };
            (t, LiteralValue::TypedInt(v64, IntSuffix::I64))
        } else if let Ok(u64v) = u64::try_from(val) {
            ("uint64".to_string(), LiteralValue::TypedUInt(u64v, IntSuffix::U64))
        } else {
            return;
        };
        let id = self.alloc_id();
        let decl = StaticDecl {
            name: self.ident(name),
            ty: Some(TypeNode::new(
                TypeNodeKind::Ident(self.ident(&effective_ty)),
                self.span,
            )),
            init: Expr::new(ExprKind::Literal(init_lit), self.span),
            span: self.span,
        };
        self.extra_items
            .push(Item::new(id, self.visibility, ItemKind::Static(decl), self.span));
    }

    fn emit_macro_constants(&mut self, defines: &HashMap<String, String>) {
        let mut pending: Vec<(&String, &String)> = defines.iter().collect();
        pending.sort_by(|a, b| a.0.cmp(b.0));

        let mut resolved_ints: HashMap<String, (i128, bool)> = HashMap::new();
        for _ in 0..4 {
            let mut progress = false;
            for (name, raw) in &pending {
                if resolved_ints.contains_key(*name) {
                    continue;
                }
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Some(val) = eval_c_int_expr(trimmed, &self.int_consts, self.target) {
                    let is_unsigned = is_unsigned_int_literal(trimmed);
                    self.int_consts.insert((*name).clone(), val);
                    resolved_ints.insert((*name).clone(), (val, is_unsigned));
                    progress = true;
                }
            }
            if !progress {
                break;
            }
        }

        for (name, raw) in pending {
            if name.starts_with("__") || is_yukino_keyword(name) {
                continue;
            }
            if let Some((val, is_unsigned)) = resolved_ints.get(name) {
                let ty = if *is_unsigned && *val >= 0 {
                    if *val <= i128::from(u32::MAX) {
                        "c_uint"
                    } else {
                        "uint64"
                    }
                } else if i32::try_from(*val).is_ok() {
                    "c_int"
                } else {
                    "int64"
                };
                self.emit_macro_number_static(name, *val, ty);
                continue;
            }
            let trimmed = raw.trim();
            if let Some(unquoted) = parse_c_string_literal(trimmed) {
                self.symbols.insert(name.clone());
                if self.emitted_statics.insert(name.clone()) {
                    let id = self.alloc_id();
                    let decl = StaticDecl {
                        name: self.ident(name),
                        ty: Some(TypeNode::new(
                            TypeNodeKind::Ident(self.ident("string")),
                            self.span,
                        )),
                        init: Expr::new(ExprKind::Literal(LiteralValue::String(unquoted)), self.span),
                        span: self.span,
                    };
                    self.extra_items.push(Item::new(
                        id,
                        self.visibility,
                        ItemKind::Static(decl),
                        self.span,
                    ));
                }
                continue;
            }
            if let Some((fval, is_f32)) = parse_c_float_literal(trimmed) {
                self.symbols.insert(name.clone());
                if self.emitted_statics.insert(name.clone()) {
                    let id = self.alloc_id();
                    let (ty_name, lit) = if is_f32 {
                        ("float32", LiteralValue::TypedFloat(fval, FloatSuffix::F32))
                    } else {
                        ("float64", LiteralValue::Float(fval))
                    };
                    let decl = StaticDecl {
                        name: self.ident(name),
                        ty: Some(TypeNode::new(
                            TypeNodeKind::Ident(self.ident(ty_name)),
                            self.span,
                        )),
                        init: Expr::new(ExprKind::Literal(lit), self.span),
                        span: self.span,
                    };
                    self.extra_items.push(Item::new(
                        id,
                        self.visibility,
                        ItemKind::Static(decl),
                        self.span,
                    ));
                }
            }
        }
    }

    fn emit_macro_number_static(&mut self, name: &str, val: i128, ty: &str) {
        self.symbols.insert(name.to_string());
        if !self.emitted_statics.insert(name.to_string()) {
            return;
        }
        let lit = match ty {
            "c_uint" | "uint32" => {
                let Ok(v) = u32::try_from(val) else {
                    return;
                };
                LiteralValue::UInt(u64::from(v))
            }
            "uint64" => {
                let Ok(v) = u64::try_from(val) else {
                    return;
                };
                LiteralValue::TypedUInt(v, IntSuffix::U64)
            }
            "c_int" | "int32" => {
                let Ok(v) = i32::try_from(val) else {
                    return;
                };
                LiteralValue::Int(i64::from(v))
            }
            _ => {
                let Ok(v) = i64::try_from(val) else {
                    return;
                };
                LiteralValue::TypedInt(v, IntSuffix::I64)
            }
        };
        let id = self.alloc_id();
        let decl = StaticDecl {
            name: self.ident(name),
            ty: Some(TypeNode::new(TypeNodeKind::Ident(self.ident(ty)), self.span)),
            init: Expr::new(ExprKind::Literal(lit), self.span),
            span: self.span,
        };
        self.extra_items
            .push(Item::new(id, self.visibility, ItemKind::Static(decl), self.span));
    }

    fn unwrap_declarator(&mut self, current: CType, node: Node<'a>) -> (Option<String>, CType) {
        match node.kind() {
            "identifier" | "field_identifier" | "type_identifier" => {
                (Some(self.text(node).trim().to_string()), current)
            }
            "pointer_declarator" | "abstract_pointer_declarator" => {
                let inner_ty = CType::Ptr(Box::new(current));
                if let Some(inner) = node.child_by_field_name("declarator") {
                    self.unwrap_declarator(inner_ty, inner)
                } else {
                    (None, inner_ty)
                }
            }
            "array_declarator" | "abstract_array_declarator" => {
                let len = node.child_by_field_name("size").and_then(|s| {
                    eval_c_int_expr(self.text(s), &self.int_consts, self.target)
                        .and_then(|v| usize::try_from(v).ok())
                });
                let arr_ty = match len {
                    Some(n) => CType::FixedArray(Box::new(current), n),
                    None => CType::IncompleteArray(Box::new(current)),
                };
                if let Some(inner) = node.child_by_field_name("declarator") {
                    self.unwrap_declarator(arr_ty, inner)
                } else {
                    (None, arr_ty)
                }
            }
            "function_declarator" | "abstract_function_declarator" => {
                let (params, variadic) = node
                    .child_by_field_name("parameters")
                    .map(|p| self.parse_parameter_list(p))
                    .unwrap_or_default();
                let fn_ty = CType::FnType {
                    params,
                    variadic,
                    ret: Box::new(current),
                };
                if let Some(inner) = node.child_by_field_name("declarator") {
                    self.unwrap_declarator(fn_ty, inner)
                } else {
                    (None, fn_ty)
                }
            }
            "parenthesized_declarator"
            | "abstract_parenthesized_declarator"
            | "attributed_declarator" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if !is_storage_or_attr_kind(child.kind()) {
                        return self.unwrap_declarator(current, child);
                    }
                }
                (None, current)
            }
            _ => (None, current),
        }
    }

    fn extract_declarator_name(&self, node: Node<'a>) -> Option<String> {
        match node.kind() {
            "identifier" | "field_identifier" | "type_identifier" => {
                Some(self.text(node).trim().to_string())
            }
            "pointer_declarator" | "array_declarator" | "function_declarator" => node
                .child_by_field_name("declarator")
                .and_then(|d| self.extract_declarator_name(d)),
            "parenthesized_declarator" | "attributed_declarator" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if !is_storage_or_attr_kind(child.kind())
                        && let Some(n) = self.extract_declarator_name(child)
                    {
                        return Some(n);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn parse_parameter_list(&mut self, list_node: Node<'a>) -> (Vec<(Option<String>, CType)>, bool) {
        let mut params = Vec::new();
        let mut variadic = false;
        let mut cursor = list_node.walk();

        for child in list_node.named_children(&mut cursor) {
            match child.kind() {
                "variadic_parameter" => variadic = true,
                "parameter_declaration" => {
                    let Some(type_node) = child.child_by_field_name("type") else {
                        continue;
                    };
                    let base = self.resolve_type_specifier(type_node, None, &[]);
                    if let Some(decl) = child.child_by_field_name("declarator") {
                        let (pname, pty) = self.unwrap_declarator(base, decl);
                        params.push((pname, pty));
                    } else {
                        params.push((None, base));
                    }
                }
                _ => {}
            }
        }

        // void param
        if params.len() == 1
            && params[0].0.is_none()
            && matches!(&params[0].1, CType::Named(n) if n == "c_void" || n == "void")
        {
            params.clear();
        }
        (params, variadic)
    }

    fn ctype_to_type_node(&self, ctype: &CType, is_param_or_ret: bool) -> TypeNode {
        match ctype {
            CType::Named(name) => TypeNode::new(TypeNodeKind::Ident(self.ident(name)), self.span),
            CType::Ptr(inner) => {
                // Function pointers in Yukino map directly to `cfun(...) -> R`.
                if let CType::FnType { params, ret, .. } = inner.as_ref() {
                    return self.fn_type_to_cfun_node(params, ret);
                }
                TypeNode::new(
                    TypeNodeKind::Pointer(Box::new(self.ctype_to_type_node(inner, is_param_or_ret))),
                    self.span,
                )
            }
            CType::FixedArray(inner, len) => TypeNode::new(
                TypeNodeKind::Array(
                    Box::new(self.ctype_to_type_node(inner, false)),
                    Some(*len),
                ),
                self.span,
            ),
            CType::IncompleteArray(inner) => TypeNode::new(
                TypeNodeKind::Pointer(Box::new(self.ctype_to_type_node(inner, false))),
                self.span,
            ),
            CType::FnType { params, ret, .. } => self.fn_type_to_cfun_node(params, ret),
        }
    }

    fn fn_type_to_cfun_node(&self, params: &[(Option<String>, CType)], ret: &CType) -> TypeNode {
        let param_nodes = params
            .iter()
            .map(|(_, pt)| match pt {
                CType::FixedArray(inner, _) | CType::IncompleteArray(inner) => TypeNode::new(
                    TypeNodeKind::Pointer(Box::new(self.ctype_to_type_node(inner, true))),
                    self.span,
                ),
                other => self.ctype_to_type_node(other, true),
            })
            .collect();
        let ret_node = match ret {
            CType::Named(n) if n == "c_void" || n == "void" => {
                TypeNode::new(TypeNodeKind::Ident(self.ident("void")), self.span)
            }
            other => self.ctype_to_type_node(other, true),
        };
        TypeNode::new(TypeNodeKind::CFun(param_nodes, Box::new(ret_node)), self.span)
    }

    fn collect_storage_classes(&self, node: Node<'a>) -> Vec<&'a str> {
        let mut res = Vec::new();
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.kind() == "storage_class_specifier" {
                res.push(self.text(child).trim());
            }
        }
        res
    }

    fn collect_c_attrs(&self, node: Node<'a>) -> Vec<CAttributeNode> {
        let mut attrs = Vec::new();
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "ms_call_modifier" => {
                    let raw = self.text(child).trim().trim_start_matches('_');
                    if !raw.is_empty() {
                        attrs.push(CAttributeNode::new(self.ident(raw), None, self.span));
                    }
                }
                "attribute_specifier" => {
                    let raw = self.text(child);
                    if raw.contains("packed") {
                        attrs.push(CAttributeNode::new(self.ident("packed"), None, self.span));
                    }
                    if let Some(align) = extract_aligned_arg(raw) {
                        attrs.push(CAttributeNode::new(
                            self.ident("align"),
                            Some(align),
                            self.span,
                        ));
                    }
                    for cc in ["cdecl", "stdcall", "fastcall"] {
                        if raw.contains(cc) {
                            attrs.push(CAttributeNode::new(self.ident(cc), None, self.span));
                        }
                    }
                }
                "parenthesized_declarator" | "function_declarator" | "pointer_declarator" => {
                    attrs.extend(self.collect_c_attrs(child));
                }
                _ => {}
            }
        }
        attrs
    }
}

fn is_declarator_field_child(parent: Node<'_>, child: Node<'_>) -> bool {
    if let Some(idx) = parent.child_index_for_node(child)
        && let Some(field) = parent.field_name_for_child(idx as u32)
    {
        return field == "declarator";
    }
    child.kind() == "init_declarator"
}

trait NodeFieldExt {
    fn child_index_for_node(&self, target: Node<'_>) -> Option<usize>;
}

impl NodeFieldExt for Node<'_> {
    fn child_index_for_node(&self, target: Node<'_>) -> Option<usize> {
        let mut cursor = self.walk();
        self.children(&mut cursor)
            .position(|c| c.id() == target.id())
    }
}

fn is_storage_or_attr_kind(kind: &str) -> bool {
    matches!(
        kind,
        "storage_class_specifier"
            | "type_qualifier"
            | "attribute_specifier"
            | "attribute_declaration"
            | "ms_declspec_modifier"
            | "ms_call_modifier"
            | "bitfield_clause"
    )
}

fn is_fn_declarator(mut node: Node<'_>) -> bool {
    loop {
        match node.kind() {
            "function_declarator" => return true,
            "pointer_declarator" => {
                let Some(inner) = node.child_by_field_name("declarator") else {
                    return false;
                };
                node = inner;
            }
            "parenthesized_declarator" | "attributed_declarator" => {
                let mut cursor = node.walk();
                let mut next = None;
                for child in node.named_children(&mut cursor) {
                    if !is_storage_or_attr_kind(child.kind()) {
                        next = Some(child);
                        break;
                    }
                }
                let Some(n) = next else {
                    return false;
                };
                node = n;
            }
            _ => return false,
        }
    }
}

fn flush_bitfield(
    pending: &mut Option<(String, u32, u32)>,
    counter: &mut usize,
    fields: &mut Vec<FieldDef>,
    span: Span,
) {
    if let Some((unit_ty, _, _)) = pending.take() {
        let fname = format!("__bitfield_{}", *counter);
        *counter += 1;
        fields.push(FieldDef {
            name: Ident::new(fname, span),
            attrs: Vec::new(),
            ty: TypeNode::new(TypeNodeKind::Ident(Ident::new(unit_ty, span)), span),
            default_val: None,
            span,
        });
    }
}

fn bitfield_storage_bits(ty_name: &str, target: TargetTriple) -> u32 {
    let spec = target.spec();
    match ty_name {
        "c_char" | "c_schar" | "c_uchar" | "c_bool" | "bool" | "int8" | "uint8" => 8,
        "c_short" | "c_ushort" | "int16" | "uint16" => 16,
        "c_long" | "c_ulong" => {
            if spec.data_model == yukic_target::DataModel::Lp64 {
                64
            } else {
                32
            }
        }
        "c_longlong" | "c_ulonglong" | "int64" | "uint64" => 64,
        "c_size_t" | "c_ssize_t" => u32::from(spec.pointer_width),
        _ => 32,
    }
}

fn extract_aligned_arg(raw: &str) -> Option<String> {
    let idx = raw.find("aligned")?;
    let after = raw[idx + "aligned".len()..].trim_start();
    let after_paren = after.strip_prefix('(')?;
    let close = after_paren.find(')')?;
    let val = after_paren[..close].trim();
    if val.chars().all(|c| c.is_ascii_digit()) && !val.is_empty() {
        Some(val.to_string())
    } else {
        None
    }
}

fn map_primitive_specifier(raw: &str) -> String {
    let tokens: Vec<&str> = raw
        .split_whitespace()
        .filter(|t| !matches!(*t, "const" | "volatile" | "restrict" | "__restrict"))
        .collect();
    if tokens.is_empty() {
        return "c_int".to_string();
    }
    let has_unsigned = tokens.contains(&"unsigned");
    let has_signed = tokens.contains(&"signed");
    let long_count = tokens.iter().filter(|t| **t == "long").count();

    if tokens.contains(&"void") {
        return "c_void".to_string();
    }
    if tokens.contains(&"_Bool") || tokens.contains(&"bool") {
        return "bool".to_string();
    }
    if tokens.contains(&"double") {
        return "float64".to_string();
    }
    if tokens.contains(&"float") {
        return "float32".to_string();
    }
    if tokens.contains(&"char") {
        if has_unsigned {
            return "c_uchar".to_string();
        }
        if has_signed {
            return "c_schar".to_string();
        }
        return "c_char".to_string();
    }
    if tokens.contains(&"short") {
        return if has_unsigned { "c_ushort" } else { "c_short" }.to_string();
    }
    if long_count >= 2 {
        return if has_unsigned {
            "c_ulonglong"
        } else {
            "c_longlong"
        }
        .to_string();
    }
    if long_count == 1 {
        return if has_unsigned { "c_ulong" } else { "c_long" }.to_string();
    }
    if tokens.contains(&"size_t") {
        return "c_size_t".to_string();
    }
    if tokens.contains(&"ssize_t") {
        return "c_ssize_t".to_string();
    }
    if has_unsigned {
        return "c_uint".to_string();
    }
    "c_int".to_string()
}

fn map_type_identifier(raw: &str) -> String {
    match raw {
        "void" => "c_void".to_string(),
        "bool" | "_Bool" => "bool".to_string(),
        "size_t" => "c_size_t".to_string(),
        "ssize_t" => "c_ssize_t".to_string(),
        "int8_t" => "int8".to_string(),
        "uint8_t" => "uint8".to_string(),
        "int16_t" => "int16".to_string(),
        "uint16_t" => "uint16".to_string(),
        "int32_t" => "int32".to_string(),
        "uint32_t" => "uint32".to_string(),
        "int64_t" => "int64".to_string(),
        "uint64_t" => "uint64".to_string(),
        other => other.to_string(),
    }
}

fn is_unsigned_int_literal(s: &str) -> bool {
    let trimmed = s.trim();
    let lower = trimmed.to_ascii_lowercase();
    lower.ends_with('u') || lower.ends_with("ul") || lower.ends_with("ull")
}

fn parse_c_string_literal(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if !(trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2) {
        return None;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next()? {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '0' => out.push('\0'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                other => out.push(other),
            }
        } else if ch == '"' {
            return None;
        } else {
            out.push(ch);
        }
    }
    Some(out)
}

fn parse_c_float_literal(s: &str) -> Option<(f64, bool)> {
    let trimmed = s
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim();
    if trimmed.is_empty() {
        return None;
    }
    let (num_str, is_f32) = if let Some(prefix) = trimmed.strip_suffix(['f', 'F']) {
        (prefix, true)
    } else if let Some(prefix) = trimmed.strip_suffix(['l', 'L']) {
        (prefix, false)
    } else {
        (trimmed, false)
    };
    if !num_str.contains(['.', 'e', 'E']) {
        return None;
    }
    let val = num_str.parse::<f64>().ok()?;
    Some((val, is_f32))
}

fn is_yukino_keyword(name: &str) -> bool {
    matches!(
        name,
        "set"
            | "rewrite"
            | "fun"
            | "cfun"
            | "struct"
            | "cstruct"
            | "cunion"
            | "cvar"
            | "cload"
            | "class"
            | "interface"
            | "enum"
            | "type"
            | "load"
            | "public"
            | "static"
            | "alias"
            | "externc"
            | "if"
            | "elif"
            | "else"
            | "while"
            | "for"
            | "loop"
            | "switch"
            | "return"
            | "break"
            | "continue"
            | "true"
            | "false"
            | "null"
            | "this"
            | "super"
            | "bool"
            | "void"
            | "string"
            | "int"
            | "float"
            | "ref"
            | "move"
            | "region"
            | "defer"
            | "sus"
            | "catch"
            | "finally"
            | "space"
    )
}
