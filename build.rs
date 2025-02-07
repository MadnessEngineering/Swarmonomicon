fn main() {
    // Tell cargo to look for OpenCV libraries
    pkg_config::Config::new()
        .probe("opencv4")
        .unwrap_or_else(|e| {
            println!("cargo:warning=Failed to find OpenCV4: {}", e);
            println!("cargo:warning=Make sure OpenCV is installed on your system");
            std::process::exit(1);
        });

    // Tell cargo to invalidate the built crate whenever the build script changes
    println!("cargo:rerun-if-changed=build.rs");
} 
