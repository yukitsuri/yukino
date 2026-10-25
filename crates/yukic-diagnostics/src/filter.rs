use std::collections::HashSet;

use crate::code::{DiagnosticCode, WarningCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarningGroup {
    UnusedSymbol,     // unused_variable, unused_import, dead_function
    PredictableCode,  // predictable_condition, unreachable_code
    PerformanceHints, // suggest_switch, suggest_stable
    CodeClarity,      // pointless_mutability, test_ignored
}

impl WarningGroup {
    pub fn name(&self) -> &'static str {
        match self {
            Self::UnusedSymbol => "unused_symbol",
            Self::PredictableCode => "predictable_code",
            Self::PerformanceHints => "performance_hints",
            Self::CodeClarity => "code_clarity",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "unused_symbol" => Some(Self::UnusedSymbol),
            "predictable_code" => Some(Self::PredictableCode),
            "performance_hints" => Some(Self::PerformanceHints),
            "code_clarity" => Some(Self::CodeClarity),
            _ => None,
        }
    }

    pub fn for_warning(code: WarningCode) -> Option<Self> {
        match code {
            WarningCode::W0001 | WarningCode::W0002 | WarningCode::W0003 => {
                Some(Self::UnusedSymbol)
            }
            WarningCode::W0004 | WarningCode::W0412 => Some(Self::PredictableCode),
            WarningCode::W0415 | WarningCode::W0416 | WarningCode::W0505 => {
                Some(Self::PerformanceHints)
            }
            WarningCode::W0005
            | WarningCode::W0101
            | WarningCode::W0102
            | WarningCode::W0201
            | WarningCode::W0202
            | WarningCode::W0203
            | WarningCode::W0204
            | WarningCode::W0205
            | WarningCode::W0420
            | WarningCode::W0423
            | WarningCode::W0418
            | WarningCode::W0501 => Some(Self::CodeClarity),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WarningFilter {
    suppress_all: bool,
    codes: HashSet<WarningCode>,
    groups: HashSet<WarningGroup>,
    custom: HashSet<String>,
    denied_codes: HashSet<WarningCode>,
    denied_groups: HashSet<WarningGroup>,
    denied_custom: HashSet<String>,
}

impl WarningFilter {
    pub fn new() -> Self { Self::default() }

    pub fn suppress_all(&mut self) {
        self.suppress_all = true;
    }

    pub fn ignore_code(&mut self, code: WarningCode) {
        self.codes.insert(code);
    }

    pub fn ignore_group(&mut self, group: WarningGroup) {
        self.groups.insert(group);
    }

    pub fn ignore_name(&mut self, name: &str) {
        if let Some(group) = WarningGroup::from_name(name) { self.ignore_group(group); } 

        else 
        if 
        let Some(code) = WarningCode::from_name(name) { self.ignore_code(code); } 

        else { self.custom.insert(name.to_string()); }
    }

    pub fn deny_code(&mut self, code: WarningCode) {
        self.denied_codes.insert(code);
    }

    pub fn deny_group(&mut self, group: WarningGroup) {
        self.denied_groups.insert(group);
    }

    pub fn deny_name(&mut self, name: &str) {
        if let Some(group) = WarningGroup::from_name(name) {
            self.deny_group(group);
        } else if let Some(code) = WarningCode::from_name(name) {
            self.deny_code(code);
        } else {
            self.denied_custom.insert(name.to_string());
        }
    }

    pub fn is_denied(&self, code: &DiagnosticCode) -> bool {
        let DiagnosticCode::Warning(warn) = code else {
            return false;
        };

        if self.denied_codes.contains(warn) {
            return true;
        }

        if let Some(group) = WarningGroup::for_warning(*warn) && self.denied_groups.contains(&group) {
            return true;
        }

        self.denied_custom.contains(warn.name()) || self.denied_custom.contains(warn.as_str())
    }

    pub fn is_suppressed(&self, code: &DiagnosticCode) -> bool {
        let DiagnosticCode::Warning(warn) = code else {
            return false;
        };

        if self.suppress_all {
            return true;
        }

        if self.codes.contains(warn) { return true; }

        if let Some(group) = WarningGroup::for_warning(*warn) && self.groups.contains(&group)
        {
            return true;
        }

        self.custom.contains(warn.name()) || self.custom.contains(warn.as_str())
    }
}
