#!/usr/bin/env bash
set -euo pipefail

BANNER_FILE="../../../../assets/banner.txt"
TEMPLATE_FILE="src/install.sh.template"
OUTPUT_FILE="public/install"

cd "$(dirname "$0")/.."

if [ ! -f "$BANNER_FILE" ]; then
    echo "Error: Banner file not found at $BANNER_FILE"
    exit 1
fi

BANNER_CONTENT=$(cat "$BANNER_FILE")
TEMP_FILE=$(mktemp)

cat << BANNER_BLOCK_EOF > "$TEMP_FILE"
BANNER=\$(cat << 'BANNER_EOF'
$BANNER_CONTENT
BANNER_EOF
)
BANNER_BLOCK_EOF

sed -e "/%%BANNER_PLACEHOLDER%%/r $TEMP_FILE" -e "/%%BANNER_PLACEHOLDER%%/d" "$TEMPLATE_FILE" > "$OUTPUT_FILE"
rm "$TEMP_FILE"
echo "Generated $OUTPUT_FILE"
