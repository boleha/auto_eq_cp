@echo off
echo ========================================
echo AutoEq Docker 镜像构建脚本
echo ========================================
echo.

echo [1/3] 检查Docker...
docker --version >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] 未检测到Docker，请先安装Docker Desktop
    echo 下载地址: https://www.docker.com/products/docker-desktop/
    exit /b 1
)

echo [2/3] 构建Docker镜像...
docker build -t autoeq-api .
if %errorlevel% neq 0 (
    echo [错误] Docker镜像构建失败
    exit /b 1
)

echo [3/3] 验证镜像...
docker images autoeq-api

echo.
echo ========================================
echo 构建成功！
echo.
echo 运行方法:
echo   docker run -p 8000:8000 autoeq-api
echo.
echo 指定端口:
echo   docker run -p 9000:8000 autoeq-api
echo.
echo 后台运行:
echo   docker run -d --name autoeq -p 8000:8000 autoeq-api
echo.
echo 访问地址:
echo   http://localhost:8000/docs
echo ========================================