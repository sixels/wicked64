use wicked64_codegen::Emitter;
use wicked64_codegen_macro::emit;

#[test]
fn mov() {
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
        vec![
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
