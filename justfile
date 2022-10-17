asm_dir := justfile_directory() / "wicked64-codegen/lib/tests/asm"

download_test_roms:
    sh ./download_tests.sh