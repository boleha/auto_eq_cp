#!/bin/bash
echo "========================================"
echo "AutoEq Docker 镜像构建脚本"
echo "========================================"
echo

echo "[1/3] 检查Docker..."
if ! command -v docker &> /dev/null; then
    echo "[错误] 未检测到Docker"
    echo "安装方法: https://docs.docker.com/get-docker/"
    exit 1
fi

echo "[2/3] 构建Docker镜像..."
if ! docker build -t autoeq-api .; then
    echo "[错误] Docker镜像构建失败"
    exit 1
fi

echo "[3/3] 验证镜像..."
docker images autoeq-api

echo
echo "========================================"
echo "构建成功！"
echo
echo "运行方法:"
echo "  docker run -p 8000:8000 autoeq-api"
echo
echo "指定端口:"
echo "  docker run -p 9000:8000 autoeq-api"
echo
echo "后台运行:"
echo "  docker run -d --name autoeq -p 8000:8000 autoeq-api"
echo
echo "访问地址:"
echo "  http://localhost:8000/docs"
echo "========================================"