#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
AutoEq REST API - 自包含启动脚本
可以直接在Linux上运行，无需复杂安装

使用方法:
    python3 autoeq_main.py
    或 chmod +x autoeq_main.py && ./autoeq_main.py

访问地址:
    - API文档: http://localhost:8000/docs
    - ReDoc: http://localhost:8000/redoc
"""

import sys
import os

# 添加当前目录到Python路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

def main():
    try:
        # 尝试导入并启动服务
        import uvicorn
        from autoeq.rest_api import app
        
        print("🚀 启动 AutoEq REST API...")
        print("📡 服务地址: http://localhost:8000")
        print("📖 API文档: http://localhost:8000/docs")
        print("按 Ctrl+C 停止服务")
        print()
        
        uvicorn.run(app, host="0.0.0.0", port=8000)
        
    except ImportError as e:
        print(f"❌ 缺少依赖: {e}")
        print()
        print("请先安装依赖:")
        print("pip install numpy scipy matplotlib Pillow pyyaml tabulate soundfile tqdm uvicorn fastapi python-multipart")
        sys.exit(1)
    except Exception as e:
        print(f"❌ 启动失败: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()