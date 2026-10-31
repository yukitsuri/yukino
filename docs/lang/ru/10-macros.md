# Макросистема и встроенные макросы

Макросы в Yukino раскрываются на этапе синтаксического анализа до проверки типов и генерации LLVM IR. Макросистема поддерживает как пользовательские гигиенические макросы, объявляемые через `define`, так и набор встроенных макросов компилятора.

---

## Оглавление

1. [Пользовательские макросы: декларация define](#1-пользовательские-макросы-декларация-define)  
   1.1 [Макросы-выражения (Expression Macros)](#11-макросы-выражения-expression-macros)  
   1.2 [Макросы-блоки (Block Macros)](#12-макросы-блоки-block-macros)  
   1.3 [Гигиена макросов (Macro Hygiene)](#13-гигиена-макросов-macro-hygiene)  
   1.4 [Экспорт и видимость макросов (public define)](#14-экспорт-и-видимость-макросов-public-define)
2. [Встроенные макросы компилятора](#2-встроенные-макросы-компилятора)  
   2.1 [Местоположение в исходном коде: file!, line!, col!](#21-местоположение-в-исходном-коде-file-line-col)  
   2.2 [Окружение сборки: hostenv!, get_target!, get_yukic_version!](#22-окружение-сборки-hostenv-get_target-get_yukic_version)  
   2.3 [Конфигурация и манифест: yukienv!, get_profileconfig!, get_features!, is_lib_connected!](#23-конфигурация-и-манифест-yukienv-get_profileconfig-get_features-is_lib_connected)  
   2.4 [Встраивание бинарных и текстовых ресурсов: include_str!, include_bytes!](#24-встраивание-бинарных-и-текстовых-ресурсов-include_str-include_bytes)  
   2.5 [Статическая ошибка компиляции: compile_error!](#25-статическая-ошибка-компиляции-compile_error)  
   2.6 [Проверка конфигурации времени компиляции: cfg!](#26-проверка-конфигурации-времени-компиляции-cfg)  
   2.7 [Вывод в консоль времени компиляции: compile_print!](#27-вывод-в-консоль-времени-компиляции-compile_print)
3. [Пакеты процедурных макросов и Macro SDK](#3-пакеты-процедурных-макросов-и-macro-sdk)  
   3.1 [Что такое процедурные макросы и зачем они нужны](#31-что-такое-процедурные-макросы-и-зачем-они-нужны)  
   3.2 [Официальный Macro SDK (macrosdk)](#32-официальный-macro-sdk-macrosdk)  
   3.3 [Основные сущности SDK: TokenStream, MacroContext, MacroRunner](#33-основные-сущности-sdk-tokenstream-macrocontext-macrorunner)  
   3.4 [Пошаговое создание макроса: yukino new --macro](#34-пошаговое-создание-макроса-yukino-new---macro)  
   3.5 [Подключение и вызов макроса в проекте-потребителе](#35-подключение-и-вызов-макроса-в-проекте-потребителе)  
   3.6 [Архитектура исполнения, безопасность IPC и таймауты](#36-архитектура-исполнения-безопасность-ipc-и-таймауты)  
   3.7 [Отслеживание внешних файлов (track_file) и инкрементальный кэш CGU](#37-отслеживание-внешних-файлов-track_file-и-инкрементальный-кэш-cgu)  
4. [Справочник API пакета macrosdk](#4-справочник-api-пакета-macrosdk)  
   4.1 [Подключение: load macrosdk](#41-подключение-load-macrosdk)  
   4.2 [Константы](#42-константы)  
   4.3 [Типы](#43-типы)  
   4.4 [Структуры данных](#44-структуры-данных)  
   4.5 [Классы](#45-классы)  
   4.6 [Справочник функций и методов](#46-справочник-функций-и-методов)

---

## 1. Пользовательские макросы: декларация define

Пользовательские макросы объявляются ключевым словом `define` на верхнем уровне модуля. Вызов любого макроса в коде всегда оканчивается восклицательным знаком: `macro_name!(...)`.

### 1.1 Макросы-выражения (Expression Macros)

Макрос-выражение подставляет AST-дерево выражения вместо места вызова:

```yukino
// Объявление через стрелку '=>'
define square(x) => x * x
define cube(x) => x * x * x

fun calculate() -> int32 {
    // Раскрывается на этапе AST в: 5 * 5
    set a = square!(5)
    
    // Раскрывается в: (a + 1) * (a + 1) * (a + 1)
    set b = cube!(a + 1)
    return b
}
```

### 1.2 Макросы-блоки (Block Macros)

Если тело макроса заключено в фигурные скобки `{}`, оно раскрывается в последовательность инструкций (statement splicing) непосредственно в текущую область видимости:

```yukino
define init_point_2d(target_x, target_y) {
    set target_x = 100
    set target_y = 200
}

fun run() {
    set x = 0
    set y = 0
    init_point_2d!(x, y)
    // x и y инициализированы
}
```

### 1.3 Гигиена макросов (Macro Hygiene)

Макросистема Yukino является **гигиеничной**: любые внутренние переменные, создаваемые внутри тела макроса, автоматически переименовываются компилятором (gensym) во избежание непреднамеренного захвата или конфликта с переменными в вызывающем коде:

```yukino
define execute_logged(action) {
    // Внутренняя переменная 'temp_state' автоматически изолируется (например, temp_state_g1)
    set temp_state = 1
    action
}

fun main() {
    set temp_state = 999
    execute_logged!(print("Action done\n"))
    // Локальная переменная temp_state вызывающей функции гарантированно сохраняет значение 999
}
```

### 1.4 Экспорт и видимость макросов (public define)

По умолчанию макрос виден только внутри своего файла `.ykn`. Чтобы сделать макрос доступным для других модулей через `load`, используется модификатор `public`:

```yukino
// math_macros.ykn
public define max_of(a, b) => a > b ? a : b

// main.ykn
load .math_macros: max_of

fun main() {
    set m = max_of!(10, 20)
}
```

---

## 2. Встроенные макросы компилятора

Встроенные макросы обрабатываются компилятором `yukic` напрямую и предоставляют доступ к метаданным компиляции и окружению хоста.

### 2.1 Местоположение в исходном коде: file!, line!, col!

Возвращают строковые и числовые литералы текущей позиции:

```yukino
set current_source: string = file!() // Например, "src/main.ykn"
set current_line: int32 = line!()    // Номер строки (1-based)
set current_column: int32 = col!()   // Номер колонки (1-based)

fun log_error(msg: string) {
    print("${file!()}:${line!()}: ${msg}\n")
}
```

### 2.2 Окружение сборки: hostenv!, get_target!, get_yukic_version!

- `hostenv!("VARIABLE_NAME")`: считывает значение переменной окружения хоста в момент компиляции и встраивает его как строковый литерал:
  ```yukino
  static BUILD_USER: string = hostenv!("USER")
  static CI_COMMIT: string = hostenv!("GIT_COMMIT")
  ```
  Если переменная окружения не задана на хосте, компилятор выдает ошибку.

- `get_target!()`: возвращает Target Triple целевой платформы (например, `"x86_64-unknown-linux-gnu"`):
  ```yukino
  static TARGET_ARCH: string = get_target!()
  ```

- `get_yukic_version!()`: возвращает строку версии компилятора `yukic`:
  ```yukino
  static COMPILER_VER: string = get_yukic_version!()
  ```

### 2.3 Конфигурация и манифест: yukienv!, get_profileconfig!, get_features!, is_lib_connected!

- `yukienv!("key", [scope: int32])`: считывает строковое значение любого параметра или вложенного ключа из конфигурационных файлов TOML проекта в момент компиляции. Вторым опциональным параметром можно передать `int32` область видимости (`scope`):
  - `1` (по умолчанию, если параметр опущен) — проверяет собственный конфиг файл текущего пакета/библиотеки (`yuki.toml` или `lib.toml`);
  - `2` — проверяет собственный воркспейс текущего пакета (`yukispace.toml`);
  - `3` — проверяет воркспейс хост приложения (`yukispace.toml` верхнеуровневого приложения; для самого хост приложения эквивалентно `2`);
  - `4` — проверяет конфиг файл хост приложения (`yuki.toml` / `lib.toml` верхнеуровневого приложения; для самого хост приложения эквивалентно `1`);
  - `else` (любое другое число) — проверяет собственный конфиг файл (`1`).

  ```yukino
  // Чтение из своего конфиг файла (scope 1)
  static APP_NAME: string = yukienv!("bin.name")
  static APP_VER: string = yukienv!("bin.version", 1)

  // Чтение из воркспейса хост приложения (scope 3)
  static WS_NAME: string = yukienv!("workspace.name", 3)

  // Чтение из манифеста хост приложения (scope 4)
  static HOST_APP_NAME: string = yukienv!("bin.name", 4)
  ```
  Если запрашиваемый ключ отсутствует в указанном манифесте или манифест не найден, компиляция прерывается ошибкой `E0306`.

- `get_profileconfig!()`: вызывается без аргументов и возвращает имя активного профиля текущей сборки (`"debug"`, `"release"` или пользовательский профиль из `config.*`):
  ```yukino
  static CURRENT_PROFILE: string = get_profileconfig!()
  ```

- `get_features!()`: вызывается без аргументов и возвращает массив строк `string[]`, содержащий список всех включенных фич текущей сборки:
  ```yukino
  static ACTIVE_FEATURES: string[] = get_features!()
  ```

- `is_lib_connected!("lib_name", [scope: int32])`: возвращает булево значение (`true` / `false`), проверяя, подключена ли указанная библиотека (например, `"std"`) к проекту в заданной области конфигурации:
  - `1` (по умолчанию) — проверяет собственный конфиг файл (`yuki.toml` / `lib.toml`);
  - `2` — проверяет собственный воркспейс (`yukispace.toml`);
  - `3` — проверяет воркспейс хост приложения;
  - `4` — проверяет конфиг файл хост приложения;
  - `else` — проверяет собственный конфиг файл (`1`).

  ```yukino
  // Проверка в своем манифесте
  static HAS_STD: bool = is_lib_connected!("std")

  // Зависимость проверяет, подключена ли библиотека в хост приложении
  static HOST_HAS_CRYPTO: bool = is_lib_connected!("crypto", 4)
  ```

### 2.4 Встраивание бинарных и текстовых ресурсов: include_str!, include_bytes!

Позволяет упаковать внешние файлы напрямую в итоговый бинарный файл:

- `include_str!("relative/path.txt")` (также доступны псевдонимы `include_string!` и `include_strings!`): считывает файл в кодировке UTF-8 и встраивает его как константную строку `string`:
  ```yukino
  static SHADER_SRC: string = include_str!("shaders/vertex.glsl")
  ```

- `include_bytes!("relative/path.bin")`: считывает файл как сырую последовательность байт и встраивает его как `bytes`:
  ```yukino
  static FONT_DATA: bytes = include_bytes!("assets/font.ttf")
  ```

Относительный путь вычисляется относительно расположения конфигурационного файла (`yuki.toml`, `lib.toml` или `yukispace.toml`), в контексте которого вызывается макрос (если проект компилируется без манифеста, путь резолвится относительно директории исходного файла).

### 2.5 Статическая ошибка компиляции: compile_error!

Принудительно прерывает компиляцию с указанным сообщением:

```yukino
// Проверка условий сборки
define assert_supported_target() {
    compile_error!("This architecture is not currently supported")
}
```

### 2.6 Проверка конфигурации времени компиляции: cfg!

Макрос `cfg!(...)` вычисляет предикат условной компиляции на этапе синтаксического анализа и возвращает константное булево значение (`true` или `false`):

```yukino
fun connect() {
    if cfg!(feature = "ssl") {
        print("Connecting via TLS\n")
    } else {
        print("Connecting via plaintext\n")
    }
}
```

Поддерживаются произвольные предикаты с логическими операторами `all(...)`, `any(...)`, `not(...)`:
- `feature = "name"` — проверка активности фичи проекта.
- `target_os = "linux"` — проверка операционной системы (`"linux"`, `"windows"`, `"macos"`, `"android"`).
- `target_arch = "x86_64"` — проверка архитектуры процессора (`"x86_64"`, `"aarch64"`, `"arm"`).
- `target_env = "gnu"` — проверка среды исполнения и libc (`"gnu"`, `"musl"`, `"msvc"`).

Пример сложного предиката:

```yukino
if cfg!(all(target_os = "linux", not(feature = "embedded"))) {
    init_epoll_engine()
}
```

Благодаря тому, что `cfg!(...)` раскрывается напрямую в булев литерал на этапе компиляции, оптимизатор мертвого кода (`yukic-dce`) полностью устраняет недостижимые ветви ветвления, гарантируя нулевые накладные расходы в рантайме.

### 2.7 Вывод в консоль времени компиляции: compile_print!

Макрос `compile_print!(...)` выводит переданный текст напрямую в стандартный поток вывода (`stdout`) процесса компилятора в момент сборки проекта и сразу выполняет принудительный сброс буфера (`flush`):

```yukino
// Вывод информационного сообщения при компиляции:
compile_print!("compiling graphics module for target: ${get_target!()}\n")

// Использование внутри выражений (возвращает сгенерированную строку):
static BUILD_BANNER: string = compile_print!("Yukino App v1.0 [profile: ${get_profileconfig!()}]\n")
```

Особенности:
- **Автофлуш**: текст сбрасывается в `stdout` терминала немедленно, без ожидания завершения процесса или заполнения буфера строк.
- **Интерполяция строк**: поддерживает интерполированные строки `${...}` со встроенными константами окружения (`target`, `version`, `profile`, ключи `yukienv`) и вызовами других встроенных макросов (`${get_target!()}`, `${file!()}`, `${line!()}`).
- **Без неявных переводов строк**: макрос выводит ровно ту строку, которая ему передана. Если требуется перевод строки, добавьте `\n` в конце.
- **Возвращаемое значение**: раскрывается в константный строковый литерал `string`, что позволяет использовать макрос как самостоятельную инструкцию, так и в инициализаторах констант и переменных.

---

## 3. Пакеты процедурных макросов и Macro SDK

### 3.1 Что такое процедурные макросы и зачем они нужны

Обычный код программы выполняется во время работы приложения (в рантайме), когда скомпилированный бинарный файл запущен пользователем.

**Процедурный макрос** — это специальная программа на языке Yukino, которая запускается **во время сборки проекта (на этапе компиляции)**. Макрос принимает фрагмент исходного кода в виде последовательности токенов, выполняет произвольные вычисления (например, читает файлы конфигурации или схемы баз данных) и генерирует новый исходный код, который компилятор тут же подставляет на место вызова макроса.

Зачем нужны процедурные макросы:
1. **Избавление от шаблонного кода (Boilerplate):** если вам нужно вручную объявить десятки однотипных структур данных, классов или функций, макрос сделает это автоматически по описанию или списку параметров.
2. **Кодогенерация на основе внешних данных:** макрос может прочитать файл `schema.json` или таблицу БД во время сборки и создать строго типизированные структуры прямо в вашем коде.
3. **Проверки и валидация на этапе компиляции:** если входные данные для макроса некорректны, он может сразу выдать понятную ошибку компиляции с указанием строки и файла.

В отличие от простых синтаксических макросов `define`, процедурные макросы имеют доступ ко всей стандартной библиотеке `std` (файловая система, разбор JSON/TOML, криптография, сетевые утилиты), что делает их полноценным инструментом метапрограммирования.

### 3.2 Официальный Macro SDK (macrosdk)

Для удобной разработки процедурных макросов Yukino предоставляет официальный встроенный пакет — **Macro SDK (`macrosdk`)**.

Macro SDK изолирован от обычных приложений и подключается на уровне манифеста пакета через массив `includes` в `lib.toml`:

```toml
[lib]
name = "my_codegen"
version = "0.1.0"
entry = "my_codegen/lib.ykn"
output = "macro"
includes = ["stdlib", "macrosdk"]
```

При подключении `"macrosdk"` в манифесте компилятор автоматически связывает код с хост-библиотекой SDK, открывая доступ к модулю `macrosdk`.

### 3.3 Основные сущности SDK: TokenStream, MacroContext, MacroRunner

Пакет `macrosdk` предоставляет три главных инструмента разработчика:

#### 1. TokenStream — поток токенов и кода
Класс `TokenStream` представляет фрагмент исходного кода. Он позволяет инкапсулировать входные аргументы макроса, объединять несколько частей кода и отдавать итоговый результат компилятору:
- `token_stream(code: string) -> TokenStream` — функция-фабрика, создающая поток токенов из переданной строки с кодом.
- `.to_string() -> string` — возвращает текстовое представление кода потока.
- `.is_empty() -> bool` — проверяет, пуст ли поток.
- `.append(other: TokenStream) -> TokenStream` — присоединяет другой поток токенов с новой строки.
- `.append_str(snippet: string) -> TokenStream` — присоединяет строковый фрагмент кода.

#### 2. MacroContext — контекст вызова макроса
Класс `MacroContext` передается компилятором в функцию макроса при каждом вызове. Он предоставляет метаданные окружения и методы взаимодействия с процессом компиляции:
- `ctx.macro_name: string` — имя вызванного макроса.
- `ctx.working_dir: string` — абсолютный путь к рабочей директории проекта.
- `ctx.kind: string` — категория макроса (`"FunctionLike"`, `"Attribute"` или `"Mutation"`).
- `ctx.attr_args: string` — строка аргументов, переданная атрибуту макроса (например, `"GET, \"/users\""`).
- `ctx.is_attribute() -> bool` — проверяет, вызван ли макрос как атрибут `#[...]`.
- `ctx.is_mutation() -> bool` — проверяет, вызван ли макрос как мутация типа `#[mutation(...)]`.
- `ctx.is_function_like() -> bool` — проверяет, вызван ли макрос функционально: `name!(...)`.
- `ctx.track_file(path: string) -> MacroContext` — регистрирует внешний файл, прочитанный макросом (например, схему JSON). При изменении этого файла компилятор автоматически перекомпилирует проект.
- `ctx.error(msg: string) -> MacroContext` — генерирует ошибку компиляции.
- `ctx.error_at(span: Span, msg: string) -> MacroContext` — генерирует ошибку с точной привязкой к месту в исходном коде.
- `ctx.warning(msg: string) -> MacroContext` — генерирует предупреждение компилятора.
- `ctx.warning_at(span: Span, msg: string) -> MacroContext` — генерирует предупреждение с привязкой к исходному коду.

#### 3. MacroRunner — диспетчер и сервер макросов
Класс `MacroRunner` управляет жизненным циклом процесса макроса. Он ожидает запросы от компилятора через межпроцессный канал (IPC), находит зарегистрированный обработчик и возвращает сгенерированный код:
- `runner.register(macro_name: string, handler: MacroFn) -> MacroRunner` — регистрирует универсальный обработчик макроса.
- `runner.register_attribute(name: string, handler: MacroFn) -> MacroRunner` — семантическая регистрация макроса-атрибута.
- `runner.register_mutation(name: string, handler: MacroFn) -> MacroRunner` — семантическая регистрация макроса мутации типа.
- `runner.run() -> int32` — запускает бесконечный цикл обработки запросов от компилятора до завершения сборки.

Сигнатура функции-обработчика макроса определяется типом:
```yukino
public type MacroFn = fun(MacroContext, TokenStream) -> TokenStream
```

### 3.4 Пошаговое создание макроса: yukino new --macro

Чтобы создать новый пакет процедурного макроса, выполните команду с флагом `--macro`:

```sh
yukino new my_codegen --macro
```

Сборщик Yukino создаст структуру проекта:
```text
my_codegen/
├── lib.toml
└── my_codegen/
    └── lib.ykn
```

Файл `lib.toml` содержит конфигурацию библиотеки с типом артефакта `output = "macro"`:
```toml
[lib]
name = "my_codegen"
version = "0.1.0"
entry = "my_codegen/lib.ykn"
yukino = ["0.1.0", "+"]
output = "macro"
includes = ["stdlib", "macrosdk"]
```

Файл `my_codegen/lib.ykn` содержит начальный код раннера. Давайте напишем реальный макрос `generate_user_model`, который принимает имя пользователя и генерирует готовую структуру:

```yukino
// my_codegen/my_codegen/lib.ykn
load macrosdk: MacroRunner, MacroContext, TokenStream, token_stream

fun expand_user_model(ref ctx: MacroContext, input: TokenStream) -> TokenStream {
    set type_name = input.to_string()
    if type_name.len() == 0 {
        ctx.error("generate_user_model requires a struct name")
        return token_stream("")
    }

    // Сообщаем разработчику предупреждение при сборке
    ctx.warning("generating data model for: ${type_name}")

    // Генерируем новый исходный код
    set code = "struct ${type_name} {\n    id: int64\n    name: string\n    active: bool\n}"
    return token_stream(code)
}

fun main() {
    set runner = init MacroRunner()
    set _ = runner.register("generate_user_model", expand_user_model)
    set _ = runner.run()
}
```

### 3.5 Подключение и вызов макроса в проекте-потребителе

Чтобы использовать созданный макрос в приложении, подключите его в манифесте вашего проекта `app.toml` в секции `[libs.macro]`:

```toml
# my_app/app.toml
[bin]
name = "my_app"
version = "0.1.0"
entry = "src/main.ykn"

[libs.macro.my_codegen]
path = "../my_codegen"
```

Теперь в коде приложения вы можете вызвать макрос точно так же, как обычный встроенный макрос, добавив восклицательный знак к имени:

```yukino
// my_app/src/main.ykn
load std.io: println

// Вызов процедурного макроса на этапе компиляции:
// Компилятор запустит my_codegen и заменит этот вызов на сгенерированную структуру!
generate_user_model!(UserAccount)

fun main() {
    // Структура UserAccount уже существует в коде программы:
    set user = UserAccount {
        id: 42,
        name: "Alice",
        active: true,
    }

    println("User ID: ${user.id}, Name: ${user.name}")
}
```

При выполнении команды `yukino build` или `yukino run`:
1. Сборщик `yukino` видит зависимость в `[libs.macro]`.
2. Пакет `my_codegen` автоматически компилируется под архитектуру текущего компьютера (хост).
3. При компиляции файла `src/main.ykn` компилятор встречает `generate_user_model!(UserAccount)`.
4. Компилятор отправляет токен `UserAccount` процессу `my_codegen`.
5. Функция `expand_user_model` генерирует структуру `struct UserAccount { ... }`.
6. Компилятор встраивает структуру в AST программы и завершает бинарную сборку приложения.

### 3.6 Архитектура исполнения, безопасность IPC и таймауты

Yukino сознательно отказывается от выполнения макросов через динамические библиотеки (`.so` / `.dll`), загружаемые напрямую в память компилятора.

Вместо этого используется модель **изолированного процесса (IPC Runner)**:

```text
[ Компилятор yukic ]
        │
   (IPC: stdin)  ──>  [ 4-байтный префикс длины ] + [ JSON MacroRequest ]
        │
        ▼
[ Бинарник макроса (Хост-процесс) ]  <──> Полный доступ к std (std.fs, std.net)
        │
   (IPC: stdout) ──>  [ 4-байтный префикс длины ] + [ JSON MacroResponse ]
        │
        ▼
[ Сплайсинг в AST компилятора ] (Выражения или стейтменты)
```

Преимущества архитектуры:
- **Полная изоляция сбоев (Crash Guard):** если в макросе возникла ошибка или паника `panic(...)`, компилятор не упадет. Он перехватит `stderr` упавшего процесса и покажет аккуратное диагностическое сообщение с указанием места вызова.
- **Защита от зависаний (Timeout):** выполнение макроса защищено ограничением по времени (по умолчанию 10 секунд). Этот лимит управляется параметром конфигурации `macrotimeout` в `yukino config` (например, `yukino config set macrotimeout 20`). Если в макросе написан бесконечный цикл или вычисление превысило таймаут, процесс будет прерван, а компилятор сообщит об ошибке превышения времени.
- **Поддержка кросс-компиляции:** если вы компилируете проект под Android или WebAssembly, макрос всё равно автоматически компилируется под процессор вашего ПК (хост), выполняя генерацию без проблем.

### 3.7 Отслеживание внешних файлов (track_file) и инкрементальный кэш CGU

Если макрос считывает внешние файлы с диска (например, `ctx.track_file("schema.json")`), компилятор Yukino автоматически вычисляет хеш содержимого каждого отслеживаемого файла и сохраняет его в ключе кэша сборки (CGU Cache).

Это дает ключевое преимущество:
- Если вы не меняли файл `schema.json`, компилятор повторно использует кэш и собирает проект за доли секунды.
- Как только вы отредактировали `schema.json`, компилятор видит изменение хеша и автоматически перезапускает макрос, обновляя сгенерированный код без необходимости вызывать `yukino clean`.

---

## 4. Справочник API пакета macrosdk

Встроенный пакет `macrosdk` предоставляет инструменты для разработки процедурных макросов времени компиляции на языке Yukino. Пакет включает примитивы для работы с потоком токенов (`TokenStream`), контекст вызова компилятора (`MacroContext`) и сервер обработки IPC-запросов (`MacroRunner`).

---

### 4.1 Подключение: load macrosdk

Импорт всего пакета:

```yukino
load macrosdk
```

В область видимости импортируются:

- **Константы:**
  - В пакете `macrosdk` глобальные константы не объявлены.
- **Типы:**
  - `MacroFn` — тип функции-обработчика процедурного макроса (`fun(MacroContext, TokenStream) -> TokenStream`).
- **Структуры данных:**
  - `Span` — координаты расположения токена в исходном коде (`file_id`, `start`, `end`, `line`, `col`);
  - `Ident` — идентификатор имени с координатами `Span`;
  - `Literal` — литерал с исходным строковым представлением и координатами `Span`;
  - `Punct` — символ пунктуации или оператор с координатами `Span`;
  - `Diagnostic` — запись диагностического сообщения компилятора (ошибка или предупреждение).
- **Классы:**
  - `TokenStream` — поток токенов и фрагментов кода, передаваемый между компилятором и макросом;
  - `MacroContext` — контекст выполнения вызова макроса, регистрация зависимостей внешних файлов и генерация диагностик;
  - `MacroRunner` — хост-сервер макроса, принимающий IPC-запросы компилятора и управляющий вызовом зарегистрированных функций.
- **Функции и фабрики:**
  - `span_default` — создает экземпляр структуры `Span` с координатами по умолчанию;
  - `token_stream` — создает экземпляр класса `TokenStream` из переданной строки исходного кода.

Для подключения пакета `macrosdk` библиотека макроса объявляет его в манифесте `lib.toml` в секции `includes`:

```toml
[build]
includes = [
    "stdlib",
    "macrosdk",
]
```

---

### 4.2 Константы

В модуле `macrosdk` глобальные константы не объявлены.

---

### 4.3 Типы

#### 4.3.1 MacroFn

Псевдоним функционального типа обработчика процедурного макроса.

```yukino
public type MacroFn = fun(MacroContext, TokenStream) -> TokenStream
```

Принимает ссылку на текущий контекст вызова `MacroContext` и входной поток токенов переданных аргументов `TokenStream`. Возвращает новый поток токенов `TokenStream` со сгенерированным кодом, который компилятор встраивает в место вызова макроса.

---

### 4.4 Структуры данных

#### 4.4.1 Span

Представляет координаты и диапазон символов токена или синтаксического узла в исходном коде.

```yukino
public struct Span {
    file_id: uint32
    start: uint32
    end: uint32
    line: uint32
    col: uint32
}
```

- `file_id` — целочисленный идентификатор файла в карте исходных кодов компилятора (`SourceMap`);
- `start` — байтовое смещение начала токена от начала файла;
- `end` — байтовое смещение конца токена от начала файла;
- `line` — номер строки в файле (начиная с 1);
- `col` — номер столбца в строке (начиная с 1).

#### 4.4.2 Ident

Представляет токен идентификатора языка (имя переменной, функции, структуры или ключевого слова).

```yukino
public struct Ident {
    name: string
    span: Span
}
```

- `name` — строковое имя идентификатора;
- `span` — координаты расположения токена в исходном коде.

#### 4.4.3 Literal

Представляет токен литерала (числовой, строковый, логический или байтовый литерал).

```yukino
public struct Literal {
    raw: string
    span: Span
}
```

- `raw` — сырое строковое представление литерала в точности так, как он записан в исходном файле;
- `span` — координаты расположения литерала в исходном коде.

#### 4.4.4 Punct

Представляет токен знака пунктуации или оператора (`+`, `-`, `*`, `::`, `->` и др.).

```yukino
public struct Punct {
    ch: string
    span: Span
}
```

- `ch` — строковый символ или последовательность символов пунктуации;
- `span` — координаты расположения знака в исходном коде.

#### 4.4.5 Diagnostic

Представляет запись компиляторной диагностики (ошибки или предупреждения), отправляемой макросом компилятору.

```yukino
public struct Diagnostic {
    severity: string
    message: string
    file_id: uint32
    line: uint32
    col: uint32
}
```

- `severity` — уровень важности сообщения (`"Error"` или `"Warning"`);
- `message` — понятный текст сообщения об ошибке или предупреждении;
- `file_id` — идентификатор файла исходного кода (0, если координаты не указаны);
- `line` — номер строки в файле (0, если координаты не указаны);
- `col` — номер столбца в строке (0, если координаты не указаны).

---

### 4.5 Классы

#### 4.5.1 class TokenStream

Класс инкапсулирует поток токенов и исходного кода, передаваемый в процедурный макрос и возвращаемый им в компилятор.

```yukino
public class TokenStream {
    code: string

    public fun init(code: string)
    public fun to_string() -> string
    public fun is_empty() -> bool
    public fun append(other: TokenStream) -> TokenStream
    public fun append_str(snippet: string) -> TokenStream
}
```

Поле `code` хранит текстовое представление кода потока токенов.

#### 4.5.2 class MacroContext

Класс предоставляет контекст выполнения текущего вызова макроса, доступ к рабочему каталогу проекта, регистрацию внешних файлов для инкрементального кэша и генерацию диагностических сообщений.

```yukino
public class MacroContext {
    public tracked_files: string[]
    public diagnostics: Diagnostic[]
    public macro_name: string
    public working_dir: string
    public kind: string
    public attr_args: string

    public fun init(
        macro_name: string,
        working_dir: string,
        kind: string = "FunctionLike",
        attr_args: string = ""
    )
    public fun is_attribute() -> bool
    public fun is_mutation() -> bool
    public fun is_function_like() -> bool
    public fun track_file(path: string) -> MacroContext
    public fun error(msg: string) -> MacroContext
    public fun error_at(span: Span, msg: string) -> MacroContext
    public fun warning(msg: string) -> MacroContext
    public fun warning_at(span: Span, msg: string) -> MacroContext
}
```

- `tracked_files` — массив путей к внешним файлам, прочитанным макросом;
- `diagnostics` — массив диагностических сообщений, накопленных в ходе работы макроса;
- `macro_name` — имя вызванного макроса;
- `working_dir` — абсолютный путь к корневому рабочему каталогу собираемого проекта;
- `kind` — строковое наименование категории макроса (`"FunctionLike"`, `"Attribute"`, `"Mutation"`);
- `attr_args` — строковые аргументы атрибута (если макрос вызван как атрибут).

#### 4.5.3 class MacroRunner

Класс реализует диспетчер и сервер макросов хост-процесса. Принимает JSON-запросы компилятора через `stdin`, вызывает зарегистрированный обработчик и возвращает результат в `stdout`.

```yukino
public class MacroRunner {
    names: string[]
    handlers: MacroFn[]

    public fun init()
    public fun register(name: string, handler: MacroFn) -> MacroRunner
    public fun register_attribute(name: string, handler: MacroFn) -> MacroRunner
    public fun register_mutation(name: string, handler: MacroFn) -> MacroRunner
    public fun run() -> int32
}
```

- `names` — массив строковых имен зарегистрированных макросов;
- `handlers` — массив соответствующих функций-обработчиков `MacroFn`.

---

### 4.6 Справочник функций и методов

#### 4.6.1 span_default

Создает экземпляр структуры `Span` с координатами по умолчанию: `file_id = 0`, смещения `start = 0`, `end = 0`, строка `line = 1`, колонка `col = 1`. Используется при генерации синтетических токенов, для которых отсутствуют точные координаты в пользовательском файле.

```yukino
fun span_default() -> Span
```

Возвращает экземпляр структуры `Span`. Значение не может быть `null`.

Функция никогда не вызывает панику.

```yukino
load macrosdk.tokens: Span, span_default
load std.io: println

set sp: Span = span_default()
println("Строка: ${sp.line}, Колонка: ${sp.col}")
```

#### 4.6.2 token_stream

Функция-фабрика, создающая новый экземпляр класса `TokenStream` на основе переданной строки с фрагментом исходного кода.

```yukino
fun token_stream(code: string) -> TokenStream
```

Возвращает новый экземпляр класса `TokenStream`. Возвращаемый экземпляр никогда не является `null`.

Функция никогда не вызывает панику.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load std.io: println

set stream: TokenStream = token_stream("println(\"hello world\")")
println("Код потока: ${stream.to_string()}")
```

#### 4.6.3 TokenStream.init

Конструктор класса `TokenStream`. Инициализирует внутренний строковый буфер кода переданным фрагментом `code`.

```yukino
fun init(code: string)
```

Конструктор инициализирует экземпляр класса `TokenStream` и не возвращает значения.

Конструктор никогда не вызывает панику.

```yukino
load macrosdk.tokens: TokenStream
load std.io: println

set stream = init TokenStream("set x: int32 = 42")
println("Код: ${stream.to_string()}")
```

#### 4.6.4 TokenStream.to_string

Преобразует поток токенов в строковое представление исходного кода для компилятора или отладочного вывода.

```yukino
fun to_string() -> string
```

Возвращает строковое значение типа `string`. Возвращаемая строка не может быть `null`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load std.io: println

set stream: TokenStream = token_stream("fun add(a: int32, b: int32) -> int32 { return a + b }")
set code: string = stream.to_string()
println("Длина кода: ${code.len()}")
```

#### 4.6.5 TokenStream.is_empty

Проверяет, содержит ли поток токенов пустую строку исходного кода.

```yukino
fun is_empty() -> bool
```

Возвращает логическое значение типа `bool`. Возвращает `true`, если длина внутреннего строкового буфера равна 0, иначе `false`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load std.io: println

set empty_stream: TokenStream = token_stream("")
set non_empty: TokenStream = token_stream("42")

println("Пустой поток: ${empty_stream.is_empty()}")
println("Непустой поток: ${non_empty.is_empty()}")
```

#### 4.6.6 TokenStream.append

Присоединяет содержимое другого потока токенов `other` к текущему потоку. Если текущий поток не пуст, добавление выполняется с новой строки (`\n`), формируя многострочный блок сгенерированного кода.

```yukino
fun append(other: TokenStream) -> TokenStream
```

Возвращает ссылку на текущий экземпляр `TokenStream` для поддержки цепочечных вызовов методов. Возвращаемое значение не может быть `null`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load std.io: println

set part1: TokenStream = token_stream("set x: int32 = 10")
set part2: TokenStream = token_stream("set y: int32 = 20")

part1.append(part2)
println(part1.to_string())
```

#### 4.6.7 TokenStream.append_str

Присоединяет строковый фрагмент кода `snippet` к текущему потоку токенов. Если текущий поток не пуст, переданный фрагмент добавляется с новой строки.

```yukino
fun append_str(snippet: string) -> TokenStream
```

Возвращает ссылку на текущий экземпляр `TokenStream` для поддержки цепочечных вызовов методов. Возвращаемое значение не может быть `null`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load std.io: println

set stream: TokenStream = token_stream("public struct Config {")
stream.append_str("    host: string")
stream.append_str("    port: int32")
stream.append_str("}")

println(stream.to_string())
```

#### 4.6.8 MacroContext.init

Конструктор класса `MacroContext`. Создает новый контекст вызова с указанным строковым именем макроса `macro_name`, абсолютным путем к рабочему каталогу проекта `working_dir`, категорией вызова `kind` (`"FunctionLike"`, `"Attribute"`, `"Mutation"`) и строкой аргументов атрибута `attr_args`. Списки отслеживаемых файлов `tracked_files` и диагностических сообщений `diagnostics` инициализируются пустыми массивами.

```yukino
fun init(
    macro_name: string,
    working_dir: string,
    kind: string = "FunctionLike",
    attr_args: string = ""
)
```

Конструктор инициализирует экземпляр `MacroContext` и не возвращает значения.

Конструктор никогда не вызывает панику.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("my_macro", "/workspace", "Attribute", "GET, \"/users\"")
println("Макрос: ${ctx.macro_name}, Категория: ${ctx.kind}, Аргументы: ${ctx.attr_args}")
```

#### 4.6.9 MacroContext.is_attribute

Проверяет, был ли макрос вызван в качестве атрибута-трансформера над объявлением элемента (`#[macro_name(...)]`).

```yukino
fun is_attribute() -> bool
```

Возвращает `true`, если категория вызова равна `"Attribute"`, иначе `false`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("logged", "/workspace", "Attribute", "")
println("Это атрибут: ${ctx.is_attribute()}")
```

#### 4.6.10 MacroContext.is_mutation

Проверяет, был ли макрос вызван в качестве макроса мутации типа через `#[mutation(macro_name)]`.

```yukino
fun is_mutation() -> bool
```

Возвращает `true`, если категория вызова равна `"Mutation"`, иначе `false`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("JsonModel", "/workspace", "Mutation", "")
println("Это мутация типа: ${ctx.is_mutation()}")
```

#### 4.6.11 MacroContext.is_function_like

Проверяет, был ли макрос вызван в функциональной форме: `macro_name!(...)`.

```yukino
fun is_function_like() -> bool
```

Возвращает `true`, если категория вызова равна `"FunctionLike"`, иначе `false`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("ping", "/workspace", "FunctionLike", "")
println("Это функциональный макрос: ${ctx.is_function_like()}")
```

#### 4.6.12 MacroContext.track_file

Регистрирует внешний файл по указанному относительному или абсолютному пути `path` как зависимость сборки. Компилятор автоматически вычисляет хеш содержимого каждого отслеживаемого файла и перезапускает макрос при любых изменениях этого файла на диске.

```yukino
fun track_file(path: string) -> MacroContext
```

Возвращает ссылку на текущий экземпляр `MacroContext` для поддержки цепочечных вызовов. Возвращаемое значение не может быть `null`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("schema_macro", "/workspace")
ctx.track_file("models/user_schema.json")

println("Количество зависимостей: ${ctx.tracked_files.len()}")
```

#### 4.6.13 MacroContext.error

Регистрирует ошибку компиляции общего характера без привязки к конкретным координатам в исходном коде (`file_id = 0`, `line = 0`, `col = 0`). Наличие хотя бы одной ошибки с уровнем `"Error"` прерывает компиляцию проекта и выводит сообщение в консоль пользователя.

```yukino
fun error(msg: string) -> MacroContext
```

Возвращает ссылку на текущий экземпляр `MacroContext` для поддержки цепочечных вызовов. Возвращаемое значение не может быть `null`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("validate_macro", "/workspace")
ctx.error("Файл конфигурации schema.json не найден")

println("Зарегистрировано диагностик: ${ctx.diagnostics.len()}")
```

#### 4.6.14 MacroContext.error_at

Регистрирует ошибку компиляции с точной привязкой к координатам исходного кода `span`. Компилятор Yukino подсветит соответствующее место в файле пользователя и выведет переданный текст ошибки.

```yukino
fun error_at(span: Span, msg: string) -> MacroContext
```

Возвращает ссылку на текущий экземпляр `MacroContext`. Возвращаемое значение не может быть `null`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.tokens: Span, span_default
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("syntax_macro", "/workspace")
set sp: Span = span_default()
ctx.error_at(sp, "Ожидалось строковое имя модели данных")

println("Уровень ошибки: ${ctx.diagnostics[0].severity}")
```

#### 4.6.15 MacroContext.warning

Регистрирует предупреждение компилятора (`"Warning"`) общего характера без привязки к конкретному месту в файле. Предупреждение выводится компилятором в терминал, но не прерывает процесс сборки проекта.

```yukino
fun warning(msg: string) -> MacroContext
```

Возвращает ссылку на текущий экземпляр `MacroContext` для поддержки цепочечных вызовов. Возвращаемое значение не может быть `null`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("lint_macro", "/workspace")
ctx.warning("Поле 'legacy_token' устарело и будет удалено в следующей версии")

println("Сообщение предупреждения: ${ctx.diagnostics[0].message}")
```

#### 4.6.16 MacroContext.warning_at

Регистрирует предупреждение компилятора с точной привязкой к координатам `span` в файле пользователя.

```yukino
fun warning_at(span: Span, msg: string) -> MacroContext
```

Возвращает ссылку на текущий экземпляр `MacroContext`. Возвращаемое значение не может быть `null`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.tokens: Span, span_default
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("lint_macro", "/workspace")
set sp: Span = span_default()
ctx.warning_at(sp, "Использование устаревшего типа поля")

println("Количество предупреждений: ${ctx.diagnostics.len()}")
```

#### 4.6.17 MacroRunner.init

Конструктор класса `MacroRunner`. Создает новый экземпляр сервера макросов с пустыми списками зарегистрированных имен макросов и обработчиков.

```yukino
fun init()
```

Конструктор инициализирует экземпляр `MacroRunner` и не возвращает значения.

Конструктор никогда не вызывает панику.

```yukino
load macrosdk.runner: MacroRunner
load std.io: println

set runner = init MacroRunner()
println("MacroRunner успешно инициализирован")
```

#### 4.6.18 MacroRunner.register

Регистрирует функцию-обработчик `handler` под указанным строковым именем макроса `name`. При встрече макроса с этим именем в исходном коде компилятор обратится к зарегистрированной функции.

```yukino
fun register(name: string, handler: MacroFn) -> MacroRunner
```

Возвращает ссылку на текущий экземпляр `MacroRunner` для поддержки цепочечных вызовов методов. Возвращаемое значение не может быть `null`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load macrosdk.context: MacroContext
load macrosdk.runner: MacroRunner
load std.io: println

fun my_handler(ctx: MacroContext, in_stream: TokenStream) -> TokenStream {
    return token_stream("println(\"вызов макроса\")")
}

set runner = init MacroRunner()
runner.register("my_macro", my_handler)
println("Обработчик успешно зарегистрирован")
```

#### 4.6.19 MacroRunner.register_attribute

Регистрирует функцию-обработчик макроса-атрибута `handler` под указанным именем `name`. При декорировании элементов языка (`fun`, `struct`, `class`, `enum`) данным атрибутом `#[name(...)]`, компилятор передает исходный код декорируемого элемента в качестве `input_tokens` и строку параметров атрибута в `ctx.attr_args`.

```yukino
fun register_attribute(name: string, handler: MacroFn) -> MacroRunner
```

Возвращает ссылку на текущий экземпляр `MacroRunner` для поддержки цепочечных вызовов методов. Возвращаемое значение не может быть `null`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load macrosdk.context: MacroContext
load macrosdk.runner: MacroRunner
load std.io: println

fun expand_trace(ref ctx: MacroContext, item_stream: TokenStream) -> TokenStream {
    return token_stream("public fun logged_worker() -> int32 { return 100 }")
}

set runner = init MacroRunner()
runner.register_attribute("trace", expand_trace)
println("Атрибут-макрос успешно зарегистрирован")
```

#### 4.6.20 MacroRunner.register_mutation

Регистрирует функцию-обработчик макроса мутации типа `handler` под указанным именем `name`. При аннотации структуры, класса или перечисления атрибутом `#[mutation(name)]`, компилятор передает декларацию типа в `input_tokens`, а сгенерированный поток кода добавляет в качестве сопутствующих элементов модуля (методов, сериализаторов, конвертеров).

```yukino
fun register_mutation(name: string, handler: MacroFn) -> MacroRunner
```

Возвращает ссылку на текущий экземпляр `MacroRunner` для поддержки цепочечных вызовов методов. Возвращаемое значение не может быть `null`.

Метод никогда не вызывает панику.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load macrosdk.context: MacroContext
load macrosdk.runner: MacroRunner
load std.io: println

fun expand_to_string(ref ctx: MacroContext, type_decl: TokenStream) -> TokenStream {
    return token_stream("public fun user_to_string(u: User) -> string { return \"User\" }")
}

set runner = init MacroRunner()
runner.register_mutation("ToString", expand_to_string)
println("Мутация типа успешно зарегистрирована")
```

#### 4.6.21 MacroRunner.run

Запускает непрерывный цикл ожидания и обработки межпроцессных запросов компилятора (IPC) через стандартные потоки `stdin` и `stdout`. Для каждого входящего запроса `MacroRunner` находит зарегистрированную функцию по имени, вызывает ее, сериализует сгенерированный поток токенов, список отслеживаемых файлов и диагностик в ответный пакет. Завершает работу и возвращает код выхода `0` при получении сигнала завершения (EOF) от компилятора.

```yukino
fun run() -> int32
```

Возвращает целочисленный код завершения процесса типа `int32` (`0` в случае штатного завершения сессии компиляции).

Метод никогда не вызывает панику; все внутренние исключения перехватываются и передаются компилятору в виде протокольных сообщений об ошибках.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load macrosdk.context: MacroContext
load macrosdk.runner: MacroRunner

fun expand_ping(ref ctx: MacroContext, in_stream: TokenStream) -> TokenStream {
    return token_stream("println(\"pong\")")
}

fun main() -> int32 {
    set runner = init MacroRunner()
    runner.register("ping", expand_ping)
    return runner.run()
}
```

---

[← Назад: Модули, импорты и видимость](9-modules.md) | [Вперед: Системный C ABI и FFI →](11-cabi.md)
