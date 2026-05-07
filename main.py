# -*- coding: utf-8 -*-
"""
AutoEq REST API 启动入口

使用方法:
    python main.py
    python main.py --port 8080
    python main.py --host 0.0.0.0 --port 8080

访问地址:
    - API文档: http://localhost:8000/docs (Swagger UI)
    - ReDoc: http://localhost:8000/redoc
"""

import argparse
import uvicorn

def main():
    parser = argparse.ArgumentParser(description='AutoEq REST API')
    parser.add_argument('--host', type=str, default='0.0.0.0', help='Host address (default: 0.0.0.0)')
    parser.add_argument('--port', type=int, default=8000, help='Port number (default: 8000)')
    parser.add_argument('--reload', action='store_true', default=False, help='Enable auto-reload')
    
    args = parser.parse_args()
    
    uvicorn.run(
        "autoeq.rest_api:app",
        host=args.host,
        port=args.port,
        reload=args.reload
    )

if __name__ == "__main__":
    main()