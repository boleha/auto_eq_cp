# API测试脚本
# 用于验证Docker镜像构建后的API功能

import requests
import json

API_URL = "http://localhost:8000"

def test_root():
    print("测试1: 首页接口...")
    r = requests.get(f"{API_URL}/")
    assert r.status_code == 200
    data = r.json()
    assert "version" in data
    print(f"  ✅ 通过 - 版本: {data['version']}")

def test_equalize():
    print("测试2: 均衡化接口...")
    payload = {
        "frequency": [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000],
        "raw": [3, 5, 4, 2, 1, 0, -1, -3, -5, -8],
        "name": "test"
    }
    r = requests.post(f"{API_URL}/equalize", json=payload)
    assert r.status_code == 200
    data = r.json()
    assert "frequency" in data
    assert "equalization" in data
    print(f"  ✅ 通过 - 返回{len(data['frequency'])}个频率点")

def test_parametric_eq():
    print("测试3: 参数均衡器接口...")
    payload = {
        "frequency": [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000],
        "raw": [3, 5, 4, 2, 1, 0, -1, -3, -5, -8],
        "name": "test"
    }
    r = requests.post(f"{API_URL}/parametric-eq", json=payload)
    assert r.status_code == 200
    data = r.json()
    assert "filters" in data
    assert "preamp" in data
    print(f"  ✅ 通过 - 生成{len(data['filters'])}个滤波器")

if __name__ == "__main__":
    print("=" * 50)
    print("AutoEq API 测试")
    print("=" * 50)
    test_root()
    test_equalize()
    test_parametric_eq()
    print("=" * 50)
    print("所有测试通过！")
    print("=" * 50)