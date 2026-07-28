package dwarf.gen;

import java.util.Optional;
import java.util.function.Function;

/**
 * Dwarf Option type — represents an optional value.
 */
public sealed interface Option<T> permits Option.Some, Option.None {

    record Some<T>(T value) implements Option<T> {}

    record None<T>() implements Option<T> {}

    static <T> Option<T> some(T value) {
        return new Some<>(value);
    }

    @SuppressWarnings("unchecked")
    static <T> Option<T> none() {
        return (Option<T>) NoneHolder.INSTANCE;
    }

    default boolean isSome() {
        return this instanceof Some;
    }

    default boolean isNone() {
        return this instanceof None;
    }

    default T unwrap() {
        if (this instanceof Some(var value)) return value;
        throw new IllegalStateException("Called unwrap on None");
    }

    default T unwrapOr(T defaultValue) {
        if (this instanceof Some(var value)) return value;
        return defaultValue;
    }

    default <U> Option<U> map(Function<? super T, ? extends U> fn) {
        if (this instanceof Some(var value)) return Option.some(fn.apply(value));
        return Option.none();
    }

    default <U> Option<U> flatMap(Function<? super T, Option<U>> fn) {
        if (this instanceof Some(var value)) return fn.apply(value);
        return Option.none();
    }
}

final class NoneHolder {
    static final Option.None<?> INSTANCE = new Option.None<>();
}
