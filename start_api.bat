@echo off
set OPENBLAS_NUM_THREADS=1
set MKL_NUM_THREADS=1
set MPLBACKEND=Agg
py -3.11 autoeq\rest_api.py
