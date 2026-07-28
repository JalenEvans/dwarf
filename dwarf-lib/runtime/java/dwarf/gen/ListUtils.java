package dwarf.gen;

import java.util.*;
import java.util.function.*;

/**
 * Dwarf List utility functions.
 */
public final class ListUtils {
    private ListUtils() {}

    public static <T, U> List<U> map(List<T> list, Function<T, U> fn) {
        List<U> result = new ArrayList<>();
        for (T item : list) result.add(fn.apply(item));
        return result;
    }

    public static <T> List<T> filter(List<T> list, Predicate<T> pred) {
        List<T> result = new ArrayList<>();
        for (T item : list) if (pred.test(item)) result.add(item);
        return result;
    }

    public static <T, U> U reduce(List<T> list, BiFunction<U, T, U> fn, U initial) {
        U acc = initial;
        for (T item : list) acc = fn.apply(acc, item);
        return acc;
    }

    public static double sum(List<? extends Number> list) {
        double total = 0;
        for (Number n : list) total += n.doubleValue();
        return total;
    }

    public static <T extends Comparable<? super T>> List<T> sort(List<T> list) {
        List<T> copy = new ArrayList<>(list);
        Collections.sort(copy);
        return copy;
    }

    public static <T> List<T> reverse(List<T> list) {
        List<T> copy = new ArrayList<>(list);
        Collections.reverse(copy);
        return copy;
    }

    public static <T> int length(List<T> list) {
        return list.size();
    }
}
