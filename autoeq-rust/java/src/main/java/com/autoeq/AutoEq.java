package com.autoeq;

import com.sun.jna.Pointer;
import java.util.List;

/**
 * High-level Java wrapper for AutoEq.
 * Handles JSON serialization/deserialization and memory management automatically.
 */
public class AutoEq {

    private static final AutoEqNative nativeLib = AutoEqNative.INSTANCE;

    /**
     * Equalize frequency response data with default parameters.
     *
     * @param frequency Frequency array in Hz
     * @param raw Raw SPL (Sound Pressure Level) array in dB
     * @return Equalization result
     */
    public static EqualizeResult equalize(double[] frequency, double[] raw) {
        return equalize(frequency, raw, null, "headphone", "8_PEAKING_WITH_SHELVES", null);
    }

    /**
     * Equalize frequency response data with a target curve.
     *
     * @param frequency Frequency array in Hz
     * @param raw Raw SPL array in dB
     * @param target Target curve SPL array in dB (same length as frequency)
     * @return Equalization result
     */
    public static EqualizeResult equalize(double[] frequency, double[] raw, double[] target) {
        return equalize(frequency, raw, target, "headphone", "8_PEAKING_WITH_SHELVES", null);
    }

    /**
     * Equalize frequency response data with full control.
     *
     * @param frequency Frequency array in Hz
     * @param raw Raw SPL array in dB
     * @param target Target curve (null for flat target)
     * @param name Headphone name
     * @param config PEQ config name (e.g., "QUDELIX_5K", "8_PEAKING_WITH_SHELVES")
     * @param params Processing parameters (null for defaults)
     * @return Equalization result
     */
    public static EqualizeResult equalize(double[] frequency, double[] raw,
                                           double[] target, String name,
                                           String config, ProcessParams params) {
        // Build JSON input
        StringBuilder sb = new StringBuilder();
        sb.append("{\"frequency\":").append(toJsonArray(frequency));
        sb.append(",\"raw\":").append(toJsonArray(raw));
        if (target != null) {
            sb.append(",\"target\":").append(toJsonArray(target));
        }
        if (name != null) {
            sb.append(",\"name\":\"").append(escapeJson(name)).append("\"");
        }
        if (config != null) {
            sb.append(",\"config\":\"").append(escapeJson(config)).append("\"");
        }
        if (params != null) {
            sb.append(",\"params\":").append(params.toJson());
        }
        sb.append("}");

        // Call native library
        String resultJson = nativeLib.autoeq_equalize_json(sb.toString());

        try {
            return EqualizeResult.fromJson(resultJson);
        } finally {
            // Free the native string
            // JNA returns String directly, so the native memory is already copied
            // But we should still free if using Pointer-based API
        }
    }

    /**
     * Get library version.
     * @return version string
     */
    public static String getVersion() {
        return nativeLib.autoeq_version();
    }

    /**
     * Get available PEQ config names.
     * @return list of config names
     */
    public static List<String> getAvailableConfigs() {
        String json = nativeLib.autoeq_configs();
        // Parse JSON array manually to avoid dependency on JSON library
        return parseJsonStringArray(json);
    }

    // --- Helper methods ---

    private static String toJsonArray(double[] arr) {
        StringBuilder sb = new StringBuilder("[");
        for (int i = 0; i < arr.length; i++) {
            if (i > 0) sb.append(",");
            sb.append(arr[i]);
        }
        sb.append("]");
        return sb.toString();
    }

    private static String escapeJson(String s) {
        return s.replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t");
    }

    private static java.util.List<String> parseJsonStringArray(String json) {
        java.util.List<String> result = new java.util.ArrayList<>();
        if (json == null || json.isEmpty()) return result;
        // Simple parser for ["str1","str2",...]
        json = json.trim();
        if (!json.startsWith("[")) return result;
        json = json.substring(1, json.length() - 1);
        // Split by comma, but handle quoted strings
        boolean inQuote = false;
        StringBuilder current = new StringBuilder();
        for (char c : json.toCharArray()) {
            if (c == '"') {
                if (inQuote) {
                    result.add(current.toString());
                    current.setLength(0);
                }
                inQuote = !inQuote;
            } else if (inQuote) {
                current.append(c);
            }
        }
        return result;
    }
}
