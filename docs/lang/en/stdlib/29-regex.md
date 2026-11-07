# Module std.regex

The `std.regex` module provides a high-performance regular expression engine with complete Unicode standard support and guaranteed linear-time matching ($O(mn)$) free from Regular Expression Denial of Service (ReDoS) backtracking vulnerabilities. The module supports pattern compilation, matching flags, offset extraction, numbered and named capture groups (`Captures`), substring replacements with group expansions (`$1`, `${name}`), and regular expression splitting.

---

## 1. Import: load std.regex

Import the entire package:

```yukino
load std.regex
```

Importing the module exposes the following items:

- **Constants:**
  - Compilation flags: `CASE_INSENSITIVE`, `MULTILINE`, `DOT_MATCHES_NEW_LINE`, `CRLF`, `SWAP_GREED`, `IGNORE_WHITESPACE`, `UNICODE`
- **Data Structures and Errors:**
  - `RegexError` - error structure raised on pattern syntax errors or invalid handles
  - `Match` - match result containing span boundaries and substring content
  - `CaptureGroup` - capture group metadata and status
- **Classes:**
  - `Captures` - collection of capture groups with numerical and named lookups
  - `Regex` - compiled regular expression object offering matching, search, replace, and split methods
- **Functions:**
  - Factories: `compile`, `compile_flags`
  - Matching and search helpers: `is_match`, `regex_find`, `regex_find_all`
  - Replace and split helpers: `regex_replace`, `regex_replace_all`, `regex_split`
  - Escaping: `escape`

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

### 2.1 Compilation Flags

Bitmask flags used to configure regular expression compilation behavior:

```yukino
public static CASE_INSENSITIVE: int32 = 1
public static MULTILINE: int32 = 2
public static DOT_MATCHES_NEW_LINE: int32 = 4
public static CRLF: int32 = 8
public static SWAP_GREED: int32 = 16
public static IGNORE_WHITESPACE: int32 = 32
public static UNICODE: int32 = 64
```

- `CASE_INSENSITIVE` (1) - enables case-insensitive matching (equivalent to `(?i)`).
- `MULTILINE` (2) - multi-line mode: `^` and `$` match begin/end of each line rather than the whole input (equivalent to `(?m)`).
- `DOT_MATCHES_NEW_LINE` (4) - dot `.` matches any character including newline `\n` (equivalent to `(?s)`).
- `CRLF` (8) - recognizes `\r\n` line endings.
- `SWAP_GREED` (16) - inverts quantifier greediness: quantifiers become lazy by default, and greedy with `?`.
- `IGNORE_WHITESPACE` (32) - ignores unescaped whitespace and allows `#` comments in pattern (equivalent to `(?x)`).
- `UNICODE` (64) - explicitly enables Unicode character classes (enabled by default).

---

## 3. Interfaces

No global interfaces are declared in `std.regex`.

---

## 4. Data Structures and Panic Errors

### 4.1 RegexError

Error structure raised via `panic` on regular expression syntax errors or invalid handles.

```yukino
public struct RegexError {
    message: string
    code: int32
}
```

- `code = 101` - syntax compilation error (unclosed brackets, invalid character classes, non-UTF-8 bytes);
- `code = 102` - regular expression handle is closed or null.

### 4.2 Match

Structure containing the boundaries and text of a matched substring.

```yukino
public struct Match {
    start: int64
    end: int64
    text: string
}
```

- `start: int64` - start byte offset in the input string (inclusive);
- `end: int64` - end byte offset in the input string (exclusive);
- `text: string` - matched substring text.

### 4.3 CaptureGroup

Metadata describing the status of an individual capture group.

```yukino
public struct CaptureGroup {
    index: int32
    name: string
    start: int64
    end: int64
    text: string
    matched: bool
}
```

- `index: int32` - zero-based group index (0 represents the entire match);
- `name: string` - group name for named captures (`(?P<name>...)`) or empty string for unnamed groups;
- `start: int64` - start byte offset (or `-1` if the group did not participate in match);
- `end: int64` - end byte offset (or `-1` if the group did not participate in match);
- `text: string` - matched text or empty string;
- `matched: bool` - boolean flag indicating whether the group participated in the match.

---

## 5. Classes

### 5.1 class Captures

Container for all capture groups produced by `captures`.

```yukino
public class Captures {
    public fun init(groups: CaptureGroup[])
    public fun len() -> int32
    public fun get(index: int32) -> Match?
    public fun get_by_name(name: string) -> Match?
    public fun all() -> Match[]
}
```

#### len

Returns the total number of capture groups in the expression including group 0.

```yukino
public fun len() -> int32
```

Returns a non-negative integer of type `int32`.

This method never panics.

#### get

Retrieves the match for the specified group index `index`. Index 0 represents the full match.

```yukino
public fun get(index: int32) -> Match?
```

Returns a `Match` instance, or `null` if the index is out of bounds or the group was not matched.

This method never panics.

#### get_by_name

Retrieves the match for the specified named capture group `name`.

```yukino
public fun get_by_name(name: string) -> Match?
```

Returns a `Match` instance, or `null` if the group name was not found or did not participate in the match.

This method never panics.

#### all

Returns all successfully matched groups as an array of `Match` structures.

```yukino
public fun all() -> Match[]
```

Returns an array of type `Match[]`.

This method never panics.

### 5.2 class Regex

Compiled regular expression object.

```yukino
public class Regex {
    public handle: ptr<void>
    public pattern: string
    public flags: int32
    public is_closed: bool

    public fun init(pattern: string, flags: int32 = 0)
    public fun is_match(text: string) -> bool
    public fun find(text: string) -> Match?
    public fun find_at(text: string, start_offset: int64) -> Match?
    public fun find_all(text: string) -> Match[]
    public fun captures(text: string) -> Captures?
    public fun replace(text: string, rep: string) -> string
    public fun replace_all(text: string, rep: string) -> string
    public fun split(text: string) -> string[]
    public fun free()
}
```

#### init

Compiles a regular expression pattern with optional bitmask flags.

```yukino
public fun init(pattern: string, flags: int32 = 0)
```

On invalid syntax, panics with a `RegexError` structure (code 101).

#### is_match

Checks whether the pattern matches anywhere in the target string.

```yukino
public fun is_match(text: string) -> bool
```

Returns a boolean value of type `bool`.

Panics with `RegexError` (code 102) if the handle is closed or null.

#### find

Finds the first match in the input string.

```yukino
public fun find(text: string) -> Match?
```

Returns a `Match` instance or `null` if no match is found.

Panics with `RegexError` (code 102) if the handle is closed or null.

#### find_at

Finds the first match in the input string starting from byte offset `start_offset`.

```yukino
public fun find_at(text: string, start_offset: int64) -> Match?
```

Returns a `Match` instance or `null` if no match is found.

Panics with `RegexError` (code 102) if the handle is closed or null.

#### find_all

Finds all non-overlapping matches in the input string.

```yukino
public fun find_all(text: string) -> Match[]
```

Returns an array of type `Match[]`.

Panics with `RegexError` (code 102) if the handle is closed or null.

#### captures

Finds the first match and returns a `Captures` object to access capture groups.

```yukino
public fun captures(text: string) -> Captures?
```

Returns a `Captures` instance or `null` if the pattern did not match.

Panics with `RegexError` (code 102) if the handle is closed or null.

#### replace

Replaces the first occurrence in `text` with replacement `rep`. Supports `$1` and `${name}` expansions.

```yukino
public fun replace(text: string, rep: string) -> string
```

Returns a new string of type `string`.

Panics with `RegexError` (code 102) if the handle is closed or null.

#### replace_all

Replaces all non-overlapping occurrences in `text` with replacement `rep`. Supports `$1` and `${name}` expansions.

```yukino
public fun replace_all(text: string, rep: string) -> string
```

Returns a new string of type `string`.

Panics with `RegexError` (code 102) if the handle is closed or null.

#### split

Splits the target string by regular expression matches.

```yukino
public fun split(text: string) -> string[]
```

Returns an array of strings of type `string[]`.

Panics with `RegexError` (code 102) if the handle is closed or null.

#### free

Frees the internal native regular expression handle.

```yukino
public fun free()
```

Returns `void`.

This method never panics.

---

## 6. Function Reference

### 6.1 compile

Compiles a regular expression using default flags.

```yukino
public fun compile(pattern: string) -> Regex
```

Returns a compiled `Regex` instance.

Panics with `RegexError` (code 101) on invalid syntax.

```yukino
load std.io: println
load std.regex: compile, Regex

fun main() {
    set re: Regex = compile("\\d+")
    println(cast(re.is_match("item 42"), "string"))
    re.free()
}
```

### 6.2 compile_flags

Compiles a regular expression using a combination of bitmask flags.

```yukino
public fun compile_flags(pattern: string, flags: int32) -> Regex
```

Returns a compiled `Regex` instance.

Panics with `RegexError` (code 101) on invalid syntax.

```yukino
load std.io: println
load std.regex: compile_flags, Regex, CASE_INSENSITIVE

fun main() {
    set re: Regex = compile_flags("warning", CASE_INSENSITIVE)
    println(cast(re.is_match("SYSTEM WARNING"), "string"))
    re.free()
}
```

### 6.3 is_match

Helper for one-shot pattern matching.

```yukino
public fun is_match(pattern: string, text: string) -> bool
```

Returns a boolean value of type `bool`.

Panics with `RegexError` (code 101) on invalid syntax.

```yukino
load std.io: println
load std.regex: is_match

fun main() {
    set ok: bool = is_match("^id_[0-9]+$", "id_992")
    println(cast(ok, "string"))
}
```

### 6.4 regex_find

Finds the first pattern match in `text`.

```yukino
public fun regex_find(pattern: string, text: string) -> Match?
```

Returns a `Match` instance or `null` if no match was found.

Panics with `RegexError` (code 101) on invalid syntax.

```yukino
load std.io: println
load std.regex: regex_find, Match

fun main() {
    set m: Match? = regex_find("\\d+", "Size: 1024 bytes")
    if m != null {
        set item: Match = m ?? Match { start: 0, end: 0, text: "" }
        println("Match: " + item.text)
    }
}
```

### 6.5 regex_find_all

Finds all non-overlapping matches in `text`.

```yukino
public fun regex_find_all(pattern: string, text: string) -> Match[]
```

Returns an array of type `Match[]`.

Panics with `RegexError` (code 101) on invalid syntax.

```yukino
load std.io: println
load std.regex: regex_find_all, Match

fun main() {
    set items: Match[] = regex_find_all("\\b\\w+\\b", "one two three")
    set i: int64 = 0
    while i < items.len() {
        println(items[i].text)
        rewrite i = i + 1
    }
}
```

### 6.6 regex_replace

Replaces the first occurrence in `text` with `rep`.

```yukino
public fun regex_replace(pattern: string, text: string, rep: string) -> string
```

Returns a modified string of type `string`.

Panics with `RegexError` (code 101) on invalid syntax.

```yukino
load std.io: println
load std.regex: regex_replace

fun main() {
    set s: string = regex_replace("\\d+", "host:8000", "8080")
    println(s) // host:8080
}
```

### 6.7 regex_replace_all

Replaces all non-overlapping occurrences in `text` with `rep`.

```yukino
public fun regex_replace_all(pattern: string, text: string, rep: string) -> string
```

Returns a modified string of type `string`.

Panics with `RegexError` (code 101) on invalid syntax.

```yukino
load std.io: println
load std.regex: regex_replace_all

fun main() {
    set s: string = regex_replace_all("\\s+", "hello    world   yukino", " ")
    println(s) // hello world yukino
}
```

### 6.8 regex_split

Splits a string by pattern occurrences.

```yukino
public fun regex_split(pattern: string, text: string) -> string[]
```

Returns an array of strings of type `string[]`.

Panics with `RegexError` (code 101) on invalid syntax.

```yukino
load std.io: println
load std.regex: regex_split

fun main() {
    set tokens: string[] = regex_split("[,;]\\s*", "a, b; c")
    set i: int64 = 0
    while i < tokens.len() {
        println(tokens[i])
        rewrite i = i + 1
    }
}
```

### 6.9 escape

Escapes all regular expression meta-characters in the given string, making it safe for interpolation into patterns.

```yukino
public fun escape(s: string) -> string
```

Returns an escaped string of type `string`.

This function never panics.

```yukino
load std.io: println
load std.regex: escape

fun main() {
    set safe: string = escape("path/to/file.ext?query=1")
    println(safe)
}
```

---

[← Previous: Module std.net](28-net.md) | [Next: Module std.toml →](30-toml.md)
