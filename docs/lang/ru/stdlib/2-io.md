# Модуль std.io

Модуль `std.io` предоставляет функционал стандартного потокового и консольного ввода-вывода: вывод в stdout и stderr, чтение строк и байт из stdin, управление блокировками потоков, проверка интерактивного терминала, управление курсором и сырым режимом (raw mode).

---

## 1. Подключение: load std.io

Импорт всего пакета:

```yukino
load std.io
```

В текущую область видимости импортируются все публичные элементы модуля `std.io`:

- **Интерфейсы потоков:**
  - `Reader<T>` — контракт на чтение элементов типа `T` через сырой буфер `ptr<T>`;
  - `Writer<T>` — контракт на запись данных типа `T` и сброс буфера;
  - `Stream<T>` — объединенный контракт чтения, записи, сброса и закрытия ресурса;
  - `Seekable` — контракт позиционирования в потоке.
- **Структуры:**
  - `IoError` — структура ошибки ввода-вывода (коды `101`–`108`).
- **Функции ввода-вывода:**
  - *Вывод в stdout / stderr:* `print`, `println`, `eprint`, `eprintln`, `flush`, `eflush`, `write_string`, `ewrite_string`;
  - *Чтение из stdin:* `read_line`, `read_all`, `read_char`, `discard_line`, `read_exact`;
  - *Асинхронный и фоновый опрос:* `poll_readable`, `enable_background_reader`, `read_char_async`;
  - *Блокировки потоков:* `lock_stdout`, `unlock_stdout`, `lock_stdin`, `unlock_stdin`;
  - *Управление терминалом:* `is_terminal`, `is_stdin_terminal`, `is_stdout_terminal`, `is_stderr_terminal`, `terminal_size`, `terminal_tuple`, `terminal_cols`, `terminal_rows`, `set_raw_mode`, `clear_screen`, `cursor_move_to`, `hide_cursor`, `show_cursor`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.io` глобальные константы не объявлены.

---

## 3. Интерфейсы

### 3.1 Reader<T>

Обобщенный контракт для объектов, поддерживающих низкоуровневое чтение элементов типа `T` в память по указателю.

```yukino
public interface Reader<T> {
    fun read(buf: ptr<T>, max_len: int64) -> int64
}
```

### 3.2 Writer<T>

Обобщенный контракт для объектов, поддерживающих запись элементов типа `T` и принудительный сброс буфера.

```yukino
public interface Writer<T> {
    fun write(data: T) -> int64
    fun flush() -> int32
}
```

### 3.3 Stream<T>

Объединенный обобщенный интерфейс для потоковых ресурсов, поддерживающих чтение элементов типа `T`, запись данных типа `T` и закрытие.

```yukino
public interface Stream<T> {
    fun read(buf: ptr<T>, max_len: int64) -> int64
    fun write(data: T) -> int64
    fun flush() -> int32
    fun close() -> int32
}
```

### 3.4 Seekable

Контракт для ресурсов с поддержкой перемещения указателя чтения/записи.

```yukino
public interface Seekable {
    fun seek(offset: int64, whence: int32) -> int64
}
```

---

## 4. Структуры данных и ошибок

### 4.1 IoError

Структура ошибки, генерируемая при системных сбоях операций ввода-вывода.

```yukino
public struct IoError {
    message: string
    code: int32
}
```

- `code: 101` — ошибка чтения строки из stdin (`read_line`);
- `code: 102` — ошибка полного вычитывания stdin (`read_all`);
- `code: 103` — сбой записи данных в stdout (`write_string`);
- `code: 104` — сбой записи данных в stderr (`ewrite_string`);
- `code: 105` — сбой сброса буфера stdout (`flush`);
- `code: 106` — сбой сброса буфера stderr (`eflush`);
- `code: 107` — в функцию передан нулевой указатель буфера при положительной длине (`read_exact`);
- `code: 108` — системная ошибка чтения точного количества байт из stdin (`read_exact`).

---

## 5. Справочник функций

### 5.1 print

Записывает строку `msg` в стандартный вывод `stdout` без добавления завершающего символа перевода строки и без принудительного сброса буфера; для немедленного вывода используйте функцию `flush`.

```yukino
public fun print(msg: string)
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: print

print("Загрузка: ")
print("100%\n")
```

### 5.2 println

Записывает строку `msg` в стандартный вывод `stdout` и добавляет завершающий символ перевода строки `\n`.

```yukino
public fun println(msg: string)
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: println

println("Привет, мир!")
```

### 5.3 eprint

Записывает строку `msg` в стандартный поток ошибок `stderr` без добавления завершающего символа перевода строки.

```yukino
public fun eprint(msg: string)
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: eprint

eprint("Предупреждение: низкий уровень памяти")
```

### 5.4 eprintln

Записывает строку `msg` в стандартный поток ошибок `stderr` с автоматическим добавлением завершающего символа перевода строки `\n`.

```yukino
public fun eprintln(msg: string)
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: eprintln

eprintln("Критическая ошибка конфигурации")
```

### 5.5 flush

Принудительно сбрасывает внутренний буфер вывода `stdout` в терминал или связанный файловый дескриптор.

```yukino
public fun flush() -> int32
```

Возвращает `0` при успешном сбросе буфера.

При системном сбое сброса буфера вызывает панику со структурой `IoError` с кодом `105`.

```yukino
load std.io: print, flush

print("Обработка данных...")
flush()
```

### 5.6 eflush

Принудительно сбрасывает внутренний буфер потока ошибок `stderr`.

```yukino
public fun eflush() -> int32
```

Возвращает `0` при успешном сбросе буфера.

При системном сбое сброса буфера вызывает панику со структурой `IoError` с кодом `106`.

```yukino
load std.io: eprint, eflush

eprint("Сбой узла!")
eflush()
```

### 5.7 write_string

Записывает строку `msg` в стандартный вывод `stdout` и возвращает количество фактически переданных байт.

```yukino
public fun write_string(msg: string) -> int64
```

Возвращает количество успешно записанных байт как `int64`.

При системном сбое записи данных вызывает панику со структурой `IoError` с кодом `103`.

```yukino
load std.io: write_string, println

set written = write_string("пакет данных\n")
println("Записано байт: ${written}")
```

### 5.8 ewrite_string

Записывает строку `msg` в стандартный поток ошибок `stderr` и возвращает число переданных байт.

```yukino
public fun ewrite_string(msg: string) -> int64
```

Возвращает количество успешно записанных байт в поток ошибок как `int64`.

При системном сбое записи вызывает панику со структурой `IoError` с кодом `104`.

```yukino
load std.io: ewrite_string

set written = ewrite_string("ошибка дескриптора\n")
```

### 5.9 read_line

Считывает одну строку из стандартного ввода `stdin` до первого встреченного символа перевода строки `\n`. Возвращает прочитанную строку без символов перевода строки. Если достигнут конец потока ввода (EOF), возвращает пустую строку `""`.

```yukino
public fun read_line() -> string
```

Возвращает считанную строку `string`.

При системной ошибке чтения из `stdin` вызывает панику со структурой `IoError` с кодом `101`.

```yukino
load std.io: print, println, read_line

print("Введите ваше имя: ")
set name = read_line()
println("Здравствуйте, ${name}")
```

### 5.10 read_all

Вычитывает все доступные данные из стандартного ввода `stdin` до достижения признака конца файла (EOF) и возвращает их в виде единой строки.

```yukino
public fun read_all() -> string
```

Возвращает все прочитанные данные в виде строки `string`.

При системной ошибке вычитывания потока вызывает панику со структурой `IoError` с кодом `102`.

```yukino
load std.io: read_all, println

set content = read_all()
println("Получено байт: ${content.len()}")
```

### 5.11 discard_line

Считывает и отбрасывает все символы из входного буфера `stdin` вплоть до первого встреченного перевода строки `\n` или признака конца потока EOF. Используется для очистки остатков пользовательского ввода после частичного чтения.

```yukino
public fun discard_line()
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: read_char, discard_line

set choice = read_char(false)
discard_line()
```

### 5.12 read_char

Считывает один байт или символ из стандартного ввода `stdin`. Если флаг `clean_buffer` установлен в `true`, остаток текущей строки вычитывается и отбрасывается.

```yukino
public fun read_char(clean_buffer: bool = false) -> int32
```

Возвращает числовой код символа как `int32`, либо `-1` при достижении признака конца файла (EOF).

Функция никогда не вызывает панику.

```yukino
load std.io: read_char, println

set code = read_char(true)
if code != -1 {
    set ch: char = cast(code, "char")
    println("Считан символ: ${ch} (код: ${code})")
}
```

### 5.13 read_exact

Считывает до `n` байт из стандартного ввода `stdin` одной операцией чтения напрямую в буфер памяти по сырому указателю `buf`.

```yukino
public fun read_exact(buf: ptr<uint8>, n: int64) -> int64
```

Возвращает фактическое число прочитанных байт как `int64`.

Вызывает панику со структурой `IoError` с кодом `107`, если передан нулевой указатель `buf == null` при `n > 0`. При системном сбое чтения вызывает панику со структурой `IoError` с кодом `108`.

```yukino
load std.io: read_exact

fun read_header(buffer_ptr: ptr<uint8>) -> int64 {
    return read_exact(buffer_ptr, 16)
}
```

### 5.14 is_stdin_terminal

Проверяет, подключен ли стандартный ввод `stdin` (файловый дескриптор 0) к интерактивному терминалу (TTY).

```yukino
public fun is_stdin_terminal() -> bool
```

Возвращает `true`, если ввод подключен к интерактивному терминалу, иначе `false` (при перенаправлении из файла или конвейера).

Функция никогда не вызывает панику.

```yukino
load std.io: is_stdin_terminal, println

if is_stdin_terminal() {
    println("Интерактивный терминальный ввод")
}
```

### 5.15 is_stdout_terminal

Проверяет, подключен ли стандартный вывод `stdout` (файловый дескриптор 1) к интерактивному терминалу.

```yukino
public fun is_stdout_terminal() -> bool
```

Возвращает `true`, если стандартный вывод направлен в терминал, иначе `false` (при выводе в файл или конвейер).

Функция никогда не вызывает панику.

```yukino
load std.io: is_stdout_terminal, println

if is_stdout_terminal() {
    println("Вывод направлен в окно терминала")
}
```

### 5.16 is_stderr_terminal

Проверяет, подключен ли стандартный поток ошибок `stderr` (файловый дескриптор 2) к интерактивному терминалу.

```yukino
public fun is_stderr_terminal() -> bool
```

Возвращает `true`, если поток ошибок направлен в интерактивный терминал, иначе `false`.

Функция никогда не вызывает панику.

```yukino
load std.io: is_stderr_terminal, eprintln

if is_stderr_terminal() {
    eprintln("Терминал поддерживает цветовые коды ошибок")
}
```

### 5.17 is_terminal

Проверяет, является ли терминалом стандартный вывод программы. Является кратким синонимом функции `is_stdout_terminal()`.

```yukino
public fun is_terminal() -> bool
```

Возвращает `true`, если стандартный вывод направлен в терминал, иначе `false`.

Функция никогда не вызывает панику.

```yukino
load std.io: is_terminal, println

if is_terminal() {
    println("Интерактивный режим работы")
}
```

### 5.18 poll_readable

Опрашивает стандартный ввод `stdin` на наличие готовых для чтения данных в течение таймаута `timeout_ms` миллисекунд.

```yukino
public fun poll_readable(timeout_ms: int64 = 0) -> bool
```

Возвращает `true`, если во входном потоке есть данные, готовые к чтению без блокировки потока, иначе `false`.

Функция никогда не вызывает панику.

```yukino
load std.io: poll_readable, read_line, println

if poll_readable(500) {
    set input = read_line()
    println("Получено: ${input}")
}
```

### 5.19 enable_background_reader

Запускает внутренний фоновый сервисный поток для асинхронного неблокирующего вычитывания байтов из стандартного ввода.

```yukino
public fun enable_background_reader()
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: enable_background_reader

enable_background_reader()
```

### 5.20 read_char_async

Выполняет неблокирующее чтение одного байта из внутренней очереди фонового сервисного потока ввода.

```yukino
public fun read_char_async() -> int32
```

Возвращает числовой код байта как `int32`, либо `-1`, если новые данные в буфере отсутствуют.

Функция никогда не вызывает панику.

```yukino
load std.io: enable_background_reader, read_char_async, println

enable_background_reader()
set ch = read_char_async()
if ch != -1 {
    println("Код нажатой клавиши: ${ch}")
}
```

### 5.21 lock_stdout

Захватывает внутренний мьютекс стандартного вывода `stdout`. Предотвращает перемешивание фрагментов строк при параллельном выводе из нескольких системных потоков.

```yukino
public fun lock_stdout()
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: lock_stdout, unlock_stdout, print

lock_stdout()
print("Секция 1: ")
print("OK\n")
unlock_stdout()
```

### 5.22 unlock_stdout

Освобождает внутренний мьютекс стандартного вывода `stdout`, позволяя другим потокам продолжить запись в поток.

```yukino
public fun unlock_stdout()
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: lock_stdout, unlock_stdout, println

lock_stdout()
println("Атомарное сообщение")
unlock_stdout()
```

### 5.23 lock_stdin

Захватывает внутренний мьютекс стандартного ввода `stdin`, исключая состояние гонки при параллельном чтении данных из нескольких потоков.

```yukino
public fun lock_stdin()
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: lock_stdin, unlock_stdin, read_line

lock_stdin()
set s = read_line()
unlock_stdin()
```

### 5.24 unlock_stdin

Освобождает ранее захваченный мьютекс стандартного ввода `stdin`.

```yukino
public fun unlock_stdin()
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: lock_stdin, unlock_stdin, read_char

lock_stdin()
set ch = read_char(false)
unlock_stdin()
```

### 5.25 terminal_tuple

Запрашивает у операционной системы текущие размеры окна терминала и возвращает кортеж, состоящий из количества колонок и строк.

```yukino
public fun terminal_tuple() -> (int32, int32)
```

Возвращает кортеж `(int32, int32)`, где первое значение — ширина (`cols`), второе — высота (`rows`). Если определить размеры не удалось, возвращаются значения по умолчанию `(80, 24)`.

Функция никогда не вызывает панику.

```yukino
load std.io: terminal_tuple, println

set (cols, rows) = terminal_tuple()
println("Размер окна: ${cols}x${rows}")
```

### 5.26 terminal_size

Возвращает текущие габариты текстового терминала в виде анонимного кортежа `(int32, int32)`.

```yukino
public fun terminal_size() -> (int32, int32)
```

Возвращает кортеж `(cols, rows)`, где `sz.0` — количество колонок (ширина), а `sz.1` — количество строк (высота). Если операционная система не вернула размеры, используются значения по умолчанию `cols = 80, rows = 24`.

Функция никогда не вызывает панику.

```yukino
load std.io: terminal_size, println

set sz = terminal_size()
println("Разрешение терминала: ${sz.0}x${sz.1}")
```

### 5.27 terminal_cols

Возвращает ширину текущего экрана терминала в знакоместах (колонках).

```yukino
public fun terminal_cols() -> int32
```

Возвращает количество колонок терминала как `int32` (по умолчанию `80`).

Функция никогда не вызывает панику.

```yukino
load std.io: terminal_cols, println

set width = terminal_cols()
println("Ширина окна: ${width}")
```

### 5.28 terminal_rows

Возвращает высоту текущего экрана терминала в знакоместах (строках).

```yukino
public fun terminal_rows() -> int32
```

Возвращает количество строк терминала как `int32` (по умолчанию `24`).

Функция никогда не вызывает панику.

```yukino
load std.io: terminal_rows, println

set height = terminal_rows()
println("Высота окна: ${height}")
```

### 5.29 set_raw_mode

Переключает режим работы терминала между каноническим и сырым (raw mode). В сыром режиме отключаются строковая буферизация ввода и эхо-печать, позволяя перехватывать отдельные нажатия клавиш без ожидания клавиши Enter.

```yukino
public fun set_raw_mode(enable: bool) -> bool
```

Возвращает `true`, если режим терминала успешно изменен, иначе `false`.

Функция никогда не вызывает панику.

```yukino
load std.io: set_raw_mode, read_char

set_raw_mode(true)
set key = read_char(false)
set_raw_mode(false)
```

### 5.30 clear_screen

Очищает экран терминала и перемещает текстовый курсор в левый верхний угол (координаты 1;1), отправляя ANSI-последовательность `\x1b[2J\x1b[H`.

```yukino
public fun clear_screen()
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: clear_screen, println

clear_screen()
println("Экран терминала очищен")
```

### 5.31 cursor_move_to

Перемещает текстовый курсор терминала в абсолютную позицию по указанным координатам колонки `col` и строки `row`, используя ANSI-последовательность `\x1b[row;colH`.

```yukino
public fun cursor_move_to(col: int32, row: int32)
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: cursor_move_to, print

cursor_move_to(10, 5)
print("Текст на строке 5, колонке 10")
```

### 5.32 hide_cursor

Скрывает мигающий текстовый курсор в окне терминала с помощью ANSI-команды `\x1b[?25l`.

```yukino
public fun hide_cursor()
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: hide_cursor

hide_cursor()
```

### 5.33 show_cursor

Делает текстовый курсор видимым в окне терминала с помощью ANSI-команды `\x1b[?25h`.

```yukino
public fun show_cursor()
```

Функция ничего не возвращает (`void`).

Функция никогда не вызывает панику.

```yukino
load std.io: show_cursor

show_cursor()
```

---

[← Назад: Модуль std.strings](1-strings.md) | [Вперед: Модуль std.threads →](3-threads.md)
