package dwarf.gen;

import java.io.IOException;
import java.nio.file.*;

/**
 * Dwarf I/O operations.
 */
public final class IOUtils {
    private IOUtils() {}

    public static void print(Object value) {
        System.out.println(value);
    }

    public static String readFile(String path) throws IOException {
        return Files.readString(Path.of(path));
    }

    public static void writeFile(String path, String data) throws IOException {
        Files.writeString(Path.of(path), data);
    }
}
