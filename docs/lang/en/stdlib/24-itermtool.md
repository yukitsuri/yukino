# Module std.itermtool

The `std.itermtool` (Interactive Terminal Tool) module provides a low-level interactive terminal engine and foundational primitives for building full-screen TUI frameworks and terminal interfaces on par with `ratatui` (serving as an equivalent to `crossterm` in the Rust ecosystem). The module delivers cross-platform terminal raw mode (**Raw Mode**), alternate screen buffer (**Alternate Screen**), 0-based cursor navigation, TrueColor 24-bit RGB and 256-color palette rendering, background rectangular fills, non-blocking event polling and keyboard parsing (**KeyEvent**), and comprehensive mouse capture and parsing (**MouseEvent**): buttons (left, middle, right, back, forward), movement, dragging with held buttons, and scroll wheel operations (up, down, left, right) over the modern SGR 1006 protocol without the overhead of heavy runtimes.

---

## 1. Import: load std.itermtool

Import the entire package:

```yukino
load std.itermtool
```

This brings the following entities into scope:

- **Key Constants:** `KEY_CHAR`, `KEY_ENTER`, `KEY_ESC`, `KEY_BACKSPACE`, `KEY_TAB`, `KEY_SPACE`, `KEY_UP`, `KEY_DOWN`, `KEY_LEFT`, `KEY_RIGHT`, `KEY_HOME`, `KEY_END`, `KEY_PAGE_UP`, `KEY_PAGE_DOWN`, `KEY_DELETE`, `KEY_INSERT`, `KEY_F1`..`KEY_F12`, `KEY_UNKNOWN`.
- **Mouse Button Constants:** `MOUSE_BTN_LEFT`, `MOUSE_BTN_MIDDLE`, `MOUSE_BTN_RIGHT`, `MOUSE_BTN_NONE`, `MOUSE_BTN_BACK`, `MOUSE_BTN_FORWARD`.
- **Mouse Action Constants:** `MOUSE_DOWN`, `MOUSE_UP`, `MOUSE_DRAG`, `MOUSE_MOVE`, `MOUSE_SCROLL_UP`, `MOUSE_SCROLL_DOWN`, `MOUSE_SCROLL_LEFT`, `MOUSE_SCROLL_RIGHT`.
- **Event Kind Constants:** `EVENT_NONE`, `EVENT_KEY`, `EVENT_MOUSE`.
- **Classes:** `Color`, `Cell`, `KeyEvent`, `MouseEvent`, `Event`.
- **Terminal and Screen Functions:** `enable_raw_mode`, `disable_raw_mode`, `is_raw_mode`, `enter_alternate_screen`, `leave_alternate_screen`, `disable_line_wrap`, `enable_line_wrap`, `clear_all`, `clear_line`, `clear_from_cursor`, `clear_to_cursor`, `clear_until_new_line`, `size`, `cols`, `rows`, `has_resized`, `scroll_up`, `scroll_down`, `set_title`.
- **Cursor and Drawing Functions:** `move_to`, `move_up`, `move_down`, `move_left`, `move_right`, `move_to_column`, `hide_cursor`, `show_cursor`, `save_cursor`, `restore_cursor`, `print_at`, `print_styled_at`, `draw_hline`, `draw_vline`, `fill_rect`, `draw_box`.
- **Color and Style Functions:** `ansi_fg_rgb`, `ansi_bg_rgb`, `ansi_fg_256`, `ansi_bg_256`, `ansi_fg_color`, `ansi_bg_color`, `ansi_reset`, `reset_color`, `style_bold`, `style_dim`, `style_italic`, `style_underline`, `style_reversed`, `style_reset`, `color_reset`, `color_named`, `color_256`, `color_rgb`, `empty_cell`.
- **Mouse Control Functions:** `enable_mouse`, `disable_mouse`, `enable_mouse_capture`, `disable_mouse_capture`, `is_mouse_active`, `toggle_mouse`, `toggle_mouse_capture`, `enable_mouse_motion`, `disable_mouse_motion`, `mouse_event`, `decode_mouse_sgr`, `parse_mouse_sgr`, `parse_mouse_x10`, `read_mouse`, `try_read_mouse`.
- **Event and Key Functions:** `poll_event`, `read_key`, `try_read_key`, `key_char`, `key_special`, `event_key`, `event_mouse`, `read_event`, `try_read_event`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

Numeric codes for special keyboard keys, mouse buttons, actions, and event kinds:

```yukino
// Keyboard
public static KEY_CHAR: int32 = 0
public static KEY_ENTER: int32 = 1
public static KEY_ESC: int32 = 2
public static KEY_BACKSPACE: int32 = 3
public static KEY_TAB: int32 = 4
public static KEY_SPACE: int32 = 5

public static KEY_UP: int32 = 10
public static KEY_DOWN: int32 = 11
public static KEY_LEFT: int32 = 12
public static KEY_RIGHT: int32 = 13

public static KEY_HOME: int32 = 14
public static KEY_END: int32 = 15
public static KEY_PAGE_UP: int32 = 16
public static KEY_PAGE_DOWN: int32 = 17
public static KEY_DELETE: int32 = 18
public static KEY_INSERT: int32 = 19

public static KEY_F1: int32 = 20
public static KEY_F2: int32 = 21
public static KEY_F3: int32 = 22
public static KEY_F4: int32 = 23
public static KEY_F5: int32 = 24
public static KEY_F6: int32 = 25
public static KEY_F7: int32 = 26
public static KEY_F8: int32 = 27
public static KEY_F9: int32 = 28
public static KEY_F10: int32 = 29
public static KEY_F11: int32 = 30
public static KEY_F12: int32 = 31

public static KEY_UNKNOWN: int32 = 99

// Mouse buttons
public static MOUSE_BTN_LEFT: int32 = 0
public static MOUSE_BTN_MIDDLE: int32 = 1
public static MOUSE_BTN_RIGHT: int32 = 2
public static MOUSE_BTN_NONE: int32 = 3
public static MOUSE_BTN_BACK: int32 = 4
public static MOUSE_BTN_FORWARD: int32 = 5

// Mouse actions
public static MOUSE_DOWN: int32 = 0
public static MOUSE_UP: int32 = 1
public static MOUSE_DRAG: int32 = 2
public static MOUSE_MOVE: int32 = 3
public static MOUSE_SCROLL_UP: int32 = 4
public static MOUSE_SCROLL_DOWN: int32 = 5
public static MOUSE_SCROLL_LEFT: int32 = 6
public static MOUSE_SCROLL_RIGHT: int32 = 7

// Event categories
public static EVENT_NONE: int32 = 0
public static EVENT_KEY: int32 = 1
public static EVENT_MOUSE: int32 = 2
```

---

## 3. Interfaces

No interfaces are declared in `std.itermtool`.

---

## 4. Data Structures and Error Types (Panics)

No error structures are declared in `std.itermtool`. All functions safely handle boundary conditions, window resizing events, and unknown escape sequences, returning safe default values or boolean flags, and never cause program crashes or panics.

---

## 5. Classes

### 5.1 class Color

Color representation for terminal foreground text and background cells. Supports 4 modes: reset, named basic ANSI color, 256-color palette index, and 24-bit TrueColor RGB.

```yukino
public class Color {
    public kind: int32
    public r: int32
    public g: int32
    public b: int32
    public index: int32
    public name: string
}
```

#### 5.1.1 init

Initializes a new `Color` instance.

```yukino
public fun init(kind: int32, r: int32, g: int32, b: int32, index: int32, name: string)
```

Returns an initialized `Color` object.

This method never panics.

```yukino
load std.itermtool.color: Color, color_rgb
load std.io: println

fun main() {
    set c = color_rgb(255, 128, 0)
    println("Kind: " + cast(c.kind, "string"))
}
```

#### 5.1.2 to_fg

Generates the ANSI escape sequence to set this color as the foreground text color.

```yukino
public fun to_fg() -> string
```

Returns a non-empty escape string, or an empty string if set to reset mode.

This method never panics.

```yukino
load std.itermtool.color: color_rgb, ansi_reset
load std.io: println

fun main() {
    set c = color_rgb(100, 200, 255)
    println(c.to_fg() + "Cyan text" + ansi_reset())
}
```

#### 5.1.3 to_bg

Generates the ANSI escape sequence to set this color as the background fill color.

```yukino
public fun to_bg() -> string
```

Returns an ANSI escape string, or an empty string if reset.

This method never panics.

```yukino
load std.itermtool.color: color_rgb, ansi_reset
load std.io: println

fun main() {
    set c = color_rgb(30, 40, 60)
    println(c.to_bg() + "Text on dark blue background" + ansi_reset())
}
```

---

### 5.2 class Cell

Represents a single cell of a TUI grid buffer in Ratatui style: symbol character, foreground and background colors, and style modifier flags.

```yukino
public class Cell {
    public symbol: string
    public fg: Color
    public bg: Color
    public bold: bool
    public dim: bool
    public italic: bool
    public underline: bool
    public reversed: bool
}
```

#### 5.2.1 init

Initializes a cell with symbol, colors, and styling attributes.

```yukino
public fun init(
    symbol: string,
    fg: Color,
    bg: Color,
    bold: bool = false,
    dim: bool = false,
    italic: bool = false,
    underline: bool = false,
    reversed: bool = false
)
```

Returns an initialized `Cell` object.

This method never panics.

```yukino
load std.itermtool.color: Cell, color_rgb, color_named

fun main() {
    set c = init Cell("X", color_named("yellow"), color_rgb(20, 30, 50), true)
}
```

#### 5.2.2 render

Renders the cell into an escape sequence string ready to be flushed to the terminal.

```yukino
public fun render() -> string
```

Returns a string representation of the styled cell.

This method never panics.

```yukino
load std.itermtool.color: Cell, color_named, color_rgb
load std.io: print

fun main() {
    set c = init Cell("A", color_named("green"), color_rgb(0, 0, 0), true)
    print(c.render())
}
```

---

### 5.3 class KeyEvent

Describes a keyboard event: numeric key code, string character representation, and modifier flags for Ctrl, Alt, and Shift.

```yukino
public class KeyEvent {
    public code: int32
    public char_val: string
    public ctrl: bool
    public alt: bool
    public shift: bool
}
```

#### 5.3.1 init

Creates a keyboard event instance.

```yukino
public fun init(code: int32, char_val: string, ctrl: bool = false, alt: bool = false, shift: bool = false)
```

Returns an initialized `KeyEvent` object.

This method never panics.

```yukino
load std.itermtool.key: KeyEvent, KEY_CHAR

fun main() {
    set ev = init KeyEvent(KEY_CHAR, "s", true, false, false)
}
```

#### 5.3.2 is_char

Checks whether the event corresponds to a specific character press without Ctrl or Alt modifiers.

```yukino
public fun is_char(c: string) -> bool
```

Returns a boolean value of type `bool`.

This method never panics.

```yukino
load std.itermtool.key: key_char
load std.io: println

fun main() {
    set ev = key_char("q")
    if ev.is_char("q") {
        println("Key q pressed")
    }
}
```

#### 5.3.3 is_ctrl

Checks whether the event represents a Ctrl key chord (e.g. Ctrl+C).

```yukino
public fun is_ctrl(c: string) -> bool
```

Returns a boolean value of type `bool`.

This method never panics.

```yukino
load std.itermtool.key: key_char
load std.io: println

fun main() {
    set ev = key_char("c", true)
    if ev.is_ctrl("c") {
        println("Ctrl+C pressed")
    }
}
```

#### 5.3.4 is_alt

Checks whether the event represents an Alt key chord.

```yukino
public fun is_alt(c: string) -> bool
```

Returns a boolean value of type `bool`.

This method never panics.

```yukino
load std.itermtool.key: key_char
load std.io: println

fun main() {
    set ev = key_char("x", false, true)
    if ev.is_alt("x") {
        println("Alt+X pressed")
    }
}
```

#### 5.3.5 to_string

Formats a human-readable description of the key chord with modifiers (e.g. `Ctrl+'c'`, `Shift+Up`, `Enter`, `F5`).

```yukino
public fun to_string() -> string
```

Returns a string description of the key.

This method never panics.

```yukino
load std.itermtool.key: key_special, KEY_UP
load std.io: println

fun main() {
    set ev = key_special(KEY_UP, true)
    println("Key: " + ev.to_string())
}
```

### 5.4 class MouseEvent

Represents a mouse event in the terminal window. Encapsulates the action kind (`kind`: press, release, move, drag, scroll), the button involved (`button`), 0-based screen coordinates (`x`, `y`), and modifier keys (`ctrl`, `alt`, `shift`).

```yukino
public class MouseEvent {
    public kind: int32
    public button: int32
    public x: int32
    public y: int32
    public ctrl: bool
    public alt: bool
    public shift: bool
}
```

#### 5.4.1 init

Constructor initializing a mouse event instance.

```yukino
public fun init(kind: int32, button: int32, x: int32, y: int32, ctrl: bool = false, alt: bool = false, shift: bool = false)
```

Initializes the instance fields with the provided values.

This method never panics.

```yukino
load std.itermtool.mouse: MouseEvent, MOUSE_DOWN, MOUSE_BTN_LEFT
load std.io: println

fun main() {
    set ev = init MouseEvent(MOUSE_DOWN, MOUSE_BTN_LEFT, 10, 5)
    println("Mouse position: " + cast(ev.x, "string") + "x" + cast(ev.y, "string"))
}
```

#### 5.4.2 is_left_click / is_right_click / is_middle_click

Predicates for checking specific mouse button clicks.

```yukino
public fun is_left_click() -> bool
public fun is_right_click() -> bool
public fun is_middle_click() -> bool
```

Returns `true` if the event is a `MOUSE_DOWN` action matching the corresponding button, otherwise `false`.

These methods never panic.

```yukino
load std.itermtool.mouse: mouse_event, MOUSE_DOWN, MOUSE_BTN_LEFT
load std.io: println

fun main() {
    set ev = mouse_event(MOUSE_DOWN, MOUSE_BTN_LEFT, 0, 0)
    if ev.is_left_click() {
        println("Left mouse button pressed")
    }
}
```

#### 5.4.3 is_scroll_up / is_scroll_down / is_scroll_left / is_scroll_right

Predicates checking the scroll wheel orientation and movement.

```yukino
public fun is_scroll_up() -> bool
public fun is_scroll_down() -> bool
public fun is_scroll_left() -> bool
public fun is_scroll_right() -> bool
```

Returns `true` if the event matches the specified wheel direction, otherwise `false`.

These methods never panic.

```yukino
load std.itermtool.mouse: mouse_event, MOUSE_SCROLL_UP, MOUSE_BTN_NONE
load std.io: println

fun main() {
    set ev = mouse_event(MOUSE_SCROLL_UP, MOUSE_BTN_NONE, 15, 20)
    if ev.is_scroll_up() {
        println("Scrolled up at 15, 20")
    }
}
```

#### 5.4.4 is_down / is_up / is_drag / is_move

Predicates verifying the physical mouse action category.

```yukino
public fun is_down() -> bool
public fun is_up() -> bool
public fun is_drag() -> bool
public fun is_move() -> bool
```

Returns `true` if the action kind equals `MOUSE_DOWN`, `MOUSE_UP`, `MOUSE_DRAG`, or `MOUSE_MOVE`, otherwise `false`.

These methods never panic.

```yukino
load std.itermtool.mouse: mouse_event, MOUSE_DRAG, MOUSE_BTN_LEFT
load std.io: println

fun main() {
    set ev = mouse_event(MOUSE_DRAG, MOUSE_BTN_LEFT, 5, 5)
    if ev.is_drag() {
        println("Dragging with held button")
    }
}
```

#### 5.4.5 to_string

Formats a descriptive string containing action name, button name, coordinates, and active modifiers.

```yukino
public fun to_string() -> string
```

Returns a string representation of the mouse event.

This method never panics.

```yukino
load std.itermtool.mouse: mouse_event, MOUSE_DOWN, MOUSE_BTN_LEFT
load std.io: println

fun main() {
    set ev = mouse_event(MOUSE_DOWN, MOUSE_BTN_LEFT, 12, 8, true)
    println(ev.to_string()) // "Ctrl+MouseEvent(Down, btn=Left, x=12, y=8)"
}
```

### 5.5 class Event

Universal polymorphic container encapsulating either a keyboard event (`KeyEvent`) or a mouse event (`MouseEvent`).

```yukino
public class Event {
    public kind: int32
    public key: KeyEvent?
    public mouse: MouseEvent?
}
```

#### 5.5.1 init

Universal event constructor.

```yukino
public fun init(kind: int32, key: KeyEvent? = null, mouse: MouseEvent? = null)
```

Initializes the event with the given kind (`EVENT_KEY`, `EVENT_MOUSE`, or `EVENT_NONE`) and associated event object.

This method never panics.

```yukino
load std.itermtool.event: Event, EVENT_KEY
load std.itermtool.key: key_char
load std.io: println

fun main() {
    set ev = init Event(EVENT_KEY, key_char("q"))
    if ev.is_key() {
        println("Keyboard event")
    }
}
```

#### 5.5.2 is_key / is_mouse

Predicates identifying the received event type.

```yukino
public fun is_key() -> bool
public fun is_mouse() -> bool
```

Returns `true` if the event is a keyboard or mouse event respectively.

These methods never panic.

```yukino
load std.itermtool.event: read_event
load std.io: println

fun main() {
    set ev = read_event()
    if ev.is_mouse() {
        println("User interacted with the mouse")
    }
}
```

#### 5.5.3 to_string

Formats a readable string detailing the underlying event.

```yukino
public fun to_string() -> string
```

Returns a string such as `Event::Key('q')` or `Event::Mouse(MouseEvent(Down, btn=Left, x=0, y=0))`.

This method never panics.

```yukino
load std.itermtool.event: event_key
load std.itermtool.key: key_char
load std.io: println

fun main() {
    set ev = event_key(key_char("a"))
    println(ev.to_string())
}
```

---

## 6. Function Reference

### 6.1 Terminal and Screen Control

#### 6.1.1 enable_raw_mode

Switches the terminal to raw mode, disabling OS line buffering and local echoing. On Windows, enables Virtual Terminal Input.

```yukino
public fun enable_raw_mode() -> bool
```

Returns `true` on success, or `false` on failure.

This function never panics.

```yukino
load std.itermtool.terminal: enable_raw_mode, disable_raw_mode

fun main() {
    if enable_raw_mode() {
        disable_raw_mode()
    }
}
```

#### 6.1.2 disable_raw_mode

Restores canonical terminal line buffering and echo.

```yukino
public fun disable_raw_mode() -> bool
```

Returns a boolean value of type `bool`.

This function never panics.

```yukino
load std.itermtool.terminal: disable_raw_mode

fun main() {
    disable_raw_mode()
}
```

#### 6.1.3 enter_alternate_screen

Enters the private alternate screen buffer (`\x1b[?1049h`), standard for full-screen TUI applications.

```yukino
public fun enter_alternate_screen()
```

This function returns no value.

This function never panics.

```yukino
load std.itermtool.terminal: enter_alternate_screen, leave_alternate_screen

fun main() {
    enter_alternate_screen()
    leave_alternate_screen()
}
```

#### 6.1.4 leave_alternate_screen

Leaves the private alternate screen buffer (`\x1b[?1049l`) and restores the caller's previous shell scrollback buffer.

```yukino
public fun leave_alternate_screen()
```

This function returns no value.

This function never panics.

```yukino
load std.itermtool.terminal: leave_alternate_screen

fun main() {
    leave_alternate_screen()
}
```

#### 6.1.5 clear_all

Clears the entire screen and moves cursor to home position (`\x1b[2J\x1b[H`).

```yukino
public fun clear_all()
```

This function returns no value and never panics.

```yukino
load std.itermtool.terminal: clear_all

fun main() {
    clear_all()
}
```

#### 6.1.6 size

Queries the current terminal window dimensions from the operating system.

```yukino
public fun size() -> (int32, int32)
```

Returns a tuple of two 32-bit integers `(cols, rows)`.

This function never panics, falling back to `(80, 24)` if querying fails.

```yukino
load std.itermtool.terminal: size
load std.io: println

fun main() {
    set sz = size()
    println("Cols: " + cast(sz.0, "string") + ", Rows: " + cast(sz.1, "string"))
}
```

#### 6.1.7 has_resized

Checks if the terminal window size has changed since the recorded `last_cols` and `last_rows`.

```yukino
public fun has_resized(last_cols: int32, last_rows: int32) -> bool
```

Returns `true` if current width or height differs from the provided values, otherwise `false`. This function never panics.

```yukino
load std.itermtool.terminal: has_resized, size, clear_all

fun main() {
    set cols = 80
    set rows = 24
    if has_resized(cols, rows) {
        clear_all()
    }
}
```

#### 6.1.8 disable_line_wrap

Disables terminal auto-wrap mode (`\x1b[?7l`, DECAWM). Prevents unintended line wrapping and viewport scrolling when rendering interface elements wider than the terminal window. Automatically invoked inside `enter_alternate_screen()`.

```yukino
public fun disable_line_wrap()
```

This function returns no value and never panics.

```yukino
load std.itermtool.terminal: disable_line_wrap

fun main() {
    disable_line_wrap()
}
```

#### 6.1.9 enable_line_wrap

Enables terminal auto-wrap mode (`\x1b[?7h`). Automatically restored inside `leave_alternate_screen()`.

```yukino
public fun enable_line_wrap()
```

This function returns no value and never panics.

```yukino
load std.itermtool.terminal: enable_line_wrap

fun main() {
    enable_line_wrap()
}
```

#### 6.1.10 set_title

Sets the terminal window or tab title.

```yukino
public fun set_title(title: string)
```

This function returns no value and never panics.

```yukino
load std.itermtool.terminal: set_title

fun main() {
    set_title("My TUI App")
}
```

---

### 6.2 Cursor Navigation and Drawing

#### 6.2.1 move_to

Positions the cursor at 0-based `(x, y)` grid coordinates (`0, 0` is the top-left cell).

```yukino
public fun move_to(x: int32, y: int32)
```

This function returns no value and never panics.

```yukino
load std.itermtool.draw: move_to
load std.io: print

fun main() {
    move_to(10, 5)
    print("Placed at x=10, y=5")
}
```

#### 6.2.2 hide_cursor / show_cursor

Hides or shows the terminal hardware text cursor.

```yukino
public fun hide_cursor()
public fun show_cursor()
```

These functions return no value and never panic.

```yukino
load std.itermtool.draw: hide_cursor, show_cursor

fun main() {
    hide_cursor()
    show_cursor()
}
```

#### 6.2.3 print_at

Moves cursor to 0-based `(x, y)` coordinates and prints text.

```yukino
public fun print_at(x: int32, y: int32, text: string)
```

This function returns no value and never panics.

```yukino
load std.itermtool.draw: print_at

fun main() {
    print_at(0, 0, "Header text")
}
```

#### 6.2.4 print_styled_at

Moves cursor to `(x, y)`, sets foreground and background escape sequences, prints text, and resets colors.

```yukino
public fun print_styled_at(x: int32, y: int32, text: string, fg_seq: string, bg_seq: string)
```

This function returns no value and never panics.

```yukino
load std.itermtool.draw: print_styled_at
load std.itermtool.color: ansi_fg_rgb, ansi_bg_rgb

fun main() {
    print_styled_at(2, 2, "Alert", ansi_fg_rgb(255, 255, 0), ansi_bg_rgb(180, 0, 0))
}
```

#### 6.2.5 fill_rect

Fills a rectangular region of the screen with a background color escape sequence and filler character (default space), providing Ratatui-style canvas backgrounds.

```yukino
public fun fill_rect(x: int32, y: int32, width: int32, height: int32, bg_seq: string, ch: string = " ")
```

This function returns no value and never panics.

```yukino
load std.itermtool.draw: fill_rect
load std.itermtool.color: ansi_bg_rgb

fun main() {
    set bg = ansi_bg_rgb(20, 30, 45)
    fill_rect(0, 0, 80, 24, bg, " ")
}
```

#### 6.2.6 draw_box

Renders a bordered box panel using box-drawing characters (`┌─┐│└─┘`).

```yukino
public fun draw_box(x: int32, y: int32, width: int32, height: int32, fg_seq: string = "", bg_seq: string = "")
```

This function returns no value and never panics.

```yukino
load std.itermtool.draw: draw_box
load std.itermtool.color: ansi_fg_rgb

fun main() {
    draw_box(4, 2, 40, 10, ansi_fg_rgb(80, 160, 240), "")
}
```

---

### 6.3 Colors and ANSI Generators

#### 6.3.1 ansi_fg_rgb / ansi_bg_rgb

Generates a 24-bit TrueColor escape sequence for text (`\x1b[38;2;r;g;bm`) or background (`\x1b[48;2;r;g;bm`).

```yukino
public fun ansi_fg_rgb(r: int32, g: int32, b: int32) -> string
public fun ansi_bg_rgb(r: int32, g: int32, b: int32) -> string
```

Returns an ANSI escape string.

These functions never panic.

```yukino
load std.itermtool.color: ansi_fg_rgb, ansi_reset
load std.io: println

fun main() {
    println(ansi_fg_rgb(255, 100, 50) + "Orange text" + ansi_reset())
}
```

#### 6.3.2 ansi_fg_256 / ansi_bg_256

Generates an ANSI escape sequence from the 256-color palette (`0..255`).

```yukino
public fun ansi_fg_256(idx: int32) -> string
public fun ansi_bg_256(idx: int32) -> string
```

Returns an escape sequence string.

These functions never panic.

```yukino
load std.itermtool.color: ansi_fg_256, ansi_reset
load std.io: println

fun main() {
    println(ansi_fg_256(214) + "Color 214" + ansi_reset())
}
```

#### 6.3.3 ansi_reset

Returns the universal ANSI graphic reset sequence (`\x1b[0m`).

```yukino
public fun ansi_reset() -> string
```

Returns `"\x1b[0m"`.

This function never panics.

```yukino
load std.itermtool.color: ansi_reset
load std.io: print

fun main() {
    print(ansi_reset())
}
```

---

### 6.4 Keyboard Event Handling

#### 6.4.1 poll_event

Performs a non-blocking or timed check for available input bytes on standard input.

```yukino
public fun poll_event(timeout_ms: int64 = 0) -> bool
```

Returns `true` if an event is available, or `false` on timeout.

This function never panics.

```yukino
load std.itermtool.event: poll_event, read_key
load std.io: println

fun main() {
    if poll_event(100) {
        set ev = read_key()
        println("Pressed: " + ev.to_string())
    }
}
```

#### 6.4.2 read_key

Blocks until a key is pressed, parsing raw bytes and VT escape sequences into a structured `KeyEvent` object. Correctly recognizes arrows, F1-F12 keys, chords with Ctrl/Alt/Shift, and multi-byte UTF-8 inputs.

```yukino
public fun read_key() -> KeyEvent
```

Returns an initialized `KeyEvent` object.

This function never panics. Returns `KEY_UNKNOWN` on read failure.

```yukino
load std.itermtool.event: read_key
load std.itermtool.key: KEY_ESC
load std.io: println

fun main() {
    set ev = read_key()
    if ev.code == KEY_ESC {
        println("Escape pressed")
    }
}
```

#### 6.4.3 try_read_key

Attempts to immediately read the next keyboard event without blocking the thread.

```yukino
public fun try_read_key() -> KeyEvent?
```

Returns a `KeyEvent` if a key was pressed, or `null` if the input buffer is empty.

This function never panics.

```yukino
load std.itermtool.event: try_read_key
load std.io: println

fun main() {
    set ev = try_read_key()
    if ev != null {
        set k = ev ?? panic("unreachable")
        println("Key: " + k.to_string())
    }
}
```

### 6.5 Mouse Control and Events

#### 6.5.1 enable_mouse / disable_mouse / toggle_mouse

Controls mouse reporting in the terminal. The `enable_mouse` function enables mouse tracking (alias: `enable_mouse_capture`), while `disable_mouse` disables it (alias: `disable_mouse_capture`). Configures modern SGR 1006 protocol reporting button clicks, dragging, and wheel events, and adjusts Windows Console input mode when running on Windows.

To toggle mouse tracking dynamically, use `toggle_mouse` (alias: `toggle_mouse_capture`). To inspect whether mouse capture is active, use `is_mouse_active`.

```yukino
public fun enable_mouse()
public fun disable_mouse()
public fun enable_mouse_capture()
public fun disable_mouse_capture()
public fun is_mouse_active() -> bool
public fun toggle_mouse() -> bool
public fun toggle_mouse_capture() -> bool
```

`enable_mouse`, `disable_mouse`, `enable_mouse_capture`, and `disable_mouse_capture` return no value (`void`). `is_mouse_active`, `toggle_mouse`, and `toggle_mouse_capture` return a boolean `bool` reflecting whether mouse reporting is currently enabled.

These functions never panic.

```yukino
load std.itermtool.mouse: enable_mouse, disable_mouse, toggle_mouse, is_mouse_active
load std.itermtool.terminal: enable_raw_mode, disable_raw_mode
load std.io: println

fun main() {
    enable_raw_mode()
    set active = toggle_mouse() // Enables mouse capture
    println("Mouse active: " + cast(active, "string"))

    toggle_mouse() // Disables mouse capture
    disable_raw_mode()
}
```

#### 6.5.2 enable_mouse_motion / disable_mouse_motion

Enables or disables full mouse motion tracking (VT Any-Event Tracking 1003). When enabled, the terminal emits move events whenever the mouse cursor moves, even without held buttons.

```yukino
public fun enable_mouse_motion()
public fun disable_mouse_motion()
```

These functions return no value (`void`).

These functions never panic.

```yukino
load std.itermtool.mouse: enable_mouse, enable_mouse_motion, disable_mouse_motion, disable_mouse

fun main() {
    enable_mouse()
    enable_mouse_motion() // Report hovering coordinates

    disable_mouse_motion()
    disable_mouse()
}
```

#### 6.5.3 read_mouse

Blocks until a mouse escape sequence arrives from standard input and decodes it into a structured `MouseEvent`.

```yukino
public fun read_mouse() -> MouseEvent?
```

Returns a `MouseEvent` instance, or `null` if the input stream ended or the sequence was not a mouse event.

This function never panics.

```yukino
load std.itermtool.mouse: enable_mouse, disable_mouse, read_mouse
load std.itermtool.terminal: enable_raw_mode, disable_raw_mode
load std.io: println

fun main() {
    enable_raw_mode()
    enable_mouse()

    set ev = read_mouse()
    if ev != null {
        set m = ev ?? panic("unreachable")
        println("Click at: " + cast(m.x, "string") + "," + cast(m.y, "string"))
    }

    disable_mouse()
    disable_raw_mode()
}
```

#### 6.5.4 try_read_mouse

Performs a non-blocking check on the input buffer for an available mouse event.

```yukino
public fun try_read_mouse() -> MouseEvent?
```

Returns a `MouseEvent` if a mouse sequence is waiting in the input buffer, or `null` otherwise.

This function never panics.

```yukino
load std.itermtool.mouse: try_read_mouse
load std.io: println

fun main() {
    set ev = try_read_mouse()
    if ev != null {
        set m = ev ?? panic("unreachable")
        println("Mouse: " + m.to_string())
    }
}
```

#### 6.5.5 decode_mouse_sgr

Low-level decoder translating SGR 1006 parameters into a `MouseEvent` instance. Automatically parses Ctrl, Alt, and Shift modifiers, action kind (down, up, drag, move, scroll), and normalizes 1-based terminal coordinates to 0-based `itermtool` coordinates.

```yukino
public fun decode_mouse_sgr(code: int32, x_raw: int32, y_raw: int32, is_release: bool) -> MouseEvent
```

Returns the created `MouseEvent` instance.

This function never panics.

```yukino
load std.itermtool.mouse: decode_mouse_sgr
load std.io: println

fun main() {
    // 0 = Left button, x=10, y=5, press
    set ev = decode_mouse_sgr(0, 10, 5, false)
    println("Normalized 0-based coords: " + cast(ev.x, "string") + "x" + cast(ev.y, "string")) // 9x4
}
```

### 6.6 Unified Terminal Events

#### 6.6.1 read_event

Blocks until any terminal event (keyboard or mouse) arrives and returns a polymorphic `Event` wrapper. Recommended as the primary event loop function for full TUI applications.

```yukino
public fun read_event() -> Event
```

Returns an `Event` object. If a key event was captured, `key` holds `KeyEvent`; if a mouse action was captured, `mouse` holds `MouseEvent`.

This function never panics.

```yukino
load std.itermtool.event: read_event
load std.itermtool.mouse: enable_mouse, disable_mouse
load std.itermtool.terminal: enable_raw_mode, disable_raw_mode
load std.io: println

fun main() {
    enable_raw_mode()
    enable_mouse()

    set ev = read_event()
    if ev.is_key() {
        println("Key: " + ev.to_string())
    } elif ev.is_mouse() {
        println("Mouse: " + ev.to_string())
    }

    disable_mouse()
    disable_raw_mode()
}
```

#### 6.6.2 try_read_event

Non-blocking check and read of the next available terminal event without delaying the execution thread.

```yukino
public fun try_read_event() -> Event?
```

Returns an `Event` instance if input is ready, or `null` if the buffer is empty.

This function never panics.

```yukino
load std.itermtool.event: try_read_event
load std.io: println

fun main() {
    set ev = try_read_event()
    if ev != null {
        set e = ev ?? panic("unreachable")
        println("Event: " + e.to_string())
    }
}
```

---

[← Previous: Module std.line](23-line.md) | [Next: Module std.stdotool →](25-stdotool.md)
