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

from copy import deepcopy

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
from scipy.signal import find_peaks


def _adaptive_peak_config(target, frequency, n_filters):
    """从目标曲线提取 n 个最显著峰/谷频点，构造固定 fc 配置（只优化 gain/q）。
    任意曲线都自适应，保证全频段贴合。"""
    import numpy as np
    freq = np.asarray(frequency)
    tgt = np.asarray(target)
    m = (freq >= 20) & (freq <= 18000)
    f, t = freq[m], tgt[m]
    if len(f) < 3:
        return None

    peaks_p, _ = find_peaks(t, prominence=0.5)
    peaks_n, _ = find_peaks(-t, prominence=0.5)
    cand = [(ix, t[ix]) for ix in peaks_p] + [(ix, -t[ix]) for ix in peaks_n]
    cand.sort(key=lambda x: -abs(x[1]))

    picked = []
    for ix, h in cand:
        if len(picked) >= n_filters:
            break
        ok = True
        for pix in picked:
            ratio = max(f[ix], f[pix]) / min(f[ix], f[pix])
            if np.log2(ratio) < 0.3:
                ok = False
                break
        if ok:
            picked.append(ix)

    if len(picked) < n_filters:
        # 不足 n 个：用对数均匀补足（25Hz-18kHz）
        extra = [25.0 * (18000 / 25.0) ** (i / (n_filters - 1)) for i in range(n_filters)] if n_filters > 1 else [1000.0]
        picked_f = sorted(f[i] for i in picked)
        for fc in extra:
            if len(picked_f) >= n_filters:
                break
            if all(np.log2(max(fc, pf) / min(fc, pf)) >= 0.3 for pf in picked_f):
                picked_f.append(fc)
        return {'filters': [{'type': 'PEAKING', 'fc': round(float(fc), 1)} for fc in sorted(picked_f)]}

    return {'filters': [{'type': 'PEAKING', 'fc': round(float(f[i]), 1)} for i in sorted(picked)]}

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


class EqRangeInput(BaseModel):
    frequency: List[float] = Field(..., description="频率点数组，单位Hz")
    raw: List[float] = Field(..., description="对应频率的增益值数组，单位dB")


class EqRange(BaseModel):
    low: float = Field(default=20, description="EQ优化频率下限，单位Hz")
    high: float = Field(default=20000, description="EQ优化频率上限，单位Hz")


class GainRange(BaseModel):
    low: Optional[float] = Field(default=None, description="增益下限，单位dB，不设置则不限制")
    high: Optional[float] = Field(default=None, description="增益上限，单位dB，不设置则不限制")


class QRange(BaseModel):
    low: Optional[float] = Field(default=None, description="Q值下限，不设置则不限制")
    high: Optional[float] = Field(default=None, description="Q值上限，不设置则不限制")


class EqRangeRequest(BaseModel):
    select: EqRangeInput = Field(..., description="耳机实测频响曲线")
    target: EqRangeInput = Field(..., description="目标频响曲线")
    eq_range: EqRange = Field(default_factory=EqRange, description="EQ优化频率范围")
    fs: int = Field(default=DEFAULT_FS, description="采样率，单位Hz")
    config: str = Field(default="8_PEAKING_WITH_SHELVES", description="参数均衡器配置名称")
    preamp: float = Field(default=DEFAULT_PREAMP, description="前置增益，单位dB")
    max_filters: Optional[int] = Field(default=None, description="最大滤波器数量，不设置则返回所有匹配的滤波器")
    gain_range: Optional[GainRange] = Field(default=None, description="增益过滤范围")
    q_range: Optional[QRange] = Field(default=None, description="Q值过滤范围")
    window_size: float = Field(default=1/24, description="平滑窗口大小（倍频程），越小越贴合原始曲线")
    treble_window_size: float = Field(default=1.0, description="高频平滑窗口大小，越小高频越贴合")
    treble_f_lower: float = Field(default=6000.0, description="高频处理下限频率，Hz")
    treble_f_upper: float = Field(default=8000.0, description="高频处理上限频率，Hz")
    treble_gain_k: float = Field(default=1.0, description="高频增益系数，>1更激进，<1更保守")
    max_gain: float = Field(default=20.0, description="最大增益限制，dB")
    max_slope: float = Field(default=18.0, description="EQ曲线最大斜率限制，dB/倍频程")
    tilt: float = Field(default=0.0, description="目标曲线倾斜度，正数=温暖，负数=明亮，dB/倍频程")
    bass_boost_gain: float = Field(default=0.0, description="低音增强量，dB")
    bass_boost_fc: float = Field(default=105.0, description="低音增强中心频率，Hz")
    bass_boost_q: float = Field(default=0.7, description="低音增强Q值")
    treble_boost_gain: float = Field(default=0.0, description="高音增强量，dB")
    treble_boost_fc: float = Field(default=10000.0, description="高音增强中心频率，Hz")
    treble_boost_q: float = Field(default=0.7, description="高音增强Q值")
    min_mean_error: bool = Field(default=False, description="最小化平均误差，避免1kHz处偏差影响整体")


class EqRangeFilterOutput(BaseModel):
    type: str = Field(..., description="滤波器类型: LowShelf / HighShelf / Peaking")
    fc: float = Field(..., description="中心频率，单位Hz")
    gain: float = Field(..., description="增益，单位dB")
    q: float = Field(..., description="品质因数Q")


class EqRangeResponse(BaseModel):
    preamp: float = Field(..., description="前置增益，单位dB")
    filters: List[EqRangeFilterOutput] = Field(..., description="滤波器参数列表")
    eq_range: EqRange = Field(..., description="EQ优化频率范围")
    gain_range: Optional[GainRange] = Field(default=None, description="增益过滤范围")
    q_range: Optional[QRange] = Field(default=None, description="Q值过滤范围")
    fs: int = Field(..., description="采样率，单位Hz")
    max_filters: Optional[int] = Field(default=None, description="最大滤波器数量限制")
    params: dict = Field(..., description="使用的处理参数")


@app.post("/eq-by-range", response_model=EqRangeResponse, summary="按频率范围生成参数均衡器")
async def eq_by_range(request: EqRangeRequest):
    try:
        fr = FrequencyResponse(
            name='select',
            frequency=np.array(request.select.frequency),
            raw=np.array(request.select.raw)
        )
        fr.interpolate()
        fr.center()

        target = FrequencyResponse(
            name='target',
            frequency=np.array(request.target.frequency),
            raw=np.array(request.target.raw)
        )
        target.interpolate()
        target.center()

        fr.process(
            target=target,
            bass_boost_gain=request.bass_boost_gain,
            bass_boost_fc=request.bass_boost_fc,
            bass_boost_q=request.bass_boost_q,
            treble_boost_gain=request.treble_boost_gain,
            treble_boost_fc=request.treble_boost_fc,
            treble_boost_q=request.treble_boost_q,
            tilt=request.tilt,
            fs=request.fs,
            max_gain=request.max_gain,
            max_slope=request.max_slope,
            window_size=request.window_size,
            treble_window_size=request.treble_window_size,
            treble_f_lower=request.treble_f_lower,
            treble_f_upper=request.treble_f_upper,
            treble_gain_k=request.treble_gain_k,
            min_mean_error=request.min_mean_error,
        )

        # 配置选择：max_filters≤5 用 5_PEAKING（5 个 fc 全自由重新优化），
        # 避免"10 个滤波器解截断成 5 个"导致曲线不贴合。
        effective_config = request.config
        if request.config == '10_PEAKING' and request.max_filters is not None and request.max_filters <= 5:
            effective_config = '5_PEAKING'
        peq_config = PEQ_CONFIGS.get(effective_config, PEQ_CONFIGS['8_PEAKING_WITH_SHELVES'])

        # PEAKING / SHELVES 按请求的 max_filters 和 eq_range 动态分段：
        # 每个滤波器在自己的对数频段内自由寻找最佳 fc/q/gain，
        # 而不是先用全频配置计算后再把区间外滤波器删掉。
        peaking_configs = {'5_PEAKING', '8_PEAKING', '10_PEAKING'}
        shelf_configs = {
            'FIXED_5_WITH_SHELVES',
            'FIXED_8_WITH_SHELVES',
            'FIXED_10_WITH_SHELVES',
            '8_PEAKING_WITH_SHELVES',
        }
        dynamic_configs = peaking_configs | shelf_configs
        if effective_config in dynamic_configs and request.max_filters is not None:
            count = max(1, int(request.max_filters))
            band_low = max(20.0, float(request.eq_range.low))
            band_high = min(20000.0, float(request.eq_range.high))
            if band_high <= band_low:
                raise HTTPException(status_code=400, detail='eq_range.high 必须大于 eq_range.low')
            treble_start = min(max(float(request.treble_f_lower), band_low), band_high)

            def log_edges(low, high, amount):
                if amount <= 0 or high <= low:
                    return []
                ratio = (high / low) ** (1 / amount)
                return [low * ratio ** i for i in range(amount + 1)]

            # 高频加密：预留约 30% 的滤波器给 treble 区域，避免所有滤波器
            # 都集中在低中频。5/8/10 个滤波器分别约为 3+2、5+3、7+3。
            if treble_start < band_high and count >= 3:
                high_count = min(count - 1, max(2, int(np.ceil(count * 0.3))))
                low_count = count - high_count
                low_edges = log_edges(band_low, treble_start, low_count)
                high_edges = log_edges(treble_start, band_high, high_count)
                edges = low_edges[:-1] + high_edges
            else:
                edges = log_edges(band_low, band_high, count)

            if effective_config in peaking_configs:
                peq_config = deepcopy(PEQ_CONFIGS['5_PEAKING'])
                peq_config['filters'] = [
                    {'type': 'PEAKING', 'min_fc': edges[i], 'max_fc': edges[i + 1]}
                    for i in range(count)
                ]
            elif count >= 3:
                # shelf 配置：第一个区间放 LowShelf，最后一个区间放
                # HighShelf，中间区间放 Peaking，所有 fc 都在自己的区间内优化。
                peq_config = deepcopy(PEQ_CONFIGS[effective_config])
                peq_config.setdefault('optimizer', {})['banded_visual'] = True
                peq_config['filters'] = [
                    {
                        'type': 'LOW_SHELF' if i == 0 else 'HIGH_SHELF' if i == count - 1 else 'PEAKING',
                        'min_fc': edges[i],
                        'max_fc': edges[i + 1],
                    }
                    for i in range(count)
                ]

        # AUTO：对数均匀频点（25Hz-18kHz，n 个），保证全频段覆盖且高频贴合。
        # （峰谷自适应会把低频峰谷全占满导致高频没滤波器，对数均匀最稳）
        if effective_config == 'AUTO':
            n_filters = request.max_filters if request.max_filters is not None else 8
            n = max(2, n_filters)
            log_fcs = [25.0 * (18000.0 / 25.0) ** (i / (n - 1)) for i in range(n)]
            adaptive_config = {'filters': [{'type': 'PEAKING', 'fc': round(fc, 1)} for fc in log_fcs]}
            peq_config = adaptive_config

        # gain_range / q_range 作为优化器边界约束，避免超限。
        # 深拷贝配置后写入每个 filter 的边界，不污染全局 PEQ_CONFIGS。
        has_gain_range = request.gain_range is not None and request.gain_range.low is not None and request.gain_range.high is not None
        has_q_range = request.q_range is not None and request.q_range.low is not None and request.q_range.high is not None
        if has_gain_range or has_q_range:
            peq_config = deepcopy(peq_config)
            for fc_cfg in peq_config['filters']:
                if has_gain_range:
                    fc_cfg['min_gain'] = float(request.gain_range.low)
                    fc_cfg['max_gain'] = float(request.gain_range.high)
                if has_q_range:
                    fc_cfg['min_q'] = float(request.q_range.low)
                    fc_cfg['max_q'] = float(request.q_range.high)
        # 多起点优化，提升曲线贴合度：
        # 5/8_PEAKING 单组已足够（频段已隔离）；10_PEAKING 用 2 组平衡速度；
        # shelf 系列用 3 组。
        # 目标是合成曲线贴合测试目标，不是听感调音。
        if effective_config == '5_PEAKING':
            multi_start = 1
        elif effective_config == '8_PEAKING':
            multi_start = 1
        elif effective_config == '10_PEAKING':
            multi_start = 2
        elif effective_config in ('FIXED_5_WITH_SHELVES', 'FIXED_8_WITH_SHELVES', 'FIXED_10_WITH_SHELVES', '8_PEAKING_WITH_SHELVES'):
            multi_start = 3
        else:
            multi_start = None
        peqs = fr.optimize_parametric_eq(peq_config, request.fs, preamp=request.preamp, multi_start=multi_start)

        low = request.eq_range.low
        high = request.eq_range.high

        all_filters = []
        for peq in peqs:
            for filt in peq.filters:
                all_filters.append({
                    'type': filt.__class__.__name__,
                    'fc': filt.fc,
                    'gain': filt.gain,
                    'q': filt.q
                })

        range_filters = [f for f in all_filters if low <= f['fc'] <= high]

        # gain_range / q_range 不做事后过滤——范围约束由优化器配置的 min_gain/max_gain 负责。
        # 之前 abs(gain) <= high 会把 gain 超界的滤波器删掉（10_PEAKING 优化出的滤波器可能只剩 8 个），
        # 且高频大增益补偿滤波器被误删导致高频不贴合。

        if request.max_filters is not None and len(range_filters) > request.max_filters:
            # 按 fc 均匀分布保留（保证 20-20k 全频段覆盖），而不是按 gain 截断——
            # 否则优化器堆在低频/中频的滤波器全被保留，高频滤波器被丢掉导致高频不贴合
            range_filters = sorted(range_filters, key=lambda x: x['fc'])
            n = len(range_filters)
            step = n / request.max_filters
            range_filters = [range_filters[min(n - 1, int(i * step))] for i in range(request.max_filters)]

        return {
            'preamp': -max([p.max_gain for p in peqs]) if peqs else request.preamp,
            'filters': range_filters,
            'eq_range': EqRange(low=low, high=high),
            'gain_range': request.gain_range,
            'q_range': request.q_range,
            'fs': request.fs,
            'max_filters': request.max_filters,
            'params': {
                'window_size': request.window_size,
                'treble_window_size': request.treble_window_size,
                'treble_f_lower': request.treble_f_lower,
                'treble_f_upper': request.treble_f_upper,
                'treble_gain_k': request.treble_gain_k,
                'max_gain': request.max_gain,
                'max_slope': request.max_slope,
                'tilt': request.tilt,
                'bass_boost_gain': request.bass_boost_gain,
                'bass_boost_fc': request.bass_boost_fc,
                'bass_boost_q': request.bass_boost_q,
                'treble_boost_gain': request.treble_boost_gain,
                'treble_boost_fc': request.treble_boost_fc,
                'treble_boost_q': request.treble_boost_q,
                'min_mean_error': request.min_mean_error,
            }
        }

    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
