#!/bin/bash
# DataState 测试运行脚本

cd "$(dirname "$0")"

echo "=== 运行 DataState 测试 ==="

# 运行 cargo test
cargo test --package localdb --lib datastate -- --nocapture

echo ""
echo "=== 测试完成 ==="