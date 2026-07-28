package dwarf.gen;

/**
 * Dwarf String utility functions.
 */
public final class StringUtils {
    private StringUtils() {}

    public static String[] split(String s, String delimiter) {
        return s.split(delimiter);
    }

    public static String toUpper(String s) {
        return s.toUpperCase();
    }

    public static String toLower(String s) {
        return s.toLowerCase();
    }

    public static String reverse(String s) {
        return new StringBuilder(s).reverse().toString();
    }

    public static boolean contains(String s, String sub) {
        return s.contains(sub);
    }

    public static String trim(String s) {
        return s.trim();
    }

    public static int stringLength(String s) {
        return s.length();
    }
}
