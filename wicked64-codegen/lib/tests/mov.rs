use pretty_assertions::assert_eq;

use w64_codegen::emit;
use w64_codegen::register::Register;
use w64_codegen::Emitter;

fn registers() -> Vec<Register> {
    (0..16u8)
        .map(|n| unsafe { std::mem::transmute::<u8, Register>(n) })
        .collect()
}

fn dump_file(name: &str) -> Vec<u8> {
    let path = format!("./tests/asm/mov/{name}.bin");
    std::fs::read(&path).expect(&format!("Could not find `{path}`"))
}

#[test]
fn mov_reg_reg() {
    let mut emitter = Emitter::default();

    let regs = registers();
    for dst in regs.iter().copied() {
        for src in regs.iter().copied() {
            emit!(emitter, mov %dst, %src);
        }
    }

    assert_eq!(emitter.as_slice(), dump_file("mov_reg_reg"));
}

#[test]
fn mov_reg_immediate() {
    let mut emitter = Emitter::default();

    let byte = -0x12_i8;
    let dword = 0xfeff26_u32;
    let qword = -0xf4a2_i64;

    for reg in registers() {
        emit!(emitter,
            mov %reg, $byte;
            mov %reg, $dword;
            movabs %reg, $qword;
        );
    }
    assert_eq!(emitter.as_slice(), dump_file("mov_reg_imm"));
}

#[test]
fn mov_reg_direct() {
    let mut emitter = Emitter::default();

    let addr = 0x78563412;
    for reg in registers() {
        emit!(emitter, mov %reg,[$addr]);
    }

    assert_eq!(emitter.as_slice(), dump_file("mov_reg_direct"));
}

#[test]
fn mov_reg_indirect() {
    let mut emitter = Emitter::default();

    let disp = 0x123456;
    let regs = registers();
    for dst in regs.iter().copied() {
        for src in regs.iter().copied() {
            emit!(emitter, mov %dst, [%src]);
            emit!(emitter, mov %dst, [%src + $disp]);
            emit!(emitter, mov %dst, [%src - $disp]);
        }
    }

    assert_eq!(emitter.as_slice(), dump_file("mov_reg_indirect"));
}
