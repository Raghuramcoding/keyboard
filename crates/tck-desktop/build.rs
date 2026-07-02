fn main() {
    // Trigger rebuild when the WASM bundle changes.
    println!("cargo:rerun-if-changed=../tck-ui/dist/");
}
