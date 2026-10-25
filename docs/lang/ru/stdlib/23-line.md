# Модуль std.line

Модуль `std.line` предоставляет декларативный объектно-ориентированный парсер аргументов командной строки, спроектированный на чистом языке Yukino по аналогии с популярной библиотекой `clap`. Модуль поддерживает иерархические команды и подкоманды произвольной глубины вложенности, именованные параметры, короткие и длинные флаги, объединение коротких флагов в связки (`-abc`), присваивание через знак равенства (`--opt=val`), специальный терминатор `--` для передачи необработанных аргументов, автоматическую генерацию форматированной справки (`-h`, `--help`) и версии (`-v`, `--version`), взаимно исключающие аргументы (`conflicts_with`), а также нечеткий поиск и исправление опечаток ("Did you mean...") на базе расстояния Левенштейна со стилизацией через модуль `std.outf`.

### Удобный CLI как одна из главных целей Yukino

В философии языка Yukino первоклассная поддержка разработки CLI-инструментов стоит на одном из первых мест. Большинство современных инструментов компиляции, утилит сборки, системного администрирования и сетевых сервисов управляются через консоль. Если командный интерфейс спроектирован плохо — отсутствуют подсказки, опечатка приводит к аварийному завершению без внятного сообщения, ключи требуют неестественного порядка, а справку приходится форматировать вручную — продуктивность разработчика резко падает.

Ручной разбор аргументов через массив строк `args()` или `raw_args()` в любом нетривиальном проекте быстро превращается в запутанный код с десятками ручных проверок `i + 1 < args.len()`, забытыми проверками обязательных полей, отсутствием группировки флагов и уязвимостями выхода за границы массива. Модуль `std.line` избавляет от этой рутины: разработчик описывает *структуру* параметров программы в чистом декларативном стиле, а парсер автоматически берет на себя синтаксический анализ, приведение значений, проверку ограничений и красивый вывод.

### Архитектурные решения: почему именно так, а не иначе

1. **Явный ООП-стиль и прямой вызов `init Line(...)` вместо скрытых фабричных функций.**
   В Yukino строго соблюдается принцип понятности и предсказуемости: явное лучше неявного (`explicit > implicit`). Конструкция `init Line("app", version)` или `init Arg("config")` открыто демонстрирует разработчику и компилятору, что создается полноценный объект с выделением памяти в текущем регионе. В Yukino сознательно не используются псевдо-магические функции-обертки, скрывающие внутреннюю структуру, благодаря чему любой программист, открыв исходный код, сразу понимает типы, методы и жизненный цикл объектов.

2. **Шаблон «Строитель» (Builder Pattern) для свободного связывания методов.**
   Все методы конфигурации классов `Line` и `Arg` возвращают сам экземпляр (`return this`). Это позволяет лаконично объявлять сколь угодно сложные CLI-приложения единым блоком цепочечных вызовов без необходимости создавать десятки временных переменных и загромождать пространство имен.

3. **Четкое разделение описания грамматики (`Line`) и фактических данных (`Matches`).**
   Класс `Line` задает *правила* (какие флаги разрешены, какие аргументы обязательны, какие взаимоисключаются). Класс `Matches` хранит *результат* сопоставления этих правил с конкретной командной строкой текущего запуска. Такое разделение ответственности устраняет состояние гонки и позволяет свободно передавать результат разбора `Matches` в бизнес-логику приложения.

4. **Автоматическая валидация, код выхода `2` и алгоритм Левенштейна.**
   При обнаружении синтаксических ошибок (неизвестный флаг, пропущенное обязательное значение, неизвестная команда) `std.line` не выбрасывает неконтролируемые исключения, а формирует стандартизированное сообщение об ошибке в терминале, подсказывает похожую опцию («Did you mean...») и завершает процесс с кодом `2` (стандартный для UNIX/POSIX код синтаксических ошибок командной строки).

### Анатомия командной строки: сопоставление элементов

В `std.line` поддерживаются все классические конструкции командных строк:

| Элемент | Назначение | Синтаксис в терминале | Конфигурация в коде | Чтение значения в коде |
| :--- | :--- | :--- | :--- | :--- |
| **Флаг (Flag)** | Булев переключатель (вкл/выкл) без значения. | `-v`, `--verbose`, связка `-xvf` | `(init Arg("verbose")).short("v").long("verbose").flag()` | `matches.has_flag("verbose")` -> `bool` |
| **Опция (Option)** | Именованный параметр, требующий конкретного значения. | `-c config.json`, `--config=config.json` | `(init Arg("config")).short("c").long("config").value_name("path")` | `matches.get_value("config")` -> `string?` |
| **Позиционный аргумент** | Значение без имени ключа, определяемое позицией. | `mycli input.txt output.bin` | `(init Arg("input")).positional(true).required(true)` | `matches.get_value("input")` -> `string?` |
| **Подкоманда (Subcommand)** | Изолированное дочернее действие со своими флагами и справкой. | `git commit -m "..."`, `cargo build --release` | `cli.subcommand((init Line("build", "1.0.0"))...)` | `matches.has_subcommand("build")`, `matches.get_subcommand_matches("build")` |
| **Разделитель `--` (Rest)** | Передача всех последующих аргументов без разбора. | `mycli run -- -x -y raw_file` | Доступен автоматически во всех командах | `matches.get_rest()` -> `string[]` |

### Практические сценарии: «Если настроить в коде -> как будет принимать терминал»

#### Сценарий 1: Простой флаг-переключатель и связки (Flag Bundling)
Если объявлено несколько коротких однобуквенных флагов:

```yukino
set cli = (init Line("mytool", "1.0.0"))
    .flag("all", "a", "all", "Показывать скрытые элементы")
    .flag("long", "l", "long", "Использовать длинный формат")
    .flag("human", "h", "human-readable", "Выводить размеры в удобном виде")
```

Пользователь терминала может запустить программу любым удобным ему способом:
- Раздельно длинными ключами: `mytool --all --long`
- Раздельно короткими ключами: `mytool -a -l`
- В виде единой объединенной связки: `mytool -al` или `mytool -la`
- В коде достаточно проверить: `if matches.has_flag("all") { ... }`.

#### Сценарий 2: Опция со значением и значением по умолчанию
Если параметр может принимать значение, а при его отсутствии должна использоваться безопасная константа:

```yukino
set cli = (init Line("server", "1.0.0"))
    .arg((init Arg("port"))
        .short("p")
        .long("port")
        .value_name("number")
        .default_value("8080")
        .about("Сетевой порт для входящих подключений"))
```

Пользователь в терминале может передать:
- Через пробел: `server -p 3000` или `server --port 3000`
- Через знак равенства: `server -p=3000` или `server --port=3000`
- Вообще ничего не указывать: `server`
- В коде вызов `matches.get_value("port")` в первых случаях вернет `"3000"`, а при запуске без ключа вернет `"8080"`.
- При этом `matches.has_flag("port")` вернет `true` только если пользователь явно указал ключ в консоли.

#### Сценарий 3: Обязательные позиционные аргументы
Для утилит обработки файлов или компиляторов аргументы часто передаются без префиксов `-` или `--`:

```yukino
set cli = (init Line("compiler", "1.0.0"))
    .arg((init Arg("source")).positional(true).required(true).about("Путь к исходному коду"))
    .arg((init Arg("target")).positional(true).default_value("out.bin").about("Имя выходного бинарника"))
```

- Вызов `compiler main.ykn app.bin` запишет `"main.ykn"` в `source` и `"app.bin"` в `target`.
- Вызов `compiler main.ykn` запишет `"main.ykn"` в `source`, а для `target` использует значение по умолчанию `"out.bin"`.
- Если пользователь запустит `compiler` без аргументов, парсер сам остановит выполнение и напечатает:
  ```text
  error: the following required argument was not provided: <source>

  For more information, try '--help'.
  ```

#### Сценарий 4: Взаимоисключающие параметры (`conflicts_with`)
Если две опции логически несовместимы (например, тихий режим и режим подробного отладочного вывода):

```yukino
set cli = (init Line("app", "1.0.0"))
    .arg((init Arg("quiet")).short("q").long("quiet").flag().conflicts_with("verbose").about("Подавить весь вывод"))
    .arg((init Arg("verbose")).short("v").long("verbose").flag().about("Включить подробный лог"))
```

Если пользователь передаст `app -q -v` или `app --quiet --verbose`, парсер автоматически выведет:
```text
error: the argument '--quiet' cannot be used with '--verbose'

For more information, try '--help'.
```

#### Сценарий 5: Подкоманды (Git/Cargo-подобная структура)
Каждая подкоманда представляет собой независимый экземпляр `Line` со своим собственным набором аргументов:

```yukino
set cmd_build = (init Line("build", "1.0.0"))
    .about("Собрать проект")
    .flag("release", "r", "release", "Оптимизированная сборка")

set cli = (init Line("mytool", "1.0.0"))
    .subcommand(cmd_build)
```

- Вызов `mytool build --release` успешно активирует подкоманду.
- Вызов `mytool build --help` напечатает справку именно для команды `build`, включая `USAGE: mytool build [FLAGS]`.
- В коде проверка выполняется предельно наглядно:
  ```yukino
  if matches.has_subcommand("build") {
      set sub = matches.get_subcommand_matches("build")
      set is_release = sub.has_flag("release")
  }
  ```

#### Сценарий 6: Проброс «сырых» параметров через разделитель `--`
Если ваша утилита запускает дочерние процессы (как `yuki run` или `cargo run`):
- Вызов: `mytool run -- --custom-flag arg1 arg2`
- Все параметры после `--` не проверяются на совпадение со схемой `mytool` и доступны через `matches.get_rest()`.

#### Сценарий 7: Автоматическая версия пакета через макрос `yukienv!`
Чтобы не менять номер версии вручную в исходном коде при каждом релизе, используйте макрос времени компиляции `yukienv!("bin.version")`:

```yukino
set cli = init Line("mytool", yukienv!("bin.version"))
```
При запуске `mytool -v` или `mytool --version` будет выведена точная версия, указанная в манифесте `yuki.toml`.

---

## 1. Подключение: load std.line

Импорт всего пакета:

```yukino
load std.line
```

При этом становятся доступны следующие сущности:

- **Классы:** `Line` (псевдоним `App`), `Arg`, `Matches`.
- **Функции утилит и подсказок:** `levenshtein`, `find_suggestion`, `render_help`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.line` глобальные константы не объявлены.

---

## 3. Интерфейсы

В модуле `std.line` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

В модуле `std.line` структуры ошибок не объявляются. При обнаружении некорректных аргументов, отсутствующих значений, неизвестных подкоманд или нарушении ограничений взаимного исключения парсер формирует структурированное сообщение об ошибке с подсветкой и нечеткими рекомендациями ("Did you mean...") через модуль `std.outf` и завершает выполнение программы системным вызовом `exit(2)`.

---

## 5. Классы

### 5.1 class Line (псевдоним App)

Основной класс конфигурации приложения и парсера командной строки. Позволяет задавать имя, версию, описание, автора, регистрировать аргументы, подкоманды и управлять поведением справки.

```yukino
public class Line {
    public name: string
    public version: string
    public about_text: string
    public author_text: string
    public parent_chain: string
    public args_list: Arg[]
    public subcommands_list: Line[]
    public help_disabled: bool
    public version_disabled: bool
    public help_override: string
    public version_override: string
}
```

#### 5.1.1 init

Создает новый экземпляр парсера с указанным именем программы и версией. Для автоматического извлечения версии бинарного пакета из `yuki.toml` рекомендуется передавать макрос `yukienv!("bin.version")`.

```yukino
public fun init(name: string, version: string)
```

Возвращает инициализированный объект класса `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line
load std.io: println

fun main() {
    set cli = init Line("mycli", yukienv!("bin.version"))
    println("App: " + cli.name + " v" + cli.version)
}
```

#### 5.1.2 about

Устанавливает краткое описание назначения программы или подкоманды для отображения в заголовке справки.

```yukino
public fun about(text: string) -> Line
```

Возвращает модифицированный текущий экземпляр `Line` для поддержки цепочечных вызовов.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("calc", "1.0.0").about("Простой консольный калькулятор")
}
```

#### 5.1.3 author

Задает имя автора или команды разработчиков для вывода в справочной информации.

```yukino
public fun author(text: string) -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("tool", "1.0.0").author("Yukino Team")
}
```

#### 5.1.4 arg

Регистрирует предварительно сконфигурированный объект аргумента `Arg`.

```yukino
public fun arg(a: Arg) -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line
load std.line.arg: Arg

fun main() {
    set opt_verbose = init Arg("verbose").short("v").long("verbose").flag()
    set cli = init Line("app", "1.0.0").arg(opt_verbose)
}
```

#### 5.1.5 flag

Удобный метод быстрого добавления булевого флага, не принимающего значений.

```yukino
public fun flag(name: string, short: string, about: string) -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").flag("verbose", "v", "Включить подробный вывод")
}
```

#### 5.1.6 opt

Добавляет опциональный именованный параметр, требующий передачи строкового значения.

```yukino
public fun opt(name: string, short: string, about: string) -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").opt("config", "c", "Путь к файлу конфигурации")
}
```

#### 5.1.7 opt_default

Добавляет опциональный параметр со значением по умолчанию, используемым если параметр не был передан пользователем.

```yukino
public fun opt_default(name: string, short: string, about: string, default_val: string) -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("server", "1.0.0").opt_default("port", "p", "Порт прослушивания", "8080")
}
```

#### 5.1.8 opt_required

Добавляет обязательный именованный параметр. Если параметр не указан, парсер завершит программу с сообщением об ошибке.

```yukino
public fun opt_required(name: string, short: string, about: string) -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("db", "1.0.0").opt_required("host", "h", "Адрес сервера базы данных")
}
```

#### 5.1.9 positional

Добавляет опциональный позиционный аргумент. Позиционные аргументы сопоставляются в порядке их регистрации.

```yukino
public fun positional(name: string, about: string) -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("cp", "1.0.0").positional("source", "Исходный файл").positional("dest", "Целевой файл")
}
```

#### 5.1.10 positional_required

Добавляет обязательный позиционный аргумент.

```yukino
public fun positional_required(name: string, about: string) -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("cat", "1.0.0").positional_required("path", "Путь к читаемому файлу")
}
```

#### 5.1.11 subcommand

Регистрирует дочернюю подкоманду `Line`. Подкоманда наследует цепочку вызова родителя и может иметь собственный набор флагов, опций и вложенных подкоманд.

```yukino
public fun subcommand(sub: Line) -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set root = init Line("git", "2.40.0")
    set commit = init Line("commit", "2.40.0").flag("amend", "a", "Изменить предыдущий коммит")
    root.subcommand(commit)
}
```

#### 5.1.12 disable_help

Отключает автоматическую обработку флагов справки `-h` и `--help`.

```yukino
public fun disable_help() -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").disable_help()
}
```

#### 5.1.13 disable_version

Отключает автоматическую обработку флагов версии `-v` и `--version`.

```yukino
public fun disable_version() -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").disable_version()
}
```

#### 5.1.14 override_help

Задает произвольный пользовательский текст для вывода при запросе справки вместо стандартного сгенерированного форматирования.

```yukino
public fun override_help(text: string) -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").override_help("Пользовательская справка программы.")
}
```

#### 5.1.15 override_version

Задает произвольный пользовательский текст для вывода при запросе версии.

```yukino
public fun override_version(text: string) -> Line
```

Возвращает текущий экземпляр `Line`.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").override_version("v1.0.0-custom-build")
}
```

#### 5.1.16 print_help

Выводит сгенерированную или переопределенную справку текущей команды в стандартный поток вывода.

```yukino
public fun print_help()
```

Функция ничего не возвращает.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("demo", "1.0.0").flag("verbose", "v", "Подробный режим")
    cli.print_help()
}
```

#### 5.1.17 print_version

Выводит строку версии команды в формате `<полное_имя> <версия>` или пользовательский текст переопределения в стандартный поток вывода.

```yukino
public fun print_version()
```

Функция ничего не возвращает.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("demo", "1.0.0")
    cli.print_version()
}
```

#### 5.1.18 parse

Выполняет парсинг реальных аргументов командной строки текущего процесса, полученных через системную функцию `args()`. Если переданы флаги `-h`/`--help` или `-v`/`--version`, парсер автоматически выводит соответствующую информацию и завершает процесс с кодом `0`. При ошибках валидации выводит сообщение об ошибке и завершает процесс с кодом `2`.

```yukino
public fun parse() -> Matches
```

Возвращает объект `Matches` с результатами разбора.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("tool", yukienv!("bin.version")).flag("dry-run", "d", "Тестовый прогон")
    set matches = cli.parse()
}
```

#### 5.1.19 parse_from

Выполняет парсинг переданного массива аргументов `input_args`. Используется для модульного тестирования логики командной строки без обращения к реальному окружению процесса.

```yukino
public fun parse_from(input_args: string[]) -> Matches
```

Возвращает объект `Matches` с результатами разбора.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line
load std.io: println

fun main() {
    set cli = init Line("tool", "1.0.0").flag("verbose", "v", "Подробно")
    set matches = cli.parse_from(["-v"])
    println("Verbose: ${matches.get_bool(\"verbose\")}")
}
```

---

### 5.2 class Arg

Класс подробной конфигурации отдельного аргумента, опции или флага в builder-стиле.

```yukino
public class Arg {
    public name: string
    public short_name: string
    public long_name: string
    public about_text: string
    public is_flag: bool
    public is_required: bool
    public is_positional: bool
    public is_multiple: bool
    public default_val: string
    public has_default: bool
    public value_hint: string
    public conflicts: string[]
}
```

#### 5.2.1 init

Создает аргумент с указанным уникальным идентификатором. По умолчанию длинное имя совпадает с `name`.

```yukino
public fun init(name: string)
```

Возвращает экземпляр `Arg`.

Метод никогда не вызывает панику.

```yukino
load std.line.arg: Arg

fun main() {
    set a = init Arg("output")
}
```

#### 5.2.2 short

Назначает односимвольное короткое имя флага или опции (например, `"o"` для `-o`).

```yukino
public fun short(s: string) -> Arg
```

Возвращает модифицированный экземпляр `Arg`.

Метод никогда не вызывает панику.

```yukino
load std.line.arg: Arg

fun main() {
    set a = init Arg("output").short("o")
}
```

#### 5.2.3 long

Назначает длинное имя опции (например, `"output"` для `--output`).

```yukino
public fun long(l: string) -> Arg
```

Возвращает текущий экземпляр `Arg`.

Метод никогда не вызывает панику.

```yukino
load std.line.arg: Arg

fun main() {
    set a = init Arg("out").long("output")
}
```

#### 5.2.4 about

Задает пояснительный текст аргумента для справочной таблицы.

```yukino
public fun about(desc: string) -> Arg
```

Возвращает текущий экземпляр `Arg`.

Метод никогда не вызывает панику.

```yukino
load std.line.arg: Arg

fun main() {
    set a = init Arg("output").about("Путь к выходному файлу")
}
```

#### 5.2.5 flag

Переключает аргумент в режим булевого флага (не требует значения после себя).

```yukino
public fun flag() -> Arg
```

Возвращает текущий экземпляр `Arg`.

Метод никогда не вызывает панику.

```yukino
load std.line.arg: Arg

fun main() {
    set a = init Arg("release").short("r").flag()
}
```

#### 5.2.6 required

Устанавливает флаг обязательности аргумента.

```yukino
public fun required(r: bool) -> Arg
```

Возвращает текущий экземпляр `Arg`.

Метод никогда не вызывает панику.

```yukino
load std.line.arg: Arg

fun main() {
    set a = init Arg("token").required(true)
}
```

#### 5.2.7 positional

Объявляет аргумент позиционным (сопоставляется без префиксов `-` или `--`).

```yukino
public fun positional(p: bool) -> Arg
```

Возвращает текущий экземпляр `Arg`.

Метод никогда не вызывает панику.

```yukino
load std.line.arg: Arg

fun main() {
    set a = init Arg("input").positional(true)
}
```

#### 5.2.8 multiple

Помечает аргумент как допускающий многократную передачу.

```yukino
public fun multiple(m: bool) -> Arg
```

Возвращает текущий экземпляр `Arg`.

Метод никогда не вызывает панику.

```yukino
load std.line.arg: Arg

fun main() {
    set a = init Arg("include").short("I").multiple(true)
}
```

#### 5.2.9 default_value

Задает значение по умолчанию, подставляемое парсером, если аргумент не был передан.

```yukino
public fun default_value(val: string) -> Arg
```

Возвращает текущий экземпляр `Arg`.

Метод никогда не вызывает панику.

```yukino
load std.line.arg: Arg

fun main() {
    set a = init Arg("format").default_value("json")
}
```

#### 5.2.10 value_hint

Задает обозначение типа значения в справке (например, `<PATH>`, `<PORT>`). По умолчанию используется `"VALUE"`.

```yukino
public fun value_hint(hint: string) -> Arg
```

Возвращает текущий экземпляр `Arg`.

Метод никогда не вызывает панику.

```yukino
load std.line.arg: Arg

fun main() {
    set a = init Arg("config").value_hint("FILE")
}
```

#### 5.2.11 conflicts_with

Регистрирует ограничение взаимного исключения: текущий аргумент не может использоваться одновременно с указанным аргументом `other_name`.

```yukino
public fun conflicts_with(other_name: string) -> Arg
```

Возвращает текущий экземпляр `Arg`.

Метод никогда не вызывает панику.

```yukino
load std.line.arg: Arg

fun main() {
    set a = init Arg("quiet").conflicts_with("verbose")
}
```

---

### 5.3 class Matches

Класс результатов разбора аргументов командной строки. Предоставляет типизированные методы проверки наличия и извлечения строковых, целочисленных и вещественных значений параметров.

```yukino
public class Matches {
    public flag_names: string[]
    public opt_names: string[]
    public opt_values: string[]
    public pos_names: string[]
    public pos_values: string[]
    public all_pos: string[]
    public trailing_items: string[]
    public sub_name: string
    public sub_matches: Matches?
}
```

#### 5.3.1 init

Создает пустой объект результатов разбора.

```yukino
public fun init()
```

Возвращает пустой экземпляр `Matches`.

Метод никогда не вызывает панику.

```yukino
load std.line.matches: Matches

fun main() {
    set m = init Matches()
}
```

#### 5.3.2 get_bool

Проверяет, был ли установлен булев флаг с указанным именем.

```yukino
public fun get_bool(name: string) -> bool
```

Возвращает логическое значение `true`, если флаг был передан, иначе `false`. Не является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line
load std.io: println

fun main() {
    set cli = init Line("app", "1.0.0").flag("verbose", "v", "")
    set m = cli.parse_from(["-v"])
    if m.get_bool("verbose") {
        println("Подробный вывод активирован")
    }
}
```

#### 5.3.3 has

Проверяет наличие аргумента с указанным именем (среди флагов, опций или позиционных параметров).

```yukino
public fun has(name: string) -> bool
```

Возвращает `true`, если аргумент присутствует, иначе `false`. Не является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").opt("config", "c", "")
    set m = cli.parse_from(["-c", "app.toml"])
    set has_cfg = m.has("config")
}
```

#### 5.3.4 get

Извлекает строковое значение опции или позиционного аргумента по его имени.

```yukino
public fun get(name: string) -> string?
```

Возвращает строковое значение аргумента либо `null`, если аргумент не был передан. Является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line
load std.io: println

fun main() {
    set cli = init Line("app", "1.0.0").opt("config", "c", "")
    set m = cli.parse_from(["-c", "dev.cfg"])
    set cfg = m.get("config")
    if cfg != null {
        println("Файл: " + (cfg ?? ""))
    }
}
```

#### 5.3.5 get_or

Извлекает строковое значение опции, возвращая резервное значение `fallback`, если опция отсутствует.

```yukino
public fun get_or(name: string, fallback: string) -> string
```

Возвращает строковое значение параметра либо `fallback`. Не является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line
load std.io: println

fun main() {
    set cli = init Line("app", "1.0.0").opt("host", "h", "")
    set m = cli.parse_from([])
    println("Хост: " + m.get_or("host", "localhost"))
}
```

#### 5.3.6 get_int

Извлекает целочисленное 64-битное значение аргумента, выполняя его парсинг.

```yukino
public fun get_int(name: string) -> int64?
```

Возвращает число `int64` либо `null`, если аргумент отсутствует или строка не является корректным целым числом. Является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").opt("port", "p", "")
    set m = cli.parse_from(["-p", "8080"])
    set port = m.get_int("port")
}
```

#### 5.3.7 get_int_or

Извлекает целочисленное значение опции, возвращая резервное число `fallback` при отсутствии или ошибке преобразования.

```yukino
public fun get_int_or(name: string, fallback: int64) -> int64
```

Возвращает число `int64`. Не является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line
load std.io: println

fun main() {
    set cli = init Line("app", "1.0.0").opt("port", "p", "")
    set m = cli.parse_from([])
    println("Порт: ${m.get_int_or(\"port\", 3000)}")
}
```

#### 5.3.8 get_float

Извлекает вещественное 64-битное число с плавающей точкой по имени аргумента.

```yukino
public fun get_float(name: string) -> float64?
```

Возвращает `float64` либо `null` при отсутствии параметра или некорректном формате числа. Является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").opt("rate", "r", "")
    set m = cli.parse_from(["-r", "0.75"])
    set rate = m.get_float("rate")
}
```

#### 5.3.9 get_float_or

Извлекает вещественное число, возвращая `fallback` при его отсутствии или ошибке парсинга.

```yukino
public fun get_float_or(name: string, fallback: float64) -> float64
```

Возвращает число `float64`. Не является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").opt("scale", "s", "")
    set m = cli.parse_from([])
    set s = m.get_float_or("scale", 1.0)
}
```

#### 5.3.10 get_all

Возвращает список всех значений, переданных для указанной опции (например, при повторении флага `-I dir1 -I dir2`).

```yukino
public fun get_all(name: string) -> string[]
```

Возвращает массив строк `string[]`. Если аргумент не передавался, возвращается пустой массив. Не является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("compiler", "1.0.0").opt("include", "I", "")
    set m = cli.parse_from(["-I", "src", "-I", "lib"])
    set includes = m.get_all("include")
}
```

#### 5.3.11 trailing

Возвращает массив аргументов, переданных после разделителя `--`.

```yukino
public fun trailing() -> string[]
```

Возвращает массив строк `string[]`. Не является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line
load std.io: println

fun main() {
    set cli = init Line("runner", "1.0.0")
    set m = cli.parse_from(["--", "-custom", "arg1"])
    println("Хвостовых аргументов: ${m.trailing().len()}")
}
```

#### 5.3.12 raw_positionals

Возвращает полный список всех переданных позиционных значений (включая не сопоставленные с зарегистрированными позиционными аргументами).

```yukino
public fun raw_positionals() -> string[]
```

Возвращает массив строк `string[]`. Не является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set cli = init Line("tool", "1.0.0")
    set m = cli.parse_from(["file1.txt", "file2.txt"])
    set raw = m.raw_positionals()
}
```

#### 5.3.13 subcommand_name

Возвращает имя вызванной подкоманды.

```yukino
public fun subcommand_name() -> string?
```

Возвращает имя подкоманды строкой либо `null`, если ни одна подкоманда не была вызвана. Является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line
load std.io: println

fun main() {
    set root = init Line("git", "2.40.0")
    root.subcommand(init Line("status", "2.40.0"))
    set m = root.parse_from(["status"])
    println("Команда: " + (m.subcommand_name() ?? "нет"))
}
```

#### 5.3.14 subcommand_matches

Возвращает объект `Matches` с результатами разбора параметров выполненной подкоманды.

```yukino
public fun subcommand_matches() -> Matches?
```

Возвращает объект `Matches` либо `null`, если подкоманда не вызывалась. Является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line
load std.io: println

fun main() {
    set root = init Line("git", "2.40.0")
    root.subcommand(init Line("commit", "2.40.0").opt("message", "m", ""))
    set m = root.parse_from(["commit", "-m", "init"])
    set sub_m = m.subcommand_matches()
    if sub_m != null {
        set s = sub_m ?? m
        println("Сообщение: " + s.get_or("message", ""))
    }
}
```

#### 5.3.15 has_subcommand

Проверяет, была ли вызвана подкоманда с указанным именем `name`.

```yukino
public fun has_subcommand(name: string) -> bool
```

Возвращает `true`, если активирована подкоманда с заданным именем, иначе `false`. Не является nullable.

Метод никогда не вызывает панику.

```yukino
load std.line.line: Line

fun main() {
    set root = init Line("git", "2.40.0")
    root.subcommand(init Line("push", "2.40.0"))
    set m = root.parse_from(["push"])
    set is_push = m.has_subcommand("push")
}
```

---

## 6. Справочник функций

### 6.1 levenshtein

Вычисляет расстояние Левенштейна (минимальное количество односимвольных операций вставки, удаления или замены) между строками `a` и `b`. Используется алгоритмом нечеткого поиска опечаток для рекомендаций "Did you mean...".

```yukino
public fun levenshtein(a: string, b: string) -> int64
```

Возвращает целое неотрицательное число `int64`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.line.suggest: levenshtein
load std.io: println

fun main() {
    set w1 = "commit"
    set w2 = "comit"
    println("Дистанция: ${levenshtein(w1, w2)}")
}
```

### 6.2 find_suggestion

Выполняет поиск наиболее похожей строки среди списка кандидатов `candidates` для ошибочно введенной строки `target`. Подсказка возвращается только в том случае, если дистанция Левенштейна меньше или равна допустимому порогу опечатки (3 операции).

```yukino
public fun find_suggestion(target: string, candidates: string[]) -> string?
```

Возвращает строку наиболее подходящего кандидата либо `null`, если близких совпадений не обнаружено. Является nullable.

Функция никогда не вызывает панику.

```yukino
load std.line.suggest: find_suggestion
load std.io: println

fun main() {
    set typo = "--verison"
    set cands: string[] = ["--help", "--version", "--verbose"]
    set sug = find_suggestion(typo, cands)
    if sug != null {
        println("Возможно вы имели в виду: " + (sug ?? ""))
    }
}
```

### 6.3 render_help

Низкоуровневая функция форматирования справочной информации команды. Формирует выровненные по колонкам секции `USAGE:`, `FLAGS:`, `OPTIONS:`, `ARGS:` и `SUBCOMMANDS:`, подсвечивая заголовки и ключевые слова стилями `std.outf`.

```yukino
public fun render_help(
    name: string,
    version: string,
    about: string,
    author: string,
    parent_chain: string,
    args: Arg[],
    sub_names: string[],
    sub_abouts: string[],
    disable_help: bool,
    disable_version: bool,
    allow_short_h: bool,
    allow_long_help: bool,
    allow_short_v: bool,
    allow_long_version: bool
) -> string
```

Возвращает отформатированную строку справки типа `string`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.line.help: render_help
load std.line.arg: Arg
load std.io: println

fun main() {
    set args_list: Arg[] = []
    set sub_names: string[] = []
    set sub_abouts: string[] = []
    set text = render_help(
        "mycli", "1.0.0", "Пример приложения", "Author", "",
        args_list, sub_names, sub_abouts,
        false, false, true, true, true, true
    )
    println(text)
}
```

---

[← Назад: Модуль std.outf](22-outf.md) | [Вперед: Модуль std.itermtool →](24-itermtool.md)
