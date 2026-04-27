#!/usr/bin/env bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

BINARY=$1
if [ -z "$BINARY" ]; then
    echo "Usage: $0 <path-to-binary>"
    exit 1
fi

if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY${NC}"
    exit 1
fi

echo "Running shell tests on $BINARY..."

fail() {
    echo -e "${RED}FAIL: $1${NC}"
    exit 1
}

pass() {
    echo -e "${GREEN}PASS: $1${NC}"
}

# Test: Help output
$BINARY --help | grep -q "Usage: asleep" || fail "Help output should contain usage"
pass "Help output"

# Test: Invalid duration exit code and message
output=$($BINARY invalid 2>&1 || true)
if echo "$output" | grep -q "Error parsing duration"; then
    pass "Invalid duration handled"
else
    fail "Invalid duration did not show correct error"
fi

# Test: Sleep duration (roughly)
# We use @ as start to avoid sub-second issues with date +%s
start=$(date +%s)
$BINARY 2s --no-progress || fail "Sleep 2s failed"
end=$(date +%s)
elapsed=$((end - start))

if [ $elapsed -ge 2 ] && [ $elapsed -le 4 ]; then
    pass "Sleep duration (elapsed: ${elapsed}s)"
else
    fail "Sleep duration was out of expected range (elapsed: ${elapsed}s)"
fi

# Test: Until past
output=$($BINARY --until "@0" 2>&1 || true)
if echo "$output" | grep -q "in the past"; then
    pass "Past --until handled"
else
    fail "Past --until did not show correct error"
fi

# Test: --no-progress flag
output=$($BINARY 1s --no-progress 2>/dev/null)
if [ -z "$output" ]; then
    pass "No progress flag respected"
else
    fail "Progress output seen even with --no-progress"
fi

# Test: --monotonic flag
$BINARY 1s --monotonic || fail "--monotonic failed"
pass "--monotonic flag works"

# Test: -m flag
$BINARY 1s -m || fail "-m failed"
pass "-m flag works"

# Test: Multiple durations summing
start=$(date +%s)
$BINARY 1s 2s 1s --no-progress || fail "Multiple durations failed"
end=$(date +%s)
elapsed=$((end - start))
if [ $elapsed -ge 4 ] && [ $elapsed -le 6 ]; then
    pass "Multiple durations summed correctly (elapsed: ${elapsed}s)"
else
    fail "Multiple durations summing out of range (elapsed: ${elapsed}s)"
fi

echo -e "${GREEN}All shell tests passed!${NC}"
