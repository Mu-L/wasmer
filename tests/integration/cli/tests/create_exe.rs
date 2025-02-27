//! Tests of the `wasmer create-exe` command.

use anyhow::{bail, Context};
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;
use wasmer_integration_tests_cli::*;

fn create_exe_wabt_path() -> String {
    format!("{}/{}", C_ASSET_PATH, "wabt-1.0.37.wasmer")
}

#[allow(dead_code)]
fn create_exe_python_wasmer() -> String {
    format!("{}/{}", C_ASSET_PATH, "python-0.1.0.wasmer")
}

fn create_exe_test_wasm_path() -> String {
    format!("{}/{}", C_ASSET_PATH, "qjs.wasm")
}
const JS_TEST_SRC_CODE: &[u8] =
    b"function greet(name) { return JSON.stringify('Hello, ' + name); }; print(greet('World'));\n";

/// Data used to run the `wasmer compile` command.
#[derive(Debug)]
struct WasmerCreateExe {
    /// The directory to operate in.
    current_dir: PathBuf,
    /// Path to wasmer executable used to run the command.
    wasmer_path: PathBuf,
    /// Path to the Wasm file to compile.
    wasm_path: PathBuf,
    /// Path to the native executable produced by compiling the Wasm.
    native_executable_path: PathBuf,
    /// Compiler with which to compile the Wasm.
    compiler: Compiler,
    /// Extra CLI flags
    extra_cli_flags: Vec<String>,
}

impl Default for WasmerCreateExe {
    fn default() -> Self {
        #[cfg(not(windows))]
        let native_executable_path = PathBuf::from("wasm.out");
        #[cfg(windows)]
        let native_executable_path = PathBuf::from("wasm.exe");
        Self {
            current_dir: std::env::current_dir().unwrap(),
            wasmer_path: get_wasmer_path(),
            wasm_path: PathBuf::from(create_exe_test_wasm_path()),
            native_executable_path,
            compiler: Compiler::Cranelift,
            extra_cli_flags: vec![],
        }
    }
}

impl WasmerCreateExe {
    fn run(&self) -> anyhow::Result<Vec<u8>> {
        let mut output = Command::new(&self.wasmer_path);
        output.current_dir(&self.current_dir);
        output.arg("create-exe");
        output.arg(&self.wasm_path.canonicalize()?);
        output.arg(&self.compiler.to_flag());
        output.args(self.extra_cli_flags.iter());
        output.arg("-o");
        output.arg(&self.native_executable_path);
        if !self.extra_cli_flags.contains(&"--target".to_string()) {
            let tarball_path = get_repo_root_path().unwrap().join("link.tar.gz");
            assert!(tarball_path.exists(), "link.tar.gz does not exist");
            output.arg("--tarball");
            output.arg(&tarball_path);
        }
        let cmd = format!("{:?}", output);

        println!("(integration-test) running create-exe: {cmd}");

        let output = output.output()?;

        let stdout = std::str::from_utf8(&output.stdout)
            .expect("stdout is not utf8! need to handle arbitrary bytes");

        assert!(
            stdout.contains("headless."),
            "create-exe stdout should link with libwasmer-headless"
        );

        if !output.status.success() {
            bail!(
                "{cmd}\r\n failed with: stdout: {stdout}\n\nstderr: {}",
                std::str::from_utf8(&output.stderr)
                    .expect("stderr is not utf8! need to handle arbitrary bytes")
            );
        }
        Ok(output.stdout)
    }
}

/// Data used to run the `wasmer compile` command.
#[derive(Debug)]
struct WasmerCreateObj {
    /// The directory to operate in.
    current_dir: PathBuf,
    /// Path to wasmer executable used to run the command.
    wasmer_path: PathBuf,
    /// Path to the Wasm file to compile.
    wasm_path: PathBuf,
    /// Path to the object file produced by compiling the Wasm.
    output_object_path: PathBuf,
    /// Compiler with which to compile the Wasm.
    compiler: Compiler,
    /// Extra CLI flags
    extra_cli_flags: Vec<String>,
}

impl Default for WasmerCreateObj {
    fn default() -> Self {
        #[cfg(not(windows))]
        let output_object_path = PathBuf::from("wasm.o");
        #[cfg(windows)]
        let output_object_path = PathBuf::from("wasm.obj");
        Self {
            current_dir: std::env::current_dir().unwrap(),
            wasmer_path: get_wasmer_path(),
            wasm_path: PathBuf::from(create_exe_test_wasm_path()),
            output_object_path,
            compiler: Compiler::Cranelift,
            extra_cli_flags: vec![],
        }
    }
}

impl WasmerCreateObj {
    fn run(&self) -> anyhow::Result<Vec<u8>> {
        let mut output = Command::new(&self.wasmer_path);
        output.current_dir(&self.current_dir);
        output.arg("create-obj");
        output.arg(&self.wasm_path.canonicalize()?);
        output.arg(&self.compiler.to_flag());
        output.args(self.extra_cli_flags.iter());
        output.arg("-o");
        output.arg(&self.output_object_path);

        let cmd = format!("{:?}", output);

        println!("(integration-test) running create-obj: {cmd}");

        let output = output.output()?;

        if !output.status.success() {
            bail!(
                "{cmd}\r\n failed with: stdout: {}\n\nstderr: {}",
                std::str::from_utf8(&output.stdout)
                    .expect("stdout is not utf8! need to handle arbitrary bytes"),
                std::str::from_utf8(&output.stderr)
                    .expect("stderr is not utf8! need to handle arbitrary bytes")
            );
        }
        Ok(output.stdout)
    }
}

#[test]
fn test_create_exe_with_pirita_works_1() {
    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path();
    let wasm_out = path.join("out.obj");
    let cmd = Command::new(get_wasmer_path())
        .arg("create-obj")
        .arg(create_exe_wabt_path())
        .arg("-o")
        .arg(&wasm_out)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&cmd.stderr);

    assert_eq!(stderr.lines().map(|s| s.trim().to_string()).collect::<Vec<_>>(), vec![
        format!("error: cannot compile more than one atom at a time"),
        format!("│   1: note: use --atom <ATOM> to specify which atom to compile"),
        format!("╰─▶ 2: where <ATOM> is one of: wabt, wasm-interp, wasm-strip, wasm-validate, wasm2wat, wast2json, wat2wasm"),
    ]);

    assert!(!cmd.status.success());

    let cmd = Command::new(get_wasmer_path())
        .arg("create-obj")
        .arg(create_exe_wabt_path())
        .arg("--atom")
        .arg("wasm2wat")
        .arg("-o")
        .arg(&wasm_out)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&cmd.stderr);

    let real_out = wasm_out.canonicalize().unwrap().display().to_string();
    let real_out = real_out
        .strip_prefix(r"\\?\")
        .unwrap_or(&real_out)
        .to_string();
    assert_eq!(
        stderr
            .lines()
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>(),
        vec![format!("✔ Object compiled successfully to `{real_out}`"),]
    );

    assert!(cmd.status.success());
}

#[test]
fn test_create_exe_with_precompiled_works_1() {
    use object::{Object, ObjectSymbol};

    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path();
    let wasm_out = path.join("out.obj");
    let _ = Command::new(get_wasmer_path())
        .arg("create-obj")
        .arg(create_exe_test_wasm_path())
        .arg("--prefix")
        .arg("sha123123")
        .arg("-o")
        .arg(&wasm_out)
        .output()
        .unwrap();

    let file = std::fs::read(&wasm_out).unwrap();
    let obj_file = object::File::parse(&*file).unwrap();
    let names = obj_file
        .symbols()
        .filter_map(|s| Some(s.name().ok()?.to_string()))
        .collect::<Vec<_>>();

    assert!(
        names.contains(&"_wasmer_function_sha123123_1".to_string())
            || names.contains(&"wasmer_function_sha123123_1".to_string())
    );

    let _ = Command::new(get_wasmer_path())
        .arg("create-obj")
        .arg(create_exe_test_wasm_path())
        .arg("-o")
        .arg(&wasm_out)
        .output()
        .unwrap();

    let file = std::fs::read(&wasm_out).unwrap();
    let obj_file = object::File::parse(&*file).unwrap();
    let names = obj_file
        .symbols()
        .filter_map(|s| Some(s.name().ok()?.to_string()))
        .collect::<Vec<_>>();

    assert!(
        names.contains(
            &"_wasmer_function_6f62a6bc5c8f8e3e12a54e2ecbc5674ccfe1c75f91d8e4dd6ebb3fec422a4d6c_1"
                .to_string()
        ) || names.contains(
            &"wasmer_function_6f62a6bc5c8f8e3e12a54e2ecbc5674ccfe1c75f91d8e4dd6ebb3fec422a4d6c_1"
                .to_string()
        )
    );
}

// Ignored because of -lunwind linker issue on Windows
// see https://github.com/wasmerio/wasmer/issues/3459
#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn create_exe_works() -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let operating_dir: PathBuf = temp_dir.path().to_owned();

    let wasm_path = operating_dir.join(create_exe_test_wasm_path());
    #[cfg(not(windows))]
    let executable_path = operating_dir.join("wasm.out");
    #[cfg(windows)]
    let executable_path = operating_dir.join("wasm.exe");

    WasmerCreateExe {
        current_dir: operating_dir.clone(),
        wasm_path,
        native_executable_path: executable_path.clone(),
        compiler: Compiler::Cranelift,
        ..Default::default()
    }
    .run()
    .context("Failed to create-exe wasm with Wasmer")?;

    let result = run_code(
        &operating_dir,
        &executable_path,
        &["--eval".to_string(), "function greet(name) { return JSON.stringify('Hello, ' + name); }; print(greet('World'));".to_string()],
        false,
    )
    .context("Failed to run generated executable")?;
    let result_lines = result.lines().collect::<Vec<&str>>();
    assert_eq!(result_lines, vec!["\"Hello, World\""],);

    Ok(())
}

/// Tests that "-c" and "-- -c" are treated differently
// Ignored because of -lunwind linker issue on Windows
// see https://github.com/wasmerio/wasmer/issues/3459
#[cfg_attr(target_os = "windows", ignore)]
// #[test]
// FIXME: Fix an re-enable test
// See https://github.com/wasmerio/wasmer/issues/3615
#[allow(dead_code)]
fn create_exe_works_multi_command_args_handling() -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let operating_dir: PathBuf = temp_dir.path().to_owned();

    let wasm_path = operating_dir.join(create_exe_wabt_path());
    #[cfg(not(windows))]
    let executable_path = operating_dir.join("multicommand.out");
    #[cfg(windows)]
    let executable_path = operating_dir.join("multicommand.exe");

    WasmerCreateExe {
        current_dir: operating_dir.clone(),
        wasm_path,
        native_executable_path: executable_path.clone(),
        compiler: Compiler::Cranelift,
        ..Default::default()
    }
    .run()
    .context("Failed to create-exe wasm with Wasmer")?;

    let result = run_code(
        &operating_dir,
        &executable_path,
        &[
            "--command".to_string(),
            "wasm-strip".to_string(),
            "--".to_string(),
            "-c".to_string(),
        ],
        true,
    )
    .context("Failed to run generated executable")?;
    let result_lines = result.lines().collect::<Vec<&str>>();
    assert_eq!(
        result_lines,
        vec![
            "wasm-strip: unknown option '-c'",
            "Try '--help' for more information.",
            "WASI exited with code: 1"
        ]
    );

    let result = run_code(
        &operating_dir,
        &executable_path,
        &["-c".to_string(), "wasm-strip".to_string()],
        true,
    )
    .context("Failed to run generated executable")?;
    let result_lines = result.lines().collect::<Vec<&str>>();
    assert_eq!(
        result_lines,
        vec![
            "wasm-strip: expected filename argument.",
            "Try '--help' for more information.",
            "WASI exited with code: 1"
        ]
    );

    Ok(())
}

/// Tests that create-exe works with underscores and dashes in command names
// Ignored because of -lunwind linker issue on Windows
// see https://github.com/wasmerio/wasmer/issues/3459
#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn create_exe_works_underscore_module_name() -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let operating_dir: PathBuf = temp_dir.path().to_owned();
    let wasm_path = operating_dir.join(create_exe_wabt_path());

    let atoms = &[
        "wabt",
        "wasm-interp",
        "wasm-strip",
        "wasm-validate",
        "wasm2wat",
        "wast2json",
        "wat2wasm",
    ];

    let mut create_exe_flags = Vec::new();

    for a in atoms.iter() {
        let object_path = operating_dir.as_path().join(&format!("{a}.o"));
        let _output: Vec<u8> = WasmerCreateObj {
            current_dir: operating_dir.clone(),
            wasm_path: wasm_path.clone(),
            output_object_path: object_path.clone(),
            compiler: Compiler::Cranelift,
            extra_cli_flags: vec!["--atom".to_string(), a.to_string()],
            ..Default::default()
        }
        .run()
        .context("Failed to create-obj wasm with Wasmer")?;

        assert!(
            object_path.exists(),
            "create-obj successfully completed but object output file `{}` missing",
            object_path.display()
        );

        create_exe_flags.push("--precompiled-atom".to_string());
        create_exe_flags.push(format!(
            "{a}:{}",
            object_path.canonicalize().unwrap().display()
        ));
    }

    #[cfg(not(windows))]
    let executable_path = operating_dir.join("multicommand.out");
    #[cfg(windows)]
    let executable_path = operating_dir.join("multicommand.exe");

    WasmerCreateExe {
        current_dir: operating_dir.clone(),
        wasm_path,
        native_executable_path: executable_path.clone(),
        compiler: Compiler::Cranelift,
        extra_cli_flags: create_exe_flags,
        ..Default::default()
    }
    .run()
    .context("Failed to create-exe wasm with Wasmer")?;

    Ok(())
}

// Ignored because of -lunwind linker issue on Windows
// see https://github.com/wasmerio/wasmer/issues/3459
#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn create_exe_works_multi_command() -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let operating_dir: PathBuf = temp_dir.path().to_owned();

    let wasm_path = operating_dir.join(create_exe_wabt_path());
    #[cfg(not(windows))]
    let executable_path = operating_dir.join("multicommand.out");
    #[cfg(windows)]
    let executable_path = operating_dir.join("multicommand.exe");

    WasmerCreateExe {
        current_dir: operating_dir.clone(),
        wasm_path,
        native_executable_path: executable_path.clone(),
        compiler: Compiler::Cranelift,
        ..Default::default()
    }
    .run()
    .context("Failed to create-exe wasm with Wasmer")?;

    let result = run_code(
        &operating_dir,
        &executable_path,
        &[
            "--command".to_string(),
            "wasm2wat".to_string(),
            "--version".to_string(),
        ],
        false,
    )
    .context("Failed to run generated executable")?;

    let result_lines = result.lines().collect::<Vec<&str>>();
    assert_eq!(result_lines, vec!["1.0.37 (git~v1.0.37)"]);

    let result = run_code(
        &operating_dir,
        &executable_path,
        &[
            "-c".to_string(),
            "wasm-validate".to_string(),
            "--version".to_string(),
        ],
        false,
    )
    .context("Failed to run generated executable")?;

    let result_lines = result.lines().collect::<Vec<&str>>();
    assert_eq!(result_lines, vec!["1.0.37 (git~v1.0.37)"]);

    Ok(())
}

// Ignored because of -lunwind linker issue on Windows
// see https://github.com/wasmerio/wasmer/issues/3459
#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn create_exe_works_with_file() -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let operating_dir: PathBuf = temp_dir.path().to_owned();

    let wasm_path = operating_dir.join(create_exe_test_wasm_path());
    #[cfg(not(windows))]
    let executable_path = operating_dir.join("wasm.out");
    #[cfg(windows)]
    let executable_path = operating_dir.join("wasm.exe");

    WasmerCreateExe {
        current_dir: operating_dir.clone(),
        wasm_path,
        native_executable_path: executable_path.clone(),
        compiler: Compiler::Cranelift,
        ..Default::default()
    }
    .run()
    .context("Failed to create-exe wasm with Wasmer")?;

    {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(operating_dir.join("test.js"))?;
        f.write_all(JS_TEST_SRC_CODE)?;
    }

    // test with `--dir`
    let result = run_code(
        &operating_dir,
        &executable_path,
        &[
            "--dir=.".to_string(),
            "--script".to_string(),
            "test.js".to_string(),
        ],
        false,
    )
    .context("Failed to run generated executable")?;
    let result_lines = result.lines().collect::<Vec<&str>>();
    assert_eq!(result_lines, vec!["\"Hello, World\""],);

    // test with `--mapdir`
    let result = run_code(
        &operating_dir,
        &executable_path,
        &[
            "--mapdir=abc:.".to_string(),
            "--script".to_string(),
            "abc/test.js".to_string(),
        ],
        false,
    )
    .context("Failed to run generated executable")?;
    let result_lines = result.lines().collect::<Vec<&str>>();
    assert_eq!(result_lines, vec!["\"Hello, World\""],);

    Ok(())
}

// Ignored because of -lunwind linker issue on Windows
// see https://github.com/wasmerio/wasmer/issues/3459
#[cfg_attr(target_os = "windows", ignore)]
// #[test]
// FIXME: Fix an re-enable test
// See https://github.com/wasmerio/wasmer/issues/3615
#[allow(dead_code)]
fn create_exe_serialized_works() -> anyhow::Result<()> {
    // let temp_dir = tempfile::tempdir()?;
    // let operating_dir: PathBuf = temp_dir.path().to_owned();
    let operating_dir = PathBuf::from("/tmp/wasmer");

    let wasm_path = operating_dir.join(create_exe_test_wasm_path());
    #[cfg(not(windows))]
    let executable_path = operating_dir.join("wasm.out");
    #[cfg(windows)]
    let executable_path = operating_dir.join("wasm.exe");

    let output: Vec<u8> = WasmerCreateExe {
        current_dir: std::env::current_dir().unwrap(),
        wasm_path,
        native_executable_path: executable_path.clone(),
        compiler: Compiler::Cranelift,
        extra_cli_flags: vec!["--object-format".to_string(), "serialized".to_string()],
        ..Default::default()
    }
    .run()
    .context("Failed to create-exe wasm with Wasmer")?;

    let result = run_code(
        &operating_dir,
        &executable_path,
        &["--eval".to_string(), "function greet(name) { return JSON.stringify('Hello, ' + name); }; print(greet('World'));".to_string()],
        false,
    )
    .context("Failed to run generated executable")?;
    let result_lines = result.lines().collect::<Vec<&str>>();
    assert_eq!(result_lines, vec!["\"Hello, World\""],);

    let output_str = String::from_utf8_lossy(&output);
    assert!(
        output_str.contains("Serialized"),
        "create-exe output doesn't mention `serialized` format keyword:\n{}",
        output_str
    );

    Ok(())
}

fn create_obj(args: Vec<String>, keyword_needle: &str, keyword: &str) -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let operating_dir: PathBuf = temp_dir.path().to_owned();

    let wasm_path = operating_dir.as_path().join(create_exe_test_wasm_path());

    let object_path = operating_dir.as_path().join("wasm");
    let output: Vec<u8> = WasmerCreateObj {
        current_dir: operating_dir,
        wasm_path,
        output_object_path: object_path.clone(),
        compiler: Compiler::Cranelift,
        extra_cli_flags: args,
        ..Default::default()
    }
    .run()
    .context("Failed to create-obj wasm with Wasmer")?;

    assert!(
        object_path.exists(),
        "create-obj successfully completed but object output file `{}` missing",
        object_path.display()
    );

    let output_str = String::from_utf8_lossy(&output);
    assert!(
        output_str.contains(keyword_needle),
        "create-obj output doesn't mention `{}` format keyword:\n{}",
        keyword,
        output_str
    );

    Ok(())
}

#[test]
fn create_obj_default() -> anyhow::Result<()> {
    create_obj(vec![], "Symbols", "symbols")
}

#[test]
fn create_obj_symbols() -> anyhow::Result<()> {
    create_obj(
        vec!["--object-format".to_string(), "symbols".to_string()],
        "Symbols",
        "symbols",
    )
}

#[test]
fn create_obj_serialized() -> anyhow::Result<()> {
    create_obj(
        vec!["--object-format".to_string(), "serialized".to_string()],
        "Serialized",
        "serialized",
    )
}

fn create_exe_with_object_input(args: Vec<String>) -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let operating_dir: PathBuf = temp_dir.path().to_owned();

    let wasm_path = operating_dir.join(create_exe_test_wasm_path());

    #[cfg(not(windows))]
    let object_path = operating_dir.join("wasm.o");
    #[cfg(windows)]
    let object_path = operating_dir.join("wasm.obj");

    let mut create_obj_args = args.clone();
    create_obj_args.push("--prefix".to_string());
    create_obj_args.push("abc123".to_string());
    create_obj_args.push("--debug-dir".to_string());
    create_obj_args.push(format!(
        "{}",
        operating_dir.join("compile-create-obj").display()
    ));

    WasmerCreateObj {
        current_dir: operating_dir.clone(),
        wasm_path: wasm_path.clone(),
        output_object_path: object_path.clone(),
        compiler: Compiler::Cranelift,
        extra_cli_flags: create_obj_args,
        ..Default::default()
    }
    .run()
    .context("Failed to create-obj wasm with Wasmer")?;

    assert!(
        object_path.exists(),
        "create-obj successfully completed but object output file `{}` missing",
        object_path.display()
    );

    #[cfg(not(windows))]
    let executable_path = operating_dir.join("wasm.out");
    #[cfg(windows)]
    let executable_path = operating_dir.join("wasm.exe");

    let mut create_exe_args = args.clone();
    create_exe_args.push("--precompiled-atom".to_string());
    create_exe_args.push(format!("qjs:abc123:{}", object_path.display()));
    create_exe_args.push("--debug-dir".to_string());
    create_exe_args.push(format!(
        "{}",
        operating_dir.join("compile-create-exe").display()
    ));

    let create_exe_stdout = WasmerCreateExe {
        current_dir: std::env::current_dir().unwrap(),
        wasm_path,
        native_executable_path: executable_path.clone(),
        compiler: Compiler::Cranelift,
        extra_cli_flags: create_exe_args,
        ..Default::default()
    }
    .run()
    .context("Failed to create-exe wasm with Wasmer")?;

    let create_exe_stdout = std::str::from_utf8(&create_exe_stdout).unwrap();
    assert!(
        create_exe_stdout.contains("Using cached object file for atom \"qjs\"."),
        "missed cache hit"
    );

    let result = run_code(
        &operating_dir,
        &executable_path,
        &["--eval".to_string(), "function greet(name) { return JSON.stringify('Hello, ' + name); }; print(greet('World'));".to_string()],
        false,
    )
    .context("Failed to run generated executable")?;
    let result_lines = result.lines().collect::<Vec<&str>>();
    assert_eq!(result_lines, vec!["\"Hello, World\""],);

    Ok(())
}

// Ignored because of -lunwind linker issue on Windows
// see https://github.com/wasmerio/wasmer/issues/3459
#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn create_exe_with_object_input_default() -> anyhow::Result<()> {
    create_exe_with_object_input(vec![])
}

// Ignored because of -lunwind linker issue on Windows
// see https://github.com/wasmerio/wasmer/issues/3459
#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn create_exe_with_object_input_symbols() -> anyhow::Result<()> {
    create_exe_with_object_input(vec!["--object-format".to_string(), "symbols".to_string()])
}

// Ignored because of -lunwind linker issue on Windows
// see https://github.com/wasmerio/wasmer/issues/3459
#[cfg_attr(target_os = "windows", ignore)]
// #[test]
// FIXME: Fix an re-enable test
// See https://github.com/wasmerio/wasmer/issues/3615
#[allow(dead_code)]
fn create_exe_with_object_input_serialized() -> anyhow::Result<()> {
    create_exe_with_object_input(vec![
        "--object-format".to_string(),
        "serialized".to_string(),
    ])
}
