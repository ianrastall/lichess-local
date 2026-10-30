fn main() {
    let descriptor = protox::compile(["../proto/merci.proto"], ["../proto"]).unwrap();
    prost_build::Config::new()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(".", "#[serde(default, rename_all = \"camelCase\")]")
        .compile_fds(descriptor)
        .unwrap();
    println!("cargo:rerun-if-changed=../proto/merci.proto");
}
