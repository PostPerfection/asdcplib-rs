use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

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

    // asdcplib cmake uses _d suffix for debug builds
    let profile = env::var("PROFILE").unwrap_or_default();
    let suffix = if profile == "debug" { "_d" } else { "" };

    println!("cargo:rustc-link-lib=static=asdcp{suffix}");
    println!("cargo:rustc-link-lib=static=kumu{suffix}");
    println!("cargo:rustc-link-lib=dylib=ssl");
    println!("cargo:rustc-link-lib=dylib=crypto");

    // Check for xerces-c (optional, for timed text)
    if pkg_config::probe_library("xerces-c").is_ok() {
        println!("cargo:rustc-link-lib=dylib=xerces-c");
    }

    // Compile our C++ shim
    cc::Build::new()
        .cpp(true)
        .file("shim/asdcp_shim.cpp")
        .include("shim")
        .include(&asdcp_include)
        .flag_if_supported("-std=c++14")
        .compile("asdcp_shim");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=asdcp_shim");

    // macOS uses libc++, Linux uses libstdc++
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    println!("cargo:rerun-if-changed=shim/asdcp_shim.h");
    println!("cargo:rerun-if-changed=shim/asdcp_shim.cpp");
}
