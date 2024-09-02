fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=../../template.yaml");

    let config = sam_env::SamEnvConfig {
        template_path: "../../template.yaml".into(),
        package_name: std::env::var("CARGO_PKG_NAME").unwrap(),
        output_path: std::env::var("OUT_DIR").unwrap(),
        output_filename: "sam_env.rs".into(),
        struct_name: "SamEnv".into(),
    };
    sam_env::write_sam_env(config).unwrap();
}
