#!/bin/bash
# AutoEq Docker构建脚本
# 使用Docker打包，不依赖本地Python环境
# 支持 Linux x64 可执行文件构建

set -e

echo "=========================================="
echo "   AutoEq Docker 构建脚本"
echo "=========================================="

# 检查Docker是否安装
if ! command -v docker &> /dev/null; then
    echo "❌ 错误: Docker 未安装"
    echo "请先安装 Docker: https://docs.docker.com/get-docker/"
    exit 1
fi

# 检查Docker是否运行
if ! docker info &> /dev/null; then
    echo "❌ 错误: Docker 未运行"
    echo "请启动 Docker Desktop"
    exit 1
fi

# 创建构建容器
echo ""
echo "步骤1: 创建构建容器..."
docker build -f Dockerfile.build -t autoeq-builder .

# 创建临时容器
echo ""
echo "步骤2: 构建可执行文件..."
docker run --rm -v $(pwd)/dist:/dist autoeq-builder

# 验证构建结果
echo ""
echo "步骤3: 验证构建结果..."
if [ -f "dist/autoeq_api" ]; then
    echo "✅ 构建成功!"
    echo ""
    ls -lh dist/autoeq_api
    file dist/autoeq_api
else
    echo "❌ 构建失败!"
    exit 1
fi

echo ""
echo "=========================================="
echo "🎉 构建完成!"
echo "=========================================="
echo ""
echo "可执行文件位置: $(pwd)/dist/autoeq_api"
echo ""
echo "在Linux上运行:"
echo "  chmod +x dist/autoeq_api"
echo "  ./dist/autoeq_api --port 8000"
echo ""