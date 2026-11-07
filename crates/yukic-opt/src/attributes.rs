#![deny(unsafe_code)]

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::values::{CallSiteValue, FunctionValue};
use yukic_ast::attr::{AttributeArgs, AttributeNode};
use yukic_ast::expr::{Expr, ExprKind, LiteralValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlineKind {
    Always,
    Hint,
    Never,
}

pub fn add_enum_attribute<'ctx>(
    context: &'ctx Context,
    fn_val: FunctionValue<'ctx>,
    name: &str,
    val: u64,
) {
    let kind_id = Attribute::get_named_enum_kind_id(name);
    if kind_id != 0 {
        let attr = context.create_enum_attribute(kind_id, val);
        fn_val.add_attribute(AttributeLoc::Function, attr);
    }
}

pub fn annotate_stable<'ctx>(context: &'ctx Context, fn_val: FunctionValue<'ctx>) {
    add_enum_attribute(context, fn_val, "nounwind", 0);
    add_enum_attribute(context, fn_val, "willreturn", 0);
    let mem_id = Attribute::get_named_enum_kind_id("memory");
    if mem_id != 0 {
        // memory(none)
        add_enum_attribute(context, fn_val, "memory", 0);
    } else {
        add_enum_attribute(context, fn_val, "readnone", 0);
    }
}

pub fn annotate_inline<'ctx>(
    context: &'ctx Context,
    fn_val: FunctionValue<'ctx>,
    kind: InlineKind,
) {
    match kind {
        InlineKind::Always => add_enum_attribute(context, fn_val, "alwaysinline", 0),
        InlineKind::Hint => add_enum_attribute(context, fn_val, "inlinehint", 0),
        InlineKind::Never => add_enum_attribute(context, fn_val, "noinline", 0),
    }
}

pub fn annotate_panic<'ctx>(context: &'ctx Context, fn_val: FunctionValue<'ctx>) {
    add_enum_attribute(context, fn_val, "cold", 0);
    add_enum_attribute(context, fn_val, "noreturn", 0);
}

pub fn annotate_tail_call(call_site: CallSiteValue<'_>) {
    call_site.set_tail_call_kind(inkwell::llvm_sys::LLVMTailCallKind::LLVMTailCallKindMustTail);
}

pub fn apply_ast_attributes<'ctx>(
    context: &'ctx Context,
    fn_val: FunctionValue<'ctx>,
    attrs: &[AttributeNode],
) {
    for attr in attrs {
        match attr.path_str().as_str() {
            "stable" => annotate_stable(context, fn_val),
            "inline" => {
                let kind = parse_inline_kind(&attr.args);
                annotate_inline(context, fn_val, kind);
            }
            "no_inline" | "inline_never" => annotate_inline(context, fn_val, InlineKind::Never),
            "inline_always" => annotate_inline(context, fn_val, InlineKind::Always),
            "cold" => add_enum_attribute(context, fn_val, "cold", 0),
            "noreturn" => add_enum_attribute(context, fn_val, "noreturn", 0),
            _ => {}
        }
    }
}

fn parse_inline_kind(args: &AttributeArgs) -> InlineKind {
    match args {
        AttributeArgs::None => InlineKind::Hint,
        AttributeArgs::Positional(exprs) => {
            if let Some(first) = exprs.first() { match_inline_literal(first)} 
            else { InlineKind::Hint }
        }
        AttributeArgs::Named(pairs) => {
            if let Some((_, expr)) = pairs.first() { match_inline_literal(expr)} 
            else { InlineKind::Hint }
        }
    }
}

fn match_inline_literal(expr: &Expr) -> InlineKind {
    if let ExprKind::Literal(LiteralValue::String(val)) = &expr.kind {
        match val.as_str() {
            "always" => InlineKind::Always,
            "never" => InlineKind::Never,
            _ => InlineKind::Hint,
        }
    } 

    else { InlineKind::Hint }
}
