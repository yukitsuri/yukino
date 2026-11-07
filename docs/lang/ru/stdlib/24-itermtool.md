# Модуль std.itermtool

Модуль `std.itermtool` (Interactive Terminal Tool) предоставляет низкоуровневый интерактивный терминальный движок и фундаментальную основу для построения полноэкранных TUI-фреймворков и консольных интерфейсов уровня `ratatui` (аналог `crossterm` в экосистеме Rust). Модуль обеспечивает кроссплатформенное управление сырым режимом терминала (**Raw Mode**), приватным буфером (**Alternate Screen**), 0-индексированными координатами курсора, заливкой фонов и блоков (TrueColor 24-bit RGB, палитра 256 цветов, базовая ANSI-палитра), неблокирующий опрос и парсинг событий клавиатуры (**KeyEvent**), а также полноценный захват и обработку событий мыши (**MouseEvent**): кнопок (левая, средняя, правая, назад, вперед), перемещения, зажатия (drag) и колесика прокрутки (вверх, вниз, влево, вправо) по современному протоколу SGR 1006.

---

## 1. Подключение: load std.itermtool

Импорт всего пакета:

```yukino
load std.itermtool
```

При этом становятся доступны следующие сущности:

- **Константы клавиш:** `KEY_CHAR`, `KEY_ENTER`, `KEY_ESC`, `KEY_BACKSPACE`, `KEY_TAB`, `KEY_SPACE`, `KEY_UP`, `KEY_DOWN`, `KEY_LEFT`, `KEY_RIGHT`, `KEY_HOME`, `KEY_END`, `KEY_PAGE_UP`, `KEY_PAGE_DOWN`, `KEY_DELETE`, `KEY_INSERT`, `KEY_F1`..`KEY_F12`, `KEY_UNKNOWN`.
- **Константы кнопок мыши:** `MOUSE_BTN_LEFT`, `MOUSE_BTN_MIDDLE`, `MOUSE_BTN_RIGHT`, `MOUSE_BTN_NONE`, `MOUSE_BTN_BACK`, `MOUSE_BTN_FORWARD`.
- **Константы действий мыши:** `MOUSE_DOWN`, `MOUSE_UP`, `MOUSE_DRAG`, `MOUSE_MOVE`, `MOUSE_SCROLL_UP`, `MOUSE_SCROLL_DOWN`, `MOUSE_SCROLL_LEFT`, `MOUSE_SCROLL_RIGHT`.
- **Константы видов событий:** `EVENT_NONE`, `EVENT_KEY`, `EVENT_MOUSE`.
- **Классы:** `Color`, `Cell`, `KeyEvent`, `MouseEvent`, `Event`.
- **Функции терминала и экрана:** `enable_raw_mode`, `disable_raw_mode`, `is_raw_mode`, `enter_alternate_screen`, `leave_alternate_screen`, `disable_line_wrap`, `enable_line_wrap`, `clear_all`, `clear_line`, `clear_from_cursor`, `clear_to_cursor`, `clear_until_new_line`, `size`, `cols`, `rows`, `has_resized`, `scroll_up`, `scroll_down`, `set_title`.
- **Функции курсора и отрисовки:** `move_to`, `move_up`, `move_down`, `move_left`, `move_right`, `move_to_column`, `hide_cursor`, `show_cursor`, `save_cursor`, `restore_cursor`, `print_at`, `print_styled_at`, `draw_hline`, `draw_vline`, `fill_rect`, `draw_box`.
- **Функции цвета и стилей:** `ansi_fg_rgb`, `ansi_bg_rgb`, `ansi_fg_256`, `ansi_bg_256`, `ansi_fg_color`, `ansi_bg_color`, `ansi_reset`, `reset_color`, `style_bold`, `style_dim`, `style_italic`, `style_underline`, `style_reversed`, `style_reset`, `color_reset`, `color_named`, `color_256`, `color_rgb`, `empty_cell`.
- **Функции управления мышью:** `enable_mouse`, `disable_mouse`, `enable_mouse_capture`, `disable_mouse_capture`, `is_mouse_active`, `toggle_mouse`, `toggle_mouse_capture`, `enable_mouse_motion`, `disable_mouse_motion`, `mouse_event`, `decode_mouse_sgr`, `parse_mouse_sgr`, `parse_mouse_x10`, `read_mouse`, `try_read_mouse`.
- **Функции событий и клавиатуры:** `poll_event`, `read_key`, `try_read_key`, `key_char`, `key_special`, `event_key`, `event_mouse`, `read_event`, `try_read_event`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.itermtool` объявлены числовые коды специальных клавиш клавиатуры, кнопок и действий мыши, а также категорий событий:

```yukino
// Клавиатура
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

// Кнопки мыши
public static MOUSE_BTN_LEFT: int32 = 0
public static MOUSE_BTN_MIDDLE: int32 = 1
public static MOUSE_BTN_RIGHT: int32 = 2
public static MOUSE_BTN_NONE: int32 = 3
public static MOUSE_BTN_BACK: int32 = 4
public static MOUSE_BTN_FORWARD: int32 = 5

// Действия мыши
public static MOUSE_DOWN: int32 = 0
public static MOUSE_UP: int32 = 1
public static MOUSE_DRAG: int32 = 2
public static MOUSE_MOVE: int32 = 3
public static MOUSE_SCROLL_UP: int32 = 4
public static MOUSE_SCROLL_DOWN: int32 = 5
public static MOUSE_SCROLL_LEFT: int32 = 6
public static MOUSE_SCROLL_RIGHT: int32 = 7

// Категории унифицированных событий
public static EVENT_NONE: int32 = 0
public static EVENT_KEY: int32 = 1
public static EVENT_MOUSE: int32 = 2
```

---

## 3. Интерфейсы

В модуле `std.itermtool` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

В модуле `std.itermtool` структуры ошибок не объявляются. Все функции модуля устойчивы к граничным случаям, корректно обрабатывают изменения размеров окон и некорректные escape-последовательности, возвращают безопасные значения или логические флаги результата и никогда не вызывают аварийную остановку программы (панику).

---

## 5. Классы

### 5.1 class Color

Класс представления цвета для переднего плана (текста) и фона ячеек терминала. Поддерживает 4 режима: сброс, именованный базовый ANSI цвет, 256-цветную палитру и 24-битный TrueColor RGB.

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

Создает новый экземпляр цвета с указанными параметрами режима.

```yukino
public fun init(kind: int32, r: int32, g: int32, b: int32, index: int32, name: string)
```

Возвращает инициализированный объект класса `Color`.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.color: Color, color_rgb
load std.io: println

fun main() {
    set c = color_rgb(255, 128, 0)
    println("Kind: " + cast(c.kind, "string"))
}
```

#### 5.1.2 to_fg

Генерирует управляющую ANSI escape-последовательность для установки данного цвета в качестве цвета текста.

```yukino
public fun to_fg() -> string
```

Возвращает непустую строку escape-последовательности либо пустую строку, если цвет настроен на сброс.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.color: color_rgb, ansi_reset
load std.io: println

fun main() {
    set c = color_rgb(100, 200, 255)
    println(c.to_fg() + "Голубой текст" + ansi_reset())
}
```

#### 5.1.3 to_bg

Генерирует управляющую ANSI escape-последовательность для установки данного цвета в качестве фона.

```yukino
public fun to_bg() -> string
```

Возвращает строку escape-последовательности либо пустую строку, если цвет настроен на сброс.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.color: color_rgb, ansi_reset
load std.io: println

fun main() {
    set c = color_rgb(30, 40, 60)
    println(c.to_bg() + "Текст на темно-синем фоне" + ansi_reset())
}
```

---

### 5.2 class Cell

Представляет единичную ячейку сетки TUI-буфера в стиле Ratatui: символ, цвета текста и фона, а также булевы флаги модификаторов начертания.

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

Инициализирует ячейку указанным символом, цветами и стилями.

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

Возвращает инициализированный объект класса `Cell`.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.color: Cell, color_rgb, color_named

fun main() {
    set c = init Cell("X", color_named("yellow"), color_rgb(20, 30, 50), true)
}
```

#### 5.2.2 render

Формирует полную ANSI-строку для вывода ячейки в терминал со всеми активными модификаторами, цветами и сбросом в конце.

```yukino
public fun render() -> string
```

Возвращает строковое представление ячейки для прямой отправки в терминал.

Метод никогда не вызывает панику.

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

Описывает событие нажатия клавиши клавиатуры: числовой код, строковое значение символа, а также флаги модификаторов Ctrl, Alt и Shift.

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

Создает объект события клавиатуры.

```yukino
public fun init(code: int32, char_val: string, ctrl: bool = false, alt: bool = false, shift: bool = false)
```

Возвращает инициализированный объект класса `KeyEvent`.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.key: KeyEvent, KEY_CHAR

fun main() {
    set ev = init KeyEvent(KEY_CHAR, "s", true, false, false)
}
```

#### 5.3.2 is_char

Проверяет, соответствует ли событие нажатию указанного символа без удерживания клавиш Ctrl и Alt.

```yukino
public fun is_char(c: string) -> bool
```

Возвращает логическое значение типа `bool`.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.key: key_char
load std.io: println

fun main() {
    set ev = key_char("q")
    if ev.is_char("q") {
        println("Нажата клавиша q")
    }
}
```

#### 5.3.3 is_ctrl

Проверяет, было ли событие комбинацией клавиши Ctrl с указанным символом (например, Ctrl+C).

```yukino
public fun is_ctrl(c: string) -> bool
```

Возвращает логическое значение типа `bool`.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.key: key_char
load std.io: println

fun main() {
    set ev = key_char("c", true)
    if ev.is_ctrl("c") {
        println("Нажата комбинация Ctrl+C")
    }
}
```

#### 5.3.4 is_alt

Проверяет, было ли событие комбинацией клавиши Alt с указанным символом.

```yukino
public fun is_alt(c: string) -> bool
```

Возвращает логическое значение типа `bool`.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.key: key_char
load std.io: println

fun main() {
    set ev = key_char("x", false, true)
    if ev.is_alt("x") {
        println("Нажата комбинация Alt+X")
    }
}
```

#### 5.3.5 to_string

Формирует человекочитаемое строковое представление события клавиши с перечислением модификаторов (например, `Ctrl+'c'`, `Shift+Up`, `Enter`, `F5`).

```yukino
public fun to_string() -> string
```

Возвращает строковое значение описания клавиши.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.key: key_special, KEY_UP
load std.io: println

fun main() {
    set ev = key_special(KEY_UP, true)
    println("Key: " + ev.to_string()) // "Ctrl+Up"
}
```

### 5.4 class MouseEvent

Класс представления события манипулятора мыши в терминале. Хранит категорию действия (`kind`: нажатие, отпускание, перемещение, перетаскивание, прокрутка колесика), нажатую кнопку (`button`), 0-индексированные экранные координаты курсора (`x`, `y`) и статус зажатых клавиш-модификаторов (`ctrl`, `alt`, `shift`).

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

Конструктор инициализации события мыши.

```yukino
public fun init(kind: int32, button: int32, x: int32, y: int32, ctrl: bool = false, alt: bool = false, shift: bool = false)
```

Инициализирует поля объекта переданными значениями.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.mouse: MouseEvent, MOUSE_DOWN, MOUSE_BTN_LEFT
load std.io: println

fun main() {
    set ev = init MouseEvent(MOUSE_DOWN, MOUSE_BTN_LEFT, 10, 5)
    println("Координаты мыши: " + cast(ev.x, "string") + "x" + cast(ev.y, "string"))
}
```

#### 5.4.2 is_left_click / is_right_click / is_middle_click

Предикаты для быстрой проверки клика определенной кнопки мыши.

```yukino
public fun is_left_click() -> bool
public fun is_right_click() -> bool
public fun is_middle_click() -> bool
```

Возвращают логическое значение `true`, если произошло событие нажатия соответствующей кнопки (`MOUSE_DOWN`), иначе `false`.

Методы никогда не вызывают панику.

```yukino
load std.itermtool.mouse: mouse_event, MOUSE_DOWN, MOUSE_BTN_LEFT
load std.io: println

fun main() {
    set ev = mouse_event(MOUSE_DOWN, MOUSE_BTN_LEFT, 0, 0)
    if ev.is_left_click() {
        println("Нажата левая кнопка мыши")
    }
}
```

#### 5.4.3 is_scroll_up / is_scroll_down / is_scroll_left / is_scroll_right

Предикаты проверки направления вращения колесика мыши.

```yukino
public fun is_scroll_up() -> bool
public fun is_scroll_down() -> bool
public fun is_scroll_left() -> bool
public fun is_scroll_right() -> bool
```

Возвращают `true`, если событие представляет соответствующую операцию прокрутки колесика мыши, иначе `false`.

Методы никогда не вызывают панику.

```yukino
load std.itermtool.mouse: mouse_event, MOUSE_SCROLL_UP, MOUSE_BTN_NONE
load std.io: println

fun main() {
    set ev = mouse_event(MOUSE_SCROLL_UP, MOUSE_BTN_NONE, 15, 20)
    if ev.is_scroll_up() {
        println("Прокрутка вверх в позиции 15, 20")
    }
}
```

#### 5.4.4 is_down / is_up / is_drag / is_move

Предикаты проверки характера физического действия манипулятора.

```yukino
public fun is_down() -> bool
public fun is_up() -> bool
public fun is_drag() -> bool
public fun is_move() -> bool
```

Возвращают логическое значение `true` при совпадении категории действия (`MOUSE_DOWN`, `MOUSE_UP`, `MOUSE_DRAG` или `MOUSE_MOVE`), иначе `false`.

Методы никогда не вызывают панику.

```yukino
load std.itermtool.mouse: mouse_event, MOUSE_DRAG, MOUSE_BTN_LEFT
load std.io: println

fun main() {
    set ev = mouse_event(MOUSE_DRAG, MOUSE_BTN_LEFT, 5, 5)
    if ev.is_drag() {
        println("Перетаскивание с зажатой кнопкой")
    }
}
```

#### 5.4.5 to_string

Формирует форматированную текстовую строку с описанием параметров события мыши (например, `MouseEvent(Down, btn=Left, x=10, y=5)` или `Ctrl+MouseEvent(ScrollUp, btn=None, x=0, y=0)`).

```yukino
public fun to_string() -> string
```

Возвращает строковое значение описания события мыши.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.mouse: mouse_event, MOUSE_DOWN, MOUSE_BTN_LEFT
load std.io: println

fun main() {
    set ev = mouse_event(MOUSE_DOWN, MOUSE_BTN_LEFT, 12, 8, true)
    println(ev.to_string()) // "Ctrl+MouseEvent(Down, btn=Left, x=12, y=8)"
}
```

### 5.5 class Event

Универсальный класс полиморфного терминального события, агрегирующий как события клавиатуры (`KeyEvent`), так и события мыши (`MouseEvent`).

```yukino
public class Event {
    public kind: int32
    public key: KeyEvent?
    public mouse: MouseEvent?
}
```

#### 5.5.1 init

Конструктор универсального события.

```yukino
public fun init(kind: int32, key: KeyEvent? = null, mouse: MouseEvent? = null)
```

Инициализирует событие заданной категорией (`EVENT_KEY`, `EVENT_MOUSE` либо `EVENT_NONE`) и ассоциированным объектом.

Метод никогда не вызывает панику.

```yukino
load std.itermtool.event: Event, EVENT_KEY
load std.itermtool.key: key_char
load std.io: println

fun main() {
    set ev = init Event(EVENT_KEY, key_char("q"))
    if ev.is_key() {
        println("Событие клавиатуры")
    }
}
```

#### 5.5.2 is_key / is_mouse

Предикаты определения типа поступившего события.

```yukino
public fun is_key() -> bool
public fun is_mouse() -> bool
```

Возвращают логическое значение `true`, если событие относится к клавиатуре (`EVENT_KEY`) или мыши (`EVENT_MOUSE`) соответственно.

Методы никогда не вызывают панику.

```yukino
load std.itermtool.event: read_event
load std.io: println

fun main() {
    set ev = read_event()
    if ev.is_mouse() {
        println("Пользователь взаимодействовал с мышью")
    }
}
```

#### 5.5.3 to_string

Формирует читаемую строку события с детализацией вложенного клавиатурного или мышиного объекта.

```yukino
public fun to_string() -> string
```

Возвращает строковое значение, например `Event::Key('q')` или `Event::Mouse(MouseEvent(Down, btn=Left, x=0, y=0))`.

Метод никогда не вызывает панику.

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

## 6. Справочник функций

### 6.1 Управление терминалом и экраном

#### 6.1.1 enable_raw_mode

Переводит терминал в сырой режим (Raw Mode): отключает строчную буферизацию операционной системы и эхо-печать вводимых символов. На Windows включает режим Virtual Terminal Input.

```yukino
public fun enable_raw_mode() -> bool
```

Возвращает логическое значение `true` при успешном переключении режима, либо `false` в случае сбоя.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.terminal: enable_raw_mode, disable_raw_mode

fun main() {
    if enable_raw_mode() {
        // терминал в сыром режиме
        disable_raw_mode()
    }
}
```

#### 6.1.2 disable_raw_mode

Восстанавливает стандартный канонический режим терминала с буферизацией строк и локальным эхо.

```yukino
public fun disable_raw_mode() -> bool
```

Возвращает логическое значение типа `bool`.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.terminal: disable_raw_mode

fun main() {
    disable_raw_mode()
}
```

#### 6.1.3 enter_alternate_screen

Переключает терминал на приватный альтернативный экранный буфер (`\x1b[?1049h`), используемый для полноэкранных TUI приложений.

```yukino
public fun enter_alternate_screen()
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.terminal: enter_alternate_screen, leave_alternate_screen

fun main() {
    enter_alternate_screen()
    // отрисовка TUI...
    leave_alternate_screen()
}
```

#### 6.1.4 leave_alternate_screen

Покидает приватный экранный буфер (`\x1b[?1049l`) и возвращает пользователя в исходное командное окно шелла с полным сохранением предыдущей истории вывода.

```yukino
public fun leave_alternate_screen()
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.terminal: leave_alternate_screen

fun main() {
    leave_alternate_screen()
}
```

#### 6.1.5 clear_all

Полностью очищает экран терминала и перемещает курсор в начало координат (`\x1b[2J\x1b[H`).

```yukino
public fun clear_all()
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.terminal: clear_all

fun main() {
    clear_all()
}
```

#### 6.1.6 clear_line

Очищает текущую строку терминала целиком (`\x1b[2K`).

```yukino
public fun clear_line()
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.terminal: clear_line

fun main() {
    clear_line()
}
```

#### 6.1.7 size

Запрашивает актуальный геометрический размер окна терминала у операционной системы.

```yukino
public fun size() -> (int32, int32)
```

Возвращает кортеж из двух 32-битных целых чисел `(cols, rows)`: ширина и высота в символах.

Функция никогда не вызывает панику. В случае невозможности определения возвращает `(80, 24)`.

```yukino
load std.itermtool.terminal: size
load std.io: println

fun main() {
    set sz = size()
    println("Ширина: " + cast(sz.0, "string") + ", Высота: " + cast(sz.1, "string"))
}
```

#### 6.1.8 has_resized

Проверяет, изменился ли геометрический размер окна терминала по сравнению с переданными предыдущими размерами `last_cols` и `last_rows`.

```yukino
public fun has_resized(last_cols: int32, last_rows: int32) -> bool
```

Возвращает логическое значение `true`, если текущие ширина или высота терминала отличаются от переданных значений, иначе `false`.

Функция никогда не вызывает панику.

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

#### 6.1.9 disable_line_wrap

Отключает автоматический перенос длинных строк на следующую строку терминала (`\x1b[?7l`, режим DECAWM). Предотвращает нежелательный сдвиг и вертикальную прокрутку экранного буфера при отрисовке элементов интерфейса, выходящих за границы окна. Автоматически вызывается при вызове `enter_alternate_screen()`.

```yukino
public fun disable_line_wrap()
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.terminal: disable_line_wrap

fun main() {
    disable_line_wrap()
}
```

#### 6.1.10 enable_line_wrap

Включает автоматический перенос строк терминала (`\x1b[?7h`). Автоматически восстанавливается при вызове `leave_alternate_screen()`.

```yukino
public fun enable_line_wrap()
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.terminal: enable_line_wrap

fun main() {
    enable_line_wrap()
}
```

#### 6.1.11 set_title

Устанавливает заголовок окна консоли или вкладки терминала.

```yukino
public fun set_title(title: string)
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.terminal: set_title

fun main() {
    set_title("Мое TUI Приложение")
}
```

---

### 6.2 Управление курсором и отрисовка

#### 6.2.1 move_to

Перемещает курсор в указанную позицию экрана. Координаты являются 0-индексированными (`0, 0` — верхний левый угол), что обеспечивает удобство при расчете сеток виджетов TUI.

```yukino
public fun move_to(x: int32, y: int32)
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.draw: move_to
load std.io: print

fun main() {
    move_to(10, 5)
    print("Текст в позиции x=10, y=5")
}
```

#### 6.2.2 hide_cursor / show_cursor

Скрывает или делает видимым аппаратный курсор терминала.

```yukino
public fun hide_cursor()
public fun show_cursor()
```

Функции не возвращают значения и никогда не вызывают панику.

```yukino
load std.itermtool.draw: hide_cursor, show_cursor

fun main() {
    hide_cursor()
    // отрисовка интерфейса...
    show_cursor()
}
```

#### 6.2.3 print_at

Перемещает курсор в 0-based координаты `(x, y)` и выводит указанный текст.

```yukino
public fun print_at(x: int32, y: int32, text: string)
```

Функция не возвращает значения и никогда не вызывает панику.

```yukino
load std.itermtool.draw: print_at

fun main() {
    print_at(0, 0, "Заголовок окна")
}
```

#### 6.2.4 print_styled_at

Перемещает курсор в координаты `(x, y)`, устанавливает переданные escape-последовательности цвета/фона, выводит строку и автоматически сбрасывает атрибуты цвета.

```yukino
public fun print_styled_at(x: int32, y: int32, text: string, fg_seq: string, bg_seq: string)
```

Функция не возвращает значения и никогда не вызывает панику.

```yukino
load std.itermtool.draw: print_styled_at
load std.itermtool.color: ansi_fg_rgb, ansi_bg_rgb

fun main() {
    print_styled_at(2, 2, "Внимание", ansi_fg_rgb(255, 255, 0), ansi_bg_rgb(180, 0, 0))
}
```

#### 6.2.5 fill_rect

Заливает прямоугольную область экрана указанной ширины и высоты фоновой escape-последовательностью и символом заполнителя (по умолчанию пробел). Используется для создания фона панелей и канваса TUI в стиле Ratatui.

```yukino
public fun fill_rect(x: int32, y: int32, width: int32, height: int32, bg_seq: string, ch: string = " ")
```

Функция не возвращает значения и никогда не вызывает панику.

```yukino
load std.itermtool.draw: fill_rect
load std.itermtool.color: ansi_bg_rgb

fun main() {
    set bg = ansi_bg_rgb(20, 30, 45)
    fill_rect(0, 0, 80, 24, bg, " ")
}
```

#### 6.2.6 draw_box

Рисует рамку панели символами псевдографики (`┌─┐│└─┘`) с опциональной подсветкой границы и заливкой фона.

```yukino
public fun draw_box(x: int32, y: int32, width: int32, height: int32, fg_seq: string = "", bg_seq: string = "")
```

Функция не возвращает значения и никогда не вызывает панику.

```yukino
load std.itermtool.draw: draw_box
load std.itermtool.color: ansi_fg_rgb

fun main() {
    draw_box(4, 2, 40, 10, ansi_fg_rgb(80, 160, 240), "")
}
```

---

### 6.3 Цвета и генераторы ANSI-кодов

#### 6.3.1 ansi_fg_rgb / ansi_bg_rgb

Генерирует 24-битную TrueColor escape-последовательность для текста (`\x1b[38;2;r;g;bm`) или фона (`\x1b[48;2;r;g;bm`).

```yukino
public fun ansi_fg_rgb(r: int32, g: int32, b: int32) -> string
public fun ansi_bg_rgb(r: int32, g: int32, b: int32) -> string
```

Возвращает строковое значение ANSI escape-кода.

Функции никогда не вызывают панику.

```yukino
load std.itermtool.color: ansi_fg_rgb, ansi_reset
load std.io: println

fun main() {
    println(ansi_fg_rgb(255, 100, 50) + "Оранжевый текст" + ansi_reset())
}
```

#### 6.3.2 ansi_fg_256 / ansi_bg_256

Генерирует escape-последовательность цвета из расширенной 256-цветной палитры терминала (`0..255`).

```yukino
public fun ansi_fg_256(idx: int32) -> string
public fun ansi_bg_256(idx: int32) -> string
```

Возвращает строковое значение типа `string`.

Функции никогда не вызывают панику.

```yukino
load std.itermtool.color: ansi_fg_256, ansi_reset
load std.io: println

fun main() {
    println(ansi_fg_256(214) + "Цвет 214" + ansi_reset())
}
```

#### 6.3.3 ansi_reset

Возвращает универсальную escape-последовательность сброса всех атрибутов и цветов терминала (`\x1b[0m`).

```yukino
public fun ansi_reset() -> string
```

Возвращает строковое значение `"\x1b[0m"`.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.color: ansi_reset
load std.io: print

fun main() {
    print(ansi_reset())
}
```

---

### 6.4 Обработка клавиатурных событий

#### 6.4.1 poll_event

Выполняет неблокирующую или ограниченную по времени проверку наличия входящих байтов на стандартном потоке ввода.

```yukino
public fun poll_event(timeout_ms: int64 = 0) -> bool
```

Возвращает логическое значение `true`, если нажатие клавиши готово для считывания, либо `false` по истечении таймаута.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.event: poll_event, read_key
load std.io: println

fun main() {
    if poll_event(100) {
        set ev = read_key()
        println("Нажата: " + ev.to_string())
    }
}
```

#### 6.4.2 read_key

Блокирующе ожидает нажатия клавиши и транслирует входящий поток байт и VT escape-последовательностей в структурированный объект `KeyEvent`. Корректно определяет стрелки, функциональные клавиши F1-F12, комбинации с Ctrl/Alt/Shift и многобайтовые символы UTF-8.

```yukino
public fun read_key() -> KeyEvent
```

Возвращает объект класса `KeyEvent`.

Функция никогда не вызывает панику. В случае ошибки чтения возвращает событие с кодом `KEY_UNKNOWN`.

```yukino
load std.itermtool.event: read_key
load std.itermtool.key: KEY_ESC
load std.io: println

fun main() {
    set ev = read_key()
    if ev.code == KEY_ESC {
        println("Нажат Escape")
    }
}
```

#### 6.4.3 try_read_key

Пытается мгновенно прочитать следующее событие клавиатуры без блокировки потока.

```yukino
public fun try_read_key() -> KeyEvent?
```

Возвращает объект `KeyEvent`, если клавиша была нажата, либо `null`, если буфер ввода пуст.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.event: try_read_key
load std.io: println

fun main() {
    set ev = try_read_key()
    if ev != null {
        set k = ev ?? panic("unreachable")
        println("Клавиша: " + k.to_string())
    }
}
```

### 6.5 Управление мышью и события мыши

#### 6.5.1 enable_mouse / disable_mouse / toggle_mouse

Управляет протоколом захвата мыши в терминале. Функция `enable_mouse` включает отслеживание мыши (алиас: `enable_mouse_capture`), а `disable_mouse` отключает его (алиас: `disable_mouse_capture`). Переводит терминал в современный режим SGR 1006 с поддержкой кнопок, зажатия (drag) и колесика мыши, а также настраивает дескриптор ввода консоли Windows Console API при запуске на платформе Windows.

Для переключения режима на противоположный предназначена функция `toggle_mouse` (алиас: `toggle_mouse_capture`), а для проверки текущего статуса активности — `is_mouse_active`.

```yukino
public fun enable_mouse()
public fun disable_mouse()
public fun enable_mouse_capture()
public fun disable_mouse_capture()
public fun is_mouse_active() -> bool
public fun toggle_mouse() -> bool
public fun toggle_mouse_capture() -> bool
```

Функции `enable_mouse`, `disable_mouse`, `enable_mouse_capture`, `disable_mouse_capture` не возвращают значений (`void`). Функции `is_mouse_active`, `toggle_mouse`, `toggle_mouse_capture` возвращают логическое значение `bool`, указывающее, активен ли захват мыши в терминале после вызова.

Функции никогда не вызывают панику.

```yukino
load std.itermtool.mouse: enable_mouse, disable_mouse, toggle_mouse, is_mouse_active
load std.itermtool.terminal: enable_raw_mode, disable_raw_mode
load std.io: println

fun main() {
    enable_raw_mode()
    set active = toggle_mouse() // Включает мышь
    println("Мышь активна: " + cast(active, "string"))

    toggle_mouse() // Отключает мышь
    disable_raw_mode()
}
```

#### 6.5.2 enable_mouse_motion / disable_mouse_motion

Включает либо выключает режим полного отслеживания курсора мыши (VT Any-Event Tracking 1003). В активном состоянии терминал передает события движения мыши даже тогда, когда ни одна кнопка не нажата.

```yukino
public fun enable_mouse_motion()
public fun disable_mouse_motion()
```

Функции не возвращают значений (`void`).

Функции никогда не вызывают панику.

```yukino
load std.itermtool.mouse: enable_mouse, enable_mouse_motion, disable_mouse_motion, disable_mouse

fun main() {
    enable_mouse()
    enable_mouse_motion() // Отслеживание координат при наведении

    disable_mouse_motion()
    disable_mouse()
}
```

#### 6.5.3 read_mouse

Ожидает поступления escape-последовательности мыши из стандартного потока ввода и декодирует её в структурированный объект `MouseEvent`.

```yukino
public fun read_mouse() -> MouseEvent?
```

Возвращает объект `MouseEvent` либо `null`, если входной поток исчерпан или поступившая последовательность не относится к событиям мыши.

Функция никогда не вызывает панику.

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
        println("Клик в точке: " + cast(m.x, "string") + "," + cast(m.y, "string"))
    }

    disable_mouse()
    disable_raw_mode()
}
```

#### 6.5.4 try_read_mouse

Выполняет неблокирующий опрос буфера ввода на наличие готового события мыши.

```yukino
public fun try_read_mouse() -> MouseEvent?
```

Возвращает объект `MouseEvent`, если в буфере присутствовала последовательность мыши, либо `null`, если данных нет.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.mouse: try_read_mouse
load std.io: println

fun main() {
    set ev = try_read_mouse()
    if ev != null {
        set m = ev ?? panic("unreachable")
        println("Мышь: " + m.to_string())
    }
}
```

#### 6.5.5 decode_mouse_sgr

Низкоуровневая функция преобразования параметров протокола SGR в объект `MouseEvent`. Автоматически извлекает модификаторы Ctrl, Alt, Shift, определяет тип действия (нажатие, отпускание, перемещение, колесико) и нормализует 1-индексированные терминальные координаты к 0-индексированной сетке `itermtool`.

```yukino
public fun decode_mouse_sgr(code: int32, x_raw: int32, y_raw: int32, is_release: bool) -> MouseEvent
```

Возвращает созданный объект `MouseEvent`.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.mouse: decode_mouse_sgr
load std.io: println

fun main() {
    // 0 = Левая кнопка, x=10, y=5, нажатие
    set ev = decode_mouse_sgr(0, 10, 5, false)
    println("Координаты в 0-based сетке: " + cast(ev.x, "string") + "x" + cast(ev.y, "string")) // 9x4
}
```

### 6.6 Унифицированные терминальные события

#### 6.6.1 read_event

Блокирующе ожидает поступления любого терминального события (нажатия клавиши клавиатуры либо действия мыши) и возвращает полиморфный контейнер `Event`. Рекомендуется в качестве основного цикла событий в приложениях со смешанным управлением.

```yukino
public fun read_event() -> Event
```

Возвращает объект класса `Event`. При получении события клавиатуры поле `key` содержит `KeyEvent`, а при событии мыши поле `mouse` содержит `MouseEvent`.

Функция никогда не вызывает панику.

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
        println("Клавиша: " + ev.to_string())
    } elif ev.is_mouse() {
        println("Мышь: " + ev.to_string())
    }

    disable_mouse()
    disable_raw_mode()
}
```

#### 6.6.2 try_read_event

Неблокирующий опрос и получение следующего терминального события без задержки потока исполнения.

```yukino
public fun try_read_event() -> Event?
```

Возвращает объект `Event`, если в потоке ввода доступны данные, либо `null`, если ввод отсутствует.

Функция никогда не вызывает панику.

```yukino
load std.itermtool.event: try_read_event
load std.io: println

fun main() {
    set ev = try_read_event()
    if ev != null {
        set e = ev ?? panic("unreachable")
        println("Событие: " + e.to_string())
    }
}
```

---

[← Назад: Модуль std.line](23-line.md) | [Вперед: Модуль std.stdotool →](25-stdotool.md)
