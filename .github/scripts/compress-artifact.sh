#!/bin/bash
BINARY_FILE="target/$1/release/sculptor"
COMMON_FILES=("Config.example.toml")
ARTIFACT_OUTPUT="${OUTPUT_DIR}/sculptor-$2-$3"
if [ "$2" = "windows" ]
then zip -j "${ARTIFACT_OUTPUT}.zip" "${BINARY_FILE}.exe" "${COMMON_FILES[@]}"
else tar --transform 's|^.*/||' -czf "${ARTIFACT_OUTPUT}.tar.gz" "$BINARY_FILE" "${COMMON_FILES[@]}"
fi