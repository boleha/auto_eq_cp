package com.autoeq;

import com.sun.jna.Library;
import com.sun.jna.Native;
import com.sun.jna.Pointer;

/**
 * JNA wrapper for the AutoEq Rust native library.
 *
 * Usage:
 * <pre>
 * AutoEqNative lib = AutoEqNative.INSTANCE;
 * String result = lib.autoeq_equalize_json(inputJson);
 * // ... parse result ...
 * lib.autoeq_free_string(result);
 * </pre>
 */
public interface AutoEqNative extends Library {

    /** Singleton instance - loads the native library automatically */
    AutoEqNative INSTANCE = Native.load("autoeq", AutoEqNative.class);

    /**
     * eq-by-range: 匹配 Python /eq-by-range 接口
     *
     * @param inputJson JSON string with the following structure:
     * <pre>
     * {
     *   "select": {"frequency": [...], "raw": [...]},
     *   "target": {"frequency": [...], "raw": [...]},
     *   "eq_range": {"low": 20, "high": 20000},    // optional
     *   "fs": 44100,                                // optional
     *   "config": "8_PEAKING_WITH_SHELVES",         // optional
     *   "preamp": 0,                                // optional
     *   "max_filters": 10,                          // optional
     *   "gain_range": {"low": 0, "high": 20},       // optional
     *   "q_range": {"low": 0, "high": 10}           // optional
     * }
     * </pre>
     * @return JSON: {"preamp": ..., "filters": [...], "eq_range": {...}, ...}
     */
    String autoeq_eq_by_range(String inputJson);

    /**
     * Equalize frequency response data (完整输出).
     *
     * @param inputJson JSON string with the following structure:
     * <pre>
     * {
     *   "frequency": [20.0, 50.0, 100.0, ...],
     *   "raw": [3.5, 2.1, 0.5, ...],
     *   "target": [0.0, 0.0, 0.0, ...],   // optional, flat target if omitted
     *   "name": "My Headphone",            // optional
     *   "config": "8_PEAKING_WITH_SHELVES", // optional
     *   "params": {                        // optional
     *     "bass_boost_gain": 0.0,
     *     "bass_boost_fc": 105.0,
     *     "bass_boost_q": 0.7,
     *     "treble_boost_gain": 0.0,
     *     "treble_boost_fc": 10000.0,
     *     "treble_boost_q": 0.7,
     *     "tilt": 0.0,
     *     "fs": 44100,
     *     "max_gain": 6.0,
     *     "preamp": 0.0
     *   }
     * }
     * </pre>
     * @return JSON string with results. Must be freed with {@link #autoeq_free_string(Pointer)}.
     * <pre>
     * {
     *   "success": true,
     *   "name": "My Headphone",
     *   "frequency": [...],
     *   "raw": [...],
     *   "smoothed": [...],
     *   "equalization": [...],
     *   "target": [...],
     *   "error": [...],
     *   "parametric_eq": {
     *     "preamp": -6.0,
     *     "filters": [
     *       {"type": "LowShelf", "fc": 105.0, "gain": 3.0, "q": 0.7},
     *       ...
     *     ]
     *   },
     *   "graphic_eq": "GraphicEQ: 20 3.5; 50 2.1; ..."
     * }
     * </pre>
     */
    String autoeq_equalize_json(String inputJson);

    /**
     * Get library version.
     * @return version string. Must be freed with {@link #autoeq_free_string(Pointer)}.
     */
    String autoeq_version();

    /**
     * Get available PEQ config names as JSON array.
     * @return JSON array string. Must be freed with {@link #autoeq_free_string(Pointer)}.
     */
    String autoeq_configs();

    /**
     * Free a string allocated by this library.
     * MUST be called for every string returned by the library to avoid memory leaks.
     *
     * @param ptr Pointer to the string to free.
     */
    void autoeq_free_string(Pointer ptr);
}
