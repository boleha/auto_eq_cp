package com.autoeq;

/**
 * Processing parameters for AutoEq.
 */
public class ProcessParams {
    private Double bassBoostGain;
    private Double bassBoostFc;
    private Double bassBoostQ;
    private Double trebleBoostGain;
    private Double trebleBoostFc;
    private Double trebleBoostQ;
    private Double tilt;
    private Double fs;
    private Double maxGain;
    private Double preamp;

    public ProcessParams() {}

    // Builder-style setters
    public ProcessParams bassBoostGain(double v) { this.bassBoostGain = v; return this; }
    public ProcessParams bassBoostFc(double v) { this.bassBoostFc = v; return this; }
    public ProcessParams bassBoostQ(double v) { this.bassBoostQ = v; return this; }
    public ProcessParams trebleBoostGain(double v) { this.trebleBoostGain = v; return this; }
    public ProcessParams trebleBoostFc(double v) { this.trebleBoostFc = v; return this; }
    public ProcessParams trebleBoostQ(double v) { this.trebleBoostQ = v; return this; }
    public ProcessParams tilt(double v) { this.tilt = v; return this; }
    public ProcessParams fs(double v) { this.fs = v; return this; }
    public ProcessParams maxGain(double v) { this.maxGain = v; return this; }
    public ProcessParams preamp(double v) { this.preamp = v; return this; }

    /**
     * Convert to JSON string for the native library.
     */
    String toJson() {
        StringBuilder sb = new StringBuilder("{");
        boolean first = true;
        first = appendField(sb, first, "bass_boost_gain", bassBoostGain);
        first = appendField(sb, first, "bass_boost_fc", bassBoostFc);
        first = appendField(sb, first, "bass_boost_q", bassBoostQ);
        first = appendField(sb, first, "treble_boost_gain", trebleBoostGain);
        first = appendField(sb, first, "treble_boost_fc", trebleBoostFc);
        first = appendField(sb, first, "treble_boost_q", trebleBoostQ);
        first = appendField(sb, first, "tilt", tilt);
        first = appendField(sb, first, "fs", fs);
        first = appendField(sb, first, "max_gain", maxGain);
        first = appendField(sb, first, "preamp", preamp);
        sb.append("}");
        return sb.toString();
    }

    private boolean appendField(StringBuilder sb, boolean first, String key, Double value) {
        if (value == null) return first;
        if (!first) sb.append(",");
        sb.append("\"").append(key).append("\":").append(value);
        return false;
    }
}
