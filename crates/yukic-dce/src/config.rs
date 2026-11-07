#![deny(unsafe_code)]

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputType {
    App,
    StaticLib,
    SharedLib,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DceConfig {
    pub strip_unused: bool,
    pub output_type: OutputType,
    pub is_test: bool,
    pub inspect_attributes: Option<PathBuf>,
    pub project_name: String,
    pub verbose: bool,
}

impl Default for DceConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl DceConfig {
    pub fn new() -> Self {
        Self {
            strip_unused: true,
            output_type: OutputType::App,
            is_test: false,
            inspect_attributes: None,
            project_name: String::from("app"),
            verbose: false,
        }
    }

    pub fn app() -> Self {
        Self {
            output_type: OutputType::App,
            ..Self::new()
        }
    }

    pub fn static_lib() -> Self {
        Self {
            output_type: OutputType::StaticLib,
            ..Self::new()
        }
    }

    pub fn shared_lib() -> Self {
        Self {
            output_type: OutputType::SharedLib,
            ..Self::new()
        }
    }

    #[must_use]
    pub fn with_strip_unused(mut self, strip_unused: bool) -> Self {
        self.strip_unused = strip_unused;
        self
    }

    #[must_use]
    pub fn with_output_type(mut self, output_type: OutputType) -> Self {
        self.output_type = output_type;
        self
    }

    #[must_use]
    pub fn with_test(mut self, is_test: bool) -> Self {
        self.is_test = is_test;
        self
    }

    #[must_use]
    pub fn with_inspect_attributes(mut self, path: Option<PathBuf>) -> Self {
        self.inspect_attributes = path;
        self
    }

    #[must_use]
    pub fn with_project_name(mut self, name: impl Into<String>) -> Self {
        self.project_name = name.into();
        self
    }

    #[must_use]
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}
