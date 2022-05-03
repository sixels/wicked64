#!/bin/sh

function requires {
  command -q "$1" || {
    echo "Command $1 is required to run this script."
    exit -1
  }
}

requires curl
requires bsdtar

ROMS_DIR='./assets/test-roms/'

DILLONB_TESTS_URL='https://github.com/Dillonb/n64-tests/releases/download/latest/dillon-n64-tests.zip'

curl -L $DILLONB_TESTS_URL \
  | bsdtar xv -C "$ROMS_DIR/dillonb"
