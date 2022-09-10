use pretty_assertions::assert_eq;

use w64_codegen::emit;
use w64_codegen::Emitter;

#[path = "./common.rs"]
mod common;

use common::{dump_file, registers};

#[test]
fn mov_reg_reg() {
    let mut emitter = Emitter::default();

    let regs = registers();
    for dst in regs.iter().copied() {
        for src in regs.iter().copied() {
            emit!(emitter, mov %dst, %src);
        }
    }

    assert_eq!(emitter.as_slice(), dump_file("mov/mov_reg_reg"));
}

#[test]
fn mov_reg_imm() {
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
    assert_eq!(emitter.as_slice(), dump_file("mov/mov_reg_imm"));
}

#[test]
fn mov_reg_dir() {
    let mut emitter = Emitter::default();

    let addr = 0x78563412;
    for reg in registers() {
        emit!(emitter, mov %reg,[$addr]);
    }

    assert_eq!(emitter.as_slice(), dump_file("mov/mov_reg_dir"));
}

#[test]
fn mov_reg_ind() {
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
    assert_eq!(emitter.as_slice(), dump_file("mov/mov_reg_ind"));
}

#[test]
fn mov_ind_reg() {
    let mut emitter = Emitter::default();
    let disp32 = 0x123456;
    let disp8 = 0x70;
    let regs = registers();
    for dst in regs.iter().copied() {
        for src in regs.iter().copied() {
            emit!(emitter, mov [%dst], %src);
            emit!(emitter, mov [%dst + $disp32], %src);
            emit!(emitter, mov [%dst - $disp8], %src);
        }
    }
    assert_eq!(emitter.as_slice(), dump_file("mov/mov_ind_reg"));
}
