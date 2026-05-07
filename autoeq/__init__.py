# -*- coding: utf-8 -*-
"""
AutoEq - 自动耳机均衡器配置生成库

使用示例:
    from autoeq import equalize_data

    result = equalize_data(
        frequency=[20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000],
        raw=[-10, -5, 0, 2, 1, 0, -2, -1, 0, 5]
    )
"""

from autoeq.api import (
    equalize_data,
    equalize_file,
    optimize_parametric_eq,
    generate_graphic_eq_curve,
    get_available_configs,
    get_default_target,
)

__all__ = [
    'equalize_data',
    'equalize_file',
    'optimize_parametric_eq',
    'generate_graphic_eq_curve',
    'get_available_configs',
    'get_default_target',
]
