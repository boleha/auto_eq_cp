package com.autoeq;

import java.util.List;

/**
 * Result of the equalization process.
 */
public class EqualizeResult {
    public boolean success;
    public String error;
    public String name;
    public double[] frequency;
    public double[] raw;
    public double[] smoothed;
    public double[] equalization;
    public double[] target;
    public double[] errorCurve;
    public ParametricEq parametricEq;
    public String graphicEq;

    /**
     * Parametric EQ result.
     */
    public static class ParametricEq {
        public double preamp;
        public List<Filter> filters;

        public static class Filter {
            public String type;
            public double fc;
            public double gain;
            public double q;

            @Override
            public String toString() {
                return String.format("%s fc=%.1f Hz gain=%.2f dB q=%.4f", type, fc, gain, q);
            }
        }

        @Override
        public String toString() {
            StringBuilder sb = new StringBuilder();
            sb.append(String.format("Preamp: %.2f dB\n", preamp));
            if (filters != null) {
                for (int i = 0; i < filters.size(); i++) {
                    sb.append(String.format("  Filter %d: %s\n", i + 1, filters.get(i)));
                }
            }
            return sb.toString();
        }
    }

    @Override
    public String toString() {
        StringBuilder sb = new StringBuilder();
        sb.append(String.format("Name: %s\n", name));
        if (frequency != null) {
            sb.append(String.format("Frequency points: %d\n", frequency.length));
        }
        if (parametricEq != null) {
            sb.append(parametricEq);
        }
        if (graphicEq != null) {
            sb.append(graphicEq);
        }
        return sb.toString();
    }

    /**
     * Parse from JSON string. Uses simple parsing to avoid external JSON library dependency.
     * For production use, consider using Jackson or Gson.
     */
    static EqualizeResult fromJson(String json) {
        // Simple JSON parser - for production use, replace with Jackson/Gson
        EqualizeResult result = new EqualizeResult();
        if (json == null || json.isEmpty()) {
            result.success = false;
            result.error = "Empty response";
            return result;
        }

        result.success = json.contains("\"success\":true");

        // Extract error message if any
        result.error = extractString(json, "error");

        if (!result.success) {
            return result;
        }

        result.name = extractString(json, "name");
        result.frequency = extractDoubleArray(json, "frequency");
        result.raw = extractDoubleArray(json, "raw");
        result.smoothed = extractDoubleArray(json, "smoothed");
        result.equalization = extractDoubleArray(json, "equalization");
        result.target = extractDoubleArray(json, "target");
        result.errorCurve = extractDoubleArray(json, "error_curve");
        result.graphicEq = extractString(json, "graphic_eq");

        // Parse parametric_eq
        String peqJson = extractObject(json, "parametric_eq");
        if (peqJson != null) {
            result.parametricEq = parseParametricEq(peqJson);
        }

        return result;
    }

    private static ParametricEq parseParametricEq(String json) {
        ParametricEq peq = new ParametricEq();
        peq.preamp = extractDouble(json, "preamp");

        // Parse filters array
        String filtersJson = extractArray(json, "filters");
        if (filtersJson != null) {
            peq.filters = new java.util.ArrayList<>();
            // Split by },{ to get individual filter objects
            String[] parts = filtersJson.split("\\}\\s*,\\s*\\{");
            for (String part : parts) {
                String clean = part.replace("{", "").replace("}", "").trim();
                if (clean.isEmpty()) continue;
                ParametricEq.Filter filter = new ParametricEq.Filter();
                filter.type = extractString(clean, "type");
                filter.fc = extractDouble(clean, "fc");
                filter.gain = extractDouble(clean, "gain");
                filter.q = extractDouble(clean, "q");
                peq.filters.add(filter);
            }
        }
        return peq;
    }

    // --- Simple JSON extraction helpers ---

    private static String extractString(String json, String key) {
        String pattern = "\"" + key + "\":\"";
        int start = json.indexOf(pattern);
        if (start < 0) return null;
        start += pattern.length();
        int end = json.indexOf("\"", start);
        if (end < 0) return null;
        return json.substring(start, end);
    }

    private static double extractDouble(String json, String key) {
        String pattern = "\"" + key + "\":";
        int start = json.indexOf(pattern);
        if (start < 0) return 0.0;
        start += pattern.length();
        int end = start;
        while (end < json.length() && (Character.isDigit(json.charAt(end)) || json.charAt(end) == '.' || json.charAt(end) == '-')) {
            end++;
        }
        try {
            return Double.parseDouble(json.substring(start, end));
        } catch (NumberFormatException e) {
            return 0.0;
        }
    }

    private static double[] extractDoubleArray(String json, String key) {
        String pattern = "\"" + key + "\":[";
        int start = json.indexOf(pattern);
        if (start < 0) return null;
        start += pattern.length() - 1; // include [
        int end = findMatchingBracket(json, start);
        if (end < 0) return null;
        String arrayStr = json.substring(start + 1, end);
        if (arrayStr.isEmpty()) return new double[0];
        String[] parts = arrayStr.split(",");
        double[] result = new double[parts.length];
        for (int i = 0; i < parts.length; i++) {
            try {
                result[i] = Double.parseDouble(parts[i].trim());
            } catch (NumberFormatException e) {
                result[i] = 0.0;
            }
        }
        return result;
    }

    private static String extractObject(String json, String key) {
        String pattern = "\"" + key + "\":{";
        int start = json.indexOf(pattern);
        if (start < 0) return null;
        start += pattern.length() - 1; // include {
        int end = findMatchingBrace(json, start);
        if (end < 0) return null;
        return json.substring(start, end + 1);
    }

    private static String extractArray(String json, String key) {
        String pattern = "\"" + key + "\":[";
        int start = json.indexOf(pattern);
        if (start < 0) return null;
        start += pattern.length() - 1; // include [
        int end = findMatchingBracket(json, start);
        if (end < 0) return null;
        return json.substring(start + 1, end);
    }

    private static int findMatchingBracket(String json, int openPos) {
        int depth = 0;
        for (int i = openPos; i < json.length(); i++) {
            char c = json.charAt(i);
            if (c == '[') depth++;
            else if (c == ']') {
                depth--;
                if (depth == 0) return i;
            }
        }
        return -1;
    }

    private static int findMatchingBrace(String json, int openPos) {
        int depth = 0;
        for (int i = openPos; i < json.length(); i++) {
            char c = json.charAt(i);
            if (c == '{') depth++;
            else if (c == '}') {
                depth--;
                if (depth == 0) return i;
            }
        }
        return -1;
    }
}
