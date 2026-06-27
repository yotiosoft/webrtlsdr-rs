use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=LIBRARY_PATH");
    println!("cargo:rerun-if-env-changed=LD_LIBRARY_PATH");

    match Command::new("pkg-config")
        .args(["--libs", "librtlsdr"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let libs = String::from_utf8_lossy(&output.stdout);
            emit_pkg_config_libs(&libs);
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn_missing_librtlsdr(stderr.trim());
            println!("cargo:rustc-link-lib=rtlsdr");
        }
        Err(error) => {
            warn_missing_librtlsdr(&format!("failed to run pkg-config: {error}"));
            println!("cargo:rustc-link-lib=rtlsdr");
        }
    }
}

fn emit_pkg_config_libs(libs: &str) {
    for token in libs.split_whitespace() {
        if let Some(path) = token.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={path}");
        } else if let Some(lib) = token.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={lib}");
        }
    }
}

fn warn_missing_librtlsdr(detail: &str) {
    println!(
        "cargo:warning=librtlsdr was not found by pkg-config. Install librtlsdr-dev (or an equivalent package) and ensure the shared library is discoverable. Detail: {detail}"
    );
}
