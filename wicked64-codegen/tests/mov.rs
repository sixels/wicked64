use wicked64_codegen::Emitter;
use wicked64_codegen_macro::emit;

#[test]
fn mov_reg_reg() {
    let mut emitter = Emitter::new();

    let rax = 0;
    let r9 = 9;

    emit!(emitter,
        mov rcx, r8;
        mov r9, rax;
        mov rcx, rbx;
        mov r9, r11;
        mov r9, $rax;
        mov $r9, rax;
        mov $r9, $rax;
    );

    assert_eq!(
        emitter.as_slice(),
        &[
            0x4c, 0x89, 0xc1, // mov rcx, r8
            0x49, 0x89, 0xc1, // mov r9,rax
            0x48, 0x89, 0xd9, // mov rcx, rbx
            0x4d, 0x89, 0xd9, // mov r9, r11
            0x49, 0x89, 0xc1, // mov r9,rax
            0x49, 0x89, 0xc1, // mov r9,rax
            0x49, 0x89, 0xc1, // mov r9,rax
        ],
    );
}

#[test]
fn mov_reg_immediate() {
    let mut emitter = Emitter::new();

    let val = 0x3412;
    let r11 = 11;

    emit!(emitter,
        mov rcx, 0x3412;
        mov rbx, 0x3412;
        mov r9, 0x3412;
        mov r11, 0x3412;
        mov rax, 0x3412;
        mov r8, 0x3412;
        mov $r11, 0x3412;
        mov r11, $val;
    );

    assert_eq!(
        emitter.as_slice(),
        &[
            0xb9, 0x12, 0x34, 0x00, 0x00, // mov rcx, 0x3412
            0xbb, 0x12, 0x34, 0x00, 0x00, // mov rbx, 0x3412
            0x41, 0xb9, 0x12, 0x34, 0x00, 0x00, // mov r9, 0x3412
            0x41, 0xbb, 0x12, 0x34, 0x00, 0x00, // mov r11, 0x3412
            0xb8, 0x12, 0x34, 0x00, 0x00, // mov rax, 0x3412
            0x41, 0xb8, 0x12, 0x34, 0x00, 0x00, // mov r8, 0x3412
            0x41, 0xbb, 0x12, 0x34, 0x00, 0x00, // mov r11, 0x3412
            0x41, 0xbb, 0x12, 0x34, 0x00, 0x00, // mov r11, 0x3412
        ]
    )
}
