import sys
sys.path.insert(0, r'd:\workspace\project\project_python\auto\auto_eq_cp')

from autoeq.frequency_response import FrequencyResponse
from autoeq.peq import PEQ, PEQ_CONFIGS
import numpy as np

# Read input file
fr = FrequencyResponse.read_csv(r'd:\workspace\project\project_python\auto\auto_eq_cp\test_file\OlA  II.txt')
fr.interpolate(step=1.01)
fr.center()

# Read target
target = FrequencyResponse.read_csv(r'd:\workspace\project\project_python\auto\auto_eq_cp\test_file\harman2016.txt')
target.interpolate(f=fr.f)
target.center()

# Compensate and smooth
fr.compensate(target)
fr.smoothen()
fr.equalize()

# Create PEQ
config = PEQ_CONFIGS['8_PEAKING_WITH_SHELVES']
eq_target = fr.equalization if len(fr.equalization) > 0 else fr.error

peq = PEQ(fr.f, 44100, eq_target, config=config)

# Check initial params
initial_params = peq._init_optimizer_params()
print(f"Initial params: {initial_params}")

# Check initial bounds
bounds = peq._init_optimizer_bounds()
print(f"Bounds: {bounds}")

# Check initial loss
initial_loss = peq._optimizer_loss(initial_params, parse=False)
print(f"Initial loss: {initial_loss}")

# Optimize
peq.optimize()

# Check final loss
final_loss = peq._optimizer_loss(peq._init_optimizer_params(), parse=False)
print(f"Final loss: {final_loss}")

# Print filters
print(f"\nParametric EQ (preamp: {peq.max_gain:.2f} dB):")
for i, filt in enumerate(peq.filters):
    print(f"  Filter {i+1}: {filt.__class__.__name__} fc={filt.fc:.1f} Hz gain={filt.gain:.2f} dB q={filt.q:.4f}")
