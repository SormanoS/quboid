use std::env;

fn main() {
    println!("cargo:rerun-if-changed=quboid.exe.manifest");
    println!("cargo:rerun-if-changed=quboid.rc");
    println!("cargo:rerun-if-changed=quboid.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // Locates rc.exe in the installed Windows SDK, so no Developer
        // PowerShell and no checked-in .res file are needed.
        embed_resource::compile("quboid.rc", embed_resource::NONE)
            .manifest_required()
            .expect("failed to compile quboid.rc");
    }
}
