use std::{collections::HashSet, env, path::PathBuf};
use std::process::Command;

const GGML: &str = "ggml";
const GGML_SOURCE: &str = "ggml/src";
const GGML_HEADER_SOURCE: &str = "ggml/include";

fn bindgen() {
    // include main ggml header file
    let ggml_header_path = PathBuf::from(GGML_HEADER_SOURCE).join("ggml.h");

    let mut api = bindgen::Builder::default()
        .derive_copy(true)
        .derive_debug(true)
        .derive_partialeq(true)
        .derive_partialord(true)
        .derive_eq(true)
        .derive_ord(true)
        .derive_hash(true)
        .impl_debug(true)
        .merge_extern_blocks(true)
        .enable_function_attribute_detection()
        .sort_semantically(true)
        .header(ggml_header_path.to_string_lossy())
        // Suppress some warnings
        .raw_line("#![allow(non_upper_case_globals)]")
        .raw_line("#![allow(non_camel_case_types)]")
        .raw_line("#![allow(non_snake_case)]")
        .raw_line("#![allow(unused)]")
        .raw_line("pub const GGMLSYS_VERSION: Option<&str> = option_env!(\"CARGO_PKG_VERSION\");")
        // Do not generate code for ggml's includes (stdlib)
        .allowlist_file(ggml_header_path.to_string_lossy());

    if cfg!(feature = "cmake") {
        if cfg!(feature = "cuda") {
            let hfn = PathBuf::from(GGML_HEADER_SOURCE).join("ggml-cuda.h");
            let hfn = hfn.to_string_lossy();
            api = api.header(hfn.clone()).allowlist_file(hfn);
        }
        if cfg!(feature = "blas") {
            let hfn = PathBuf::from(GGML_HEADER_SOURCE).join("ggml-blas.h");
            let hfn = hfn.to_string_lossy();
            api = api.header(hfn.clone()).allowlist_file(hfn);
        }
        if cfg!(feature = "vulkan") {
            let hfn = PathBuf::from(GGML_HEADER_SOURCE).join("ggml-vulkan.h");
            let hfn = hfn.to_string_lossy();
            api = api.header(hfn.clone()).allowlist_file(hfn);
        }
        if cfg!(feature = "metal") {
            let hfn = PathBuf::from(GGML_HEADER_SOURCE).join("ggml-metal.h");
            let hfn = hfn.to_string_lossy();
            api = api.header(hfn.clone()).allowlist_file(hfn);
        }
    }

    let bindings = api.generate().expect("Unable to generate bindings");
    bindings .write_to_file("src/lib.rs") .expect("Couldn't write bindings");
}

fn main() {
    // By default, this crate will attempt to compile ggml with the features of your host system if
    // the host and target are the same. If they are not, it will turn off auto-feature-detection,
    // and you will need to manually specify target features through target-features.
    println!("cargo:rerun-if-changed=ggml");

    // If running on docs.rs, the filesystem is readonly so we can't actually generate
    // anything. This package should have been fetched with the bindings already generated
    // so we just exit  here.
    if env::var("DOCS_RS").is_ok() {
        return;
    }
    build();
}

fn build() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    // generate lowlevel rust binding to ggml
    bindgen();

    // This silliness is necessary to get the cc crate to discover and
    // spit out the necessary stuff to link with C++ (and CUDA if enabled).
    // let mut build = cc::Build::new();
    // build.cpp(true).file("dummy/dummy.c");

    // if cfg!(feature = "cublas") {
    //     build.cuda(true);
    // } else if cfg!(feature = "hipblas") {
    //     println!("cargo:rerun-if-changed=ROCM_PATH");
    //     build.cpp(true);
    // }
    // build.compile("dummy");

//     let rocm_path = if cfg!(feature = "hipblas") {
//         Some(PathBuf::from(
//             env::var("ROCM_PATH").unwrap_or_else(|_| String::from("/opt/rocm")),
//         ))
//     } else {
//         None
//     };

    let mut build = cmake::Config::new("ggml");
    // add ggml source file to discover cmake files
    build.build_target("ggml");

    if cfg!(feature = "static") {
        // generate static library instead of dynamic lib.
        // This option is not available now due to some
        // limitations of this ggml repo.
        build.define("GGML_STATIC", "ON");
        panic!("static feature is not available now, don't toggle it on and use dynamic libraries instead");
    }
    if cfg!(feature = "vulkan") {
        // vulkan backend needs some additional help for defining
        // its computation process, aka. shader.
        // This is the helper python script used to generate necessary
        // hpp header file.
        let mut generator = Command::new("python");
        generator.current_dir(GGML_SOURCE).args(["-m", "ggml_vk_generate_shaders"])
            .output().expect("Failed to generate vulkan shader");
        build.define("GGML_VULKAN", "ON");
    }
    if cfg!(feature = "kompute") {
        build.define("GGML_KOMPUTE", "ON");
    }

    // if cfg!(feature = "no_k_quants") {
    //     build.define("LLAMA_K_QUANTS", "OFF");
    // }
    // if cfg!(feature = "hipblas") {
    //     let rocm_path = rocm_path.as_ref().expect("Impossible: rocm_path not set!");
    //     let rocm_llvm_path = rocm_path.join("llvm").join("bin");
    //     build.define("LLAMA_HIPBLAS", "ON");
    //     build.define("CMAKE_PREFIX_PATH", rocm_path);
    //     build.define("CMAKE_C_COMPILER", rocm_llvm_path.join("clang"));
    //     build.define("CMAKE_CXX_COMPILER", rocm_llvm_path.join("clang++"));
    // } else if cfg!(feature = "clblast") {
    //     build.define("LLAMA_CLBLAST", "ON");
    // } else if cfg!(feature = "openblas") {
    //     build.define("LLAMA_BLAS", "ON");
    //     build.define("LLAMA_BLAS_VENDOR", "OpenBLAS");
    // }
    // if target_os == "macos" {
    //     build.define(
    //         "LLAMA_ACCELERATE",
    //         if cfg!(feature = "no_accelerate") {
    //             "OFF"
    //         } else {
    //             "ON"
    //         },
    //     );
    //     build.define(
    //         "LLAMA_METAL",
    //         if cfg!(feature = "metal") { "ON" } else { "OFF" },
    //     );
    // }
    let dst = build.build();
    // if cfg!(feature = "cublas") {
    //     println!("cargo:rustc-link-lib=cublas");
    // } else if cfg!(feature = "hipblas") {
    //     let rocm_path = rocm_path.as_ref().expect("Impossible: rocm_path not set!");
    //     println!(
    //         "cargo:rustc-link-search={}",
    //         rocm_path.join("lib").to_string_lossy()
    //     );
    //     println!("cargo:rustc-link-lib=hipblas");
    //     println!("cargo:rustc-link-lib=amdhip64");
    //     println!("cargo:rustc-link-lib=rocblas");
    //     let mut build = cc::Build::new();
    //     build.cpp(true).file("dummy/dummy.c").object(
    //         PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set!"))
    //             .join("build")
    //             .join("CMakeFiles")
    //             .join("ggml-rocm.dir")
    //             .join("ggml-cuda.cu.o"),
    //     );
    //     build.compile("dummy");
    // } else if cfg!(feature = "clblast") {
    //     println!("cargo:rustc-link-lib=clblast");
    //     println!(
    //         "cargo:rustc-link-lib={}OpenCL",
    //         if target_os == "macos" {
    //             "framework="
    //         } else {
    //             ""
    //         }
    //     );
    // } else if cfg!(feature = "openblas") {
    //     println!("cargo:rustc-link-lib=openblas");
    // }
    if target_os == "macos" {
        if cfg!(not(feature = "no_accelerate")) {
            println!("cargo:rustc-link-lib=framework=Accelerate");
        }
        if cfg!(feature = "metal") {
            println!("cargo:rustc-link-lib=framework=Foundation");
            println!("cargo:rustc-link-lib=framework=Metal");
            println!("cargo:rustc-link-lib=framework=MetalKit");
            println!("cargo:rustc-link-lib=framework=MetalPerformanceShaders");
        }
    }
    println!("cargo:rustc-link-search=native={}/build", dst.display());
    println!("cargo:rustc-link-lib=ggml");
    // println!("cargo:rustc-link-lib=static=ggml_static");
}

fn get_supported_target_features() -> HashSet<String> {
    env::var("CARGO_CFG_TARGET_FEATURE")
        .unwrap()
        .split(',')
        .filter(|s| x86::RELEVANT_FLAGS.contains(s))
        .map(ToString::to_string)
        .collect::<HashSet<_>>()
}

mod x86 {
    use super::HashSet;

    pub const RELEVANT_FLAGS: &[&str] = &["fma", "avx", "avx2", "f16c", "sse3"];
    pub struct Features(HashSet<String>);

    impl std::ops::Deref for Features {
        type Target = HashSet<String>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl Features {
        pub fn get() -> Self {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            if std::env::var("HOST") == std::env::var("TARGET") {
                return Self::get_host();
            }
            Self(super::get_supported_target_features())
        }

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        pub fn get_host() -> Self {
            Self(
                [
                    std::is_x86_feature_detected!("fma"),
                    std::is_x86_feature_detected!("avx"),
                    std::is_x86_feature_detected!("avx2"),
                    std::is_x86_feature_detected!("f16c"),
                    std::is_x86_feature_detected!("sse3"),
                ]
                .into_iter()
                .enumerate()
                .filter(|(_, exists)| *exists)
                .map(|(idx, _)| RELEVANT_FLAGS[idx].to_string())
                .collect::<HashSet<_>>(),
            )
        }
    }
}
