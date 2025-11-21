#!/bin/bash
set -e


echo "== INITIAL BUILDS =="

echo "-- Building long_elem_seq --"
cargo build --quiet --test long_elem_seq --features compile-stress-test

echo "-- Building calc --"
cargo build --quiet --example calc

echo "-- Building placehero --"
cargo build --quiet --package placehero


echo "== REMOVING CACHES =="

echo "-- Removing cache for long_elem_seq --"
rm -rf target/debug/incremental/long_elem_seq-*

echo "-- Removing cache for calc --"
rm -rf target/debug/incremental/calc-*

echo "-- Removing cache for placehero --"
rm -rf target/debug/incremental/placehero-*


echo "== INCREMENTAL BUILDS =="

touch xilem/stress_tests/long_elem_seq.rs
touch xilem/examples/calc.rs
touch placehero/src/main.rs

echo "-- Building long_elem_seq --"
command time -f "Built long_elem_seq in %es" cargo build --quiet --test long_elem_seq --features compile-stress-test

echo "-- Building calc --"
command time -f "Built calc in %es" cargo build --quiet --example calc

echo "-- Building placehero --"
command time -f "Built placehero in %es" cargo build --quiet --package placehero

echo "== DONE =="
