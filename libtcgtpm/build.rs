// SPDX-License-Identifier: MIT
//
// Copyright (C) 2024 IBM
// Copyright (c) 2026 Red Hat, Inc.
//
// Author: Claudio Carvalho <cclaudio@linux.ibm.com>

use std::path::{Path, PathBuf};

fn glob_c_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "c") {
                files.push(path);
            }
        }
    }
    files
}

fn glob_c_files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(glob_c_files_recursive(&path));
            } else if path.extension().is_some_and(|e| e == "c") {
                files.push(path);
            }
        }
    }
    files
}

fn generate_tpm_library_h(libtpms_dir: &Path) {
    let template = libtpms_dir.join("include/libtpms/tpm_library.h.in");
    let output = libtpms_dir.join("include/libtpms/tpm_library.h");
    if output.exists() {
        return;
    }
    let contents = std::fs::read_to_string(&template)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", template.display()));
    let contents = contents
        .replace("@LIBTPMS_VER_MAJOR@", "0")
        .replace("@LIBTPMS_VER_MINOR@", "11")
        .replace("@LIBTPMS_VER_MICRO@", "0");
    std::fs::write(&output, contents)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", output.display()));
}

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    let ossl_include_dir =
        std::env::var("DEP_OSSL_OSSL_INCLUDE_DIR").expect("DEP_OSSL_OSSL_INCLUDE_DIR not set");
    let ossl_src_include_dir = std::env::var("DEP_OSSL_OSSL_SRC_INCLUDE_DIR")
        .expect("DEP_OSSL_OSSL_SRC_INCLUDE_DIR not set");
    let libcrt_include_dir =
        std::env::var("DEP_CRT_INCLUDE_DIR").expect("DEP_CRT_INCLUDE_DIR not set");

    let deps_dir = PathBuf::from("deps");
    let libtpms_dir = deps_dir.join("libtpms");
    let libtpms_src = libtpms_dir.join("src");
    let tpmcmd = libtpms_src.join("tpm2/TPMCmd");

    generate_tpm_library_h(&libtpms_dir);

    // Collect source files (mirrors CMakeLists.txt structure)
    let mut sources: Vec<PathBuf> = Vec::new();

    // TPM2 core: command/*, crypt/*, events/*, main/*, subsystem/*, support/*, X509/*
    sources.extend(glob_c_files_recursive(&tpmcmd.join("tpm/src/command")));
    sources.extend(glob_c_files(&tpmcmd.join("tpm/src/crypt")));
    sources.extend(glob_c_files(&tpmcmd.join("tpm/src/crypt/ecc")));
    sources.extend(glob_c_files(&tpmcmd.join("tpm/src/crypt/math")));
    sources.extend(glob_c_files(&tpmcmd.join("tpm/src/events")));
    sources.extend(glob_c_files(&tpmcmd.join("tpm/src/main")));
    sources.extend(glob_c_files(&tpmcmd.join("tpm/src/subsystem")));
    sources.extend(glob_c_files(&tpmcmd.join("tpm/src/support")));
    sources.extend(glob_c_files(&tpmcmd.join("tpm/src/X509")));

    // Crypto libraries (explicit list — not all files in these dirs are needed)
    let crypto_files = [
        "tpm/cryptolibs/EccRef/TpmEcc_Signature_ECDSA.c",
        "tpm/cryptolibs/RsaRef/PrimeData.c",
        "tpm/cryptolibs/TpmBigNum/BnConvert.c",
        "tpm/cryptolibs/TpmBigNum/BnEccConstants.c",
        "tpm/cryptolibs/TpmBigNum/BnMath.c",
        "tpm/cryptolibs/TpmBigNum/BnMemory.c",
        "tpm/cryptolibs/TpmBigNum/TpmBigNumThunks.c",
        "tpm/cryptolibs/Ossl/BnToOsslMath.c",
        "tpm/cryptolibs/Ossl/ExpDCache.c",
        "tpm/cryptolibs/Ossl/Helpers.c",
        "tpm/cryptolibs/Ossl/TpmToOsslDesSupport.c",
        "tpm/cryptolibs/Ossl/TpmToOsslSupport.c",
    ];
    for f in &crypto_files {
        sources.push(tpmcmd.join(f));
    }

    // Platform library
    sources.extend(glob_c_files(&tpmcmd.join("Platform/src")));

    // Simulator (TPMCmdp.c provides _rpc__* functions used by tpm_tpm2_interface.c)
    sources.push(tpmcmd.join("Simulator/src/TPMCmdp.c"));

    // TpmConfiguration vendor handlers
    sources.push(tpmcmd.join("TpmConfiguration/TpmVendorCommandHandlers/Vendor_TCG_Test.c"));

    // libtpms TPM2 wrappers
    let libtpms_tpm2_files = [
        "tpm2/ACTCommands.c",
        "tpm2/AsymmetricCommands.c",
        "tpm2/AttestationCommands.c",
        "tpm2/AuditCommands.c",
        "tpm2/CapabilityCommands.c",
        "tpm2/ClockCommands.c",
        "tpm2/ContextCommands.c",
        "tpm2/DictionaryCommands.c",
        "tpm2/DuplicationCommands.c",
        "tpm2/EACommands.c",
        "tpm2/EphemeralCommands.c",
        "tpm2/HashCommands.c",
        "tpm2/HierarchyCommands.c",
        "tpm2/IntegrityCommands.c",
        "tpm2/ManagementCommands.c",
        "tpm2/NVCommands.c",
        "tpm2/ObjectCommands.c",
        "tpm2/RandomCommands.c",
        "tpm2/SessionCommands.c",
        "tpm2/SigningCommands.c",
        "tpm2/StartupCommands.c",
        "tpm2/SymmetricCommands.c",
        "tpm2/TestingCommands.c",
        "tpm2/Unmarshal.c",
        "tpm2/BackwardsCompatibilityBitArray.c",
        "tpm2/BackwardsCompatibilityObject.c",
        "tpm2/LibtpmsCallbacks.c",
        "tpm2/NVMarshal.c",
        "tpm2/RuntimeAlgorithm.c",
        "tpm2/RuntimeAttributes.c",
        "tpm2/RuntimeCommands.c",
        "tpm2/RuntimeProfile.c",
        "tpm2/RuntimeUtils.c",
        "tpm2/StateMarshal.c",
        "tpm2/Volatile.c",
    ];
    for f in &libtpms_tpm2_files {
        sources.push(libtpms_src.join(f));
    }

    // libtpms API layer
    let libtpms_api_files = [
        "disabled_interface.c",
        "tpm_debug.c",
        "tpm_library.c",
        "tpm_memory.c",
        "tpm_nvfile.c",
        "tpm_tpm2_interface.c",
        "tpm_tpm2_tis.c",
    ];
    for f in &libtpms_api_files {
        sources.push(libtpms_src.join(f));
    }

    let tpmcfg_dir = tpmcmd.join("TpmConfiguration");

    let mut build = cc::Build::new();
    build
        .files(&sources)
        .warnings(false)
        .define("_POSIX_", None)
        .define("TPM_POSIX", None)
        .define("TPM_LIBTPMS_CALLBACKS", None)
        .define("TPM_NV_DISK", None)
        .define("FILE_BACKED_NV", "YES")
        .define("VTPM", "YES")
        .define("HASH_LIB", "Ossl")
        .define("SYM_LIB", "Ossl")
        .define("MATH_LIB", "TpmBigNum")
        .define("BN_MATH_LIB", "Ossl")
        .include(deps_dir.join("libtpms-bare-config"))
        .include(libtpms_dir.join("include/libtpms"))
        .include(&libtpms_src)
        .include(libtpms_src.join("tpm2"))
        .include(tpmcmd.join("Platform/include"))
        .include(tpmcmd.join("Simulator/include"))
        .include(tpmcmd.join("Simulator/include/prototypes"))
        .include(&tpmcfg_dir)
        .include(tpmcmd.join("tpm/include"))
        .include(tpmcmd.join("tpm/include/private"))
        .include(tpmcmd.join("tpm/include/private/prototypes"))
        .include(tpmcmd.join("tpm/include/tpm_public"))
        .include(tpmcmd.join("tpm/include/platform_interface"))
        .include(tpmcmd.join("tpm/include/platform_interface/prototypes"))
        .include(tpmcmd.join("tpm/cryptolibs/common/include"))
        .include(tpmcmd.join("tpm/cryptolibs/TpmBigNum/include"))
        .include(tpmcmd.join("tpm/cryptolibs/TpmBigNum/include/TpmBigNum"))
        .include(tpmcmd.join("tpm/cryptolibs"))
        .include(tpmcmd.join("tpm/cryptolibs/Ossl"))
        .include(tpmcmd.join("tpm/cryptolibs/Ossl/include"))
        .include(tpmcmd.join("tpm/cryptolibs/Ossl/include/Ossl"))
        .include(&ossl_include_dir)
        .include(&ossl_src_include_dir)
        .pic(true);

    if target_os == "none" {
        build
            .flag("-nostdinc")
            .flag("-fno-stack-protector")
            .flag("-mno-sse")
            .flag("-mno-red-zone")
            .include(&libcrt_include_dir);
    }

    // Cross-compile support
    let host = std::env::var("HOST").unwrap();
    if !host.starts_with("x86_64") {
        build.compiler("x86_64-linux-gnu-gcc");
    }

    build.compile("tpms");

    // --- bindgen ---
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(&out_dir);

    let bindings = bindgen::Builder::default()
        .header("deps/libtpms/src/tpm2/TPMCmd/Platform/include/Platform.h")
        .header("deps/libtpms/include/libtpms/tpm_library.h")
        .header("deps/libtpms/include/libtpms/tpm_error.h")
        .header("deps/libtpms/include/libtpms/tpm_types.h")
        // TPMLIB high-level API
        .allowlist_function("TPMLIB_ChooseTPMVersion")
        .allowlist_function("TPMLIB_MainInit")
        .allowlist_function("TPMLIB_Terminate")
        .allowlist_function("TPMLIB_Process")
        .allowlist_function("TPMLIB_RegisterCallbacks")
        .allowlist_function("TPMLIB_SetState")
        .allowlist_function("TPMLIB_GetState")
        .allowlist_function("TPMLIB_WasManufactured")
        .allowlist_function("TPMLIB_SetProfile")
        .allowlist_type("TPMLIB_TPMVersion")
        .allowlist_type("TPMLIB_StateType")
        .allowlist_type("libtpms_callbacks")
        // _plat__* functions still used
        .allowlist_function("_plat__RunCommand")
        .allowlist_function("_plat__LocalitySet")
        .allowlist_function("_plat__SetNvAvail")
        .allowlist_function("_plat__Signal_PowerOn")
        .allowlist_function("_plat__Signal_Reset")
        .allowlist_function("_plat__NVDisable")
        .allowlist_function("_plat__NVEnable")
        .allowlist_function("TPM_Manufacture")
        .use_core()
        .clang_arg("-Wno-incompatible-library-redeclaration")
        .clang_arg("-nostdinc")
        .clang_arg(format!("-isystem{libcrt_include_dir}"))
        .clang_arg("-fno-pie")
        .clang_arg("-D__linux__")
        .clang_arg("-Ideps/libtpms/include/libtpms")
        .clang_arg("-Ideps/libtpms/src/tpm2/TPMCmd/tpm/include")
        .clang_arg("-Ideps/libtpms/src/tpm2/TPMCmd/tpm/include/tpm_public")
        .clang_arg("-Ideps/libtpms/src/tpm2/TPMCmd/tpm/include/platform_interface")
        .clang_arg("-Ideps/libtpms/src/tpm2/TPMCmd/TpmConfiguration")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .unwrap_or_else(|_| panic!("Unable to generate bindings for libtpms"));

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .unwrap_or_else(|_| panic!("Unable to write bindings.rs"));

    // libcrypto and libcrt are provided by cocoon-tpm-ossl-bare-sys and libcrt crate at link time.
}
