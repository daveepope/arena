package arena.junit.support;
import java.util.concurrent.atomic.AtomicLong;

public final class ArenaIdentifiers {
  private static final int SUFFIX_LEN = 6;
  private static final String ALPHABET = "0123456789abcdefghijklmnopqrstuvwxyz";
  private static final int BASE = ALPHABET.length();
  private static final long MASK_64 = 0xFFFFFFFFFFFFFFFFL;
  private static final AtomicLong COUNTER = new AtomicLong(0L);

  private ArenaIdentifiers() {}

  private static long seedOnce() {
    long nanos = System.nanoTime() & MASK_64;
    long pid = ProcessHandle.current().pid() & MASK_64;
    long pidRot = ((pid << 32) | (pid >>> 32)) & MASK_64;
    return nanos ^ pidRot;
  }

  private static final long SEED = seedOnce();

  private static String slugify(String s) {
    StringBuilder out = new StringBuilder();
    boolean lastDash = false;
    for (int i = 0; i < s.length(); i++) {
      char c = Character.toLowerCase(s.charAt(i));
      if (c < 128 && (Character.isLetterOrDigit(c))) {
        out.append(c);
        lastDash = false;
      } else if (!lastDash) {
        out.append('-');
        lastDash = true;
      }
    }
    while (out.length() > 0 && out.charAt(out.length() - 1) == '-') {
      out.setLength(out.length() - 1);
    }
    while (out.length() > 0 && out.charAt(0) == '-') {
      out.deleteCharAt(0);
    }
    return out.toString();
  }

  private static String newSuffix() {
    long n = (SEED + COUNTER.getAndIncrement()) & MASK_64;
    char[] digits = new char[SUFFIX_LEN];
    for (int i = 0; i < SUFFIX_LEN; i++) {
      digits[SUFFIX_LEN - 1 - i] = ALPHABET.charAt((int) (n % BASE));
      n /= BASE;
    }
    return new String(digits);
  }

  private static boolean hasSuffix(String name) {
    int dash = name.lastIndexOf('-');
    if (dash < 0) {
      return false;
    }
    String last = name.substring(dash + 1);
    if (last.length() != SUFFIX_LEN) {
      return false;
    }
    for (int i = 0; i < last.length(); i++) {
      if (ALPHABET.indexOf(last.charAt(i)) < 0) {
        return false;
      }
    }
    return true;
  }

  public static String build(String module, String name) {
    if (hasSuffix(name)) {
      return name;
    }
    String slug = slugify(name);
    String suffix = newSuffix();
    if (slug.isEmpty()) {
      return module + "-" + suffix;
    }
    return module + "-" + slug + "-" + suffix;
  }
}
