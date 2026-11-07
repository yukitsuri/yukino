#![deny(unsafe_code)]

use inkwell::targets::{InitializationConfig, Target};
use std::sync::Once;

static LLVM_INIT: Once = Once::new();

pub fn init_llvm_targets() {
    LLVM_INIT.call_once(|| {
        Target::initialize_all(&InitializationConfig::default());
    });
}
