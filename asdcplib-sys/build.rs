use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let is_windows = cfg!(target_os = "windows");

    // Build asdcplib from source via CMake
    let asdcp_dst = cmake::Config::new("asdcplib")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
        .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
        .build();

    let asdcp_lib = asdcp_dst.join("lib");
    let asdcp_include = asdcp_dst.join("include");

    // Some distros put in lib64
    let asdcp_lib64 = asdcp_dst.join("lib64");

    println!("cargo:rustc-link-search=native={}", asdcp_lib.display());
    println!("cargo:rustc-link-search=native={}", asdcp_lib64.display());

    // On Windows MSVC multi-config generators, libraries may land in a
    // per-configuration subdirectory (e.g. lib/Debug or lib/Release).
    if is_windows {
        for sub in &["Debug", "Release", "RelWithDebInfo", "MinSizeRel"] {
            println!(
                "cargo:rustc-link-search=native={}",
                asdcp_lib.join(sub).display()
            );
        }
    }

    // asdcplib cmake uses _d suffix for debug builds
    let profile = env::var("PROFILE").unwrap_or_default();
    let suffix = if profile == "debug" { "_d" } else { "" };

    // The CMake targets are named "libasdcp" / "libkumu" with PREFIX "".
    // On Linux, the linker auto-prepends "lib", so linking "asdcp_d" finds "libasdcp_d.a".
    // On Windows MSVC, there is no auto-prefix, so we must use the full name "libasdcp_d".
    let prefix = if is_windows { "lib" } else { "" };

    println!("cargo:rustc-link-lib=static={prefix}asdcp{suffix}");
    println!("cargo:rustc-link-lib=static={prefix}kumu{suffix}");

    if is_windows {
        // On Windows (vcpkg), OpenSSL libraries have different names
        println!("cargo:rustc-link-lib=dylib=libssl");
        println!("cargo:rustc-link-lib=dylib=libcrypto");
        // Windows socket libraries required by OpenSSL/asdcplib
        println!("cargo:rustc-link-lib=dylib=ws2_32");
        println!("cargo:rustc-link-lib=dylib=crypt32");
        // Advapi32 provides CryptAcquireContext/CryptGenRandom used by KM_prng
        println!("cargo:rustc-link-lib=dylib=Advapi32");
    } else {
        println!("cargo:rustc-link-lib=dylib=ssl");
        println!("cargo:rustc-link-lib=dylib=crypto");
    }

    // On macOS, OpenSSL from Homebrew is not in the default search path
    if cfg!(target_os = "macos") {
        if let Ok(openssl_dir) = env::var("OPENSSL_DIR") {
            let openssl_lib = PathBuf::from(&openssl_dir).join("lib");
            println!("cargo:rustc-link-search=native={}", openssl_lib.display());
        } else if let Ok(output) = std::process::Command::new("brew")
            .args(["--prefix", "openssl"])
            .output()
            && output.status.success()
        {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let openssl_lib = PathBuf::from(&prefix).join("lib");
            println!("cargo:rustc-link-search=native={}", openssl_lib.display());
        }
    }

    // On Windows, set vcpkg library search path
    if is_windows && let Ok(vcpkg_root) = env::var("VCPKG_ROOT") {
        let vcpkg_lib = PathBuf::from(&vcpkg_root).join("installed/x64-windows/lib");
        println!("cargo:rustc-link-search=native={}", vcpkg_lib.display());
    }

    // Check for xerces-c (optional, for timed text)
    // pkg-config is not available on Windows; vcpkg handles linking via CMake
    if !is_windows && pkg_config::probe_library("xerces-c").is_ok() {
        println!("cargo:rustc-link-lib=dylib=xerces-c");
    }

    // Compile our C++ shim
    let mut shim_build = cc::Build::new();
    shim_build
        .cpp(true)
        .file("shim/asdcp_shim.cpp")
        .include("shim")
        .include(&asdcp_include);

    if is_windows {
        // KM_WIN32 enables the Windows code paths in asdcplib headers
        // (e.g. dirent_win.h instead of dirent.h in KM_fileio.h)
        shim_build
            .define("KM_WIN32", None)
            .define("WIN32", None)
            .define("_CRT_SECURE_NO_WARNINGS", None)
            .define("_CRT_NONSTDC_NO_WARNINGS", None)
            .flag_if_supported("/std:c++14");
    } else {
        shim_build.flag_if_supported("-std=c++14");
    }

    shim_build.compile("asdcp_shim");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=asdcp_shim");

    // C++ standard library linkage
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if !is_windows {
        // MSVC handles C++ runtime linkage automatically
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    println!("cargo:rerun-if-changed=shim/asdcp_shim.h");
    println!("cargo:rerun-if-changed=shim/asdcp_shim.cpp");
}
