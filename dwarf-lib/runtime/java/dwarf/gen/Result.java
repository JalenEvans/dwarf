package dwarf.gen;

import java.util.function.Function;

/**
 * Dwarf Result type — represents success or failure.
 */
public sealed interface Result<T, E> permits Result.Ok, Result.Err {

    record Ok<T, E>(T value) implements Result<T, E> {}

    record Err<T, E>(E error) implements Result<T, E> {}

    static <T, E> Result<T, E> ok(T value) {
        return new Ok<>(value);
    }

    static <T, E> Result<T, E> err(E error) {
        return new Err<>(error);
    }

    default boolean isOk() {
        return this instanceof Ok;
    }

    default boolean isErr() {
        return this instanceof Err;
    }

    default T unwrap() {
        if (this instanceof Ok(var value)) return value;
        if (this instanceof Err(var error)) throw new IllegalStateException("Called unwrap on Err: " + error);
        throw new IllegalStateException("Unknown variant");
    }

    default T unwrapOr(T defaultValue) {
        if (this instanceof Ok(var value)) return value;
        return defaultValue;
    }

    default <U> Result<U, E> map(Function<? super T, ? extends U> fn) {
        if (this instanceof Ok(var value)) return Result.ok(fn.apply(value));
        return (Result<U, E>) this;
    }

    default <F> Result<T, F> mapErr(Function<? super E, ? extends F> fn) {
        if (this instanceof Err(var error)) return Result.err(fn.apply(error));
        return (Result<T, F>) this;
    }

    default <U> Result<U, E> andThen(Function<? super T, Result<U, E>> fn) {
        if (this instanceof Ok(var value)) return fn.apply(value);
        return (Result<U, E>) this;
    }
}
