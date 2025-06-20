#!/bin/bash

# Check all the standard Rust source files
output=$(rg "debug_assertions" -g "*.rs" .)

if [ -z "$output" ]; then
	if [ "$USING_DEBUG_ASSERTIONS" = "true" ]; then
		echo "Could not find any debug_assertions usage in Rust code."
		echo "The CI script must be modified to not expect usage."
		echo "Set USING_DEBUG_ASSERTIONS to false in .github/workflows/ci.yml."
		exit 1
	else
		echo "Expected no debug_assertions usage in Rust code and found none."
		exit 0
	fi
else
	if [ "$USING_DEBUG_ASSERTIONS" = "true" ]; then
		echo "Expected debug_assertions to be used in Rust code and found it."
		exit 0
	else
		echo "Found debug_assertions usage in Rust code."
		echo ""
		echo $output
		echo ""
		echo "The CI script must be modified to expect this usage."
		echo "Set USING_DEBUG_ASSERTIONS to true in .github/workflows/ci.yml."
		exit 1
	fi
fi
