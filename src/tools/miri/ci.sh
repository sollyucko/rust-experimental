#!/bin/bash
set -euo pipefail

# Determine configuration
export RUST_TEST_NOCAPTURE=1
export RUST_BACKTRACE=1
export RUSTC_EXTRA_FLAGS="-D warnings"
export CARGO_INCREMENTAL=0
export CARGO_EXTRA_FLAGS="--all-features"

# Prepare
echo "Build and install miri"
./miri build --all-targets --locked
./miri install # implicitly locked
echo

# Test
function run_tests {
  if [ -n "${MIRI_TEST_TARGET+exists}" ]; then
    echo "Testing foreign architecture $MIRI_TEST_TARGET"
  else
    echo "Testing host architecture"
  fi

  ./miri test --locked
  if ! [ -n "${MIRI_TEST_TARGET+exists}" ]; then
    # Only for host architecture: tests with MIR optimizations
    MIRI_TEST_FLAGS="-Z mir-opt-level=3" ./miri test --locked
  fi
  # "miri test" has built the sysroot for us, now this should pass without
  # any interactive questions.
  ${PYTHON:-python3} test-cargo-miri/run-test.py

  echo
}

# host
run_tests

if [ "${TRAVIS_OS_NAME:-}" == linux ]; then
  MIRI_TEST_TARGET=i686-unknown-linux-gnu run_tests
  MIRI_TEST_TARGET=x86_64-apple-darwin run_tests
  MIRI_TEST_TARGET=i686-pc-windows-msvc run_tests
elif [ "${TRAVIS_OS_NAME:-}" == osx ]; then
  MIRI_TEST_TARGET=x86_64-pc-windows-msvc run_tests
  MIRI_TEST_TARGET=i686-pc-windows-gnu run_tests
elif [ "${CI_WINDOWS:-}" == True ]; then
  MIRI_TEST_TARGET=x86_64-unknown-linux-gnu run_tests
  MIRI_TEST_TARGET=x86_64-apple-darwin run_tests
fi
