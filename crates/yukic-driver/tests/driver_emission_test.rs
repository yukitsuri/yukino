#![deny(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use inkwell::context::Context;
use inkwell::targets::RelocMode;
use yukic_driver::{
    DriverConfig, DriverError, EmitProgressEvent, OutputType, build_shared_linker_args,
    create_static_archive, create_target_machine_for_driver, emit_asm_file, emit_cgus_parallel,
    emit_object_file, format_emit_progress, link_executable, link_shared_library,
    list_archive_members,
};
use yukic_opt::{OptLevel, OptimizedCgu};
use yukic_target::TargetTriple;

fn create_test_module<'ctx>(context: &'ctx Context, name: &str) -> inkwell::module::Module<'ctx> {
    let module = context.create_module(name);
    let i32_type = context.i32_type();
    let fn_type = i32_type.fn_type(&[i32_type.into(), i32_type.into()], false);
    let fn_val = module.add_function("test_add", fn_type, None);

    let entry = context.append_basic_block(fn_val, "entry");
    let builder = context.create_builder();
    builder.position_at_end(entry);

    let p0 = fn_val.get_first_param().unwrap().into_int_value();
    let p1 = fn_val.get_params()[1].into_int_value();
    let sum = builder.build_int_add(p0, p1, "sum").unwrap();
    builder.build_return(Some(&sum)).unwrap();

    module
}

fn get_test_dir(sub: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("yukic_driver_emission_test")
        .join(sub);
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn test_emit_object_and_asm_files() {
    let context = Context::create();
    let module = create_test_module(&context, "test_emit_mod");
    let target = TargetTriple::host();
    let machine = create_target_machine_for_driver(target, OptLevel::O2, Some(RelocMode::PIC))
        .expect("create target machine");

    let out_dir = get_test_dir("single");
    let obj_path = out_dir.join("test_emit.o");
    let asm_path = out_dir.join("test_emit.s");

    emit_object_file(&module, &machine, &obj_path).expect("emit object file");
    assert!(obj_path.exists());
    let obj_bytes = fs::read(&obj_path).expect("read obj file");
    assert!(!obj_bytes.is_empty());
    // verify ELF header magic on Linux
    if cfg!(target_os = "linux") {
        assert_eq!(&obj_bytes[0..4], b"\x7fELF");
    }

    emit_asm_file(&module, &machine, &asm_path).expect("emit asm file");
    assert!(asm_path.exists());
    let asm_text = fs::read_to_string(&asm_path).expect("read asm file");
    assert!(asm_text.contains("test_add"));
}

#[test]
fn test_emit_cgus_parallel() {
    let out_dir = get_test_dir("parallel_cgu");
    let cgu0 = OptimizedCgu {
        name: "cgu_alpha".to_string(),
        ll_ir: r#"
target triple = "x86_64-unknown-linux-gnu"
define i32 @alpha_fn() {
    ret i32 10
}
"#
        .to_string(),
        bitcode: None,
        item_count: 1,
        estimated_weight: 10,
    };

    let cgu1 = OptimizedCgu {
        name: "cgu_beta".to_string(),
        ll_ir: r#"
target triple = "x86_64-unknown-linux-gnu"
define i32 @beta_fn() {
    ret i32 20
}
"#
        .to_string(),
        bitcode: None,
        item_count: 1,
        estimated_weight: 20,
    };

    let cgus = vec![cgu0, cgu1];
    let events_count = AtomicUsize::new(0);

    let obj_paths = emit_cgus_parallel(
        &cgus,
        TargetTriple::X86_64UnknownLinuxGnu,
        OptLevel::O1,
        RelocMode::PIC,
        &out_dir,
        2,
        None,
        None,
        None,
        None,
        |thread_id, event| {
            events_count.fetch_add(1, Ordering::SeqCst);
            let formatted = format_emit_progress(thread_id, &event);
            assert!(formatted.starts_with("thread :: "));
        },
    )
    .expect("emit cgus parallel");

    assert_eq!(obj_paths.len(), 2);
    for path in &obj_paths {
        assert!(path.exists());
        let meta = fs::metadata(path).expect("metadata");
        assert!(meta.len() > 0);
    }
    assert!(events_count.load(Ordering::SeqCst) > 0);
}

#[test]
fn test_create_static_archive_and_list_members() {
    let out_dir = get_test_dir("archive");
    let context = Context::create();
    let module = create_test_module(&context, "archive_mod");
    let target = TargetTriple::host();
    let machine = create_target_machine_for_driver(target, OptLevel::O0, Some(RelocMode::PIC))
        .expect("machine");

    let obj1 = out_dir.join("part1.o");
    let obj2 = out_dir.join("part2.o");
    emit_object_file(&module, &machine, &obj1).expect("emit part1");
    emit_object_file(&module, &machine, &obj2).expect("emit part2");

    let archive_path = out_dir.join("libstatic_test.a");
    create_static_archive(&archive_path, &[obj1, obj2], None).expect("create static archive");

    assert!(archive_path.exists());
    let bytes = fs::read(&archive_path).expect("read archive");
    assert!(bytes.starts_with(b"!<arch>\n"));

    let members = list_archive_members(&archive_path, None).expect("list members");
    assert!(members.contains(&"part1.o".to_string()));
    assert!(members.contains(&"part2.o".to_string()));
}

#[test]
fn test_link_shared_library() {
    let out_dir = get_test_dir("shared");
    let context = Context::create();
    let module = create_test_module(&context, "shared_mod");
    let target = TargetTriple::host();
    let machine = create_target_machine_for_driver(target, OptLevel::O2, Some(RelocMode::PIC))
        .expect("machine");

    let obj_path = out_dir.join("shared_comp.o");
    emit_object_file(&module, &machine, &obj_path).expect("emit obj");

    let config = DriverConfig::new("test.ykn".into()).with_output_type(OutputType::Shared);
    let shared_path = out_dir.join("libshared_test.so");
    link_shared_library(&shared_path, &[obj_path], target, &config).expect("link shared lib");

    assert!(shared_path.exists());
    let so_bytes = fs::read(&shared_path).expect("read so");
    assert!(!so_bytes.is_empty());
    if cfg!(target_os = "linux") {
        assert_eq!(&so_bytes[0..4], b"\x7fELF");
    }
}

#[test]
fn test_build_shared_linker_args_targets() {
    let out_dir = PathBuf::from("build/test");
    let objects = vec![
        PathBuf::from("build/cgu_0.o"),
        PathBuf::from("build/cgu_1.o"),
    ];
    let config = DriverConfig::new("src/lib.ykn".into())
        .with_output_type(OutputType::Shared)
        .with_codegen_units(2);

    // 1. Linux x86_64 glibc
    let args_linux = build_shared_linker_args(
        &out_dir.join("libtest.so"),
        &objects,
        TargetTriple::X86_64UnknownLinuxGnu,
        &config,
    );
    assert!(args_linux.contains(&"-shared".to_string()));
    assert!(args_linux.contains(&"--gc-sections".to_string()));
    assert!(args_linux.contains(&"relro".to_string()));
    assert!(args_linux.contains(&"now".to_string()));

    // 2. Linux AArch64 glibc
    let args_aarch64 = build_shared_linker_args(
        &out_dir.join("libtest.so"),
        &objects,
        TargetTriple::Aarch64UnknownLinuxGnu,
        &config,
    );
    assert!(args_aarch64.contains(&"-m".to_string()));
    assert!(args_aarch64.contains(&"aarch64linux".to_string()));

    // 3. Windows MinGW
    let args_win = build_shared_linker_args(
        &out_dir.join("test.dll"),
        &objects,
        TargetTriple::X86_64PcWindowsGnu,
        &config,
    );
    assert!(args_win.contains(&"-m".to_string()));
    assert!(args_win.contains(&"i386pep".to_string()));
    assert!(args_win.contains(&"-shared".to_string()));

    // 5. Android AArch64
    let args_android = build_shared_linker_args(
        &out_dir.join("libtest.so"),
        &objects,
        TargetTriple::Aarch64LinuxAndroid,
        &config,
    );
    assert!(args_android.contains(&"-m".to_string()));
    assert!(args_android.contains(&"aarch64linux".to_string()));
}

#[test]
fn test_error_handling_missing_inputs_and_tools() {
    let out_path = PathBuf::from("nonexistent/lib.a");

    let err_arch = create_static_archive(&out_path, &[], None).unwrap_err();
    assert!(matches!(err_arch, DriverError::NoInputFile));

    let err_shared = link_shared_library(
        &out_path,
        &[],
        TargetTriple::host(),
        &DriverConfig::new("test.ykn".into()),
    )
    .unwrap_err();
    assert!(matches!(err_shared, DriverError::NoInputFile));

    let err_tool = create_static_archive(
        &out_path,
        &[PathBuf::from("dummy.o")],
        Some("nonexistent-tool-binary-xyz"),
    )
    .unwrap_err();
    assert!(matches!(err_tool, DriverError::ToolNotFound(_)));
}

#[test]
fn test_run_compiler_driver_pipeline() {
    let out_dir = get_test_dir("pipeline");

    // 1. Test .ll -> .o
    let ll_path = out_dir.join("input.ll");
    fs::write(
        &ll_path,
        r#"
target triple = "x86_64-unknown-linux-gnu"
define i32 @fn_test_pipeline(i32 %x) {
    %res = add i32 %x, 1
    ret i32 %res
}
"#,
    )
    .expect("write ll");

    let obj_out = out_dir.join("output_obj.o");
    let mut config_obj = DriverConfig::new(ll_path.to_string_lossy().to_string())
        .with_emit(yukic_driver::EmitKind::Obj);
    config_obj.output_file = Some(obj_out.to_string_lossy().to_string());

    let res_obj = yukic_driver::run_compiler_driver(&config_obj).expect("compile to obj");
    assert!(res_obj.exists());
    assert!(fs::metadata(&res_obj).expect("meta").len() > 0);

    // 2. Test .ll -> .a (static archive via llvm-ar)
    let static_out = out_dir.join("libpipeline.a");
    let mut config_static = DriverConfig::new(ll_path.to_string_lossy().to_string())
        .with_output_type(OutputType::Static);
    config_static.output_file = Some(static_out.to_string_lossy().to_string());

    let res_static =
        yukic_driver::run_compiler_driver(&config_static).expect("compile to static archive");
    assert!(res_static.exists());
    let static_bytes = fs::read(&res_static).expect("read a");
    assert!(static_bytes.starts_with(b"!<arch>\n"));

    // 3. Test .ll -> .so (shared library via LLD)
    let shared_out = out_dir.join("libpipeline.so");
    let mut config_shared = DriverConfig::new(ll_path.to_string_lossy().to_string())
        .with_output_type(OutputType::Shared);
    config_shared.output_file = Some(shared_out.to_string_lossy().to_string());

    let res_shared =
        yukic_driver::run_compiler_driver(&config_shared).expect("compile to shared lib");
    assert!(res_shared.exists());
    let so_bytes = fs::read(&res_shared).expect("read so");
    assert!(!so_bytes.is_empty());
    if cfg!(target_os = "linux") {
        assert_eq!(&so_bytes[0..4], b"\x7fELF");
    }

    // 4. Test .ykn -> .a
    let ykn_path = out_dir.join("math.ykn");
    fs::write(&ykn_path, "fun calc(x: int32) -> int32 { space }").expect("write ykn");

    let ykn_static_out = out_dir.join("libmath.a");
    let mut config_ykn = DriverConfig::new(ykn_path.to_string_lossy().to_string())
        .with_output_type(OutputType::Static);
    config_ykn.output_file = Some(ykn_static_out.to_string_lossy().to_string());

    let res_ykn =
        yukic_driver::run_compiler_driver(&config_ykn).expect("compile ykn to static archive");
    assert!(res_ykn.exists());
    let ykn_bytes = fs::read(&res_ykn).expect("read ykn a");
    assert!(ykn_bytes.starts_with(b"!<arch>\n"));
}

#[test]
fn test_link_executable_and_batch_cgus() {
    let out_dir = get_test_dir("app");
    let ll_main = out_dir.join("main.ll");
    let ll_helper = out_dir.join("helper.ll");

    fs::write(
        &ll_helper,
        "target triple = \"x86_64-unknown-linux-gnu\"\ndefine i32 @calc_number() {\n    ret i32 0\n}\n",
    )
    .expect("write helper.ll");

    fs::write(
        &ll_main,
        "target triple = \"x86_64-unknown-linux-gnu\"\ndeclare i32 @calc_number()\ndefine i32 @main() {\n    %res = call i32 @calc_number()\n    ret i32 %res\n}\n",
    )
    .expect("write main.ll");

    let context = Context::create();
    let target = TargetTriple::host();
    let machine = create_target_machine_for_driver(target, OptLevel::O0, Some(RelocMode::PIC))
        .expect("machine");

    let obj_helper = out_dir.join("helper.o");
    let obj_main = out_dir.join("main.o");

    let buf_h = inkwell::memory_buffer::MemoryBuffer::create_from_memory_range_copy(
        fs::read(&ll_helper).unwrap().as_slice(),
        "helper",
    );
    let mod_h = context.create_module_from_ir(buf_h).unwrap();
    emit_object_file(&mod_h, &machine, &obj_helper).unwrap();

    let buf_m = inkwell::memory_buffer::MemoryBuffer::create_from_memory_range_copy(
        fs::read(&ll_main).unwrap().as_slice(),
        "main",
    );
    let mod_m = context.create_module_from_ir(buf_m).unwrap();
    emit_object_file(&mod_m, &machine, &obj_main).unwrap();

    let app_out = out_dir.join("test_app_bin");
    let config = DriverConfig::new(ll_main.to_string_lossy().to_string());

    link_executable(&app_out, &[obj_main, obj_helper], target, &config)
        .expect("batch link executable");
    assert!(app_out.exists());

    if cfg!(target_os = "linux") {
        let status = std::process::Command::new(&app_out)
            .status()
            .expect("run linked app");
        assert!(status.success());
    }
}

#[test]
fn test_cross_target_object_emission_all_9_triples() {
    let out_dir = get_test_dir("cross_obj");
    let context = Context::create();
    let module = create_test_module(&context, "cross_test_mod");

    for target in TargetTriple::all() {
        let machine = create_target_machine_for_driver(*target, OptLevel::O0, Some(RelocMode::PIC))
            .unwrap_or_else(|e| panic!("failed creating target machine for {target}: {e}"));
        let obj_path = out_dir.join(format!("obj_{target}.o"));
        emit_object_file(&module, &machine, &obj_path)
            .unwrap_or_else(|e| panic!("failed emitting object for {target}: {e}"));

        assert!(obj_path.exists());
        let bytes = fs::read(&obj_path).expect("read object");
        assert!(bytes.len() > 64);

        let spec = target.spec();
        match spec.object_format {
            yukic_target::spec::ObjectFormat::Elf => {
                assert_eq!(&bytes[0..4], b"\x7fELF", "ELF magic mismatch for {target}");
                let bitness = bytes[4];
                if spec.pointer_width == 64 {
                    assert_eq!(bitness, 2, "64-bit ELF mismatch for {target}");
                } else {
                    assert_eq!(bitness, 1, "32-bit ELF mismatch for {target}");
                }
            }
            yukic_target::spec::ObjectFormat::Coff => {
                assert_eq!(&bytes[0..2], b"\x64\x86", "COFF magic mismatch for {target}");
            }
        }
    }
}

#[test]
fn test_cross_link_windows_mingw_executable() {
    let out_dir = get_test_dir("windows_exe");
    let target = TargetTriple::X86_64PcWindowsGnu;
    let context = Context::create();
    let module = create_test_module(&context, "win_main_mod");
    let machine = create_target_machine_for_driver(target, OptLevel::O0, Some(RelocMode::Default))
        .expect("win machine");

    let obj_path = out_dir.join("win_main.o");
    emit_object_file(&module, &machine, &obj_path).expect("emit win obj");

    let exe_out = out_dir.join("win_app.exe");
    let config = DriverConfig::new("win_main.ykn".into());

    let mingw_installed = PathBuf::from("/usr/x86_64-w64-mingw32/lib").exists();
    let win_rt_installed =
        PathBuf::from("target/x86_64-pc-windows-gnu/debug/libyukino_rt.a").exists();

    if mingw_installed && win_rt_installed {
        link_executable(&exe_out, &[obj_path], target, &config)
            .expect("link windows executable");
        assert!(exe_out.exists());
        let exe_bytes = fs::read(&exe_out).expect("read exe");
        assert_eq!(&exe_bytes[0..2], b"MZ");
    }
}

#[test]
fn test_artifact_strings_contain_yukic_version() {
    let out_dir = get_test_dir("ident_version");
    let src_path = out_dir.join("version_test.ykn");
    fs::write(&src_path, "fun main() {}\n").expect("write ykn");

    let version_needle = format!("yukic v{}", env!("CARGO_PKG_VERSION"));

    // `.ident` reaches object/asm output only on ELF targets; COFF skips it
    let host_is_elf =
        TargetTriple::host().spec().object_format == yukic_target::spec::ObjectFormat::Elf;

    let obj_out = out_dir.join("version_test.o");
    let mut config_obj = DriverConfig::new(src_path.to_str().unwrap().to_string());
    config_obj.emit = yukic_driver::EmitKind::Obj;
    config_obj.output_file = Some(obj_out.to_str().unwrap().to_string());
    let res = yukic_driver::run_compiler_driver(&config_obj).expect("compile obj");
    assert!(res.exists());
    if host_is_elf {
        let obj_bytes = fs::read(&res).expect("read obj");
        let obj_str = String::from_utf8_lossy(&obj_bytes);
        assert!(
            obj_str.contains(&version_needle),
            "Expected object file to contain '{version_needle}'"
        );
    }

    let asm_out = out_dir.join("version_test.s");
    let mut config_asm = DriverConfig::new(src_path.to_str().unwrap().to_string());
    config_asm.emit = yukic_driver::EmitKind::Asm;
    config_asm.output_file = Some(asm_out.to_str().unwrap().to_string());
    let res_asm = yukic_driver::run_compiler_driver(&config_asm).expect("compile asm");
    assert!(res_asm.exists());
    if host_is_elf {
        let asm_content = fs::read_to_string(&res_asm).expect("read asm");
        assert!(
            asm_content.contains(&version_needle),
            "Expected asm file to contain '{version_needle}'"
        );
    }

    let ll_out = out_dir.join("version_test.ll");
    let mut config_ll = DriverConfig::new(src_path.to_str().unwrap().to_string());
    config_ll.emit = yukic_driver::EmitKind::LlvmIr;
    config_ll.output_file = Some(ll_out.to_str().unwrap().to_string());
    let res_ll = yukic_driver::run_compiler_driver(&config_ll).expect("compile ll");
    assert!(res_ll.exists());
    let ll_content = fs::read_to_string(&res_ll).expect("read ll");
    assert!(
        ll_content.contains(&version_needle),
        "Expected LLVM IR to contain '{version_needle}'"
    );

    let a_out = out_dir.join("libversion_test.a");
    let mut config_a = DriverConfig::new(src_path.to_str().unwrap().to_string());
    config_a.output_type = yukic_driver::OutputType::Static;
    config_a.output_file = Some(a_out.to_str().unwrap().to_string());
    let res_a = yukic_driver::run_compiler_driver(&config_a).expect("compile static archive");
    assert!(res_a.exists());
    if host_is_elf {
        let a_bytes = fs::read(&res_a).expect("read static archive");
        let a_str = String::from_utf8_lossy(&a_bytes);
        assert!(
            a_str.contains(&version_needle),
            "Expected static archive to contain '{version_needle}'"
        );
    }

    let so_out = out_dir.join("libversion_test.so");
    let mut config_so = DriverConfig::new(src_path.to_str().unwrap().to_string());
    config_so.output_type = yukic_driver::OutputType::Shared;
    config_so.output_file = Some(so_out.to_str().unwrap().to_string());
    let res_so = yukic_driver::run_compiler_driver(&config_so).expect("compile shared lib");
    assert!(res_so.exists());
    if host_is_elf {
        let so_bytes = fs::read(&res_so).expect("read shared lib");
        let so_str = String::from_utf8_lossy(&so_bytes);
        assert!(
            so_str.contains(&version_needle),
            "Expected shared library to contain '{version_needle}'"
        );
    }

    let app_out = out_dir.join("version_test_app");
    let mut config_app = DriverConfig::new(src_path.to_str().unwrap().to_string());
    config_app.output_type = yukic_driver::OutputType::App;
    config_app.output_file = Some(app_out.to_str().unwrap().to_string());
    let res_app = yukic_driver::run_compiler_driver(&config_app).expect("compile app");
    assert!(res_app.exists());
    if host_is_elf {
        let app_bytes = fs::read(&res_app).expect("read app");
        let app_str = String::from_utf8_lossy(&app_bytes);
        assert!(
            app_str.contains(&version_needle),
            "Expected executable to contain '{version_needle}'"
        );
    }
}

#[test]
fn test_emit_cgus_parallel_with_caching() {
    let out_dir = std::env::temp_dir().join(format!("cgu_cache_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).expect("out_dir");

    let context = Context::create();
    let mod0 = create_test_module(&context, "cgu_0");
    let mod1 = create_test_module(&context, "cgu_1");

    let cgu0 = OptimizedCgu {
        name: "cgu_0".to_string(),
        ll_ir: mod0.print_to_string().to_string(),
        bitcode: None,
        item_count: 1,
        estimated_weight: 10,
    };
    let cgu1 = OptimizedCgu {
        name: "cgu_1".to_string(),
        ll_ir: mod1.print_to_string().to_string(),
        bitcode: None,
        item_count: 1,
        estimated_weight: 20,
    };

    let cgus = vec![cgu0.clone(), cgu1.clone()];
    let cached_events = AtomicUsize::new(0);
    let emit_events = AtomicUsize::new(0);

    // 1st run: fresh compilation
    let paths1 = emit_cgus_parallel(
        &cgus,
        TargetTriple::host(),
        OptLevel::O0,
        RelocMode::PIC,
        &out_dir,
        2,
        Some("debug"),
        None,
        None,
        None,
        |_thread_id, event| match event {
            EmitProgressEvent::Cached { .. } => {
                cached_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Emitting { .. } => {
                emit_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Idle => {}
        },
    )
    .expect("first emission");

    assert_eq!(paths1.len(), 2);
    assert_eq!(cached_events.load(Ordering::SeqCst), 0);
    assert_eq!(emit_events.load(Ordering::SeqCst), 2);
    assert!(out_dir.join("cgu_manifest.json").is_file());

    // 2nd run: identical CGUs must both be cached
    cached_events.store(0, Ordering::SeqCst);
    emit_events.store(0, Ordering::SeqCst);

    let paths2 = emit_cgus_parallel(
        &cgus,
        TargetTriple::host(),
        OptLevel::O0,
        RelocMode::PIC,
        &out_dir,
        2,
        Some("debug"),
        None,
        None,
        None,
        |_thread_id, event| match event {
            EmitProgressEvent::Cached { .. } => {
                cached_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Emitting { .. } => {
                emit_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Idle => {}
        },
    )
    .expect("second emission");

    assert_eq!(paths2.len(), 2);
    assert_eq!(cached_events.load(Ordering::SeqCst), 2);
    assert_eq!(emit_events.load(Ordering::SeqCst), 0);

    // 3rd run: modify cgu1 -> cgu0 remains cached, cgu1 re-emitted
    let mut cgu1_modified = cgu1.clone();
    cgu1_modified.ll_ir.push_str("\n; modified comment");
    let cgus_partial = vec![cgu0.clone(), cgu1_modified];

    cached_events.store(0, Ordering::SeqCst);
    emit_events.store(0, Ordering::SeqCst);

    let paths3 = emit_cgus_parallel(
        &cgus_partial,
        TargetTriple::host(),
        OptLevel::O0,
        RelocMode::PIC,
        &out_dir,
        2,
        Some("debug"),
        None,
        None,
        None,
        |_thread_id, event| match event {
            EmitProgressEvent::Cached { .. } => {
                cached_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Emitting { .. } => {
                emit_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Idle => {}
        },
    )
    .expect("third emission");

    assert_eq!(paths3.len(), 2);
    assert_eq!(cached_events.load(Ordering::SeqCst), 1);
    assert_eq!(emit_events.load(Ordering::SeqCst), 1);

    // 4th run: extra dependency hash (tracked file / macro runner) changes -> re-emitted
    cached_events.store(0, Ordering::SeqCst);
    emit_events.store(0, Ordering::SeqCst);
    let extra_hashes = vec![
        vec!["blake3:dep_hash_v1".to_string()],
        vec!["blake3:dep_hash_v1".to_string()],
    ];
    let paths4 = emit_cgus_parallel(
        &cgus,
        TargetTriple::host(),
        OptLevel::O0,
        RelocMode::PIC,
        &out_dir,
        2,
        Some("debug"),
        None,
        None,
        Some(&extra_hashes),
        |_thread_id, event| match event {
            EmitProgressEvent::Cached { .. } => {
                cached_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Emitting { .. } => {
                emit_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Idle => {}
        },
    )
    .expect("fourth emission");

    assert_eq!(paths4.len(), 2);
    assert_eq!(emit_events.load(Ordering::SeqCst), 2);
    assert_eq!(cached_events.load(Ordering::SeqCst), 0);

    // 5th run: identical extra dependency -> both cached!
    cached_events.store(0, Ordering::SeqCst);
    emit_events.store(0, Ordering::SeqCst);
    let paths5 = emit_cgus_parallel(
        &cgus,
        TargetTriple::host(),
        OptLevel::O0,
        RelocMode::PIC,
        &out_dir,
        2,
        Some("debug"),
        None,
        None,
        Some(&extra_hashes),
        |_thread_id, event| match event {
            EmitProgressEvent::Cached { .. } => {
                cached_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Emitting { .. } => {
                emit_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Idle => {}
        },
    )
    .expect("fifth emission");

    assert_eq!(paths5.len(), 2);
    assert_eq!(cached_events.load(Ordering::SeqCst), 2);
    assert_eq!(emit_events.load(Ordering::SeqCst), 0);

    // 6th run: cgu0 has changed extra hash (e.g. tracked file changed) -> cgu0 re-emitted, cgu1 cached!
    cached_events.store(0, Ordering::SeqCst);
    emit_events.store(0, Ordering::SeqCst);
    let extra_hashes_modified = vec![
        vec!["blake3:dep_hash_v2_changed".to_string()],
        vec!["blake3:dep_hash_v1".to_string()],
    ];
    let paths6 = emit_cgus_parallel(
        &cgus,
        TargetTriple::host(),
        OptLevel::O0,
        RelocMode::PIC,
        &out_dir,
        2,
        Some("debug"),
        None,
        None,
        Some(&extra_hashes_modified),
        |_thread_id, event| match event {
            EmitProgressEvent::Cached { .. } => {
                cached_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Emitting { .. } => {
                emit_events.fetch_add(1, Ordering::SeqCst);
            }
            EmitProgressEvent::Idle => {}
        },
    )
    .expect("sixth emission");

    assert_eq!(paths6.len(), 2);
    assert_eq!(emit_events.load(Ordering::SeqCst), 1);
    assert_eq!(cached_events.load(Ordering::SeqCst), 1);

    assert_eq!(paths3.len(), 2);
    assert_eq!(cached_events.load(Ordering::SeqCst), 1);
    assert_eq!(emit_events.load(Ordering::SeqCst), 1);

    let _ = fs::remove_dir_all(&out_dir);
}

