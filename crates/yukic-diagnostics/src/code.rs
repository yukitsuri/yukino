use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // Lexer E01xx
    E0101, // invalid_utf8_sequence
    E0102, // unterminated_string
    E0103, // unterminated_multiline_string
    E0104, // unterminated_block_comment
    E0105, // unterminated_char_literal
    E0106, // empty_char_literal
    E0107, // multi_char_literal
    E0108, // invalid_escape_sequence
    E0109, // malformed_int_literal
    E0110, // malformed_float_literal
    E0111, // unclosed_string_interpolation
    E0112, // unrecognized_character

    // Parser E02xx
    E0201, // unexpected_token
    E0202, // unclosed_delimiter
    E0203, // invalid_macro_delimiter
    E0204, // top_level_statement_forbidden
    E0205, // non_tail_recur
    E0206, // cfun_with_generics
    E0207, // cfun_with_defaults
    E0208, // cfun_variadic_forbidden
    E0209, // cfun_inside_aggregate
    E0210, // ffi_attr_on_pure_yukino
    E0211, // cstruct_method_forbidden
    E0212, // arrow_function_block_body
    E0213, // missing_param_type
    E0214, // missing_return_type
    E0215, // conflicting_calling_conventions
    E0216, // conflicting_optimization_hints
    E0217, // invalid_packed_target
    E0218, // invalid_destructuring_pattern
    E0219, // test_function_has_parameters
    E0220, // test_function_returns_value
    E0221, // test_function_generic

    // Resolver & Macro E03xx
    E0301, // module_not_found
    E0302, // circular_module_dependency
    E0303, // private_symbol_access
    E0304, // symbol_conflict
    E0305, // undefined_symbol
    E0306, // macro_expansion_failed
    E0307, // include_file_not_found

    // Types & Sema E04xx
    E0401, // type_mismatch
    E0402, // cannot_infer_type
    E0403, // method_not_found
    E0404, // ambiguous_method_call
    E0405, // non_exhaustive_switch
    E0406, // unreachable_pattern
    E0407, // recursive_type_alias
    E0408, // interface_not_implemented
    E0409, // subclass_of_locked_class
    E0410, // missing_return
    E0416, // invalid_super_call
    E0417, // stable_calls_impure
    E0428, // invalid_operator_overload
    E0429, // test_modifier_without_test
    E0430, // invalid_type_cast
    E0431, // non_constant_cast_target

    // Borrow Checker E05xx
    E0501, // use_after_move
    E0502, // simultaneous_mut_borrow
    E0503, // mutate_while_borrowed
    E0504, // borrow_outlives_owner
    E0505, // move_in_loop
    E0508, // escaping_reference

    // Codegen & FFI E07xx
    E0701, // non_c_type_in_cfun
    E0702, // invalid_vtable_layout
    E0703, // unsupported_target_triple
    E0704, // unsupported_target_feature

    // Driver & Linker E09xx
    E0901, // linker_not_found
    E0902, // symbol_undefined_link
    E0903, // target_not_supported
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::E0101 => "E0101",
            Self::E0102 => "E0102",
            Self::E0103 => "E0103",
            Self::E0104 => "E0104",
            Self::E0105 => "E0105",
            Self::E0106 => "E0106",
            Self::E0107 => "E0107",
            Self::E0108 => "E0108",
            Self::E0109 => "E0109",
            Self::E0110 => "E0110",
            Self::E0111 => "E0111",
            Self::E0112 => "E0112",
            Self::E0201 => "E0201",
            Self::E0202 => "E0202",
            Self::E0203 => "E0203",
            Self::E0204 => "E0204",
            Self::E0205 => "E0205",
            Self::E0206 => "E0206",
            Self::E0207 => "E0207",
            Self::E0208 => "E0208",
            Self::E0209 => "E0209",
            Self::E0210 => "E0210",
            Self::E0211 => "E0211",
            Self::E0212 => "E0212",
            Self::E0213 => "E0213",
            Self::E0214 => "E0214",
            Self::E0215 => "E0215",
            Self::E0216 => "E0216",
            Self::E0217 => "E0217",
            Self::E0218 => "E0218",
            Self::E0219 => "E0219",
            Self::E0220 => "E0220",
            Self::E0221 => "E0221",
            Self::E0301 => "E0301",
            Self::E0302 => "E0302",
            Self::E0303 => "E0303",
            Self::E0304 => "E0304",
            Self::E0305 => "E0305",
            Self::E0306 => "E0306",
            Self::E0307 => "E0307",
            Self::E0401 => "E0401",
            Self::E0402 => "E0402",
            Self::E0403 => "E0403",
            Self::E0404 => "E0404",
            Self::E0405 => "E0405",
            Self::E0406 => "E0406",
            Self::E0407 => "E0407",
            Self::E0408 => "E0408",
            Self::E0409 => "E0409",
            Self::E0410 => "E0410",
            Self::E0416 => "E0416",
            Self::E0417 => "E0417",
            Self::E0428 => "E0428",
            Self::E0429 => "E0429",
            Self::E0430 => "E0430",
            Self::E0431 => "E0431",
            Self::E0501 => "E0501",
            Self::E0502 => "E0502",
            Self::E0503 => "E0503",
            Self::E0504 => "E0504",
            Self::E0505 => "E0505",
            Self::E0508 => "E0508",
            Self::E0701 => "E0701",
            Self::E0702 => "E0702",
            Self::E0703 => "E0703",
            Self::E0704 => "E0704",
            Self::E0901 => "E0901",
            Self::E0902 => "E0902",
            Self::E0903 => "E0903",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::E0101 => "invalid_utf8_sequence",
            Self::E0102 => "unterminated_string",
            Self::E0103 => "unterminated_multiline_string",
            Self::E0104 => "unterminated_block_comment",
            Self::E0105 => "unterminated_char_literal",
            Self::E0106 => "empty_char_literal",
            Self::E0107 => "multi_char_literal",
            Self::E0108 => "invalid_escape_sequence",
            Self::E0109 => "malformed_int_literal",
            Self::E0110 => "malformed_float_literal",
            Self::E0111 => "unclosed_string_interpolation",
            Self::E0112 => "unrecognized_character",
            Self::E0201 => "unexpected_token",
            Self::E0202 => "unclosed_delimiter",
            Self::E0203 => "invalid_macro_delimiter",
            Self::E0204 => "top_level_statement_forbidden",
            Self::E0205 => "non_tail_recur",
            Self::E0206 => "cfun_with_generics",
            Self::E0207 => "cfun_with_defaults",
            Self::E0208 => "cfun_variadic_forbidden",
            Self::E0209 => "cfun_inside_aggregate",
            Self::E0210 => "ffi_attr_on_pure_yukino",
            Self::E0211 => "cstruct_method_forbidden",
            Self::E0212 => "arrow_function_block_body",
            Self::E0213 => "missing_param_type",
            Self::E0214 => "missing_return_type",
            Self::E0215 => "conflicting_calling_conventions",
            Self::E0216 => "conflicting_optimization_hints",
            Self::E0217 => "invalid_packed_target",
            Self::E0218 => "invalid_destructuring_pattern",
            Self::E0219 => "test_function_has_parameters",
            Self::E0220 => "test_function_returns_value",
            Self::E0221 => "test_function_generic",
            Self::E0301 => "module_not_found",
            Self::E0302 => "circular_module_dependency",
            Self::E0303 => "private_symbol_access",
            Self::E0304 => "symbol_conflict",
            Self::E0305 => "undefined_symbol",
            Self::E0306 => "macro_expansion_failed",
            Self::E0307 => "include_file_not_found",
            Self::E0401 => "type_mismatch",
            Self::E0402 => "cannot_infer_type",
            Self::E0403 => "method_not_found",
            Self::E0404 => "ambiguous_method_call",
            Self::E0405 => "non_exhaustive_switch",
            Self::E0406 => "unreachable_pattern",
            Self::E0407 => "recursive_type_alias",
            Self::E0408 => "interface_not_implemented",
            Self::E0409 => "subclass_of_locked_class",
            Self::E0410 => "missing_return",
            Self::E0416 => "invalid_super_call",
            Self::E0417 => "stable_calls_impure",
            Self::E0428 => "invalid_operator_overload",
            Self::E0429 => "test_modifier_without_test",
            Self::E0430 => "invalid_type_cast",
            Self::E0431 => "non_constant_cast_target",
            Self::E0501 => "use_after_move",
            Self::E0502 => "simultaneous_mut_borrow",
            Self::E0503 => "mutate_while_borrowed",
            Self::E0504 => "borrow_outlives_owner",
            Self::E0505 => "move_in_loop",
            Self::E0508 => "escaping_reference",
            Self::E0701 => "non_c_type_in_cfun",
            Self::E0702 => "invalid_vtable_layout",
            Self::E0703 => "unsupported_target_triple",
            Self::E0704 => "unsupported_target_feature",
            Self::E0901 => "linker_not_found",
            Self::E0902 => "symbol_undefined_link",
            Self::E0903 => "target_not_supported",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarningCode {
    W0001, // unused_variable
    W0002, // unused_import
    W0003, // dead_function
    W0004, // unreachable_code
    W0005, // pointless_mutability
    W0101, // non_ascii_whitespace
    W0102, // confusing_unicode_identifier
    W0201, // empty_body
    W0202, // redundant_semicolon
    W0203, // empty_defer
    W0204, // empty_block
    W0205, // redundant_parentheses
    W0412, // predictable_condition
    W0415, // suggest_switch
    W0416, // suggest_stable
    W0418, // deprecated_usage
    W0420, // deprecated_item
    W0423, // test_ignored
    W0501, // unpinned_affine_discard
    W0505, // unnecessary_mut_borrow
}

impl WarningCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::W0001 => "W0001",
            Self::W0002 => "W0002",
            Self::W0003 => "W0003",
            Self::W0004 => "W0004",
            Self::W0005 => "W0005",
            Self::W0101 => "W0101",
            Self::W0102 => "W0102",
            Self::W0201 => "W0201",
            Self::W0202 => "W0202",
            Self::W0203 => "W0203",
            Self::W0204 => "W0204",
            Self::W0205 => "W0205",
            Self::W0412 => "W0412",
            Self::W0415 => "W0415",
            Self::W0416 => "W0416",
            Self::W0418 => "W0418",
            Self::W0420 => "W0420",
            Self::W0423 => "W0423",
            Self::W0501 => "W0501",
            Self::W0505 => "W0505",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::W0001 => "unused_variable",
            Self::W0002 => "unused_import",
            Self::W0003 => "dead_function",
            Self::W0004 => "unreachable_code",
            Self::W0005 => "pointless_mutability",
            Self::W0101 => "non_ascii_whitespace",
            Self::W0102 => "confusing_unicode_identifier",
            Self::W0201 => "empty_body",
            Self::W0202 => "redundant_semicolon",
            Self::W0203 => "empty_defer",
            Self::W0204 => "empty_block",
            Self::W0205 => "redundant_parentheses",
            Self::W0412 => "predictable_condition",
            Self::W0415 => "suggest_switch",
            Self::W0416 => "suggest_stable",
            Self::W0418 => "deprecated_usage",
            Self::W0420 => "deprecated_item",
            Self::W0423 => "test_ignored",
            Self::W0501 => "unpinned_affine_discard",
            Self::W0505 => "unnecessary_mut_borrow",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "unused_variable" => Some(Self::W0001),
            "unused_import" => Some(Self::W0002),
            "dead_function" => Some(Self::W0003),
            "unreachable_code" => Some(Self::W0004),
            "pointless_mutability" => Some(Self::W0005),
            "non_ascii_whitespace" => Some(Self::W0101),
            "confusing_unicode_identifier" => Some(Self::W0102),
            "empty_body" => Some(Self::W0201),
            "redundant_semicolon" => Some(Self::W0202),
            "empty_defer" => Some(Self::W0203),
            "empty_block" => Some(Self::W0204),
            "redundant_parentheses" => Some(Self::W0205),
            "predictable_condition" => Some(Self::W0412),
            "suggest_switch" => Some(Self::W0415),
            "suggest_stable" => Some(Self::W0416),
            "deprecated_usage" | "deprecated_item" | "deprecated" => Some(Self::W0418),
            "test_ignored" => Some(Self::W0423),
            "unpinned_affine_discard" => Some(Self::W0501),
            "unnecessary_mut_borrow" => Some(Self::W0505),
            _ => None,
        }
    }
}

impl fmt::Display for WarningCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanicCode {
    R0101, // integer_overflow
    R0102, // division_by_zero
    R0104, // negative_integer_exponent
    R0105, // shift_out_of_bounds
    R0201, // array_out_of_bounds
    R0202, // region_out_of_memory
    R0203, // null_dereference
    R0303, // unhandled_panic
    R0305, // invalid_downcast
    R0306, // assertion_failed
}

impl PanicCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::R0101 => "R0101",
            Self::R0102 => "R0102",
            Self::R0104 => "R0104",
            Self::R0105 => "R0105",
            Self::R0201 => "R0201",
            Self::R0202 => "R0202",
            Self::R0203 => "R0203",
            Self::R0303 => "R0303",
            Self::R0305 => "R0305",
            Self::R0306 => "R0306",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::R0101 => "integer_overflow",
            Self::R0102 => "division_by_zero",
            Self::R0104 => "negative_integer_exponent",
            Self::R0105 => "shift_out_of_bounds",
            Self::R0201 => "array_out_of_bounds",
            Self::R0202 => "region_out_of_memory",
            Self::R0203 => "null_dereference",
            Self::R0303 => "unhandled_panic",
            Self::R0305 => "invalid_downcast",
            Self::R0306 => "assertion_failed",
        }
    }
}

impl fmt::Display for PanicCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    Error(ErrorCode),
    Warning(WarningCode),
    Panic(PanicCode),
    Custom(String),
}

impl DiagnosticCode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Error(err) => err.as_str(),
            Self::Warning(warn) => warn.as_str(),
            Self::Panic(pan) => pan.as_str(),
            Self::Custom(custom) => custom.as_str(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Error(err) => err.name(),
            Self::Warning(warn) => warn.name(),
            Self::Panic(pan) => pan.name(),
            Self::Custom(_) => "custom_diagnostic",
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<ErrorCode> for DiagnosticCode { fn from(code: ErrorCode) -> Self { Self::Error(code) } }
impl From<WarningCode> for DiagnosticCode { fn from(code: WarningCode) -> Self { Self::Warning(code) } }
impl From<PanicCode> for DiagnosticCode { fn from(code: PanicCode) -> Self { Self::Panic(code) } }

impl FromStr for DiagnosticCode {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let code = match s {
            "E0101" => Self::Error(ErrorCode::E0101),
            "E0102" => Self::Error(ErrorCode::E0102),
            "E0103" => Self::Error(ErrorCode::E0103),
            "E0104" => Self::Error(ErrorCode::E0104),
            "E0105" => Self::Error(ErrorCode::E0105),
            "E0106" => Self::Error(ErrorCode::E0106),
            "E0107" => Self::Error(ErrorCode::E0107),
            "E0108" => Self::Error(ErrorCode::E0108),
            "E0109" => Self::Error(ErrorCode::E0109),
            "E0110" => Self::Error(ErrorCode::E0110),
            "E0111" => Self::Error(ErrorCode::E0111),
            "E0112" => Self::Error(ErrorCode::E0112),
            "E0201" => Self::Error(ErrorCode::E0201),
            "E0202" => Self::Error(ErrorCode::E0202),
            "E0203" => Self::Error(ErrorCode::E0203),
            "E0204" => Self::Error(ErrorCode::E0204),
            "E0205" => Self::Error(ErrorCode::E0205),
            "E0206" => Self::Error(ErrorCode::E0206),
            "E0207" => Self::Error(ErrorCode::E0207),
            "E0208" => Self::Error(ErrorCode::E0208),
            "E0209" => Self::Error(ErrorCode::E0209),
            "E0210" => Self::Error(ErrorCode::E0210),
            "E0211" => Self::Error(ErrorCode::E0211),
            "E0212" => Self::Error(ErrorCode::E0212),
            "E0213" => Self::Error(ErrorCode::E0213),
            "E0214" => Self::Error(ErrorCode::E0214),
            "E0215" => Self::Error(ErrorCode::E0215),
            "E0216" => Self::Error(ErrorCode::E0216),
            "E0217" => Self::Error(ErrorCode::E0217),
            "E0218" => Self::Error(ErrorCode::E0218),
            "E0219" => Self::Error(ErrorCode::E0219),
            "E0220" => Self::Error(ErrorCode::E0220),
            "E0221" => Self::Error(ErrorCode::E0221),
            "E0301" => Self::Error(ErrorCode::E0301),
            "E0302" => Self::Error(ErrorCode::E0302),
            "E0303" => Self::Error(ErrorCode::E0303),
            "E0304" => Self::Error(ErrorCode::E0304),
            "E0305" => Self::Error(ErrorCode::E0305),
            "E0306" => Self::Error(ErrorCode::E0306),
            "E0307" => Self::Error(ErrorCode::E0307),
            "E0401" => Self::Error(ErrorCode::E0401),
            "E0402" => Self::Error(ErrorCode::E0402),
            "E0403" => Self::Error(ErrorCode::E0403),
            "E0404" => Self::Error(ErrorCode::E0404),
            "E0405" => Self::Error(ErrorCode::E0405),
            "E0406" => Self::Error(ErrorCode::E0406),
            "E0407" => Self::Error(ErrorCode::E0407),
            "E0408" => Self::Error(ErrorCode::E0408),
            "E0409" => Self::Error(ErrorCode::E0409),
            "E0410" => Self::Error(ErrorCode::E0410),
            "E0416" => Self::Error(ErrorCode::E0416),
            "E0417" => Self::Error(ErrorCode::E0417),
            "E0429" => Self::Error(ErrorCode::E0429),
            "E0430" => Self::Error(ErrorCode::E0430),
            "E0431" => Self::Error(ErrorCode::E0431),
            "E0501" => Self::Error(ErrorCode::E0501),
            "E0502" => Self::Error(ErrorCode::E0502),
            "E0503" => Self::Error(ErrorCode::E0503),
            "E0504" => Self::Error(ErrorCode::E0504),
            "E0505" => Self::Error(ErrorCode::E0505),
            "E0508" => Self::Error(ErrorCode::E0508),
            "E0701" => Self::Error(ErrorCode::E0701),
            "E0702" => Self::Error(ErrorCode::E0702),
            "E0703" => Self::Error(ErrorCode::E0703),
            "E0704" => Self::Error(ErrorCode::E0704),
            "E0901" => Self::Error(ErrorCode::E0901),
            "E0902" => Self::Error(ErrorCode::E0902),
            "E0903" => Self::Error(ErrorCode::E0903),
            "W0001" => Self::Warning(WarningCode::W0001),
            "W0002" => Self::Warning(WarningCode::W0002),
            "W0003" => Self::Warning(WarningCode::W0003),
            "W0004" => Self::Warning(WarningCode::W0004),
            "W0005" => Self::Warning(WarningCode::W0005),
            "W0101" => Self::Warning(WarningCode::W0101),
            "W0102" => Self::Warning(WarningCode::W0102),
            "W0201" => Self::Warning(WarningCode::W0201),
            "W0202" => Self::Warning(WarningCode::W0202),
            "W0203" => Self::Warning(WarningCode::W0203),
            "W0204" => Self::Warning(WarningCode::W0204),
            "W0205" => Self::Warning(WarningCode::W0205),
            "W0412" => Self::Warning(WarningCode::W0412),
            "W0415" => Self::Warning(WarningCode::W0415),
            "W0416" => Self::Warning(WarningCode::W0416),
            "W0418" | "W0420" => Self::Warning(WarningCode::W0418),
            "W0423" => Self::Warning(WarningCode::W0423),
            "W0501" => Self::Warning(WarningCode::W0501),
            "W0505" => Self::Warning(WarningCode::W0505),
            "R0101" => Self::Panic(PanicCode::R0101),
            "R0102" => Self::Panic(PanicCode::R0102),
            "R0104" => Self::Panic(PanicCode::R0104),
            "R0105" => Self::Panic(PanicCode::R0105),
            "R0201" => Self::Panic(PanicCode::R0201),
            "R0202" => Self::Panic(PanicCode::R0202),
            "R0203" => Self::Panic(PanicCode::R0203),
            "R0303" => Self::Panic(PanicCode::R0303),
            "R0305" => Self::Panic(PanicCode::R0305),
            "R0306" => Self::Panic(PanicCode::R0306),
            other => Self::Custom(other.to_string()),
        };
        Ok(code)
    }
}
