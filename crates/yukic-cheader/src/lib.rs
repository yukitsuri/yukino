#![deny(unsafe_code)]

mod const_eval;
pub mod preproc;
pub mod translator;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use yukic_ast::item::{Item, Visibility};
use yukic_span::Span;
use yukic_target::TargetTriple;

pub use preproc::{PreprocessConfig, PreprocessError, PreprocessOutput, preprocess_file};
pub use translator::{TranslateOutput, translate_c_source};

#[derive(Clone, Debug)]
pub struct LoadedHeader {
    pub namespace: String,
    pub items: Vec<Item>,
    pub symbols: BTreeSet<String>,
    pub tracked_files: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub enum HeaderLoadError {
    NotFound(PathBuf),
    Preprocess(PreprocessError),
    Translate(String),
}

impl std::fmt::Display for HeaderLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(p) => write!(f, "C header file not found: '{}'", p.display()),
            Self::Preprocess(e) => write!(f, "C preprocessor error in {e}"),
            Self::Translate(msg) => write!(f, "C header translation error: {msg}"),
        }
    }
}

impl std::error::Error for HeaderLoadError {}

pub fn header_namespace_for_path(header_rel: &str) -> String {
    let raw = Path::new(header_rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("c_header");
    let mut clean = String::with_capacity(raw.len());
    for (idx, ch) in raw.chars().enumerate() {
        if idx == 0 && !(ch.is_ascii_alphabetic() || ch == '_') {
            clean.push('_');
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            clean.push(ch);
        } else {
            clean.push('_');
        }
    }
    if clean.is_empty() {
        "c_header".to_string()
    } else {
        clean
    }
}

pub fn load_header_from_manifest(
    manifest_dir: &Path,
    rel_header_path: &str,
    visibility: Visibility,
    span: Span,
    cfg: &PreprocessConfig,
) -> Result<LoadedHeader, HeaderLoadError> {
    let resolved = if Path::new(rel_header_path).is_absolute() {
        PathBuf::from(rel_header_path)
    } else {
        manifest_dir.join(rel_header_path)
    };
    if !resolved.is_file() {
        return Err(HeaderLoadError::NotFound(resolved));
    }
    let pp = preprocess_file(&resolved, cfg).map_err(HeaderLoadError::Preprocess)?;
    let namespace = header_namespace_for_path(rel_header_path);
    let translated = translate_c_source(
        &pp.source,
        &pp.defines,
        rel_header_path,
        &namespace,
        cfg.target,
        visibility,
        span,
    )
    .map_err(HeaderLoadError::Translate)?;

    Ok(LoadedHeader {
        namespace,
        items: translated.items,
        symbols: translated.symbols,
        tracked_files: pp.tracked_files,
    })
}

pub fn load_header_from_str(
    header_name: &str,
    content: &str,
    visibility: Visibility,
    span: Span,
    target: TargetTriple,
) -> Result<LoadedHeader, HeaderLoadError> {
    let cfg = PreprocessConfig {
        target,
        ..PreprocessConfig::default()
    };
    let pp = preproc::preprocess_str(Path::new(header_name), content, &cfg)
        .map_err(HeaderLoadError::Preprocess)?;
    let namespace = header_namespace_for_path(header_name);
    let translated = translate_c_source(
        &pp.source,
        &pp.defines,
        header_name,
        &namespace,
        target,
        visibility,
        span,
    )
    .map_err(HeaderLoadError::Translate)?;

    Ok(LoadedHeader {
        namespace,
        items: translated.items,
        symbols: translated.symbols,
        tracked_files: pp.tracked_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::item::{ExterncItem, ItemKind};
    use yukic_ast::types::TypeNodeKind;
    use yukic_span::{FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), yukic_span::BytePos(0), yukic_span::BytePos(0))
    }

    #[test]
    fn test_preproc_and_macros_structs_unions_enums_cfuns_cvars() {
        let header = r#"
#pragma once
#include <stdint.h>
#include <stdbool.h>

#define MAGIC_NUM 42
#define MASK_VAL (1 << 3)
#define APP_NAME "yukino_c"
#define PI_APPROX 3.5

#if defined(__linux__) || defined(_WIN32)
typedef int32_t my_handle;
#endif

enum Status {
    STATUS_OK = 0,
    STATUS_ERR = 5,
    STATUS_NEXT
};

struct Vec2 {
    float x;
    float y;
};

union Packet {
    int i;
    float f;
    unsigned char bytes[4];
};

struct BitFlags {
    unsigned int a : 3;
    unsigned int b : 5;
    int value;
};

typedef int (*cmp_fn)(const void *a, const void *b);

extern int global_counter;
int add_numbers(int a, int b);
int log_fmt(const char *fmt, ...);
void ping(void);
static inline int inline_helper(int x) { return x + 1; }
"#;

        let loaded = load_header_from_str(
            "demo_api.h",
            header,
            Visibility::Public,
            dummy_span(),
            TargetTriple::X86_64UnknownLinuxGnu,
        )
        .expect("header translation should succeed");

        assert_eq!(loaded.namespace, "demo_api");
        assert!(loaded.symbols.contains("Vec2"));
        assert!(loaded.symbols.contains("Packet"));
        assert!(loaded.symbols.contains("add_numbers"));
        assert!(loaded.symbols.contains("log_fmt"));
        assert!(loaded.symbols.contains("global_counter"));
        assert!(loaded.symbols.contains("STATUS_OK"));
        assert!(loaded.symbols.contains("STATUS_NEXT"));
        assert!(loaded.symbols.contains("MAGIC_NUM"));
        assert!(loaded.symbols.contains("MASK_VAL"));
        assert!(loaded.symbols.contains("APP_NAME"));
        assert!(loaded.symbols.contains("my_handle"));
        assert!(loaded.symbols.contains("cmp_fn"));
        assert!(!loaded.symbols.contains("inline_helper"));

        let externc = loaded
            .items
            .iter()
            .find_map(|it| match &it.kind {
                ItemKind::Externc(b) => Some(b),
                _ => None,
            })
            .expect("should produce externc block");

        let has_vec2 = externc.items.iter().any(|ei| {
            matches!(ei, ExterncItem::Cstruct(cs) if cs.name.name == "Vec2" && !cs.is_union && cs.fields.len() == 2)
        });
        assert!(has_vec2);

        let has_packet = externc.items.iter().any(|ei| {
            matches!(ei, ExterncItem::Cstruct(cs) if cs.name.name == "Packet" && cs.is_union && cs.fields.len() == 3)
        });
        assert!(has_packet);

        let has_bitflags = externc.items.iter().any(|ei| {
            matches!(ei, ExterncItem::Cstruct(cs) if cs.name.name == "BitFlags" && cs.fields.len() == 2)
        });
        assert!(has_bitflags);

        let has_variadic = externc.items.iter().any(|ei| {
            matches!(ei, ExterncItem::Cfun(cf) if cf.name.name == "log_fmt" && cf.is_variadic && cf.params.len() == 1)
        });
        assert!(has_variadic);

        let has_ping = externc.items.iter().any(|ei| {
            matches!(ei, ExterncItem::Cfun(cf) if cf.name.name == "ping" && cf.params.is_empty() && cf.return_type.is_none())
        });
        assert!(has_ping);

        let has_cvar = externc.items.iter().any(|ei| {
            matches!(ei, ExterncItem::Cvar(cv) if cv.name.name == "global_counter")
        });
        assert!(has_cvar);

        let has_cmp_alias = loaded.items.iter().any(|it| match &it.kind {
            ItemKind::TypeAlias(ta) if ta.name.name == "cmp_fn" => {
                matches!(ta.target.kind, TypeNodeKind::CFun(_, _))
            }
            _ => false,
        });
        assert!(has_cmp_alias);
    }

    #[test]
    fn test_pragma_pack_extern_c_opaque_fn_macros_and_array_decay() {
        let header = r#"
#pragma pack(push, 1)
struct PackedHdr {
    uint8_t tag;
    uint32_t len;
};
#pragma pack(pop)

#ifdef __cplusplus
extern "C" {
#endif
typedef struct OpaqueHandle OpaqueHandle;

struct WithAnon {
    int kind;
    union {
        int a;
        double b;
    };
    int fixed[4];
};

#define MK_TAG(prefix, id) prefix##_##id
#define STR_OF(x) #x
#define MK_VAL 100
#if defined(MISSING_MACRO)
int bad_decl(void);
#elif MK_VAL == 100
int take_buf(int buf[4], int open_arr[]);
#else
int fallback_decl(void);
#endif
#ifdef __cplusplus
}
#endif
"#;
        let loaded = load_header_from_str(
            "adv.h",
            header,
            Visibility::Public,
            dummy_span(),
            TargetTriple::X86_64UnknownLinuxGnu,
        )
        .expect("advanced header translation should succeed");

        let externc = loaded
            .items
            .iter()
            .find_map(|it| match &it.kind {
                ItemKind::Externc(b) => Some(b),
                _ => None,
            })
            .expect("should produce externc block");

        let packed_hdr = externc
            .items
            .iter()
            .find_map(|ei| match ei {
                ExterncItem::Cstruct(cs) if cs.name.name == "PackedHdr" => Some(cs),
                _ => None,
            })
            .expect("PackedHdr missing");
        assert!(packed_hdr.c_attrs.iter().any(|a| a.name.name == "packed"));

        let opaque = externc
            .items
            .iter()
            .find_map(|ei| match ei {
                ExterncItem::Cstruct(cs) if cs.name.name == "OpaqueHandle" => Some(cs),
                _ => None,
            })
            .expect("OpaqueHandle missing");
        assert!(opaque.fields.is_empty());

        let with_anon = externc
            .items
            .iter()
            .find_map(|ei| match ei {
                ExterncItem::Cstruct(cs) if cs.name.name == "WithAnon" => Some(cs),
                _ => None,
            })
            .expect("WithAnon missing");
        assert_eq!(with_anon.fields.len(), 3);
        assert!(matches!(
            with_anon.fields[2].ty.kind,
            TypeNodeKind::Array(_, Some(4))
        ));

        let take_buf = externc
            .items
            .iter()
            .find_map(|ei| match ei {
                ExterncItem::Cfun(cf) if cf.name.name == "take_buf" => Some(cf),
                _ => None,
            })
            .expect("take_buf missing from #elif branch");
        assert!(matches!(take_buf.params[0].ty.kind, TypeNodeKind::Pointer(_)));
        assert!(matches!(take_buf.params[1].ty.kind, TypeNodeKind::Pointer(_)));
        assert!(!loaded.symbols.contains("bad_decl"));
        assert!(!loaded.symbols.contains("fallback_decl"));
    }

    #[test]
    fn test_disk_header_with_nested_include_and_tracked_files() {
        let dir = std::env::temp_dir().join("yukic_cheader_disk_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("include")).unwrap();
        std::fs::write(
            dir.join("include/sub.h"),
            "#pragma once\n#define SUB_VAL 99\nstruct Sub { int a; };\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("include/main.h"),
            "#pragma once\n#include \"sub.h\"\n#include \"sub.h\"\nint get_sub(struct Sub s);\n",
        )
        .unwrap();

        let loaded = load_header_from_manifest(
            &dir,
            "include/main.h",
            Visibility::Local,
            dummy_span(),
            &PreprocessConfig::default(),
        )
        .expect("disk header load must succeed");

        assert_eq!(loaded.namespace, "main");
        assert!(loaded.symbols.contains("SUB_VAL"));
        assert!(loaded.symbols.contains("Sub"));
        assert!(loaded.symbols.contains("get_sub"));
        assert_eq!(loaded.tracked_files.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
