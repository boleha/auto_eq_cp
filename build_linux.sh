#!/bin/bash
# AutoEq Linux 可执行文件构建脚本
# 使用方法: 在Linux终端执行 ./build_linux.sh
# 要求: Python 3.8+, PyInstaller

set -e

echo "=========================================="
echo "AutoEq Linux 可执行文件构建脚本"
echo "=========================================="

# 检查是否为Linux系统
if [ "$(uname)" != "Linux" ]; then
    echo "错误: 此脚本只能在Linux系统上运行！"
    echo "当前系统: $(uname)"
    exit 1
fi

# 检查Python版本
PYTHON_VERSION=$(python3 --version | awk '{print $2}')
echo "检测到 Python 版本: $PYTHON_VERSION"

# 安装依赖
echo ""
echo "步骤1: 安装系统依赖..."
sudo apt-get update
sudo apt-get install -y python3 python3-dev python3-pip libsndfile1

echo ""
echo "步骤2: 安装Python依赖..."
python3 -m pip install --upgrade pip
python3 -m pip install -e .
python3 -m pip install uvicorn fastapi python-multipart pyinstaller

echo ""
echo "步骤3: 使用PyInstaller构建可执行文件..."
python3 -m PyInstaller --onefile --name autoeq_api main.py

echo ""
echo "步骤4: 验证构建结果..."
if [ -f "dist/autoeq_api" ]; then
    echo "✅ 构建成功！"
    echo "可执行文件位置: $(pwd)/dist/autoeq_api"
    echo ""
    echo "测试运行:"
    ./dist/autoeq_api --help 2>&1 || echo "服务已启动方式运行"
else
    echo "❌ 构建失败！"
    exit 1
fi

echo ""
echo "=========================================="
echo "构建完成！"
echo "运行命令: ./dist/autoeq_api"
echo "访问地址: http://localhost:8000"
echo "API文档: http://localhost:8000/docs"
echo "=========================================="