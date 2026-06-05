$env:OPENBLAS_NUM_THREADS=1
$env:MKL_NUM_THREADS=1
$env:MPLBACKEND=Agg
py -3.11 autoeq/rest_api.py
