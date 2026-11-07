# Модуль std.outf

Модуль `std.outf` (output format) предоставляет инструменты для стилизации консольного вывода, управления цветом терминала (8 базовых цветов, 8 ярких цветов, палитра из 256 цветов и 24-битный TrueColor RGB), применения текстовых модификаторов (жирный, курсив, подчеркивание и другие), а также автоматического определения возможностей терминала и поддержки стандартных переменных окружения (`NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `COLORTERM`, `TERM`). Модуль спроектирован по аналогии с популярными библиотеками терминального форматирования (такими как `owo-colors` и `colored`), принимает строковые аргументы по значению (`text: string`) как легковесные Copy-дескрипторы, а также цепочечный fluent-интерфейс через класс `Styled`.

---

## 1. Подключение: load std.outf

Импорт всего пакета:

```yukino
load std.outf
```

При этом становятся доступны следующие сущности:

- **Константы режимов цвета:** `COLOR_AUTO`, `COLOR_ALWAYS`, `COLOR_NEVER`, `RESET`.
- **Классы:** `Styled`.
- **Функции управления и детекции:** `set_color_mode`, `get_color_mode`, `is_color_enabled`, `supports_color`, `supports_truecolor`.
- **Функции стилей текста:** `bold`, `dim`, `italic`, `underline`, `blink`, `reversed`, `hidden`, `strikethrough`.
- **Функции цвета текста (Foreground):** `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `gray`, `bright_black`, `bright_red`, `bright_green`, `bright_yellow`, `bright_blue`, `bright_magenta`, `bright_cyan`, `bright_white`, `color256`, `rgb`.
- **Функции цвета фона (Background):** `bg_black`, `bg_red`, `bg_green`, `bg_yellow`, `bg_blue`, `bg_magenta`, `bg_cyan`, `bg_white`, `bg_gray`, `bg_bright_black`, `bg_bright_red`, `bg_bright_green`, `bg_bright_yellow`, `bg_bright_blue`, `bg_bright_magenta`, `bg_bright_cyan`, `bg_bright_white`, `bg_color256`, `bg_rgb`.
- **Фабричные функции fluent-интерфейса:** `style`, `styled`.
- **Вспомогательные утилиты ANSI:** `clear_style`, `strip_ansi`, `visible_len`, `visible_bytes`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.outf` объявлены глобальные константы для управления поведением окрашивания и управляющие ANSI-последовательности:

```yukino
public static COLOR_AUTO: int32 = 0
public static COLOR_ALWAYS: int32 = 1
public static COLOR_NEVER: int32 = 2

public static RESET: string = "\x1b[0m"
```

- `COLOR_AUTO`: автоматический режим (по умолчанию). Вывод ANSI-последовательностей зависит от переменных окружения (`CLICOLOR_FORCE`, `NO_COLOR`, `CLICOLOR`, `TERM`) и того, подключен ли стандартный поток вывода к интерактивному терминалу (TTY).
- `COLOR_ALWAYS`: принудительное включение цветов. ANSI-последовательности форматирования генерируются всегда, независимо от окружения и перенаправления вывода в файл или конвейер.
- `COLOR_NEVER`: принудительное отключение цветов. Функции стилизации возвращают исходный текст без каких-либо управляющих ANSI-кодов.
- `RESET`: стандартная управляющая последовательность сброса всех графических атрибутов терминала `\x1b[0m`.

---

## 3. Интерфейсы

В модуле `std.outf` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

В модуле `std.outf` структуры данных и ошибок не объявлены. Все функции модуля устойчивы к некорректным входным диапазонам, безопасно нормализуют цветовые каналы и не вызывают аварийную остановку выполнения.

---

## 5. Классы

### 5.1 Styled

Класс `Styled` предоставляет fluent-интерфейс для одновременного комбинирования нескольких стилей оформления, цвета текста и цвета фона в единую ANSI-последовательность вида `\x1b[1;31;44m...\x1b[0m` без создания лишних промежуточных строк.

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

Все методы-модификаторы возвращают текущий экземпляр `this`, обеспечивая последовательное сцепление вызовов. Метод `to_string()` (а также его синоним `render()`) формирует результирующую строку. Если окрашивание отключено (`COLOR_NEVER` или терминал не поддерживает цвета при `COLOR_AUTO`), методы возвращают исходный текст без управляющих кодов.

```yukino
load std.outf: style, styled, set_color_mode, COLOR_ALWAYS
load std.io: println

fun main() {
    set_color_mode(COLOR_ALWAYS)
    set msg = "Important Notice"

    // Цепочечное применение жирного шрифта, красного цвета текста и синего фона
    set formatted = style(msg).bold().red().bg_blue().to_string()
    println(formatted)

    // Применение TrueColor RGB и курсива
    set custom = styled(msg).italic().rgb(255, 128, 0).bg_rgb(20, 20, 20).render()
    println(custom)
}
```

---

## 6. Справочник функций

### 6.1 Управление режимами цвета и детекция терминала

#### set_color_mode

Устанавливает глобальный режим окрашивания консольного вывода. Принимает одну из констант `COLOR_AUTO`, `COLOR_ALWAYS` или `COLOR_NEVER`. Настройка является потокобезопасной и действует на все последующие вызовы функций модуля `std.outf`.

```yukino
public fun set_color_mode(mode: int32)
```

Возвращаемое значение отсутствует (тип `void`).

Функция никогда не вызывает панику.

```yukino
load std.outf: set_color_mode, COLOR_ALWAYS

fun main() {
    set_color_mode(COLOR_ALWAYS)
}
```

#### get_color_mode

Возвращает текущий установленный глобальный режим окрашивания консольного вывода.

```yukino
public fun get_color_mode() -> int32
```

Возвращает целочисленное значение типа `int32` (`COLOR_AUTO = 0`, `COLOR_ALWAYS = 1` или `COLOR_NEVER = 2`). Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

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

Проверяет, включено ли формирование ANSI-цветов в данный момент. Если режим равен `COLOR_ALWAYS`, возвращает `true`. Если режим равен `COLOR_NEVER`, возвращает `false`. В режиме `COLOR_AUTO` анализирует переменные окружения: `CLICOLOR_FORCE!=0` принудительно включает цвета, `NO_COLOR` (если установлена непустая строка) или `CLICOLOR=0` либо `TERM=dumb` отключают цвета; при отсутствии явных переменных проверяет, подключен ли стандартный поток вывода к интерактивному терминалу (`is_stdout_terminal()`).

```yukino
public fun is_color_enabled() -> bool
```

Возвращает логическое значение типа `bool`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

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

Проверяет базовую способность терминала выводить ANSI-цвета на основе переменной окружения `TERM` и детектора интерактивного вывода `is_stdout_terminal()`.

```yukino
public fun supports_color() -> bool
```

Возвращает логическое значение типа `bool`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.outf: supports_color
load std.io: println

fun main() {
    println("Terminal supports colors: " + if supports_color() { "yes" } else { "no" })
}
```

#### supports_truecolor

Проверяет, поддерживает ли текущий терминал 24-битную палитру TrueColor RGB. Проверка осуществляется по переменной окружения `COLORTERM` (значения `truecolor` или `24bit`).

```yukino
public fun supports_truecolor() -> bool
```

Возвращает логическое значение типа `bool`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

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

### 6.2 Стили оформления текста

Функции данной группы обрамляют переданную строку управляющими ANSI-кодами соответствующего текстового атрибута (`\x1b[...m`) и завершают ее сбросом `\x1b[0m`. Если цвета отключены, возвращается копия исходного текста без изменений.

- `bold(text: string) -> string` — жирный шрифт или увеличенная яркость (код `1`).
- `dim(text: string) -> string` — приглушенный или бледный текст (код `2`).
- `italic(text: string) -> string` — начертание курсивом (код `3`).
- `underline(text: string) -> string` — подчеркнутый текст (код `4`).
- `blink(text: string) -> string` — мигающий текст (код `5`).
- `reversed(text: string) -> string` — инверсия цветов текста и фона (код `7`).
- `hidden(text: string) -> string` — скрытый текст (код `8`).
- `strikethrough(text: string) -> string` — зачеркнутый текст (код `9`).

Все функции принимают строку `text: string`, возвращают новую строку типа `string` (не nullable) и никогда не вызывают панику.

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

### 6.3 Цвета текста (Foreground)

#### Стандартные 8 цветов

- `black(text: string) -> string` — черный цвет (код `30`).
- `red(text: string) -> string` — красный цвет (код `31`).
- `green(text: string) -> string` — зеленый цвет (код `32`).
- `yellow(text: string) -> string` — желтый цвет (код `33`).
- `blue(text: string) -> string` — синий цвет (код `34`).
- `magenta(text: string) -> string` — пурпурный цвет (код `35`).
- `cyan(text: string) -> string` — бирюзовый (голубой) цвет (код `36`).
- `white(text: string) -> string` — белый цвет (код `37`).

#### Яркие 8 цветов

- `gray(text: string) -> string` — серый (яркий черный) цвет (код `90`).
- `bright_black(text: string) -> string` — яркий черный цвет (код `90`, синоним `gray`).
- `bright_red(text: string) -> string` — яркий красный цвет (код `91`).
- `bright_green(text: string) -> string` — яркий зеленый цвет (код `92`).
- `bright_yellow(text: string) -> string` — яркий желтый цвет (код `93`).
- `bright_blue(text: string) -> string` — яркий синий цвет (код `94`).
- `bright_magenta(text: string) -> string` — яркий пурпурный цвет (код `95`).
- `bright_cyan(text: string) -> string` — яркий бирюзовый цвет (код `96`).
- `bright_white(text: string) -> string` — яркий белый цвет (код `97`).

#### color256

Окрашивает текст по номеру цвета из расширенной палитры ANSI 256 (`\x1b[38;5;<code>m`). Значение кода автоматически ограничивается диапазоном `0..255`.

```yukino
public fun color256(text: string, code: int32) -> string
```

Возвращает отформатированную строку типа `string`. Возвращаемое значение не nullable. Функция никогда не вызывает панику.

#### rgb

Окрашивает текст в произвольный цвет 24-битной палитры TrueColor RGB (`\x1b[38;2;<r>;<g>;<b>m`). Компоненты цвета автоматически ограничиваются диапазоном `0..255`.

```yukino
public fun rgb(text: string, r: int32, g: int32, b: int32) -> string
```

Возвращает отформатированную строку типа `string`. Возвращаемое значение не nullable. Функция никогда не вызывает панику.

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

### 6.4 Цвета фона (Background)

#### Стандартные 8 фонов

- `bg_black(text: string) -> string` — черный фон (код `40`).
- `bg_red(text: string) -> string` — красный фон (код `41`).
- `bg_green(text: string) -> string` — зеленый фон (код `42`).
- `bg_yellow(text: string) -> string` — желтый фон (код `43`).
- `bg_blue(text: string) -> string` — синий фон (код `44`).
- `bg_magenta(text: string) -> string` — пурпурный фон (код `45`).
- `bg_cyan(text: string) -> string` — бирюзовый фон (код `46`).
- `bg_white(text: string) -> string` — белый фон (код `47`).

#### Яркие 8 фонов

- `bg_gray(text: string) -> string` — серый фон (код `100`).
- `bg_bright_black(text: string) -> string` — яркий черный фон (код `100`).
- `bg_bright_red(text: string) -> string` — яркий красный фон (код `101`).
- `bg_bright_green(text: string) -> string` — яркий зеленый фон (код `102`).
- `bg_bright_yellow(text: string) -> string` — яркий желтый фон (код `103`).
- `bg_bright_blue(text: string) -> string` — яркий синий фон (код `104`).
- `bg_bright_magenta(text: string) -> string` — яркий пурпурный фон (код `105`).
- `bg_bright_cyan(text: string) -> string` — яркий бирюзовый фон (код `106`).
- `bg_bright_white(text: string) -> string` — яркий белый фон (код `107`).

#### bg_color256

Задает цвет фона по номеру из палитры 256 цветов (`\x1b[48;5;<code>m`). Значение кода автоматически ограничивается диапазоном `0..255`.

```yukino
public fun bg_color256(text: string, code: int32) -> string
```

Возвращает отформатированную строку типа `string`. Возвращаемое значение не nullable. Функция никогда не вызывает панику.

#### bg_rgb

Задает цвет фона в 24-битном TrueColor RGB (`\x1b[48;2;<r>;<g>;<b>m`). Компоненты цвета автоматически ограничиваются диапазоном `0..255`.

```yukino
public fun bg_rgb(text: string, r: int32, g: int32, b: int32) -> string
```

Возвращает отформатированную строку типа `string`. Возвращаемое значение не nullable. Функция никогда не вызывает панику.

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

### 6.5 Построение fluent-стилей

#### style

Создает экземпляр класса `Styled` для построения составного цепочечного форматирования.

```yukino
public fun style(text: string) -> Styled
```

Возвращает новый экземпляр класса `Styled`. Возвращаемое значение не nullable. Функция никогда не вызывает панику.

#### styled

Полный синоним функции `style`. Служит для лаконичной записи цепочечных выражений.

```yukino
public fun styled(text: string) -> Styled
```

Возвращает новый экземпляр класса `Styled`. Возвращаемое значение не nullable. Функция никогда не вызывает панику.

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

### 6.6 Вспомогательные утилиты ANSI

#### clear_style

Добавляет к копии переданной строки сбрасывающую последовательность `\x1b[0m`, гарантируя восстановление стандартных параметров терминала после вывода.

```yukino
public fun clear_style(text: string) -> string
```

Возвращает результирующую строку типа `string`. Возвращаемое значение не nullable. Функция никогда не вызывает панику.

```yukino
load std.outf: clear_style
load std.io: print

fun main() {
    set raw = "\x1b[31mPartial warning"
    print(clear_style(raw))
}
```

#### strip_ansi

Удаляет из строки все управляющие последовательности ANSI (последовательности CSI `\x1b[...` и OSC `\x1b]...`), возвращая чистый текст без графических атрибутов. Полезна при логировании в файлы или расчете длины для выравнивания таблиц.

```yukino
public fun strip_ansi(text: string) -> string
```

Возвращает очищенную строку типа `string`. Возвращаемое значение не nullable. Функция никогда не вызывает панику.

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

Вычисляет видимую длину строки в символах Юникода (UTF-8 кодовых точках) после удаления всех управляющих ANSI-кодов. Позволяет корректно выравнивать колонки псевдографики и таблиц с окрашенным текстом.

```yukino
public fun visible_len(text: string) -> int64
```

Возвращает количество видимых символов типа `int64`. Возвращаемое значение не nullable. Функция никогда не вызывает панику.

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

Вычисляет видимый размер строки в байтах после удаления всех управляющих ANSI-кодов.

```yukino
public fun visible_bytes(text: string) -> int64
```

Возвращает размер очищенного текста в байтах типа `int64`. Возвращаемое значение не nullable. Функция никогда не вызывает панику.

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

[← Назад: Модуль std.jsonc](21-jsonc.md) | [Вперед: Модуль std.line →](23-line.md)
