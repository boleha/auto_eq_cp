# AutoEq Linux 部署包

这是一个完整的 AutoEq REST API 部署包，可直接在 Linux 服务器上部署运行。

## 快速开始

### 方法一：一键启动（推荐）

```bash
# 进入部署目录
cd deploy

# 赋予脚本执行权限
chmod +x start.sh

# 启动服务
./start.sh
```

### 方法二：手动启动

```bash
# 进入部署目录
cd deploy

# 设置Python路径
export PYTHONPATH="$(pwd):$PYTHONPATH"

# 安装依赖（首次运行）
pip3 install numpy scipy matplotlib Pillow pyyaml tabulate soundfile tqdm uvicorn fastapi python-multipart

# 启动服务
python3 autoeq_main.py
```

## 访问地址

- **API服务**: http://localhost:8000
- **API文档**: http://localhost:8000/docs
- **ReDoc文档**: http://localhost:8000/redoc

## 接口说明

### 1. 均衡化处理
```bash
POST /equalize
Content-Type: application/json

{
    "frequency": [20, 50, 200, 1000, 3000, 10000, 20000],
    "raw": [2, 2, 0, 1, 10, 0, -15],
    "name": "headphone"
}
```

### 2. 生成参数均衡器
```bash
POST /parametric-eq
Content-Type: application/json

{
    "frequency": [20, 50, 200, 1000, 3000, 10000, 20000],
    "raw": [2, 2, 0, 1, 10, 0, -15],
    "config": "8_PEAKING_WITH_SHELVES"
}
```

### 3. 获取配置列表
```bash
GET /configs
```

## 文件结构

```
deploy/
├── autoeq/                    # AutoEq核心模块
│   ├── __init__.py
│   ├── __main__.py
│   ├── api.py
│   ├── batch_processing.py
│   ├── constants.py
│   ├── csv.py
│   ├── frequency_response.py
│   ├── peq.py
│   ├── rest_api.py
│   └── utils.py
├── autoeq_main.py             # 主启动文件
├── start.sh                   # 一键启动脚本
└── README.md                  # 说明文档
```

## 系统要求

- Linux 操作系统
- Python 3.8 或更高版本
- 推荐配置：2GB以上内存

## 停止服务

按 `Ctrl+C` 停止服务。

## 常见问题

### Q: 缺少 libsndfile
```bash
sudo apt-get install libsndfile1
```

### Q: 端口被占用
修改 `autoeq_main.py` 中的端口号，或释放端口。

### Q: 权限问题
```bash
chmod -R 755 deploy/
```