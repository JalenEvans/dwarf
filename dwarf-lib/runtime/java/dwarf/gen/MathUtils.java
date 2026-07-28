package dwarf.gen;

/**
 * Dwarf Math utility functions.
 */
public final class MathUtils {
    private MathUtils() {}

    public static double abs(double x) {
        return Math.abs(x);
    }

    public static double max(double a, double b) {
        return Math.max(a, b);
    }

    public static double min(double a, double b) {
        return Math.min(a, b);
    }
}
