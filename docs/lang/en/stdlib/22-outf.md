# Module std.outf

The `std.outf` (output format) module provides tools for console output styling, terminal color control (8 standard colors, 8 bright colors, 256-color palette, and 24-bit TrueColor RGB), text modifiers (bold, dim, italic, underline, blink, reversed, hidden, strikethrough), and automatic terminal capability detection adhering to standard environment variables (`NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `COLORTERM`, `TERM`). Inspired by modern terminal styling libraries (such as `owo-colors` and `colored`), `std.outf` takes string arguments by value (`text: string`) as lightweight Copy descriptors, and exposes a fluent chainable API through the `Styled` class.

---

## 1. Import: load std.outf

Importing the entire package:

```yukino
load std.outf
```

This brings the following entities into the current scope:

- **Color mode constants:** `COLOR_AUTO`, `COLOR_ALWAYS`, `COLOR_NEVER`, `RESET`.
- **Classes:** `Styled`.
- **Control and detection functions:** `set_color_mode`, `get_color_mode`, `is_color_enabled`, `supports_color`, `supports_truecolor`.
- **Text style functions:** `bold`, `dim`, `italic`, `underline`, `blink`, `reversed`, `hidden`, `strikethrough`.
- **Foreground color functions:** `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `gray`, `bright_black`, `bright_red`, `bright_green`, `bright_yellow`, `bright_blue`, `bright_magenta`, `bright_cyan`, `bright_white`, `color256`, `rgb`.
- **Background color functions:** `bg_black`, `bg_red`, `bg_green`, `bg_yellow`, `bg_blue`, `bg_magenta`, `bg_cyan`, `bg_white`, `bg_gray`, `bg_bright_black`, `bg_bright_red`, `bg_bright_green`, `bg_bright_yellow`, `bg_bright_blue`, `bg_bright_magenta`, `bg_bright_cyan`, `bg_bright_white`, `bg_color256`, `bg_rgb`.
- **Fluent builder factory functions:** `style`, `styled`.
- **ANSI utility functions:** `clear_style`, `strip_ansi`, `visible_len`, `visible_bytes`.

All listed entities are also automatically available when loading the entire standard library via `load std`.

---

## 2. Constants

The `std.outf` module defines global constants for controlling coloring behavior and standard ANSI control sequences:

```yukino
public static COLOR_AUTO: int32 = 0
public static COLOR_ALWAYS: int32 = 1
public static COLOR_NEVER: int32 = 2

public static RESET: string = "\x1b[0m"
```

- `COLOR_AUTO`: Automatic mode (default). ANSI sequence generation depends on environment variables (`CLICOLOR_FORCE`, `NO_COLOR`, `CLICOLOR`, `TERM`) and whether standard output is connected to an interactive terminal (TTY).
- `COLOR_ALWAYS`: Force coloring on. Formatting sequences are always emitted, regardless of redirection or environment flags.
- `COLOR_NEVER`: Force coloring off. Styling functions return copies of the original text with all ANSI escapes omitted.
- `RESET`: The canonical terminal graphics reset escape sequence `\x1b[0m`.

---

## 3. Interfaces

No interfaces are declared in the `std.outf` module.

---

## 4. Data Structures and Errors (Panics)

No data structures or panic errors are declared in the `std.outf` module. All module functions are robust against invalid input ranges, safely normalize color channels, and never trigger runtime panics.

---

## 5. Classes

### 5.1 Styled

The `Styled` class provides a fluent builder interface for chaining multiple text styles, foreground colors, and background colors into a single compact ANSI escape sequence (e.g., `\x1b[1;31;44m...\x1b[0m`) without generating intermediate string allocations.

```yukino
public class Styled {
    fun bold() -> Styled
    fun dim() -> Styled
    fun italic() -> Styled
    fun underline() -> Styled
    fun blink() -> Styled
    fun reversed() -> Styled
    fun hidden() -> Styled
    fun strikethrough() -> Styled

    fun black() -> Styled
    fun red() -> Styled
    fun green() -> Styled
    fun yellow() -> Styled
    fun blue() -> Styled
    fun magenta() -> Styled
    fun cyan() -> Styled
    fun white() -> Styled
    fun gray() -> Styled
    fun bright_black() -> Styled
    fun bright_red() -> Styled
    fun bright_green() -> Styled
    fun bright_yellow() -> Styled
    fun bright_blue() -> Styled
    fun bright_magenta() -> Styled
    fun bright_cyan() -> Styled
    fun bright_white() -> Styled

    fun bg_black() -> Styled
    fun bg_red() -> Styled
    fun bg_green() -> Styled
    fun bg_yellow() -> Styled
    fun bg_blue() -> Styled
    fun bg_magenta() -> Styled
    fun bg_cyan() -> Styled
    fun bg_white() -> Styled
    fun bg_gray() -> Styled
    fun bg_bright_black() -> Styled
    fun bg_bright_red() -> Styled
    fun bg_bright_green() -> Styled
    fun bg_bright_yellow() -> Styled
    fun bg_bright_blue() -> Styled
    fun bg_bright_magenta() -> Styled
    fun bg_bright_cyan() -> Styled
    fun bg_bright_white() -> Styled

    fun color256(code: int32) -> Styled
    fun bg_color256(code: int32) -> Styled

    fun rgb(r: int32, g: int32, b: int32) -> Styled
    fun bg_rgb(r: int32, g: int32, b: int32) -> Styled

    fun to_string() -> string
    fun render() -> string
}
```

All modifier methods return `this` to allow method chaining. The `to_string()` method (and its alias `render()`) produces the formatted string. If coloring is disabled (`COLOR_NEVER` or terminal does not support colors in `COLOR_AUTO`), it returns the original unformatted text.

```yukino
load std.outf: style, styled, set_color_mode, COLOR_ALWAYS
load std.io: println

fun main() {
    set_color_mode(COLOR_ALWAYS)
    set msg = "Important Notice"

    // Chain bold styling, red foreground, and blue background
    set formatted = style(msg).bold().red().bg_blue().to_string()
    println(formatted)

    // TrueColor RGB with italic modifier
    set custom = styled(msg).italic().rgb(255, 128, 0).bg_rgb(20, 20, 20).render()
    println(custom)
}
```

---

## 6. Function Reference

### 6.1 Color Mode Control and Terminal Detection

#### set_color_mode

Configures the global color mode for console output. Accepts one of the mode constants: `COLOR_AUTO`, `COLOR_ALWAYS`, or `COLOR_NEVER`. This setting is thread-safe and immediately affects all subsequent styling operations.

```yukino
public fun set_color_mode(mode: int32)
```

Returns nothing (`void`).

This function never triggers a panic.

```yukino
load std.outf: set_color_mode, COLOR_ALWAYS

fun main() {
    set_color_mode(COLOR_ALWAYS)
}
```

#### get_color_mode

Returns the currently configured global color mode.

```yukino
public fun get_color_mode() -> int32
```

Returns an integer value of type `int32` (`COLOR_AUTO = 0`, `COLOR_ALWAYS = 1`, or `COLOR_NEVER = 2`). The return value is not nullable.

This function never triggers a panic.

```yukino
load std.outf: get_color_mode, COLOR_AUTO
load std.io: println

fun main() {
    if get_color_mode() == COLOR_AUTO {
        println("Color mode is set to AUTO")
    }
}
```

#### is_color_enabled

Determines whether ANSI color styling is currently enabled. Returns `true` if `COLOR_ALWAYS` is set, or `false` if `COLOR_NEVER` is set. Under `COLOR_AUTO`, checks environment variables: `CLICOLOR_FORCE!=0` forces colors on, `NO_COLOR` (non-empty string), `CLICOLOR=0`, or `TERM=dumb` disables colors; otherwise verifies if standard output is connected to a terminal device (`is_stdout_terminal()`).

```yukino
public fun is_color_enabled() -> bool
```

Returns a boolean value of type `bool`. The return value is not nullable.

This function never triggers a panic.

```yukino
load std.outf: is_color_enabled
load std.io: println

fun main() {
    if is_color_enabled() {
        println("Terminal colors are active")
    }
}
```

#### supports_color

Tests whether the terminal output environment supports basic ANSI coloring based on the `TERM` environment variable and `is_stdout_terminal()`.

```yukino
public fun supports_color() -> bool
```

Returns a boolean value of type `bool`. The return value is not nullable.

This function never triggers a panic.

```yukino
load std.outf: supports_color
load std.io: println

fun main() {
    println("Terminal supports colors: " + if supports_color() { "yes" } else { "no" })
}
```

#### supports_truecolor

Checks whether the current terminal supports 24-bit TrueColor RGB palettes by inspecting the `COLORTERM` environment variable (`truecolor` or `24bit`).

```yukino
public fun supports_truecolor() -> bool
```

Returns a boolean value of type `bool`. The return value is not nullable.

This function never triggers a panic.

```yukino
load std.outf: supports_truecolor
load std.io: println

fun main() {
    if supports_truecolor() {
        println("24-bit TrueColor is supported")
    }
}
```

---

### 6.2 Text Styles

Functions in this group wrap the string in the corresponding ANSI SGR attribute sequence (`\x1b[...m`) and append the reset escape `\x1b[0m`. When coloring is disabled, a plain copy of the text is returned.

- `bold(text: string) -> string` - Bold or increased intensity (code `1`).
- `dim(text: string) -> string` - Dim or faint text (code `2`).
- `italic(text: string) -> string` - Italicized font (code `3`).
- `underline(text: string) -> string` - Singly underlined text (code `4`).
- `blink(text: string) -> string` - Blinking text (code `5`).
- `reversed(text: string) -> string` - Invert foreground and background colors (code `7`).
- `hidden(text: string) -> string` - Concealed or invisible text (code `8`).
- `strikethrough(text: string) -> string` - Crossed-out text (code `9`).

All functions accept a string `text: string`, return a newly allocated string of type `string` (non-nullable), and never trigger a panic.

```yukino
load std.outf: bold, underline, italic, set_color_mode, COLOR_ALWAYS
load std.io: println

fun main() {
    set_color_mode(COLOR_ALWAYS)
    set title = "Chapter 1"
    println(bold(title))
    println(underline(title))
    println(italic(title))
}
```

---

### 6.3 Foreground Colors

#### Standard 8 Colors

- `black(text: string) -> string` - Black foreground (code `30`).
- `red(text: string) -> string` - Red foreground (code `31`).
- `green(text: string) -> string` - Green foreground (code `32`).
- `yellow(text: string) -> string` - Yellow foreground (code `33`).
- `blue(text: string) -> string` - Blue foreground (code `34`).
- `magenta(text: string) -> string` - Magenta foreground (code `35`).
- `cyan(text: string) -> string` - Cyan foreground (code `36`).
- `white(text: string) -> string` - White foreground (code `37`).

#### Bright 8 Colors

- `gray(text: string) -> string` - Gray / bright black foreground (code `90`).
- `bright_black(text: string) -> string` - Bright black foreground (code `90`, alias for `gray`).
- `bright_red(text: string) -> string` - Bright red foreground (code `91`).
- `bright_green(text: string) -> string` - Bright green foreground (code `92`).
- `bright_yellow(text: string) -> string` - Bright yellow foreground (code `93`).
- `bright_blue(text: string) -> string` - Bright blue foreground (code `94`).
- `bright_magenta(text: string) -> string` - Bright magenta foreground (code `95`).
- `bright_cyan(text: string) -> string` - Bright cyan foreground (code `96`).
- `bright_white(text: string) -> string` - Bright white foreground (code `97`).

#### color256

Applies a foreground color from the extended 256-color palette (`\x1b[38;5;<code>m`). The color index is automatically clamped to `0..255`.

```yukino
public fun color256(text: string, code: int32) -> string
```

Returns a formatted string of type `string`. The return value is not nullable. This function never triggers a panic.

#### rgb

Applies a 24-bit TrueColor RGB foreground color (`\x1b[38;2;<r>;<g>;<b>m`). Each color component is clamped to `0..255`.

```yukino
public fun rgb(text: string, r: int32, g: int32, b: int32) -> string
```

Returns a formatted string of type `string`. The return value is not nullable. This function never triggers a panic.

```yukino
load std.outf: red, green, bright_cyan, color256, rgb, set_color_mode, COLOR_ALWAYS
load std.io: println

fun main() {
    set_color_mode(COLOR_ALWAYS)
    set ok = "OK"
    set err = "ERROR"
    set custom = "CUSTOM COLOR"

    println(green(ok))
    println(red(err))
    println(bright_cyan(custom))
    println(color256(custom, 208))
    println(rgb(custom, 255, 128, 64))
}
```

---

### 6.4 Background Colors

#### Standard 8 Backgrounds

- `bg_black(text: string) -> string` - Black background (code `40`).
- `bg_red(text: string) -> string` - Red background (code `41`).
- `bg_green(text: string) -> string` - Green background (code `42`).
- `bg_yellow(text: string) -> string` - Yellow background (code `43`).
- `bg_blue(text: string) -> string` - Blue background (code `44`).
- `bg_magenta(text: string) -> string` - Magenta background (code `45`).
- `bg_cyan(text: string) -> string` - Cyan background (code `46`).
- `bg_white(text: string) -> string` - White background (code `47`).

#### Bright 8 Backgrounds

- `bg_gray(text: string) -> string` - Gray background (code `100`).
- `bg_bright_black(text: string) -> string` - Bright black background (code `100`).
- `bg_bright_red(text: string) -> string` - Bright red background (code `101`).
- `bg_bright_green(text: string) -> string` - Bright green background (code `102`).
- `bg_bright_yellow(text: string) -> string` - Bright yellow background (code `103`).
- `bg_bright_blue(text: string) -> string` - Bright blue background (code `104`).
- `bg_bright_magenta(text: string) -> string` - Bright magenta background (code `105`).
- `bg_bright_cyan(text: string) -> string` - Bright cyan background (code `106`).
- `bg_bright_white(text: string) -> string` - Bright white background (code `107`).

#### bg_color256

Applies a background color from the extended 256-color palette (`\x1b[48;5;<code>m`). The color index is clamped to `0..255`.

```yukino
public fun bg_color256(text: string, code: int32) -> string
```

Returns a formatted string of type `string`. The return value is not nullable. This function never triggers a panic.

#### bg_rgb

Applies a 24-bit TrueColor RGB background color (`\x1b[48;2;<r>;<g>;<b>m`). Each component is clamped to `0..255`.

```yukino
public fun bg_rgb(text: string, r: int32, g: int32, b: int32) -> string
```

Returns a formatted string of type `string`. The return value is not nullable. This function never triggers a panic.

```yukino
load std.outf: bg_blue, bg_color256, bg_rgb, set_color_mode, COLOR_ALWAYS
load std.io: println

fun main() {
    set_color_mode(COLOR_ALWAYS)
    set text = " Highlighted Text "
    println(bg_blue(text))
    println(bg_color256(text, 236))
    println(bg_rgb(text, 30, 30, 30))
}
```

---

### 6.5 Fluent Style Builders

#### style

Creates a new instance of the `Styled` class for building composite chained formatting.

```yukino
public fun style(text: string) -> Styled
```

Returns an instance of the `Styled` class. The return value is not nullable. This function never triggers a panic.

#### styled

Alias for `style`, provided for concise and natural reading flow in expressions.

```yukino
public fun styled(text: string) -> Styled
```

Returns an instance of the `Styled` class. The return value is not nullable. This function never triggers a panic.

```yukino
load std.outf: style, styled, set_color_mode, COLOR_ALWAYS
load std.io: println

fun main() {
    set_color_mode(COLOR_ALWAYS)
    set log = "CRITICAL ERROR"
    println(style(log).bold().red().bg_black().render())
    println(styled(log).underline().yellow().to_string())
}
```

---

### 6.6 ANSI Utilities

#### clear_style

Appends the reset sequence `\x1b[0m` to a copy of the input string to ensure all active terminal attributes are cleared after output.

```yukino
public fun clear_style(text: string) -> string
```

Returns the resulting string of type `string`. The return value is not nullable. This function never triggers a panic.

```yukino
load std.outf: clear_style
load std.io: print

fun main() {
    set raw = "\x1b[31mPartial warning"
    print(clear_style(raw))
}
```

#### strip_ansi

Removes all ANSI CSI (`\x1b[...`) and OSC (`\x1b]...`) escape sequences from the string, returning clean plain text. Useful for log files or text alignment calculations.

```yukino
public fun strip_ansi(text: string) -> string
```

Returns the cleaned string of type `string`. The return value is not nullable. This function never triggers a panic.

```yukino
load std.outf: red, bold, strip_ansi, set_color_mode, COLOR_ALWAYS
load std.io: println

fun main() {
    set_color_mode(COLOR_ALWAYS)
    set sample = "Hello"
    set colored_sample = bold(sample)
    set clean = strip_ansi(colored_sample)
    println("Original colored: " + colored_sample)
    println("Stripped clean:   " + clean)
}
```

#### visible_len

Computes the visible length of a string in Unicode codepoints after stripping all ANSI escape codes. Ensures proper column alignment in CLI tables and banners.

```yukino
public fun visible_len(text: string) -> int64
```

Returns the number of visible codepoints of type `int64`. The return value is not nullable. This function never triggers a panic.

```yukino
load std.outf: green, visible_len, set_color_mode, COLOR_ALWAYS
load std.io: println

fun main() {
    set_color_mode(COLOR_ALWAYS)
    set text = "Привет"
    set formatted = green(text)
    println("Byte length:    ${formatted.len()}")
    println("Visible length: ${visible_len(formatted)}")
}
```

#### visible_bytes

Computes the byte length of a string after stripping all ANSI escape codes.

```yukino
public fun visible_bytes(text: string) -> int64
```

Returns the byte count of type `int64`. The return value is not nullable. This function never triggers a panic.

```yukino
load std.outf: red, visible_bytes, set_color_mode, COLOR_ALWAYS
load std.io: println

fun main() {
    set_color_mode(COLOR_ALWAYS)
    set sample = "Data"
    set styled_sample = red(sample)
    println("Visible bytes: ${visible_bytes(styled_sample)}")
}
```

---

[← Previous: Module std.jsonc](21-jsonc.md) | [Next: Module std.line →](23-line.md)
