#!/bin/bash
set -euo pipefail
USAGE="\
Usage: $0 [-t target]... [-o output_dir] [-h]
	-t target					add a build target
	-o output_dir			set output directory for compressed files (default: current directory)
	-h								Show this help message and exit

Environment variables (override options):
	OUTPUT_DIR							output directory for compressed files
	CARGO_BUILD_TARGETS			comma-separated list of targets
"
targets=()
output_dir=

while getopts "t:o:h" opt
do
	case $opt in
		t) targets+=("$OPTARG") ;;
		o) output_dir="$OPTARG" ;;
		h) echo "$USAGE"; exit 0 ;;
		*) echo "Invalid option: ${opt}" >&2; echo "$USAGE"; exit 1 ;;
	esac
done

output_dir="${OUTPUT_DIR:-${output_dir:-.}}"

if [ "${CARGO_BUILD_TARGETS+set}" ] # if set (might be empty)
then IFS=',' read -ra targets <<< "$CARGO_BUILD_TARGETS"
fi

compress-artifact() {
	local build_dir os arch binary_file common_files output_file

	build_dir="$1"
	os="$2"
	arch="$3"

	binary_file="${build_dir}/sculptor"
	# can be extended to include more files if needed
	common_files=("Config.example.toml")
	output_file="${output_dir}/sculptor-${os}-${arch}"

	if [ "$2" = "windows" ]
	then zip -j "${output_file}.zip" "${binary_file}.exe" "${common_files[@]}"
	else tar --transform 's|^.*/||' -czf "${output_file}.tar.gz" "$binary_file" "${common_files[@]}"
	fi
}

for target in "${targets[@]}"
do
	build_dir="target/${target}/release"
	# add more targets as needed, for now only linux and windows
	if [[ "$target" =~ ^([^-]+)(-[^-]+)*-(linux|windows)(-[^-]+)*$ ]]
	then
		os="${BASH_REMATCH[3]}"
		arch="${BASH_REMATCH[1]}"
		compress-artifact "$build_dir" "$os" "$arch"
	else
		echo "ERROR: Invalid target: $target" >&2
		exit 1
	fi
done