# -*- coding: utf-8 -*-
"""
AutoEq REST API - 自动耳机均衡器配置生成接口

功能说明:
- 将耳机频响曲线与目标曲线匹配，生成均衡器参数
- 支持参数均衡器(PEQ)、图形均衡器、卷积均衡器
- 用于音频后期处理、耳机调音等场景

使用方法:
    python autoeq/rest_api.py
    或打包成exe后直接运行

访问地址:
- API文档: http://localhost:8000/docs (Swagger UI)
- ReDoc: http://localhost:8000/redoc
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from fastapi import FastAPI, HTTPException, UploadFile, File, Form
from pydantic import BaseModel, Field
from typing import Optional, List
import tempfile
import numpy as np
import uuid

from autoeq.frequency_response import FrequencyResponse
from autoeq.constants import DEFAULT_FS, DEFAULT_MAX_GAIN, DEFAULT_PREAMP, \
    DEFAULT_BASS_BOOST_GAIN, DEFAULT_BASS_BOOST_FC, DEFAULT_BASS_BOOST_Q, DEFAULT_TREBLE_BOOST_GAIN, \
    DEFAULT_TREBLE_BOOST_FC, DEFAULT_TREBLE_BOOST_Q, DEFAULT_TILT, PEQ_CONFIGS

app = FastAPI(
    title="AutoEq API",
    description="自动耳机均衡器配置生成器 - 将耳机频响曲线匹配到目标曲线，生成均衡器参数"
)

frequency_response_storage = {}
target_curve_storage = {}


class FrequencyResponseInput(BaseModel):
    frequency: List[float] = Field(..., description="频率点数组，单位Hz")
    raw: List[float] = Field(..., description="对应频率的增益值数组，单位dB")
    name: str = Field(default="headphone", description="耳机名称标识")


class FilterOutput(BaseModel):
    type: str
    fc: float
    gain: float
    q: float


class ParametricEqOutput(BaseModel):
    preamp: float
    filters: List[FilterOutput]


class EqualizationResult(BaseModel):
    name: str
    frequency: List[float]
    raw: List[float]
    smoothed: List[float]
    equalization: List[float]
    target: List[float]
    error: List[float]
    parametric_eq: Optional[ParametricEqOutput] = None


class OptimizeRequest(BaseModel):
    frequency: List[float]
    raw: List[float]
    name: str = "headphone"
    target_curve_id: Optional[str] = None
    fs: int = DEFAULT_FS
    config: str = "8_PEAKING_WITH_SHELVES"
    preamp: float = DEFAULT_PREAMP


class FrequencyResponseWithId(BaseModel):
    frequency: List[float]
    raw: List[float]
    name: str = "headphone"


class TargetCurveWithId(BaseModel):
    frequency: List[float]
    raw: List[float]
    name: str = "target"


@app.get("/", summary="首页")
async def root():
    return {
        "message": "AutoEq API - 自动耳机均衡器配置生成器",
        "version": "4.1.2",
        "docs": "/docs",
        "description": "将耳机频响曲线匹配到目标曲线，生成参数均衡器、图形均衡器、卷积均衡器配置"
    }


@app.get("/configs", summary="获取PEQ配置列表")
async def list_configs():
    return {"configs": list(PEQ_CONFIGS.keys())}


@app.post("/frequency-response", summary="上传频响数据")
async def upload_frequency_response(data: FrequencyResponseWithId):
    fr_id = str(uuid.uuid4())
    frequency_response_storage[fr_id] = {
        'name': data.name,
        'frequency': np.array(data.frequency),
        'raw': np.array(data.raw)
    }
    return {"id": fr_id, "name": data.name}


@app.get("/frequency-response/{fr_id}", summary="获取频响数据")
async def get_frequency_response(fr_id: str):
    if fr_id not in frequency_response_storage:
        raise HTTPException(status_code=404, detail="频响数据不存在")
    data = frequency_response_storage[fr_id]
    return {
        "id": fr_id,
        "name": data['name'],
        "frequency": data['frequency'].tolist(),
        "raw": data['raw'].tolist()
    }


@app.delete("/frequency-response/{fr_id}", summary="删除频响数据")
async def delete_frequency_response(fr_id: str):
    if fr_id not in frequency_response_storage:
        raise HTTPException(status_code=404, detail="频响数据不存在")
    del frequency_response_storage[fr_id]
    return {"message": "删除成功"}


@app.post("/target-curve", summary="上传目标曲线")
async def upload_target_curve(data: TargetCurveWithId):
    target_id = str(uuid.uuid4())
    target_curve_storage[target_id] = {
        'name': data.name,
        'frequency': np.array(data.frequency),
        'raw': np.array(data.raw)
    }
    return {"id": target_id, "name": data.name}


@app.get("/target-curve/{target_id}", summary="获取目标曲线")
async def get_target_curve(target_id: str):
    if target_id not in target_curve_storage:
        raise HTTPException(status_code=404, detail="目标曲线不存在")
    data = target_curve_storage[target_id]
    return {
        "id": target_id,
        "name": data['name'],
        "frequency": data['frequency'].tolist(),
        "raw": data['raw'].tolist()
    }


@app.delete("/target-curve/{target_id}", summary="删除目标曲线")
async def delete_target_curve(target_id: str):
    if target_id not in target_curve_storage:
        raise HTTPException(status_code=404, detail="目标曲线不存在")
    del target_curve_storage[target_id]
    return {"message": "删除成功"}


@app.post("/equalize", response_model=EqualizationResult, summary="完整均衡化处理")
async def equalize(data: FrequencyResponseInput, target_curve_id: Optional[str] = None):
    try:
        fr = FrequencyResponse(
            name=data.name,
            frequency=np.array(data.frequency),
            raw=np.array(data.raw)
        )
        fr.interpolate()
        fr.center()

        if target_curve_id and target_curve_id in target_curve_storage:
            target_data = target_curve_storage[target_curve_id]
            target = FrequencyResponse(
                name=target_data['name'],
                frequency=target_data['frequency'].copy(),
                raw=target_data['raw'].copy()
            )
        else:
            target = FrequencyResponse(
                name='flat_target',
                frequency=fr.frequency.copy(),
                raw=np.zeros(len(fr.frequency))
            )
        target.interpolate()
        target.center()

        fr.process(
            target=target,
            bass_boost_gain=DEFAULT_BASS_BOOST_GAIN,
            bass_boost_fc=DEFAULT_BASS_BOOST_FC,
            bass_boost_q=DEFAULT_BASS_BOOST_Q,
            treble_boost_gain=DEFAULT_TREBLE_BOOST_GAIN,
            treble_boost_fc=DEFAULT_TREBLE_BOOST_FC,
            treble_boost_q=DEFAULT_TREBLE_BOOST_Q,
            tilt=DEFAULT_TILT,
            fs=DEFAULT_FS,
            max_gain=DEFAULT_MAX_GAIN,
        )

        result = {
            'name': fr.name,
            'frequency': fr.frequency.tolist(),
            'raw': fr.raw.tolist(),
            'smoothed': fr.smoothed.tolist() if len(fr.smoothed) else [],
            'equalization': fr.equalization.tolist() if len(fr.equalization) else [],
            'target': fr.target.tolist() if len(fr.target) else [],
            'error': fr.error.tolist() if len(fr.error) else [],
        }

        return result

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/parametric-eq", response_model=ParametricEqOutput, summary="生成参数均衡器")
async def optimize_parametric_eq(request: OptimizeRequest):
    try:
        fr = FrequencyResponse(
            name=request.name,
            frequency=np.array(request.frequency),
            raw=np.array(request.raw)
        )
        fr.interpolate()
        fr.center()

        if request.target_curve_id and request.target_curve_id in target_curve_storage:
            target_data = target_curve_storage[request.target_curve_id]
            target = FrequencyResponse(
                name=target_data['name'],
                frequency=target_data['frequency'].copy(),
                raw=target_data['raw'].copy()
            )
        else:
            target = FrequencyResponse(
                name='flat_target',
                frequency=fr.frequency.copy(),
                raw=np.zeros(len(fr.frequency))
            )
        target.interpolate()
        target.center()

        fr.process(
            target=target,
            bass_boost_gain=DEFAULT_BASS_BOOST_GAIN,
            bass_boost_fc=DEFAULT_BASS_BOOST_FC,
            bass_boost_q=DEFAULT_BASS_BOOST_Q,
            treble_boost_gain=DEFAULT_TREBLE_BOOST_GAIN,
            treble_boost_fc=DEFAULT_TREBLE_BOOST_FC,
            treble_boost_q=DEFAULT_TREBLE_BOOST_Q,
            tilt=DEFAULT_TILT,
            fs=request.fs,
            max_gain=DEFAULT_MAX_GAIN,
        )

        peq_config = PEQ_CONFIGS.get(request.config, PEQ_CONFIGS['8_PEAKING_WITH_SHELVES'])
        peqs = fr.optimize_parametric_eq(peq_config, request.fs, preamp=request.preamp)

        return {
            'preamp': -max([p.max_gain for p in peqs]) if peqs else request.preamp,
            'filters': [
                {
                    'type': filt.__class__.__name__,
                    'fc': filt.fc,
                    'gain': filt.gain,
                    'q': filt.q
                }
                for peq in peqs
                for filt in peq.filters
            ]
        }

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/graphic-eq-string", summary="生成图形均衡器字符串")
async def generate_graphic_eq_string(data: FrequencyResponseInput, target_curve_id: Optional[str] = None):
    try:
        fr = FrequencyResponse(
            name=data.name,
            frequency=np.array(data.frequency),
            raw=np.array(data.raw)
        )
        fr.interpolate()
        fr.center()

        if target_curve_id and target_curve_id in target_curve_storage:
            target_data = target_curve_storage[target_curve_id]
            target = FrequencyResponse(
                name=target_data['name'],
                frequency=target_data['frequency'].copy(),
                raw=target_data['raw'].copy()
            )
        else:
            target = FrequencyResponse(
                name='flat_target',
                frequency=fr.frequency.copy(),
                raw=np.zeros(len(fr.frequency))
            )
        target.interpolate()
        target.center()

        fr.process(
            target=target,
            bass_boost_gain=DEFAULT_BASS_BOOST_GAIN,
            bass_boost_fc=DEFAULT_BASS_BOOST_FC,
            bass_boost_q=DEFAULT_BASS_BOOST_Q,
            treble_boost_gain=DEFAULT_TREBLE_BOOST_GAIN,
            treble_boost_fc=DEFAULT_TREBLE_BOOST_FC,
            treble_boost_q=DEFAULT_TREBLE_BOOST_Q,
            tilt=DEFAULT_TILT,
            fs=DEFAULT_FS,
            max_gain=DEFAULT_MAX_GAIN,
        )

        return {
            'graphic_eq_string': fr.eqapo_graphic_eq(normalize=True, preamp=DEFAULT_PREAMP)
        }

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/convolution-eq", summary="生成卷积均衡器脉冲响应")
async def generate_convolution_eq(data: FrequencyResponseInput, fs: int = DEFAULT_FS, target_curve_id: Optional[str] = None):
    try:
        fr = FrequencyResponse(
            name=data.name,
            frequency=np.array(data.frequency),
            raw=np.array(data.raw)
        )
        fr.interpolate()
        fr.center()

        if target_curve_id and target_curve_id in target_curve_storage:
            target_data = target_curve_storage[target_curve_id]
            target = FrequencyResponse(
                name=target_data['name'],
                frequency=target_data['frequency'].copy(),
                raw=target_data['raw'].copy()
            )
        else:
            target = FrequencyResponse(
                name='flat_target',
                frequency=fr.frequency.copy(),
                raw=np.zeros(len(fr.frequency))
            )
        target.interpolate()
        target.center()

        fr.process(
            target=target,
            bass_boost_gain=DEFAULT_BASS_BOOST_GAIN,
            bass_boost_fc=DEFAULT_BASS_BOOST_FC,
            bass_boost_q=DEFAULT_BASS_BOOST_Q,
            treble_boost_gain=DEFAULT_TREBLE_BOOST_GAIN,
            treble_boost_fc=DEFAULT_TREBLE_BOOST_FC,
            treble_boost_q=DEFAULT_TREBLE_BOOST_Q,
            tilt=DEFAULT_TILT,
            fs=fs,
            max_gain=DEFAULT_MAX_GAIN,
        )

        ir = fr.minimum_phase_impulse_response(fs=fs, normalize=True, preamp=DEFAULT_PREAMP)

        return {
            'impulse_response': ir.tolist(),
            'length': len(ir),
            'sample_rate': fs
        }

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/equalize-file", summary="上传CSV文件进行均衡化")
async def equalize_csv_file(
    file: UploadFile = File(..., description="耳机频响测量CSV文件"),
    target_file: Optional[UploadFile] = File(None, description="目标曲线CSV文件（可选）"),
    bass_boost_gain: float = Form(DEFAULT_BASS_BOOST_GAIN, description="低音增强量，单位dB"),
    bass_boost_fc: float = Form(DEFAULT_BASS_BOOST_FC, description="低音增强中心频率，单位Hz"),
    bass_boost_q: float = Form(DEFAULT_BASS_BOOST_Q, description="低音增强Q值"),
    treble_boost_gain: float = Form(DEFAULT_TREBLE_BOOST_GAIN, description="高音增强量，单位dB"),
    treble_boost_fc: float = Form(DEFAULT_TREBLE_BOOST_FC, description="高音增强中心频率，单位Hz"),
    treble_boost_q: float = Form(DEFAULT_TREBLE_BOOST_Q, description="高音增强Q值"),
    tilt: float = Form(DEFAULT_TILT, description="频响倾斜度，单位dB/倍频程"),
    fs: int = Form(DEFAULT_FS, description="采样率，单位Hz"),
    max_gain: float = Form(DEFAULT_MAX_GAIN, description="最大增益限制，单位dB"),
    preamp: float = Form(DEFAULT_PREAMP, description="前置增益，单位dB"),
    config: str = Form("8_PEAKING_WITH_SHELVES", description="参数均衡器配置名称")
):
    try:
        original_filename = file.filename or "headphone"
        with tempfile.NamedTemporaryFile(delete=False, suffix='.csv') as tmp:
            content = await file.read()
            tmp.write(content)
            tmp_path = tmp.name

        target_path = None
        if target_file:
            with tempfile.NamedTemporaryFile(delete=False, suffix='.csv') as tmp:
                content = await target_file.read()
                tmp.write(content)
                target_path = tmp.name

        try:
            fr = FrequencyResponse.read_csv(tmp_path)
            fr.name = original_filename.replace('.csv', '')
            fr.interpolate()
            fr.center()

            if target_path:
                target_fr = FrequencyResponse.read_csv(target_path)
                target = FrequencyResponse(
                    name=target_fr.name,
                    frequency=target_fr.frequency.copy(),
                    raw=target_fr.raw.copy()
                )
            else:
                target = FrequencyResponse(
                    name='flat_target',
                    frequency=fr.frequency.copy(),
                    raw=np.zeros(len(fr.frequency))
                )
            target.interpolate()
            target.center()

            fr.process(
                target=target,
                bass_boost_gain=bass_boost_gain,
                bass_boost_fc=bass_boost_fc,
                bass_boost_q=bass_boost_q,
                treble_boost_gain=treble_boost_gain,
                treble_boost_fc=treble_boost_fc,
                treble_boost_q=treble_boost_q,
                tilt=tilt,
                fs=fs,
                max_gain=max_gain,
            )

            peq_config = PEQ_CONFIGS.get(config, PEQ_CONFIGS['8_PEAKING_WITH_SHELVES'])
            peqs = fr.optimize_parametric_eq(peq_config, fs, preamp=preamp)

            return {
                'name': fr.name,
                'frequency': fr.frequency.tolist(),
                'raw': fr.raw.tolist(),
                'smoothed': fr.smoothed.tolist() if len(fr.smoothed) else [],
                'equalization': fr.equalization.tolist() if len(fr.equalization) else [],
                'target': fr.target.tolist() if len(fr.target) else [],
                'error': fr.error.tolist() if len(fr.error) else [],
                'parametric_eq': {
                    'preamp': -max([p.max_gain for p in peqs]) if peqs else preamp,
                    'filters': [
                        {
                            'type': filt.__class__.__name__,
                            'fc': filt.fc,
                            'gain': filt.gain,
                            'q': filt.q
                        }
                        for peq in peqs
                        for filt in peq.filters
                    ]
                },
                'graphic_eq_string': fr.eqapo_graphic_eq(normalize=True, preamp=preamp)
            }
        finally:
            os.unlink(tmp_path)
            if target_path:
                os.unlink(target_path)

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
