@echo off
REM AutoEq Docker 构建脚本 (Windows)
REM 使用Docker打包，不依赖本地Python环境

echo ==========================================
echo    AutoEq Docker 构建脚本
echo ==========================================

REM 检查Docker是否安装
docker --version >nul 2>&1
if errorlevel 1 (
    echo ERROR: Docker 未安装
    echo 请先安装 Docker Desktop: https://docs.docker.com/desktop/install/windows-install/
    exit /b 1
)

REM 检查Docker是否运行
docker info >nul 2>&1
if errorlevel 1 (
    echo ERROR: Docker 未运行
    echo 请启动 Docker Desktop
    exit /b 1
)

REM 创建构建镜像
echo.
echo 步骤1: 创建构建镜像...
docker build -f Dockerfile.build -t autoeq-builder .

if errorlevel 1 (
    echo ERROR: 镜像构建失败
    exit /b 1
)

REM 创建dist目录
if not exist dist mkdir dist

REM 构建可执行文件
echo.
echo 步骤2: 构建可执行文件...
docker run --rm -v "%CD%\dist:/dist" autoeq-builder

if errorlevel 1 (
    echo ERROR: 构建失败
    exit /b 1
)

REM 验证构建结果
echo.
echo 步骤3: 验证构建结果...
if exist "dist\autoeq_api.exe" (
    echo 构建成功!
    dir dist\autoeq_api.exe
) else if exist "dist\autoeq_api" (
    echo 构建成功!
    dir dist\autoeq_api
) else (
    echo ERROR: 构建失败，未找到可执行文件
    exit /b 1
)

echo.
echo ==========================================
echo    构建完成!
echo ==========================================
echo.
echo 可执行文件位置: %CD%\dist\
echo.
echo 在Linux上运行:
echo   chmod +x autoeq_api
echo   ./autoeq_api --port 8000
echo.
pause