use w64_codegen::register::Register;

pub fn registers() -> Vec<Register> {
    (0..16u8)
        .map(|n| unsafe { std::mem::transmute::<u8, Register>(n) })
        .collect()
}

pub fn dump_file(name: &str) -> Vec<u8> {
    let path = format!("./tests/asm/{name}.bin");
    std::fs::read(&path).expect(&format!("Could not find `{path}`"))
}
