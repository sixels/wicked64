asm_dir := justfile_directory() / "wicked64-codegen/lib/tests/asm"

download_test_roms:
    sh ./download_tests.sh

gen_asm:
    #!/usr/bin/env sh
    for name in `find {{asm_dir / "**/*.asm"}} -type f | awk -F "/" '{ print $(NF-1)"/"$NF }' | cut -d '.' -f 1`; do
        just dump_asm_test "$name"
    done

@dump_asm_test NAME:
    mkdir -p "/tmp/w64-codegen/$(dirname {{NAME}})"
    nasm -O0 -felf64 {{asm_dir / NAME + ".asm"}} -o {{"/tmp/w64-codegen" / NAME + ".o"}}
    objcopy --output-target="binary" --only-section=".text" {{"/tmp/w64-codegen" / NAME + ".o"}} {{ asm_dir / NAME + ".bin" }}