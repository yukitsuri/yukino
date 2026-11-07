pub mod model;
pub mod wire;

pub const MACRO_PROTOCOL_VERSION: u32 = 1;

pub use model::{
    MacroDiagnostic, MacroDiagnosticSeverity, MacroKind, MacroRequest, MacroResponse, MacroSpan,
};
pub use wire::{
    MAX_PAYLOAD_BYTES, read_request, read_response, write_request, write_response,
};
