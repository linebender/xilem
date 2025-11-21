#!/bin/bash
set -e


echo "== INITIAL BUILDS =="

echo "-- Building stress_test --"
cargo build --quiet --test stress_test

echo "-- Building calc --"
cargo build --quiet --example calc

echo "-- Building placehero --"
cargo build --quiet --package placehero


echo "== REMOVING CACHES =="

echo "-- Removing cache for stress_test --"
rm -rf target/debug/incremental/stress_test-*

echo "-- Removing cache for calc --"
rm -rf target/debug/incremental/calc-*

echo "-- Removing cache for placehero --"
rm -rf target/debug/incremental/placehero-*


echo "== INCREMENTAL BUILDS =="

touch xilem/tests/stress_test.rs
touch xilem/examples/calc.rs
touch placehero/src/main.rs

echo "-- Building stress_test --"
command time -f "Built stress_test in %es" cargo build --quiet --test stress_test

echo "-- Building calc --"
command time -f "Built calc in %es" cargo build --quiet --example calc

echo "-- Building placehero --"
command time -f "Built placehero in %es" cargo build --quiet --package placehero

echo "== DONE =="
