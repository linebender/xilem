#!/bin/bash
set -e

# This script provides a relatively predictable way to test the build speed
# of various targets on your machine.
# It first performs a clean build of each target, them removes their incremental
# cache, then builds the target again and prints the build times to stdout.

# Expect to take at least 30s to run the script, probably more.

echo "== INITIAL BUILDS =="

echo "-- Building long_elem_seq --"
cargo rustc --profile build-perf --quiet --package xilem --test long_elem_seq

echo "-- Building calc --"
cargo build --profile build-perf --quiet --example calc

echo "-- Building placehero --"
cargo build --profile build-perf --quiet --package placehero


echo "== REMOVING CACHES =="

echo "-- Removing cache for long_elem_seq --"
rm -rf target/build-perf/incremental/long_elem_seq-*

echo "-- Removing cache for calc --"
rm -rf target/build-perf/incremental/calc-*

echo "-- Removing cache for placehero --"
rm -rf target/build-perf/incremental/placehero-*


echo "== INCREMENTAL BUILDS =="

touch xilem/stress_tests/long_elem_seq.rs
touch xilem/examples/calc.rs
touch placehero/src/main.rs

echo "-- Building long_elem_seq --"
command time -f "Built long_elem_seq in %es" \
    cargo rustc --profile build-perf --quiet --package xilem --test long_elem_seq -- --cfg compile_stress_test

echo "-- Building calc --"
command time -f "Built calc in %es" \
    cargo build --profile build-perf --quiet --example calc

echo "-- Building placehero --"
command time -f "Built placehero in %es" \
    cargo build --profile build-perf --quiet --package placehero

echo "== DONE =="
