# Модуль std.stdotool

Модуль `std.stdotool` (Standard Output Tool) предоставляет инструменты для динамического визуального вывода в обычном консольном потоке `stdout`. В отличие от полноэкранных TUI-модулей, требующих захвата терминала через приватный буфер (alternate screen), `stdotool` работает непосредственно в основном потоке терминала с сохранением привычной истории вывода. Модуль включает как низкоуровневые примитивы управления положением каретки и стирания строк, так и гибкие визуальные строительные блоки: циклический проигрыватель пользовательских анимационных кадров (`Frames`), настраиваемый конструктор полос прогресса любой геометрии и символики (`Bar`), а также контроллеры динамической однострочной (`LiveLine`) и многострочной (`LiveArea`) перерисовки без мерцания (например, для создания параллельных загрузок в стиле `pacman -Syyu`).

---

## 1. Подключение: load std.stdotool

Импорт всего пакета:

```yukino
load std.stdotool
```

При этом становятся доступны следующие сущности:

- **Классы:** `Frames`, `Bar`, `LiveLine`, `LiveArea`.
- **Функции управления курсором и строками:** `up`, `down`, `forward`, `back`, `to_col`, `carriage_return`, `clear_line`, `clear_to_end`, `clear_from_start`, `clear_lines_up`, `hide_cursor`, `show_cursor`, `rewrite_line`.
- **Функции форматирования:** `format_bytes`, `format_speed`, `format_percent`, `format_duration`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.stdotool` глобальные константы не объявлены. Все анимационные кадры и визуальные символы передаются пользователем явно.

---

## 3. Интерфейсы

В модуле `std.stdotool` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

В модуле `std.stdotool` структуры данных и ошибок не объявлены. Все операции выполняются детерминированно без генерации паник.

---

## 5. Классы

### 5.1 class Frames

Представляет универсальный контроллер циклической смены анимационных кадров. Разработчик передает произвольный массив строковых спрайтов (любые символы, знаки или текстовые фазы анимации).

```yukino
public class Frames {
    public items: string[]
    public index: int32
}
```

#### 5.1.1 init

Инициализирует аниматор указанным массивом строковых кадров со стартовой позицией на нулевом элементе.

```yukino
public fun init(items: string[])
```

Возвращает инициализированный объект класса `Frames`.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.frames: Frames

fun main() {
    set spinner = init Frames(["-", "\\", "|", "/"])
}
```

#### 5.1.2 next

Возвращает значение текущего кадра и автоматически перемещает внутренний указатель на следующий элемент. По достижении конца массива циклически возвращается к началу.

```yukino
public fun next() -> string
```

Возвращает не-nullable строку следующего кадра либо пустую строку, если переданный массив был пуст.

Метод никогда не вызывает панику.

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

Возвращает текущий активный кадр без сдвига внутреннего указателя.

```yukino
public fun current() -> string
```

Возвращает не-nullable строковое значение текущего кадра.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.frames: Frames
load std.io: println

fun main() {
    set f = init Frames(["A", "B"])
    println(f.current()) // "A"
}
```

#### 5.1.4 reset

Сбрасывает указатель активного кадра на начальный индекс (0).

```yukino
public fun reset()
```

Метод не возвращает значения.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.frames: Frames

fun main() {
    set f = init Frames(["1", "2"])
    f.next()
    f.reset()
}
```

#### 5.1.5 len

Возвращает общее количество кадров в анимационной последовательности.

```yukino
public fun len() -> int32
```

Возвращает 32-битное целое число, обозначающее длину последовательности.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.frames: Frames
load std.io: println

fun main() {
    set f = init Frames(["A", "B", "C"])
    println("Кадров: " + cast(f.len(), "string"))
}
```

---

### 5.2 class Bar

Представляет визуальный конструктор полосы прогресса. Разработчик полностью определяет символы заполненной части, курсора (головы), незаполненного следа и обрамляющих скобок.

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

Инициализирует конструктор полосы указанными символами для тела, головы, незаполненного хвоста и граничных скобок.

```yukino
public fun init(
    body: string = "=",
    head: string = ">",
    trail: string = " ",
    left: string = "[",
    right: string = "]"
)
```

Возвращает инициализированный объект класса `Bar`.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.bar: Bar

fun main() {
    // Стиль Pacman: ест точки
    set pacman_bar = init Bar("-", "C", "•", "[", "]")

    // Традиционный стиль блоков
    set block_bar = init Bar("█", "█", "░", "[", "]")
}
```

#### 5.2.2 render

Строит строковое представление полосы прогресса для текущего значения `current` из общего объема `total`, вписывая результат в точно заданную ширину `width` символов.

```yukino
public fun render(current: int64, total: int64, width: int32) -> string
```

Возвращает сформированную строку полосы прогресса. Возвращаемое значение гарантированно не равно null.

Метод никогда не вызывает панику. При `total <= 0` возвращает полностью незаполненную полосу; при `current >= total` полоса заполняется целиком.

```yukino
load std.stdotool.bar: Bar
load std.io: println

fun main() {
    set bar = init Bar("-", "C", "•", "[", "]")
    println(bar.render(45, 100, 20))
}
```

#### 5.2.3 set_head

Динамически заменяет ведущий символ (голову) полосы прогресса. Используется для анимации открывающегося и закрывающегося рта пакмана или вращающихся маркеров.

```yukino
public fun set_head(head: string)
```

Метод не возвращает значения.

Метод никогда не вызывает панику.

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

Управляет однострочным динамическим выводом в терминал, обеспечивая перезапись текущей строки на месте с автоматической очисткой остаточных символов предыдущего сообщения.

```yukino
public class LiveLine {
    public last_len: int32
    public active: bool
}
```

#### 5.3.1 init

Инициализирует контроллер динамической строки. При передаче непустого аргумента `initial_text` сразу выполняет первичный вывод.

```yukino
public fun init(initial_text: string = "")
```

Возвращает инициализированный объект класса `LiveLine`.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.live_line: LiveLine

fun main() {
    set line = init LiveLine()
}
```

#### 5.3.2 write

Очищает текущую строку терминала и выводит новый текст на месте без перевода строки.

```yukino
public fun write(text: string)
```

Метод не возвращает значения.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.live_line: LiveLine

fun main() {
    set line = init LiveLine()
    line.write("Загрузка 10%...")
    line.write("Загрузка 20%...")
}
```

#### 5.3.3 clear

Стирает содержимое динамической строки с экрана.

```yukino
public fun clear()
```

Метод не возвращает значения.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.live_line: LiveLine

fun main() {
    set line = init LiveLine("Временный статус")
    line.clear()
}
```

#### 5.3.4 finish

Завершает динамический вывод переводом строки (`\n`), фиксируя текущее состояние в постоянной истории терминала.

```yukino
public fun finish()
```

Метод не возвращает значения.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.live_line: LiveLine

fun main() {
    set line = init LiveLine()
    line.write("Готово")
    line.finish()
}
```

#### 5.3.5 finish_text

Записывает финальное сообщение поверх текущей строки и фиксирует его переводом строки (`\n`).

```yukino
public fun finish_text(final_text: string)
```

Метод не возвращает значения.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.live_line: LiveLine

fun main() {
    set line = init LiveLine()
    line.write("Синхронизация...")
    line.finish_text("Синхронизация завершена [OK]")
}
```

---

### 5.4 class LiveArea

Управляет многострочной динамической сценой из произвольного количества строк без перехода в альтернативный экран. На каждом шаге перемещает курсор на высоту ранее выведенного блока, стирает старые строки и атомарно выводит обновленные данные.

```yukino
public class LiveArea {
    public rendered_count: int32
}
```

#### 5.4.1 init

Инициализирует новую пустую многострочную область вывода.

```yukino
public fun init()
```

Возвращает объект класса `LiveArea`.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.live_area: LiveArea

fun main() {
    set area = init LiveArea()
}
```

#### 5.4.2 render

Отрисовывает переданный массив строк на экране. Если ранее область уже была выведена, курсор автоматически поднимается на нужное количество строк вверх, обновляя содержимое каждой строки без мерцания.

```yukino
public fun render(lines: string[])
```

Метод не возвращает значения.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.live_area: LiveArea

fun main() {
    set area = init LiveArea()
    area.render(["Пакет 1: [===>   ]", "Пакет 2: [=======>]"])
}
```

#### 5.4.3 clear

Полностью стирает ранее выведенную многострочную сцену с экрана терминала и возвращает курсор в исходную позицию.

```yukino
public fun clear()
```

Метод не возвращает значения.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.live_area: LiveArea

fun main() {
    set area = init LiveArea()
    area.render(["Строка 1", "Строка 2"])
    area.clear()
}
```

#### 5.4.4 finish

Завершает динамический цикл обновления многострочной сцены, оставляя последнее состояние зафиксированным в истории вывода терминала.

```yukino
public fun finish()
```

Метод не возвращает значения.

Метод никогда не вызывает панику.

```yukino
load std.stdotool.live_area: LiveArea
load std.io: println

fun main() {
    set area = init LiveArea()
    area.render(["core.db [DONE]", "extra.db [DONE]"])
    area.finish()
    println("Загрузка окончена.")
}
```

---

## 6. Справочник функций

### 6.1 Управление кареткой и строками

#### 6.1.1 up

Перемещает курсор на `n` строк вверх в текущем окне терминала (`\x1b[nA`).

```yukino
public fun up(n: int32 = 1)
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: up

fun main() {
    up(2)
}
```

#### 6.1.2 down

Перемещает курсор на `n` строк вниз (`\x1b[nB`).

```yukino
public fun down(n: int32 = 1)
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: down

fun main() {
    down(1)
}
```

#### 6.1.3 forward

Смещает курсор вперед на `n` позиций вправо (`\x1b[nC`).

```yukino
public fun forward(n: int32 = 1)
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: forward

fun main() {
    forward(4)
}
```

#### 6.1.4 back

Смещает курсор назад на `n` позиций влево (`\x1b[nD`).

```yukino
public fun back(n: int32 = 1)
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: back

fun main() {
    back(2)
}
```

#### 6.1.5 to_col

Устанавливает курсор в указанную колонку текущей строки (0-индексированное значение).

```yukino
public fun to_col(col: int32)
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: to_col

fun main() {
    to_col(10)
}
```

#### 6.1.6 carriage_return

Возвращает курсор в начало текущей строки (`\r`).

```yukino
public fun carriage_return()
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: carriage_return

fun main() {
    carriage_return()
}
```

#### 6.1.7 clear_line

Полностью стирает текущую строку и возвращает курсор в её начало (`\r\x1b[2K`).

```yukino
public fun clear_line()
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: clear_line

fun main() {
    clear_line()
}
```

#### 6.1.8 clear_to_end

Стирает символы строки от текущей позиции курсора до правого края (`\x1b[K`).

```yukino
public fun clear_to_end()
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: clear_to_end

fun main() {
    clear_to_end()
}
```

#### 6.1.9 clear_lines_up

Стирает `n` строк выше текущей позиции и устанавливает курсор в начало первой очищенной строки.

```yukino
public fun clear_lines_up(n: int32)
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: clear_lines_up

fun main() {
    clear_lines_up(3)
}
```

#### 6.1.10 hide_cursor

Временно скрывает мигающий курсор терминала (`\x1b[?25l`), предотвращая визуальное мерцание во время отрисовки анимаций.

```yukino
public fun hide_cursor()
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: hide_cursor

fun main() {
    hide_cursor()
}
```

#### 6.1.11 show_cursor

Восстанавливает видимость курсора терминала (`\x1b[?25h`).

```yukino
public fun show_cursor()
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: show_cursor

fun main() {
    show_cursor()
}
```

#### 6.1.12 rewrite_line

Очищает текущую строку и сразу записывает переданный текст на ее место.

```yukino
public fun rewrite_line(text: string)
```

Функция не возвращает значения.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.cursor: rewrite_line

fun main() {
    rewrite_line("Обновленный статус")
}
```

---

### 6.2 Утилиты форматирования

#### 6.2.1 format_bytes

Форматирует количество байт в краткий человекочитаемый вид (`B`, `KiB`, `MiB`, `GiB`).

```yukino
public fun format_bytes(bytes: int64) -> string
```

Возвращает не-nullable строку вида `"142.5 KiB"`.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.format: format_bytes
load std.io: println

fun main() {
    println(format_bytes(2048)) // "2.0 KiB"
}
```

#### 6.2.2 format_speed

Форматирует скорость передачи байтов в секунду.

```yukino
public fun format_speed(bytes_per_sec: int64) -> string
```

Возвращает не-nullable строку вида `"2.4 MiB/s"`.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.format: format_speed
load std.io: println

fun main() {
    println(format_speed(1572864)) // "1.5 MiB/s"
}
```

#### 6.2.3 format_percent

Вычисляет процент завершения задачи и возвращает выровненную по правому краю трехзначную строку вида `" 45%"` или `"100%"`.

```yukino
public fun format_percent(current: int64, total: int64) -> string
```

Возвращает не-nullable строку.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.format: format_percent
load std.io: println

fun main() {
    println(format_percent(50, 100)) // " 50%"
}
```

#### 6.2.4 format_duration

Форматирует длительность в секундах в формат времени `"MM:SS"` либо `"HH:MM:SS"`.

```yukino
public fun format_duration(seconds: int64) -> string
```

Возвращает не-nullable строку времени.

Функция никогда не вызывает панику.

```yukino
load std.stdotool.format: format_duration
load std.io: println

fun main() {
    println(format_duration(95)) // "01:35"
}
```

---

[← Назад: Модуль std.itermtool](24-itermtool.md) | [Вперед: Модуль std.hash →](26-hash.md)
