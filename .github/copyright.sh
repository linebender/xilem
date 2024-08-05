#!/bin/bash

# If there are new files with headers that can't match the conditions here,
# then the files can be ignored by an additional glob argument via the -g flag.
# For example:
#   -g "!src/special_file.rs"
#   -g "!src/special_directory"

# Check all the standard Rust source files
# Exclude the Emoji picker example because it also has some MIT licensed content
output=$(rg "^// Copyright (19|20)[\d]{2} (.+ and )?the Xilem Authors( and .+)?$\n^// SPDX-License-Identifier: Apache-2\.0$\n\n" --files-without-match --multiline -g "*.rs" -g "!xilem/examples/emoji_picker.rs" .)

if [ -n "$output" ]; then
	echo -e "The following files lack the correct copyright header:\n"
	echo $output
	echo -e "\n\nPlease add the following header:\n"
	echo "// Copyright $(date +%Y) the Xilem Authors"
	echo "// SPDX-License-Identifier: Apache-2.0"
	echo -e "\n... rest of the file ...\n"
	exit 1
fi

echo "All files have correct copyright headers."
exit 0
