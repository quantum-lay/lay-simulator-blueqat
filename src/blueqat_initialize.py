from blueqat import Circuit
from blueqat.backends.numpy_backend import NumPyBackend

_measure = NumPyBackend.gate_measure
def measure(self_, gate, ctx):
    ctx = _measure(self_, gate, ctx)
    ctx.save_cache = True
    return ctx

NumPyBackend.gate_measure = measure
