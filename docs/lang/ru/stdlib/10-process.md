# Модуль std.process

Модуль `std.process` предоставляет интерфейс для запуска дочерних процессов, управления потоками ввода-вывода (stdin/stdout/stderr), чтения вывода команд, взаимодействия через системные оболочки, а также управления параметрами текущего процесса и переменными окружения.

---

## 1. Подключение: load std.process

Импорт всего пакета:

```yukino
load std.process
```

В текущую область видимости импортируются все публичные элементы модуля `std.process`:

- **Константы стандартных потоков (stdio):**
  - `STDIO_INHERIT`, `STDIO_PIPED`, `STDIO_NULL`.
- **Структуры данных и ошибок:**
  - `ProcessError` — структура ошибки системного вызова или запуска процесса;
  - `ExitStatus` — статус завершения процесса (код, флаг успеха, сигнал).
- **Классы:**
  - `ProcessOutput` — захваченный вывод и статус завершенной команды;
  - `ChildProcess` — дескриптор запущенного асинхронного дочернего процесса;
  - `Command` — построитель для детальной настройки запуска процесса.
- **Высокоуровневые функции запуска:**
  - `command_new`, `command_shell`, `exec`, `run`, `run_shell`, `read_output`, `read_output_shell`, `default_shell`.
- **Свойства процесса и переменные окружения:**
  - `current_pid`, `parent_pid`, `current_exe`, `current_dir`, `set_current_dir`, `env_var`, `set_env_var`, `remove_env_var`, `abort_process`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

### 2.1 Константы стандартных потоков ввода-вывода (stdio)

Константы определяют режим перенаправления дескрипторов `stdin`, `stdout` и `stderr` при конфигурации запуска дочернего процесса в построителе `Command`.

```yukino
public static STDIO_INHERIT: int32 = 0
public static STDIO_PIPED: int32 = 1
public static STDIO_NULL: int32 = 2
```

- `STDIO_INHERIT` (`0`) — дочерний процесс наследует стандартный поток родительского процесса.
- `STDIO_PIPED` (`1`) — создается анонимный системный канал (pipe) для программного перенаправления и захвата потока ввода-вывода.
- `STDIO_NULL` (`2`) — поток перенаправляется в системное устройство-заглушку (`/dev/null` на POSIX-системах или `NUL` на Windows).

---

## 3. Структуры данных и ошибок (паники)

### 3.1 ProcessError

Структура ошибки, используемая при возбуждении паники операциями запуска процессов, ожидания их завершения и выполнения команд оболочки.

```yukino
public struct ProcessError {
    message: string
    code: int32
}
```

- `message` — текстовое описание ошибки выполнения системной операции или запуска процесса.
- `code` — числовой код ошибки операционной системы либо ненулевой статус возврата.

### 3.2 ExitStatus

Представляет статус завершения дочернего процесса.

```yukino
public struct ExitStatus {
    code: int32
    success: bool
    signal: int32
}
```

- `code` — целочисленный код завершения процесса (`0` означает успешное завершение).
- `success` — логический флаг успешности, равный `true`, если код возврата равен `0`.
- `signal` — номер системного сигнала, если процесс был завершен сигналом, либо `0` при нормальном завершении.

---

## 4. Классы

### 4.1 ProcessOutput

Инкапсулирует результат выполнения команды: статус завершения и захваченный вывод стандартных потоков `stdout` и `stderr`.

```yukino
public class ProcessOutput
```

#### stdout_string
```yukino
fun stdout_string() -> string
```
Возвращает захваченный стандартный вывод команды в виде строки.

Возвращает строковое значение типа `string`.

Функция никогда не вызывает панику.

#### stderr_string
```yukino
fun stderr_string() -> string
```
Возвращает захваченный стандартный поток ошибок команды в виде строки.

Возвращает строковое значение типа `string`.

Функция никогда не вызывает панику.

#### success
```yukino
fun success() -> bool
```
Проверяет, завершился ли процесс успешно с нулевым кодом возврата.

Возвращает `true`, если код возврата равен 0, иначе `false`.

Функция никогда не вызывает панику.

#### code
```yukino
fun code() -> int32
```
Возвращает целочисленный код завершения процесса.

Возвращает числовое значение типа `int32`.

Функция никогда не вызывает панику.

```yukino
load std.process: run
load std.io: println

set out = run("echo", ["hello"])
if out.success() {
    println("Вывод: " + out.stdout_string())
}
```

---

### 4.2 ChildProcess

Представляет запущенный дочерний процесс, с которым можно взаимодействовать параллельно с основным кодом программы.

```yukino
public class ChildProcess
```

#### id
```yukino
fun id() -> int32
```
Возвращает системный идентификатор (PID) дочернего процесса.

Возвращает числовое значение идентификатора типа `int32`.

Функция никогда не вызывает панику.

#### wait
```yukino
fun wait() -> ExitStatus
```
Блокирует вызывающий поток до завершения дочернего процесса и возвращает статус его завершения.

Возвращает структуру `ExitStatus`.

В случае ошибки системного вызова ожидания вызывает панику со структурой `ProcessError`.

#### try_wait (nullable)
```yukino
fun try_wait() -> ExitStatus?
```
Проверяет, завершился ли дочерний процесс, без блокировки вызывающего потока.

Возвращает структуру `ExitStatus`, если процесс уже завершился, либо `null`, если процесс все еще выполняется.

В случае ошибки системного вызова опроса вызывает панику со структурой `ProcessError`.

#### kill
```yukino
fun kill() -> bool
```
Принудительно и немедленно завершает дочерний процесс (`SIGKILL` на POSIX-системах, `TerminateProcess` на Windows).

Возвращает `true` в случае успешной отправки команды завершения, иначе `false`.

Функция никогда не вызывает панику.

#### terminate
```yukino
fun terminate() -> bool
```
Отправляет дочернему процессу сигнал мягкого завершения (`SIGTERM` на POSIX-системах).

Возвращает `true` в случае успешной отправки сигнала, иначе `false`.

Функция никогда не вызывает панику.

#### write_stdin
```yukino
fun write_stdin(data: string) -> bool
```
Записывает переданную строку данных `data` в перенаправленный канал `stdin` дочернего процесса.

Возвращает `true`, если запись прошла успешно, иначе `false`.

Функция никогда не вызывает панику.

#### write_and_close_stdin
```yukino
fun write_and_close_stdin(data: string) -> bool
```
Записывает строку данных в канал `stdin` дочернего процесса и немедленно закрывает дескриптор ввода, передавая маркер конца файла (EOF).

Возвращает `true` в случае успешной записи и закрытия канала, иначе `false`.

Функция никогда не вызывает панику.

#### wait_with_output
```yukino
fun wait_with_output() -> ProcessOutput
```
Ожидает завершения дочернего процесса, полностью вычитывает все данные из перенаправленных каналов `stdout` и `stderr`, закрывает внутренние дескрипторы и формирует итоговый объект результата.

Возвращает объект `ProcessOutput`.

В случае сбоя системного вызова ожидания или ошибки чтения каналов вызывает панику со структурой `ProcessError`.

```yukino
load std.process: command_new, STDIO_PIPED
load std.io: println

set cmd = command_new("cat")
cmd.stdin(STDIO_PIPED)
cmd.stdout(STDIO_PIPED)
set child = cmd.spawn()
child.write_and_close_stdin("Yukino Systems")
set out = child.wait_with_output()
println("Вывод: " + out.stdout_string())
```

---

### 4.3 Command

Построитель для детальной настройки аргументов запускаемого процесса, рабочей директории, переменных окружения и перенаправления стандартных потоков.

```yukino
public class Command
```

#### arg
```yukino
fun arg(a: string) -> Command
```
Добавляет один строковый аргумент `a` к списку аргументов запускаемой команды. Возвращает текущий экземпляр `Command` для цепочечных вызовов. Функция никогда не вызывает панику.

#### args
```yukino
fun args(arr: string[]) -> Command
```
Добавляет массив строковых аргументов `arr` к списку аргументов запускаемой команды. Возвращает текущий экземпляр `Command`. Функция никогда не вызывает панику.

#### current_dir
```yukino
fun current_dir(dir: string) -> Command
```
Задает рабочую директорию `dir`, в которой будет запущен процесс. Возвращает текущий экземпляр `Command`. Функция никогда не вызывает панику.

#### env
```yukino
fun env(key: string, val: string) -> Command
```
Устанавливает или переопределяет переменную окружения с именем `key` и значением `val` для запускаемого процесса. Возвращает текущий экземпляр `Command`. Функция никогда не вызывает панику.

#### env_clear
```yukino
fun env_clear() -> Command
```
Очищает все унаследованные переменные окружения, настраивая запуск процесса с абсолютно пустым набором переменных (за исключением явно заданных через метод `env`). Возвращает текущий экземпляр `Command`. Функция никогда не вызывает панику.

#### stdin / stdout / stderr
```yukino
fun stdin(mode: int32) -> Command
fun stdout(mode: int32) -> Command
fun stderr(mode: int32) -> Command
```
Устанавливает режим работы соответствующего стандартного потока ввода-вывода (`STDIO_INHERIT`, `STDIO_PIPED` или `STDIO_NULL`). Возвращает текущий экземпляр `Command`. Функции никогда не вызывают панику.

#### output
```yukino
fun output() -> ProcessOutput
```
Запускает команду, автоматически настраивая перенаправление `stdout` и `stderr` в каналы, ожидает полного завершения процесса и возвращает объект `ProcessOutput`.

Возвращает объект `ProcessOutput` с захваченным выводом и кодом завершения.

В случае невозможности запустить исполняемый файл или системного сбоя вызывает панику со структурой `ProcessError`.

#### status
```yukino
fun status() -> ExitStatus
```
Запускает команду с режимами стандартных потоков, настроенными через методы `stdin`/`stdout`/`stderr` (по умолчанию — унаследованными), ожидает завершения процесса и возвращает структуру статуса.

Возвращает структуру `ExitStatus`.

В случае ошибки запуска процесса вызывает панику со структурой `ProcessError`.

#### spawn
```yukino
fun spawn() -> ChildProcess
```
Асинхронно порождает дочерний процесс в соответствии с заданной конфигурацией и возвращает дескриптор `ChildProcess` для управления им.

Возвращает объект `ChildProcess`.

В случае ошибки создания процесса вызывает панику со структурой `ProcessError`.

```yukino
load std.process: command_new
load std.io: println

set cmd = command_new("echo")
cmd.arg("Привет, мир!")
set out = cmd.output()
println(out.stdout_string())
```

---

## 5. Справочник функций: Высокоуровневый запуск процессов

### 5.1 command_new

Создает новый экземпляр построителя `Command` для запуска указанной исполняемой программы `prog`.

```yukino
public fun command_new(prog: string) -> Command
```

Возвращает новый экземпляр класса `Command`.

Функция никогда не вызывает панику.

```yukino
load std.process: command_new
load std.io: println

set cmd = command_new("ls")
cmd.arg("-la")
set st = cmd.status()
println("Код завершения: ${st.code}")
```

### 5.2 command_shell

Создает новый экземпляр построителя `Command` для выполнения переданного строкового скрипта `script` через указанную командную оболочку `launcher` (например, `"sh -c"`).

```yukino
public fun command_shell(launcher: string, script: string) -> Command
```

Возвращает сконфигурированный экземпляр класса `Command`.

Функция никогда не вызывает панику.

```yukino
load std.process: command_shell
load std.io: println

set cmd = command_shell("sh -c", "echo 42")
set out = cmd.output()
println("Результат: " + out.stdout_string())
```

### 5.3 exec

Синхронно выполняет скрипт через командную оболочку `launcher`, напрямую наследуя стандартные потоки ввода-вывода текущего терминала.

```yukino
public fun exec(launcher: string, script: string) -> int32
```

Возвращает целочисленный код возврата выполненной команды типа `int32`.

В случае ошибки создания процесса командной оболочки вызывает панику со структурой `ProcessError`.

```yukino
load std.process: exec, default_shell
load std.io: println

set code = exec(default_shell(), "echo 'Yukino exec'")
println("Код: ${code}")
```


### 5.4 run

Запускает указанную исполняемую программу `program` с аргументами `args`, ожидает ее завершения и захватывает потоки `stdout` и `stderr`.

```yukino
public fun run(program: string, args: string[]) -> ProcessOutput
```

Возвращает объект `ProcessOutput`, содержащий статус завершения и строковый вывод обоих потоков.

В случае невозможности запуска программы вызывает панику со структурой `ProcessError`.

```yukino
load std.process: run
load std.io: println

set res = run("uname", ["-s"])
println("ОС: " + res.stdout_string())
```

### 5.5 run_shell

Выполняет скрипт `script` в системной оболочке `launcher`, захватывая весь поток вывода команды.

```yukino
public fun run_shell(launcher: string, script: string) -> ProcessOutput
```

Возвращает объект `ProcessOutput` с результатами работы оболочки.

В случае ошибки запуска процесса оболочки вызывает панику со структурой `ProcessError`.

```yukino
load std.process: run_shell, default_shell
load std.io: println

set out = run_shell(default_shell(), "echo 'Строка'")
println("Вывод: " + out.stdout_string())
```

### 5.6 read_output

Запускает программу `program` с аргументами `args`, проверяет статус завершения (`code == 0`) и возвращает очищенную от пробелов и переводов строк на конце строку стандартного вывода `stdout`.

```yukino
public fun read_output(program: string, args: string[]) -> string
```

Возвращает строку вывода типа `string`.

Если запуск не удался либо программа завершилась с ненулевым кодом ошибки, вызывает панику со структурой `ProcessError`.

```yukino
load std.process: read_output, ProcessError
load std.io: println

sus {
    set branch = read_output("git", ["branch", "--show-current"])
    println("Ветка: " + branch)
} catch err: ProcessError {
    println("Ошибка git: ${err.message}")
}
```

### 5.7 read_output_shell

Выполняет скрипт `script` через командную оболочку `launcher`, проверяет успешность выполнения (`code == 0`) и возвращает обрезанную строку вывода `stdout`.

```yukino
public fun read_output_shell(launcher: string, script: string) -> string
```

Возвращает строковый вывод типа `string`.

Если запуск завершился сбоем или ненулевым кодом возврата, вызывает панику со структурой `ProcessError`.

```yukino
load std.process: read_output_shell, default_shell
load std.io: println

set who = read_output_shell(default_shell(), "whoami")
println("Пользователь: " + who)
```

### 5.8 default_shell

Возвращает стандартную системную строку запуска командной оболочки для текущей операционной системы: `"sh -c"` на POSIX/Linux системах либо `"powershell -Command"` на Windows.

```yukino
public fun default_shell() -> string
```

Возвращает строковое значение типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.process: default_shell
load std.io: println

set shell = default_shell()
println("Стандартная оболочка: " + shell)
```

---

## 6. Справочник функций: Управление текущим процессом и переменными окружения

### 6.1 current_pid

Возвращает системный идентификатор текущего процесса (PID).

```yukino
public fun current_pid() -> int32
```

Возвращает числовой идентификатор процесса типа `int32`.

Функция никогда не вызывает панику.

```yukino
load std.process: current_pid
load std.io: println

set pid = current_pid()
println("PID текущего процесса: ${pid}")
```

### 6.2 parent_pid (nullable)

Возвращает идентификатор родительского процесса (PPID).

```yukino
public fun parent_pid() -> int32?
```

Возвращает целочисленный идентификатор родительского процесса типа `int32` либо `null`, если идентификатор недоступен или равен нулю.

Функция никогда не вызывает панику.

```yukino
load std.process: parent_pid
load std.io: println

set ppid = parent_pid()
if ppid != null {
    println("PPID родителя: ${ppid}")
}
```

### 6.3 current_exe

Возвращает абсолютный канонический путь к исполняемому файлу текущего процесса в файловой системе.

```yukino
public fun current_exe() -> string
```

Возвращает строковый путь типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.process: current_exe
load std.io: println

set exe = current_exe()
println("Исполняемый файл: " + exe)
```

### 6.4 current_dir

Возвращает абсолютный путь к текущей рабочей директории процесса.

```yukino
public fun current_dir() -> string
```

Возвращает строку пути типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.process: current_dir
load std.io: println

set cwd = current_dir()
println("Рабочая директория: " + cwd)
```

### 6.5 set_current_dir

Изменяет текущую рабочую директорию процесса на указанный путь `path`.

```yukino
public fun set_current_dir(path: string) -> bool
```

Возвращает `true`, если директория была успешно изменена, иначе `false`.

Функция никогда не вызывает панику.

```yukino
load std.process: set_current_dir
load std.io: println

set ok = set_current_dir("/tmp")
println("Успешно: ${ok}")
```

### 6.6 env_var (nullable)

Считывает строковое значение переменной окружения по указанному имени `name`.

```yukino
public fun env_var(name: string) -> string?
```

Возвращает строковое значение переменной либо `null`, если переменная окружения не найдена или не установлена.

Функция никогда не вызывает панику.

```yukino
load std.process: env_var
load std.io: println

set home = env_var("HOME")
if home != null {
    println("Домашний каталог: " + home)
}
```

### 6.7 set_env_var

Устанавливает или перезаписывает значение переменной окружения с именем `name` значением `value` для текущего процесса.

```yukino
public fun set_env_var(name: string, value: string) -> bool
```

Возвращает логическое значение `true` при успешной установке переменной, иначе `false`.

Функция никогда не вызывает панику.

```yukino
load std.process: set_env_var, env_var
load std.io: println

set_env_var("YUKINO_MODE", "release")
set mode = env_var("YUKINO_MODE") ?? "debug"
println("Режим: " + mode)
```

### 6.8 remove_env_var

Удаляет указанную переменную окружения по имени `name` из текущего процесса.

```yukino
public fun remove_env_var(name: string) -> bool
```

Возвращает логическое значение `true` при успешном удалении переменной, иначе `false`.

Функция никогда не вызывает панику.

```yukino
load std.process: remove_env_var
load std.io: println

set removed = remove_env_var("TEMP_KEY")
println("Удалено: ${removed}")
```

### 6.9 abort_process

Немедленно выполняет аварийное завершение процесса через системный вызов `abort`, без выполнения деструкторов и раскрутки стека.

```yukino
public fun abort_process()
```

Функция не возвращает управление и немедленно завершает работу процесса.

Функция никогда не вызывает панику.

```yukino
load std.process: abort_process

// Аварийный выход из программы
// abort_process()
```

---

[← Назад: Модуль std.collections](9-collections.md) | [Вперед: Модуль std.os →](11-os.md)
