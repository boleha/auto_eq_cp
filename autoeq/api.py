# -*- coding: utf-8 -*-
"""
AutoEq Python API - 自动耳机均衡器配置生成

功能说明:
- 将耳机频响曲线与目标曲线匹配，生成均衡器参数
- 支持参数均衡器(PEQ)、图形均衡器、卷积均衡器
- 用于音频后期处理、耳机调音等场景
"""

import numpy as np
from autoeq.frequency_response import FrequencyResponse
from autoeq.constants import (
    DEFAULT_FS, DEFAULT_MAX_GAIN, DEFAULT_PREAMP,
    DEFAULT_BASS_BOOST_GAIN, DEFAULT_BASS_BOOST_FC, DEFAULT_BASS_BOOST_Q,
    DEFAULT_TREBLE_BOOST_GAIN, DEFAULT_TREBLE_BOOST_FC, DEFAULT_TREBLE_BOOST_Q,
    DEFAULT_TILT, PEQ_CONFIGS
)


def equalize_data(frequency, raw, target_curve=None, name="headphone",
                  bass_boost_gain=DEFAULT_BASS_BOOST_GAIN,
                  bass_boost_fc=DEFAULT_BASS_BOOST_FC,
                  bass_boost_q=DEFAULT_BASS_BOOST_Q,
                  treble_boost_gain=DEFAULT_TREBLE_BOOST_GAIN,
                  treble_boost_fc=DEFAULT_TREBLE_BOOST_FC,
                  treble_boost_q=DEFAULT_TREBLE_BOOST_Q,
                  tilt=DEFAULT_TILT,
                  fs=DEFAULT_FS,
                  max_gain=DEFAULT_MAX_GAIN):
    """
    对频响数据进行均衡化处理

    Args:
        frequency: 频率点数组 (Hz)
        raw: 对应频率的增益值数组 (dB)
        target_curve: 目标曲线数组，如果为None则使用平直目标
        name: 数据名称
        bass_boost_gain: 低音增强量 (dB)
        bass_boost_fc: 低音增强中心频率 (Hz)
        bass_boost_q: 低音增强Q值
        treble_boost_gain: 高音增强量 (dB)
        treble_boost_fc: 高音增强中心频率 (Hz)
        treble_boost_q: 高音增强Q值
        tilt: 频响倾斜度 (dB/oct)
        fs: 采样率 (Hz)
        max_gain: 最大增益限制 (dB)

    Returns:
        dict: 包含frequency, raw, smoothed, equalization, target, error
    """
    fr = FrequencyResponse(
        name=name,
        frequency=np.array(frequency),
        raw=np.array(raw)
    )
    fr.interpolate()
    fr.center()

    if target_curve is not None:
        target = FrequencyResponse(
            name='target',
            frequency=fr.frequency.copy(),
            raw=np.array(target_curve)
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

    return {
        'name': fr.name,
        'frequency': fr.frequency.tolist(),
        'raw': fr.raw.tolist(),
        'smoothed': fr.smoothed.tolist() if len(fr.smoothed) else [],
        'equalization': fr.equalization.tolist() if len(fr.equalization) else [],
        'target': fr.target.tolist() if len(fr.target) else [],
        'error': fr.error.tolist() if len(fr.error) else [],
    }


def equalize_file(input_file, output_dir=None, target_file=None, name=None,
                  bass_boost_gain=DEFAULT_BASS_BOOST_GAIN,
                  bass_boost_fc=DEFAULT_BASS_BOOST_FC,
                  bass_boost_q=DEFAULT_BASS_BOOST_Q,
                  treble_boost_gain=DEFAULT_TREBLE_BOOST_GAIN,
                  treble_boost_fc=DEFAULT_TREBLE_BOOST_FC,
                  treble_boost_q=DEFAULT_TREBLE_BOOST_Q,
                  tilt=DEFAULT_TILT,
                  fs=DEFAULT_FS,
                  max_gain=DEFAULT_MAX_GAIN,
                  preamp=DEFAULT_PREAMP,
                  config="8_PEAKING_WITH_SHELVES"):
    """
    处理CSV频响文件并生成均衡器配置

    Args:
        input_file: 输入CSV文件路径
        output_dir: 输出目录，默认为输入文件所在目录
        target_file: 目标曲线CSV文件路径
        name: 名称，默认使用文件名
        其他参数同 equalize_data

    Returns:
        dict: 包含频响数据和均衡器参数
    """
    fr = FrequencyResponse.read_csv(input_file)
    if name:
        fr.name = name
    if output_dir:
        import os
        os.makedirs(output_dir, exist_ok=True)

    fr.interpolate()
    fr.center()

    if target_file:
        target_fr = FrequencyResponse.read_csv(target_file)
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


def optimize_parametric_eq(frequency, raw, name="headphone",
                           fs=DEFAULT_FS,
                           config="8_PEAKING_WITH_SHELVES",
                           preamp=DEFAULT_PREAMP,
                           target_curve=None,
                           bass_boost_gain=DEFAULT_BASS_BOOST_GAIN,
                           bass_boost_fc=DEFAULT_BASS_BOOST_FC,
                           bass_boost_q=DEFAULT_BASS_BOOST_Q,
                           treble_boost_gain=DEFAULT_TREBLE_BOOST_GAIN,
                           treble_boost_fc=DEFAULT_TREBLE_BOOST_FC,
                           treble_boost_q=DEFAULT_TREBLE_BOOST_Q,
                           tilt=DEFAULT_TILT,
                           max_gain=DEFAULT_MAX_GAIN):
    """
    生成参数均衡器(PEQ)配置

    Args:
        frequency: 频率点数组
        raw: 增益值数组
        name: 名称
        fs: 采样率
        config: PEQ配置名称
        preamp: 前置增益
        target_curve: 目标曲线数组
        bass_boost_gain: 低音增强量
        bass_boost_fc: 低音中心频率
        bass_boost_q: 低音Q值
        treble_boost_gain: 高音增强量
        treble_boost_fc: 高音中心频率
        treble_boost_q: 高音Q值
        tilt: 频响倾斜
        max_gain: 最大增益

    Returns:
        dict: 包含preamp和filters列表
    """
    fr = FrequencyResponse(
        name=name,
        frequency=np.array(frequency),
        raw=np.array(raw)
    )
    fr.interpolate()
    fr.center()

    if target_curve is not None:
        target = FrequencyResponse(
            name='target',
            frequency=fr.frequency.copy(),
            raw=np.array(target_curve)
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
    }


def generate_graphic_eq_curve(frequency, raw, name="headphone",
                              target_curve=None,
                              preamp=DEFAULT_PREAMP,
                              bass_boost_gain=DEFAULT_BASS_BOOST_GAIN,
                              bass_boost_fc=DEFAULT_BASS_BOOST_FC,
                              bass_boost_q=DEFAULT_BASS_BOOST_Q,
                              treble_boost_gain=DEFAULT_TREBLE_BOOST_GAIN,
                              treble_boost_fc=DEFAULT_TREBLE_BOOST_FC,
                              treble_boost_q=DEFAULT_TREBLE_BOOST_Q,
                              tilt=DEFAULT_TILT,
                              fs=DEFAULT_FS,
                              max_gain=DEFAULT_MAX_GAIN):
    """
    生成EqualizerAPO图形均衡器格式字符串

    Args:
        frequency: 频率点数组
        raw: 增益值数组
        name: 名称
        target_curve: 目标曲线数组
        preamp: 前置增益
        其他参数同 equalize_data

    Returns:
        str: EqualizerAPO GraphicEQ 格式字符串
    """
    fr = FrequencyResponse(
        name=name,
        frequency=np.array(frequency),
        raw=np.array(raw)
    )
    fr.interpolate()
    fr.center()

    if target_curve is not None:
        target = FrequencyResponse(
            name='target',
            frequency=fr.frequency.copy(),
            raw=np.array(target_curve)
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

    return fr.eqapo_graphic_eq(normalize=True, preamp=preamp)


def get_available_configs():
    """
    获取所有可用的PEQ配置名称

    Returns:
        list: 配置名称列表
    """
    return list(PEQ_CONFIGS.keys())


def get_default_target():
    """
    获取默认的平直目标曲线

    Returns:
        dict: 包含frequency和raw数组
    """
    fr = FrequencyResponse(name='flat_target')
    return {
        'name': fr.name,
        'frequency': fr.frequency.tolist(),
        'raw': fr.raw.tolist()
    }
