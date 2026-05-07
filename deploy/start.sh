#!/bin/bash
# AutoEq REST API 一键启动脚本
# 使用方法: ./start.sh

set -e

echo "=========================================="
echo "   AutoEq REST API 启动脚本"
echo "=========================================="

# 检查是否为Linux系统
if [ "$(uname)" != "Linux" ]; then
    echo "警告: 建议在Linux系统上运行此服务"
    echo "当前系统: $(uname)"
fi

# 检查Python版本
if ! command -v python3 &> /dev/null; then
    echo "❌ 错误: 未找到 python3，请先安装"
    echo "安装命令: sudo apt-get install python3 python3-pip"
    exit 1
fi

PYTHON_VERSION=$(python3 --version | awk '{print $2}')
echo "✅ Python 版本: $PYTHON_VERSION"

# 检查依赖是否已安装
echo ""
echo "检查依赖..."
if python3 -c "import numpy, scipy, matplotlib, uvicorn, fastapi" &> /dev/null; then
    echo "✅ 所有依赖已安装"
else
    echo "🔄 安装依赖中..."
    pip3 install numpy scipy matplotlib Pillow pyyaml tabulate soundfile tqdm uvicorn fastapi python-multipart
    echo "✅ 依赖安装完成"
fi

# 设置Python路径
export PYTHONPATH="$(pwd):$PYTHONPATH"

# 启动服务
echo ""
echo "🚀 启动 AutoEq REST API..."
echo "📡 服务地址: http://localhost:8000"
echo "📖 API文档: http://localhost:8000/docs"
echo "🔴 按 Ctrl+C 停止服务"
echo ""

python3 autoeq_main.py