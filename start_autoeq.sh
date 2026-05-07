#!/bin/bash
# AutoEq REST API 启动脚本 (Linux)
# 使用方法: ./start_autoeq.sh

echo "启动 AutoEq REST API..."

# 检查Python是否安装
if ! command -v python3 &> /dev/null; then
    echo "错误: 未找到 python3，请先安装 Python 3.8+"
    exit 1
fi

# 检查是否已安装依赖
if ! python3 -c "import autoeq, uvicorn, fastapi" &> /dev/null; then
    echo "正在安装依赖..."
    python3 -m pip install -e .
    python3 -m pip install uvicorn fastapi python-multipart
fi

# 启动服务
echo "启动 API 服务..."
python3 -m uvicorn autoeq.rest_api:app --host 0.0.0.0 --port 8000

echo "服务已停止"