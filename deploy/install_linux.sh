#!/bin/bash
# AutoEq Linux 完整安装脚本
# 自动安装Python环境和所有依赖

set -e

echo "=========================================="
echo "   AutoEq Linux 完整安装脚本"
echo "=========================================="

# 检查是否为root用户
if [ "$(id -u)" != "0" ]; then
    echo "⚠️  建议使用root权限运行此脚本"
    echo "   sudo $0"
    echo ""
fi

# 更新系统
echo "步骤1: 更新系统..."
sudo apt-get update -y

# 安装系统依赖
echo ""
echo "步骤2: 安装系统依赖..."
sudo apt-get install -y \
    python3 \
    python3-dev \
    python3-pip \
    python3-venv \
    libsndfile1 \
    libopenblas-dev \
    gfortran \
    git

# 创建虚拟环境
echo ""
echo "步骤3: 创建Python虚拟环境..."
python3 -m venv autoeq_env
source autoeq_env/bin/activate

# 升级pip
echo ""
echo "步骤4: 升级pip..."
pip install --upgrade pip

# 安装Python依赖
echo ""
echo "步骤5: 安装Python依赖..."
pip install numpy scipy matplotlib Pillow pyyaml tabulate soundfile tqdm uvicorn fastapi python-multipart

# 验证安装
echo ""
echo "步骤6: 验证安装..."
if python -c "import autoeq, uvicorn, fastapi; print('✅ 所有依赖安装成功')"; then
    echo ""
    echo "=========================================="
    echo "🎉 安装完成！"
    echo "=========================================="
    echo ""
    echo "启动命令:"
    echo "  cd deploy"
    echo "  source autoeq_env/bin/activate"
    echo "  python autoeq_main.py"
    echo ""
    echo "访问地址:"
    echo "  API服务: http://localhost:8000"
    echo "  API文档: http://localhost:8000/docs"
else
    echo "❌ 安装失败"
    exit 1
fi