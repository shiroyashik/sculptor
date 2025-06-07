#!/bin/bash
set -euo pipefail

export OUTPUT_DIR
mkdir -p "$OUTPUT_DIR"

printenv CARGO_BUILD_TARGETS | awk -F, '{
  for (i = 1; i <= NF; i++)
    if (match($i, /^([^-]+)(-[^-]+)*-(linux|windows|darwin)(-[^-]+)*$/, matches))
      printf "%s\0%s\0%s\0", $i, matches[3], matches[1]
    else print "DEBUG: awk: No regex match for field index " i ": \047" $i "\047" > /dev/stderr
}' | xargs -0 -n 3 "$(dirname -- "$(readlink -f -- "$0")")/compress-artifact.sh"