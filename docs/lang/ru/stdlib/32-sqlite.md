# Модуль std.sqlite

Модуль `std.sqlite` предоставляет полнофункциональные и высокопроизводительные средства для работы со встраиваемой реляционной базой данных SQLite 3. Модуль включает в себя управление соединениями (в файле или в оперативной памяти), подготовленные выражения (prepared statements), параметризованные запросы с защитой от SQL-инъекций, транзакции, а также безопасную навигацию по результатам выборок через Fluent API классов `SqliteRow` и `SqliteValue`, спроектированных по аналогии с модулями `std.json` и `std.jsonc`.

---

## 1. Подключение: load std.sqlite

Импорт всего пакета:

```yukino
load std.sqlite
```

При этом становятся доступны следующие сущности:

- **Константы:** `SQLITE_OK`, `SQLITE_ERROR`, `SQLITE_ROW`, `SQLITE_DONE`, `SQLITE_INTEGER`, `SQLITE_FLOAT`, `SQLITE_TEXT`, `SQLITE_BLOB`, `SQLITE_NULL`, `SQLITE_OPEN_READONLY`, `SQLITE_OPEN_READWRITE`, `SQLITE_OPEN_CREATE`, `SQLITE_OPEN_MEMORY`.
- **Структуры ошибок:** `SqliteError`.
- **Классы:** `SqliteConnection`, `SqliteStatement`, `SqliteRow`, `SqliteValue`.
- **Функции управления соединениями:** `open`, `try_open`, `open_memory`, `open_with_flags`, `try_open_with_flags`, `sqlite_open`, `sqlite_try_open`, `sqlite_open_memory`, `sqlite_version`.
- **Фабричные функции конструирования значений:** `sqlite_null`, `sqlite_int`, `sqlite_float`, `sqlite_string`, `sqlite_text`, `sqlite_blob`, `sqlite_bool`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.sqlite` объявлены целочисленные константы кодов результатов, типов данных и флагов открытия базы данных:

```yukino
public static SQLITE_OK: int32 = 0
public static SQLITE_ERROR: int32 = 1
public static SQLITE_ROW: int32 = 100
public static SQLITE_DONE: int32 = 101

public static SQLITE_INTEGER: int32 = 1
public static SQLITE_FLOAT: int32 = 2
public static SQLITE_TEXT: int32 = 3
public static SQLITE_BLOB: int32 = 4
public static SQLITE_NULL: int32 = 5

public static SQLITE_OPEN_READONLY: int32 = 1
public static SQLITE_OPEN_READWRITE: int32 = 2
public static SQLITE_OPEN_CREATE: int32 = 4
public static SQLITE_OPEN_MEMORY: int32 = 128
```

Константы типов данных соответствуют типам SQLite и используются для инспекции свойства `kind` в классе `SqliteValue`.

---

## 3. Интерфейсы

В модуле `std.sqlite` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 SqliteError

Структура `SqliteError` представляет ошибку выполнения SQL-запроса, сбоя подготовки выражения, нарушения ограничений целостности или ошибки файлового ввода-вывода базы данных.

```yukino
public struct SqliteError {
    message: string
    code: int32
}
```

- `message`: текстовое описание ошибки, возвращенное ядром SQLite или рантаймом Yukino.
- `code`: целочисленный код ошибки SQLite (например, `1` для синтаксической ошибки, `19` для `SQLITE_CONSTRAINT`).

Пример безопасного перехвата ошибки через блок `sus / catch`:

```yukino
load std.sqlite: open, SqliteConnection, SqliteError
load std.io: println

fun main() {
    set db = open(":memory:")
    sus {
        set _ = db.execute("SELECT * FROM table_does_not_exist;")
    } catch err: SqliteError {
        println("Перехвачена ошибка SQLite: " + err.message + " (код: " + cast(err.code, "string") + ")")
    }
    db.close()
}
```

---

## 5. Классы

### 5.1 SqliteValue

Класс `SqliteValue` представляет типизированное значение ячейки базы данных (null, целое число, число с плавающей точкой, текст или бинарный BLOB). По своему публичному API класс спроектирован полностью идентично классу `JsonValue` из модуля `std.json`, что позволяет разработчику использовать привычные методы извлечения данных и безопасные fallback-значения.

```yukino
public class SqliteValue {
    public kind: int32
    public int_val: int64
    public float_val: float64
    public str_val: string
    public blob_val: uint8[]
}
```

#### Методы проверки типа (предикаты)

- `is_null() -> bool`: возвращает `true`, если значение является `NULL`.
- `is_int() -> bool`: возвращает `true`, если значение имеет целочисленный тип.
- `is_float() -> bool`: возвращает `true`, если значение имеет тип числа с плавающей точкой.
- `is_number() -> bool`: возвращает `true`, если значение является числовым (целым или с плавающей точкой).
- `is_string() -> bool` (а также `is_text() -> bool`): возвращает `true`, если значение является текстовой строкой.
- `is_blob() -> bool`: возвращает `true`, если значение является бинарным массивом байтов.

#### Методы извлечения значений

- `as_int() -> int64`: возвращает целочисленное значение. Если тип ячейки вещественный, выполняет приведение к `int64`. Для прочих типов возвращает `0`.
- `as_int_or(fallback: int64) -> int64`: возвращает целое число либо значение `fallback`, если ячейка не содержит число.
- `as_float() -> float64`: возвращает число с плавающей точкой. Если ячейка целочисленная, выполняет приведение к `float64`. Для остальных типов возвращает `0.0`.
- `as_float_or(fallback: float64) -> float64`: возвращает вещественное число либо `fallback`.
- `as_string() -> string`: возвращает строковое представление ячейки. Для чисел выполняет автоматическое форматирование в строку, для прочих нетекстовых типов возвращает пустую строку `""`.
- `as_string_or(fallback: string) -> string`: возвращает строку либо `fallback`.
- `as_bool() -> bool`: возвращает логическое значение. Число `0` и пустая строка интерпретируются как `false`, ненулевые числа и непустые строки — как `true`. Для `NULL` возвращает `false`.
- `as_bool_or(fallback: bool) -> bool`: возвращает логическое значение либо `fallback`.
- `as_blob() -> uint8[]`: возвращает массив байтов типа `uint8[]`. Если ячейка не является BLOB, возвращает пустой массив.

---

### 5.2 SqliteRow

Класс `SqliteRow` инкапсулирует отдельную строку результирующего набора выборки. Доступ к значениям колонок реализован по принципу Fluent API без риска паники: при обращении к несуществующей колонке или выходе индекса за границы возвращается безопасный узел `SqliteValue` типа `NULL`.

```yukino
public class SqliteRow {
    public col_names: string[]
    public col_vals: SqliteValue[]
}
```

#### Методы доступа к данным строки

- `get(key: string) -> SqliteValue`: возвращает значение колонки по ее имени `key`. Если колонка отсутствует, возвращает узел `SqliteValue` типа `NULL`. Это позволяет строить непрерывные цепочки вида `row.get("age").as_int_or(18)`.
- `get_opt(key: string) -> SqliteValue?`: возвращает nullable-значение `SqliteValue?` либо `null`, если колонка с таким именем отсутствует в выборке.
- `at(index: int64) -> SqliteValue`: возвращает значение колонки по ее числовому индексу (начиная с 0). Если индекс выходит за границы, возвращает безопасный узел типа `NULL`.
- `at_opt(index: int64) -> SqliteValue?`: возвращает nullable-значение по индексу либо `null` при выходе за пределы диапазона колонок.
- `has(key: string) -> bool`: возвращает `true`, если строка содержит колонку с именем `key`.
- `keys() -> string[]`: возвращает массив названий всех колонок строки.
- `values() -> SqliteValue[]`: возвращает массив всех значений колонок строки в исходном порядке.
- `len() -> int64`: возвращает количество колонок в строке.

---

### 5.3 SqliteStatement

Класс `SqliteStatement` инкапсулирует скомпилированное подготовленное выражение SQLite. Позволяет связывать параметры, пошагово итерировать строки и повторно использовать запрос.

```yukino
public class SqliteStatement {
    public db_handle: ptr<void>
    public handle: ptr<void>
    public is_finalized: bool
}
```

#### Методы подготовленного выражения

- `step() -> bool`: выполняет переход к следующей строке выборки. Возвращает `true`, если доступна очередная строка данных, или `false`, если выполнение завершено. В случае ошибки выполнения генерирует панику со структурой `SqliteError`.
- `current_row() -> SqliteRow`: извлекает данные текущей строки выборки и возвращает объект `SqliteRow`. Не nullable.
- `reset()`: сбрасывает состояние выражения для повторного выполнения с сохранением привязанных параметров.
- `finalize()`: освобождает дескриптор подготовленного выражения в памяти.
- `bind_null(index: int32)`: привязывает значение `NULL` к параметру по 1-based индексу `index`.
- `bind_int(index: int32, val: int64)`: привязывает 64-битное целое число к параметру `index`.
- `bind_float(index: int32, val: float64)`: привязывает вещественное число к параметру `index`.
- `bind_string(index: int32, val: string)`: привязывает текстовую строку к параметру `index`.
- `bind_blob(index: int32, val: uint8[])`: привязывает массив байтов к параметру `index`.
- `bind_value(index: int32, val: SqliteValue)`: привязывает динамическое значение `SqliteValue` в соответствии с его типом.
- `bind_params(params: SqliteValue[])`: последовательно привязывает массив параметров к позиционным маркерам `?` (начиная с индекса 1).

---

### 5.4 SqliteConnection

Класс `SqliteConnection` представляет активное соединение с базой данных SQLite. Предоставляет методы выполнения команд DDL/DML, параметризованных выборок, управления транзакциями и проверки метаданных.

```yukino
public class SqliteConnection {
    public handle: ptr<void>
    public path: string
    public is_closed: bool
}
```

#### Методы соединения

- `execute(sql: string) -> int64`: выполняет команду изменения данных или структуры (например, `CREATE TABLE`, `INSERT`, `UPDATE`, `DELETE`) и возвращает количество затронутых строк. В случае ошибки синтаксиса или ограничений вызывает панику с `SqliteError`.
- `execute_params(sql: string, params: SqliteValue[]) -> int64`: выполняет параметризованный запрос изменения данных с безопасным связыванием переданных параметров.
- `query(sql: string) -> SqliteRow[]`: выполняет запрос на выборку данных (`SELECT`) и возвращает массив всех найденных строк `SqliteRow[]`. В случае ошибки запроса вызывает панику с `SqliteError`.
- `query_params(sql: string, params: SqliteValue[]) -> SqliteRow[]`: выполняет параметризованную выборку данных с массивом значений параметров.
- `query_row(sql: string) -> SqliteRow`: возвращает первую строку выборки. Если выборка пуста, вызывает панику со структурой `SqliteError` (код 101).
- `query_row_opt(sql: string) -> SqliteRow?`: возвращает первую строку выборки либо `null`, если по запросу не найдено ни одной строки. Не вызывает панику при пустом результате.
- `prepare(sql: string) -> SqliteStatement`: компилирует SQL-запрос в подготовленное выражение `SqliteStatement`.
- `changes() -> int64`: возвращает количество строк, измененных последней операцией `INSERT`, `UPDATE` или `DELETE`.
- `last_insert_rowid() -> int64`: возвращает идентификатор `ROWID` последней успешно вставленной строки.
- `begin()`: начинает явную транзакцию (`BEGIN`).
- `commit()`: фиксирует текущую транзакцию (`COMMIT`).
- `rollback()`: откатывает текущую транзакцию (`ROLLBACK`).
- `close()`: закрывает соединение и освобождает связанные системные ресурсы.

---

## 6. Справочник функций

### 6.1 open

Открывает базу данных SQLite по указанному пути к файлу либо создает новую, если файл отсутствует. Для создания базы данных исключительно в оперативной памяти в качестве пути можно передать `":memory:"`.

```yukino
public fun open(path: string) -> SqliteConnection
```

Возвращает экземпляр `SqliteConnection`. Возвращаемое значение не nullable.

В случае невозможности открыть файл базы данных или некорректных прав доступа вызывает панику со структурой `SqliteError`.

```yukino
load std.sqlite: open, SqliteConnection
load std.io: println

fun main() {
    set db = open("app.db")
    println("База данных успешно открыта")
    db.close()
}
```

### 6.2 try_open

Выполняет безопасную попытку открытия базы данных SQLite без генерации паники при возникновении ошибок.

```yukino
public fun try_open(path: string) -> SqliteConnection?
```

Возвращает экземпляр `SqliteConnection` в случае успешного подключения либо `null`, если файл не может быть открыт. Возвращаемое значение nullable.

Функция никогда не вызывает панику.

```yukino
load std.sqlite: try_open, SqliteConnection
load std.io: println

fun main() {
    set maybe_db = try_open("invalid/path/db.sqlite")
    if maybe_db == null {
        println("Не удалось открыть базу данных по указанному пути")
    }
}
```

### 6.3 open_memory

Создает и открывает изолированную базу данных SQLite в оперативной памяти (`:memory:`).

```yukino
public fun open_memory() -> SqliteConnection
```

Возвращает экземпляр `SqliteConnection`. Возвращаемое значение не nullable.

В случае критической нехватки системной памяти вызывает панику со структурой `SqliteError`.

```yukino
load std.sqlite: open_memory, SqliteConnection
load std.io: println

fun main() {
    set mem_db = open_memory()
    set _ = mem_db.execute("CREATE TABLE config (key TEXT, val TEXT);")
    println("Таблица в оперативной памяти создана")
    mem_db.close()
}
```

### 6.4 open_with_flags

Открывает базу данных SQLite с явно заданными низкоуровневыми битовыми флагами (например, комбинацией `SQLITE_OPEN_READONLY` или `SQLITE_OPEN_READWRITE`).

```yukino
public fun open_with_flags(path: string, flags: int32) -> SqliteConnection
```

Возвращает экземпляр `SqliteConnection`. Возвращаемое значение не nullable.

При невозможности открыть базу данных с запрошенными флагами вызывает панику со структурой `SqliteError`.

```yukino
load std.sqlite: open_with_flags, SQLITE_OPEN_READONLY, SqliteConnection
load std.io: println

fun main() {
    set db = open_with_flags("readonly.db", SQLITE_OPEN_READONLY)
    println("База данных открыта в режиме только для чтения")
    db.close()
}
```

### 6.5 try_open_with_flags

Выполняет безопасную попытку открытия базы данных с заданными флагами без генерации паники.

```yukino
public fun try_open_with_flags(path: string, flags: int32) -> SqliteConnection?
```

Возвращает экземпляр `SqliteConnection` при успехе либо `null` при ошибке. Возвращаемое значение nullable.

Функция никогда не вызывает панику.

```yukino
load std.sqlite: try_open_with_flags, SQLITE_OPEN_READONLY, SqliteConnection
load std.io: println

fun main() {
    set db = try_open_with_flags("missing.db", SQLITE_OPEN_READONLY)
    if db == null {
        println("База данных для чтения недоступна")
    }
}
```

### 6.6 sqlite_open, sqlite_try_open, sqlite_open_memory

Префиксные псевдонимы функций управления соединениями. Они полностью эквивалентны функциям `open`, `try_open` и `open_memory` и предотвращают коллизии имен в областях видимости, где одновременно импортированы модули файлового ввода-вывода или сетевые клиенты.

```yukino
public fun sqlite_open(path: string) -> SqliteConnection
public fun sqlite_try_open(path: string) -> SqliteConnection?
public fun sqlite_open_memory() -> SqliteConnection
```

Поведение возвращаемых значений и паники полностью идентично базовым функциям `open`, `try_open` и `open_memory`.

```yukino
load std.sqlite: sqlite_open_memory, SqliteConnection
load std.io: println

fun main() {
    set db = sqlite_open_memory()
    println("База данных открыта через префиксную функцию")
    db.close()
}
```

### 6.7 sqlite_version

Возвращает текстовую строку версии встроенного движка SQLite (например, `"3.53.2"`).

```yukino
public fun sqlite_version() -> string
```

Возвращает строку типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.sqlite: sqlite_version
load std.io: println

fun main() {
    println("Версия SQLite: " + sqlite_version())
}
```

### 6.8 Фабричные функции конструирования SqliteValue

Модуль предоставляет набор вспомогательных функций для создания параметров запросов:

- `sqlite_null() -> SqliteValue`: создает значение типа `NULL`.
- `sqlite_int(val: int64) -> SqliteValue`: создает целочисленное значение.
- `sqlite_float(val: float64) -> SqliteValue`: создает вещественное число.
- `sqlite_string(val: string) -> SqliteValue` (а также `sqlite_text`): создает текстовое значение.
- `sqlite_blob(val: uint8[]) -> SqliteValue`: создает бинарное значение BLOB.
- `sqlite_bool(val: bool) -> SqliteValue`: создает целочисленное значение `1` (`true`) или `0` (`false`).

Все фабричные функции возвращают объект `SqliteValue` и никогда не вызывают панику.

```yukino
load std.sqlite: open_memory, sqlite_int, sqlite_string, sqlite_float, SqliteValue, SqliteConnection
load std.io: println

fun main() {
    set db = open_memory()
    set _ = db.execute("CREATE TABLE products (id INTEGER, name TEXT, price REAL);")
    
    set params: SqliteValue[] = [sqlite_int(101), sqlite_string("Клавиатура"), sqlite_float(49.99)]
    set _ = db.execute_params("INSERT INTO products VALUES (?, ?, ?);", params)
    
    set row = db.query_row("SELECT name, price FROM products WHERE id = 101;")
    println("Товар: " + row.get("name").as_string() + ", Цена: " + cast(row.get("price").as_float(), "string"))
    db.close()
}
```

---

[← Назад: Модуль std.yaml](31-yaml.md) | [Вперед: Модуль std.encore →](33-encore.md)
