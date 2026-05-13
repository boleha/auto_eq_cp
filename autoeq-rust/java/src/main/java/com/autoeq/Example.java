package com.autoeq;

/**
 * AutoEq Java 使用示例
 */
public class Example {

    public static void main(String[] args) {
        // 1. 获取版本号
        System.out.println("AutoEq version: " + AutoEq.getVersion());

        // 2. 获取可用的 PEQ 配置
        System.out.println("Available configs: " + AutoEq.getAvailableConfigs());

        // 3. 基本均衡 - 使用默认参数
        double[] frequency = {
            20, 25, 31.5, 40, 50, 63, 80, 100, 125, 160, 200, 250, 315, 400, 500, 630, 800,
            1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000, 10000, 12500, 16000, 20000
        };
        double[] raw = {
            5.0, 4.5, 4.0, 3.5, 3.0, 2.5, 2.0, 1.5, 1.0, 0.5, 0.0, -0.5, -1.0, -1.5, -2.0,
            -2.5, -3.0, -3.5, -4.0, -4.5, -5.0, -5.5, -6.0, -6.5, -7.0, -7.5, -8.0, -8.5,
            -9.0, -9.5, -10.0
        };

        System.out.println("\n=== Basic equalization ===");
        EqualizeResult result = AutoEq.equalize(frequency, raw);
        System.out.println(result);

        // 4. 使用自定义参数
        System.out.println("\n=== With custom params ===");
        ProcessParams params = new ProcessParams()
            .bassBoostGain(6.0)
            .bassBoostFc(105.0)
            .tilt(0.5)
            .maxGain(12.0);

        EqualizeResult result2 = AutoEq.equalize(frequency, raw, null, "My Headphone", "QUDELIX_5K", params);
        System.out.println(result2);

        // 5. 使用目标曲线
        double[] target = new double[frequency.length]; // 平坦目标
        EqualizeResult result3 = AutoEq.equalize(frequency, raw, target, "With Target", "8_PEAKING_WITH_SHELVES", null);
        System.out.println("\n=== With target curve ===");
        System.out.println("PEQ filters: " + (result3.parametricEq != null ? result3.parametricEq.filters.size() : 0));
        if (result3.graphicEq != null) {
            System.out.println("GraphicEQ length: " + result3.graphicEq.length());
        }
    }
}
