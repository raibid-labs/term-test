#!/bin/bash
# Documentation validation script for ratatui-testlib
# Checks for:
# - Required documentation files
# - Valid internal links
# - Proper structure

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Documentation Structure Check ==="

# Check required files exist
REQUIRED_FILES=(
    "README.md"
    "CHANGELOG.md"
    "CONTRIBUTING.md"
    "docs/STRUCTURE.md"
    "docs/ARCHITECTURE.md"
    "docs/README.md"
    "docs/versions/vNEXT/README.md"
)

MISSING_FILES=0
for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        echo -e "${RED}✗${NC} Missing required file: $file"
        MISSING_FILES=$((MISSING_FILES + 1))
    else
        echo -e "${GREEN}✓${NC} Found: $file"
    fi
done

if [ $MISSING_FILES -gt 0 ]; then
    echo -e "\n${RED}ERROR: $MISSING_FILES required file(s) missing${NC}"
    exit 1
fi

echo -e "\n=== Internal Link Check ==="

# Check for broken internal markdown links
BROKEN_LINKS=0

# Find all markdown files
while IFS= read -r -d '' file; do
    # Extract markdown links: [text](path)
    while IFS= read -r link; do
        # Skip external links (http/https)
        if [[ "$link" =~ ^https?:// ]]; then
            continue
        fi

        # Skip anchors only
        if [[ "$link" =~ ^# ]]; then
            continue
        fi

        # Get directory of current file
        dir=$(dirname "$file")

        # Resolve relative path
        target="$dir/$link"

        # Check if target exists
        if [ ! -f "$target" ] && [ ! -d "$target" ]; then
            echo -e "${YELLOW}⚠${NC} Broken link in $file: $link"
            BROKEN_LINKS=$((BROKEN_LINKS + 1))
        fi
    done < <(grep -oP '\[.*?\]\(\K[^)]+' "$file" 2>/dev/null || true)
done < <(find . -name "*.md" -not -path "./target/*" -not -path "./.git/*" -print0)

if [ $BROKEN_LINKS -gt 0 ]; then
    echo -e "\n${YELLOW}WARNING: $BROKEN_LINKS potentially broken internal link(s) found${NC}"
    echo -e "${YELLOW}Note: Some links may be valid but not checked properly by this script${NC}"
    # Don't fail on broken links for now, just warn
fi

echo -e "\n=== Version Directory Structure ==="

# Check versions directory exists and has vNEXT
if [ ! -d "docs/versions" ]; then
    echo -e "${RED}✗${NC} Missing docs/versions directory"
    exit 1
else
    echo -e "${GREEN}✓${NC} docs/versions directory exists"
fi

if [ ! -d "docs/versions/vNEXT" ]; then
    echo -e "${RED}✗${NC} Missing docs/versions/vNEXT directory"
    exit 1
else
    echo -e "${GREEN}✓${NC} docs/versions/vNEXT directory exists"
fi

echo -e "\n${GREEN}=== Documentation check passed ===${NC}"
