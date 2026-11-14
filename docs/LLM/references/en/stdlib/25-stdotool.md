# Module std.stdotool

The `std.stdotool` (Standard Output Tool) module provides utilities for dynamic visual rendering in standard terminal `stdout`. Unlike full-screen TUI toolkits that capture the terminal via private alternate screen buffers, `stdotool` operates directly within normal console scrolling flow, preserving previous output history. It pairs low-level cursor and line manipulation primitives with flexible visual building blocks: a cyclic custom animation frame player (`Frames`), a customizable progress bar constructor (`Bar`), and in-place single-line (`LiveLine`) and multi-line (`LiveArea`) flicker-free live re-rendering controllers (ideal for multi-download animations like `pacman -Syyu`).

---

## 1. Import: load std.stdotool

Import the entire package:

```yukino
load std.stdotool
```

This brings the following entities into scope:

- **Classes:** `Frames`, `Bar`, `LiveLine`, `LiveArea`.
- **Cursor and Line Control Functions:** `up`, `down`, `forward`, `back`, `to_col`, `carriage_return`, `clear_line`, `clear_to_end`, `clear_from_start`, `clear_lines_up`, `hide_cursor`, `show_cursor`, `rewrite_line`.
- **Formatting Functions:** `format_bytes`, `format_speed`, `format_percent`, `format_duration`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

No global constants are declared in `std.stdotool`. All animation frames and visual characters are configured explicitly by the user.

---

## 3. Interfaces

No interfaces are declared in `std.stdotool`.

---

## 4. Data and Panic Structures

No data structures or panic types are declared in `std.stdotool`. All operations execute deterministically without panicking.

---

## 5. Classes

### 5.1 class Frames

A universal cyclic frame animator. The developer passes an arbitrary array of string sprites (ASCII characters, Unicode symbols, or multi-frame text phases).

```yukino
public class Frames {
    public items: string[]
    public index: int32
}
```

#### 5.1.1 init

Initializes the frame animator with the given array of string sprites, starting at index 0.

```yukino
public fun init(items: string[])
```

Returns an initialized `Frames` object. This method never panics.

```yukino
load std.stdotool.frames: Frames

fun main() {
    set spinner = init Frames(["-", "\\", "|", "/"])
}
```

#### 5.1.2 next

Returns the current frame and advances the internal index by one, wrapping around to the beginning when reaching the end of the array.

```yukino
public fun next() -> string
```

Returns a non-nullable string representing the next frame, or an empty string if the sequence is empty. This method never panics.

```yukino
load std.stdotool.frames: Frames
load std.io: println

fun main() {
    set mouth = init Frames(["C", "c"])
    println(mouth.next()) // "C"
    println(mouth.next()) // "c"
    println(mouth.next()) // "C"
}
```

#### 5.1.3 current

Returns the current active frame without advancing the internal pointer.

```yukino
public fun current() -> string
```

Returns a non-nullable string value. This method never panics.

```yukino
load std.stdotool.frames: Frames
load std.io: println

fun main() {
    set f = init Frames(["A", "B"])
    println(f.current()) // "A"
}
```

#### 5.1.4 reset

Resets the active frame index back to 0.

```yukino
public fun reset()
```

This method returns no value and never panics.

```yukino
load std.stdotool.frames: Frames

fun main() {
    set f = init Frames(["1", "2"])
    f.next()
    f.reset()
}
```

#### 5.1.5 len

Returns the total number of frames in the sequence.

```yukino
public fun len() -> int32
```

Returns a 32-bit integer. This method never panics.

```yukino
load std.stdotool.frames: Frames
load std.io: println

fun main() {
    set f = init Frames(["A", "B", "C"])
    println("Count: " + cast(f.len(), "string"))
}
```

---

### 5.2 class Bar

A customizable progress bar renderer. Developers configure custom characters for the filled body, leading head/cursor, unfilled trail, and bounding brackets.

```yukino
public class Bar {
    public body: string
    public head: string
    public trail: string
    public left: string
    public right: string
}
```

#### 5.2.1 init

Initializes the progress bar with custom body, head, trail, and bracket characters.

```yukino
public fun init(
    body: string = "=",
    head: string = ">",
    trail: string = " ",
    left: string = "[",
    right: string = "]"
)
```

Returns an initialized `Bar` object. This method never panics.

```yukino
load std.stdotool.bar: Bar

fun main() {
    // Pacman style: eating dots
    set pacman_bar = init Bar("-", "C", "•", "[", "]")

    // Solid block style
    set block_bar = init Bar("█", "█", "░", "[", "]")
}
```

#### 5.2.2 render

Constructs a formatted progress bar string for `current` progress out of `total`, fitting exactly within `width` characters.

```yukino
public fun render(current: int64, total: int64, width: int32) -> string
```

Returns a non-nullable string. This method never panics. If `total <= 0`, it renders an empty bar; if `current >= total`, it renders a full bar.

```yukino
load std.stdotool.bar: Bar
load std.io: println

fun main() {
    set bar = init Bar("-", "C", "•", "[", "]")
    println(bar.render(45, 100, 20))
}
```

#### 5.2.3 set_head

Dynamically replaces the leading head character of the progress bar, useful for animated mouths or rotating spinners.

```yukino
public fun set_head(head: string)
```

This method returns no value and never panics.

```yukino
load std.stdotool.bar: Bar
load std.stdotool.frames: Frames

fun main() {
    set mouth = init Frames(["C", "c"])
    set bar = init Bar("-", "C", "•", "[", "]")
    bar.set_head(mouth.next())
}
```

---

### 5.3 class LiveLine

Manages dynamic single-line terminal output, rewriting the current line in-place with automatic trailing character clearing.

```yukino
public class LiveLine {
    public last_len: int32
    public active: bool
}
```

#### 5.3.1 init

Initializes the live line controller, optionally writing `initial_text`.

```yukino
public fun init(initial_text: string = "")
```

Returns an initialized `LiveLine` object. This method never panics.

```yukino
load std.stdotool.live_line: LiveLine

fun main() {
    set line = init LiveLine()
}
```

#### 5.3.2 write

Clears the current terminal line and prints updated text on the same line without appending a newline.

```yukino
public fun write(text: string)
```

This method returns no value and never panics.

```yukino
load std.stdotool.live_line: LiveLine

fun main() {
    set line = init LiveLine()
    line.write("Downloading 10%...")
    line.write("Downloading 20%...")
}
```

#### 5.3.3 clear

Erases the live line from the screen.

```yukino
public fun clear()
```

This method returns no value and never panics.

```yukino
load std.stdotool.live_line: LiveLine

fun main() {
    set line = init LiveLine("Temporary text")
    line.clear()
}
```

#### 5.3.4 finish

Finalizes the dynamic line by appending a newline (`\n`), committing the line to terminal scrollback history.

```yukino
public fun finish()
```

This method returns no value and never panics.

```yukino
load std.stdotool.live_line: LiveLine

fun main() {
    set line = init LiveLine()
    line.write("Complete")
    line.finish()
}
```

#### 5.3.5 finish_text

Writes final text in-place and appends a newline (`\n`), committing it to scrollback history.

```yukino
public fun finish_text(final_text: string)
```

This method returns no value and never panics.

```yukino
load std.stdotool.live_line: LiveLine

fun main() {
    set line = init LiveLine()
    line.write("Syncing...")
    line.finish_text("Syncing complete [OK]")
}
```

---

### 5.4 class LiveArea

Manages a dynamic multi-line region in standard stdout without alternate screen buffers. On each render call, it moves the cursor up to the top of the previously rendered area, clears lines, and repaints new lines without flicker.

```yukino
public class LiveArea {
    public rendered_count: int32
}
```

#### 5.4.1 init

Initializes an empty live area.

```yukino
public fun init()
```

Returns an initialized `LiveArea` object. This method never panics.

```yukino
load std.stdotool.live_area: LiveArea

fun main() {
    set area = init LiveArea()
}
```

#### 5.4.2 render

Renders the provided array of lines. If previously rendered, the cursor is moved up by `rendered_count` lines and all lines are repainted in-place.

```yukino
public fun render(lines: string[])
```

This method returns no value and never panics.

```yukino
load std.stdotool.live_area: LiveArea

fun main() {
    set area = init LiveArea()
    area.render(["Task 1: [===>   ]", "Task 2: [=======>]"])
}
```

#### 5.4.3 clear

Erases the previously rendered multi-line block from the screen and returns the cursor to the top position.

```yukino
public fun clear()
```

This method returns no value and never panics.

```yukino
load std.stdotool.live_area: LiveArea

fun main() {
    set area = init LiveArea()
    area.render(["Line 1", "Line 2"])
    area.clear()
}
```

#### 5.4.4 finish

Finalizes the multi-line block, keeping the last frame fixed in terminal scrollback history.

```yukino
public fun finish()
```

This method returns no value and never panics.

```yukino
load std.stdotool.live_area: LiveArea
load std.io: println

fun main() {
    set area = init LiveArea()
    area.render(["core.db [DONE]", "extra.db [DONE]"])
    area.finish()
    println("Done.")
}
```

---

## 6. Function Reference

### 6.1 Cursor and Line Control

#### 6.1.1 up

Moves the cursor up by `n` lines (`\x1b[nA`).

```yukino
public fun up(n: int32 = 1)
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: up

fun main() {
    up(2)
}
```

#### 6.1.2 down

Moves the cursor down by `n` lines (`\x1b[nB`).

```yukino
public fun down(n: int32 = 1)
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: down

fun main() {
    down(1)
}
```

#### 6.1.3 forward

Moves the cursor forward by `n` columns (`\x1b[nC`).

```yukino
public fun forward(n: int32 = 1)
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: forward

fun main() {
    forward(4)
}
```

#### 6.1.4 back

Moves the cursor backward by `n` columns (`\x1b[nD`).

```yukino
public fun back(n: int32 = 1)
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: back

fun main() {
    back(2)
}
```

#### 6.1.5 to_col

Moves the cursor to the 0-based column index on the current line (`\x1b[nG`).

```yukino
public fun to_col(col: int32)
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: to_col

fun main() {
    to_col(10)
}
```

#### 6.1.6 carriage_return

Returns the cursor to the beginning of the line (`\r`).

```yukino
public fun carriage_return()
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: carriage_return

fun main() {
    carriage_return()
}
```

#### 6.1.7 clear_line

Clears the entire current line and returns the cursor to the start (`\r\x1b[2K`).

```yukino
public fun clear_line()
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: clear_line

fun main() {
    clear_line()
}
```

#### 6.1.8 clear_to_end

Clears characters from the cursor position to the end of the line (`\x1b[K`).

```yukino
public fun clear_to_end()
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: clear_to_end

fun main() {
    clear_to_end()
}
```

#### 6.1.9 clear_lines_up

Erases `n` lines above the cursor and positions the cursor at the first cleared line.

```yukino
public fun clear_lines_up(n: int32)
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: clear_lines_up

fun main() {
    clear_lines_up(3)
}
```

#### 6.1.10 hide_cursor

Hides the blinking terminal cursor (`\x1b[?25l`) to prevent visual flicker during animations.

```yukino
public fun hide_cursor()
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: hide_cursor

fun main() {
    hide_cursor()
}
```

#### 6.1.11 show_cursor

Restores terminal cursor visibility (`\x1b[?25h`).

```yukino
public fun show_cursor()
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: show_cursor

fun main() {
    show_cursor()
}
```

#### 6.1.12 rewrite_line

Clears the current line and writes the provided text in place.

```yukino
public fun rewrite_line(text: string)
```

This function returns no value and never panics.

```yukino
load std.stdotool.cursor: rewrite_line

fun main() {
    rewrite_line("Updated status")
}
```

---

### 6.2 Formatting Utilities

#### 6.2.1 format_bytes

Formats byte counts into a human-readable string (`B`, `KiB`, `MiB`, `GiB`).

```yukino
public fun format_bytes(bytes: int64) -> string
```

Returns a non-nullable string like `"142.5 KiB"`. This function never panics.

```yukino
load std.stdotool.format: format_bytes
load std.io: println

fun main() {
    println(format_bytes(2048)) // "2.0 KiB"
}
```

#### 6.2.2 format_speed

Formats transfer speed in bytes per second.

```yukino
public fun format_speed(bytes_per_sec: int64) -> string
```

Returns a non-nullable string like `"2.4 MiB/s"`. This function never panics.

```yukino
load std.stdotool.format: format_speed
load std.io: println

fun main() {
    println(format_speed(1572864)) // "1.5 MiB/s"
}
```

#### 6.2.3 format_percent

Calculates completion percentage and returns a right-aligned 4-character string (`" 45%"` or `"100%"`).

```yukino
public fun format_percent(current: int64, total: int64) -> string
```

Returns a non-nullable string. This function never panics.

```yukino
load std.stdotool.format: format_percent
load std.io: println

fun main() {
    println(format_percent(50, 100)) // " 50%"
}
```

#### 6.2.4 format_duration

Formats duration in seconds to `"MM:SS"` or `"HH:MM:SS"`.

```yukino
public fun format_duration(seconds: int64) -> string
```

Returns a non-nullable string. This function never panics.

```yukino
load std.stdotool.format: format_duration
load std.io: println

fun main() {
    println(format_duration(95)) // "01:35"
}
```

---

[← Previous: Module std.itermtool](24-itermtool.md) | [Next: Module std.hash →](26-hash.md)
