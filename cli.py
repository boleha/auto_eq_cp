# -*- coding: utf-8 -*-
import sys
import json
from autoeq.api import equalize_data, optimize_parametric_eq

def main():
    input_json = sys.stdin.read()
    data = json.loads(input_json)

    action = data.get("action")
    frequency = data.get("frequency", [])
    raw = data.get("raw", [])
    fs = data.get("fs", 48000)
    max_gain = data.get("max_gain", 12)
    config = data.get("config", "8_PEAKING_WITH_SHELVES")

    if action == "equalize":
        result = equalize_data(
            frequency=frequency,
            raw=raw,
            fs=fs,
            max_gain=max_gain
        )
    elif action == "optimize_peq":
        result = optimize_parametric_eq(
            frequency=frequency,
            raw=raw,
            config=config,
            fs=fs,
            max_gain=max_gain
        )
    else:
        result = {"error": "unknown action: " + str(action)}

    print(json.dumps(result))

if __name__ == "__main__":
    main()
