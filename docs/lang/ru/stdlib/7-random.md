# Модуль std.random

Модуль `std.random` предоставляет генераторы псевдослучайных чисел (PRNG), алгоритм SplitMix64, детерминированные изолированные генераторы с заданным сидом, глобальные генераторы случайных величин, выборки, распределения Гаусса и генерацию UUID версии 4.

---

## 1. Подключение: load std.random

Импорт всего пакета:

```yukino
load std.random
```

В область видимости импортируются:

- **Классы и структуры:**
  - `class Rng` — объектно-ориентированный генератор на базе SplitMix64;
  - `struct RandomError` — структура ошибки при некорректных аргументах генерации.
- **Фабрики экземпляров Rng:**
  - `rng_new` — создает генератор, инициализированный системной энтропией;
  - `rng_with_seed` — создает детерминированный генератор с заданным фиксированным сидом `uint64`.
- **Глобальные функции генерации:**
  - Примитивы: `rand_u64`, `rand_u32`, `rand_i64`, `rand_i32`, `rand_int`, `rand_bool`, `rand_float`, `rand_float32`;
  - Диапазоны: `rand_range`, `rand_int`;
  - Утилиты: `rand_choice_char`, `rand_uuid_v4`, `rand_gaussian`;
  - Системная энтропия: `rand_bytes`, `rand_seed`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Структуры и классы

### 2.1 RandomError

Структура ошибки, вызываемая через `panic` при передаче некорректных аргументов в функции генерации.

```yukino
public struct RandomError {
    message: string
    code: int32
}
```

- `code = 1` — некорректный диапазон в `rand_int` и в методе `range_int` класса `Rng` (`min > max`);
- `code = 2` — некорректный диапазон в `rand_range` и в методе `range_float` класса `Rng` (`min > max`);
- `code = 3` — пустая строка вариантов в `rand_choice_char`.

### 2.2 class Rng

Экземплярный генератор псевдослучайных чисел. Реализует алгоритм SplitMix64 с периодом $2^{64}$. Сохраняет собственное изолированное состояние, обеспечивая полную детерминированность и воспроизводимость.

```yukino
public class Rng {
    state: uint64

    fun init(seed: uint64)
    fun next_u64() -> uint64
    fun next_u32() -> uint32
    fun next_i64() -> int64
    fun next_i32() -> int32
    fun next_bool() -> bool
    fun next_float() -> float64
    fun next_float32() -> float32
    fun range_int(min: int64, max: int64) -> int64
    fun range_float(min: float64, max: float64) -> float64
    fun fill_bytes(buf: ptr<uint8>, len: int64)
}
```

Конструктор `init(seed: uint64)` задает начальное состояние генератора; сид `0` автоматически заменяется фиксированным значением по умолчанию `0x853c49e6748fea9b`.

---

## 3. Методы класса Rng

### 3.1 next_u64

Генерирует следующее 64-битное беззнаковое псевдослучайное число.

```yukino
fun next_u64() -> uint64
```

Возвращает псевдослучайное значение типа `uint64`.

Функция никогда не паникует.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set val = rng.next_u64()
println("Случайное uint64: ${val}")
```

### 3.2 next_u32

Генерирует следующее 32-битное беззнаковое число (старшие 32 бита результата `next_u64`).

```yukino
fun next_u32() -> uint32
```

Возвращает псевдослучайное значение типа `uint32`.

Функция никогда не паникует.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set val = rng.next_u32()
println("Случайное uint32: ${val}")
```

### 3.3 next_i64

Генерирует следующее 64-битное знаковое псевдослучайное число.

```yukino
fun next_i64() -> int64
```

Возвращает псевдослучайное значение типа `int64`.

Функция никогда не паникует.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set val = rng.next_i64()
println("Случайное int64: ${val}")
```

### 3.4 next_i32

Генерирует следующее 32-битное знаковое псевдослучайное число.

```yukino
fun next_i32() -> int32
```

Возвращает псевдослучайное значение типа `int32`.

Функция никогда не паникует.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set val = rng.next_i32()
println("Случайное int32: ${val}")
```

### 3.5 next_bool

Генерирует псевдослучайное булево значение с вероятностью 50%.

```yukino
fun next_bool() -> bool
```

Возвращает значение типа `bool` (`true` или `false`).

Функция никогда не паникует.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set flag = rng.next_bool()
println("Булево значение: ${flag}")
```

### 3.6 next_float

Генерирует вещественное число с плавающей точкой в полуинтервале `[0.0, 1.0)`.

```yukino
fun next_float() -> float64
```

Возвращает число типа `float64` в диапазоне от 0.0 (включительно) до 1.0 (не включая).

Функция никогда не паникует.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set prob = rng.next_float()
println("Вероятность: ${prob}")
```

### 3.7 next_float32

Генерирует 32-битное число с плавающей точкой в полуинтервале `[0.0, 1.0)`.

```yukino
fun next_float32() -> float32
```

Возвращает число типа `float32` в диапазоне от 0.0 (включительно) до 1.0 (не включая).

Функция никогда не паникует.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set val = rng.next_float32()
println("float32: ${val}")
```

### 3.8 range_int

Генерирует равномерно распределенное целое число в закрытом отрезке `[min, max]`.

```yukino
fun range_int(min: int64, max: int64) -> int64
```

Возвращает случайное целое число `int64` в диапазоне от `min` до `max` включительно.

В случае если `min > max`, вызывает панику со структурой `RandomError` (`code = 1`).

```yukino
load std.random: rng_new, RandomError
load std.io: println

set rng = rng_new()
sus {
    set roll = rng.range_int(1, 6)
    println("Бросок кубика: ${roll}")
} catch err: RandomError {
    println("Ошибка диапазона: ${err.message}")
}
```

### 3.9 range_float

Генерирует равномерно распределенное вещественное число в полуинтервале `[min, max)`.

```yukino
fun range_float(min: float64, max: float64) -> float64
```

Возвращает число `float64` в полуинтервале `[min, max)`.

В случае если `min > max`, вызывает панику со структурой `RandomError` (`code = 2`).

```yukino
load std.random: rng_new, RandomError
load std.io: println

set rng = rng_new()
sus {
    set temp = rng.range_float(15.0, 25.0)
    println("Температура: ${temp}")
} catch err: RandomError {
    println("Ошибка диапазона: ${err.message}")
}
```

### 3.10 fill_bytes

Заполняет переданный буфер памяти байтами псевдослучайной последовательности.

```yukino
fun fill_bytes(buf: ptr<uint8>, len: int64)
```

Заполняет `len` байтов по указателю `buf` и не возвращает значений.

Функция никогда не паникует.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set raw_mem: uint8[] = []
for i in 0..32 {
    raw_mem.push(0)
}
rng.fill_bytes(raw_mem.as_ptr(), 32)
println("32 байта заполнены")
```

---

## 4. Глобальные функции модуля

### 4.1 rng_new

Создает новый изолированный генератор `Rng`, сидированный системной энтропией операционной системы (на Unix — из `/dev/urandom`; при недоступности источника используется программная резервная энтропия).

```yukino
public fun rng_new() -> Rng
```

Возвращает новый экземпляр класса `Rng`.

Функция никогда не паникует.

```yukino
load std.random: rng_new, Rng
load std.io: println

set rng = rng_new()
println("Случайное число: ${rng.next_u64()}")
```

### 4.2 rng_with_seed

Создает детерминированный генератор `Rng` с заданным фиксированным начальным сидом.

```yukino
public fun rng_with_seed(seed: uint64) -> Rng
```

Возвращает экземпляр `Rng`, воспроизводимо выдающий идентичную последовательность при одинаковом `seed`.

Функция никогда не паникует.

```yukino
load std.random: rng_with_seed, Rng
load std.io: println

set rng = rng_with_seed(42u64)
set first = rng.next_i64()
println("Детерминированное значение: ${first}")
```

### 4.3 rand_u64

Возвращает случайное 64-битное беззнаковое число из глобального потокобезопасного генератора.

```yukino
public fun rand_u64() -> uint64
```

Возвращает случайное значение `uint64`.

Функция никогда не паникует.

```yukino
load std.random: rand_u64
load std.io: println

set val = rand_u64()
println("Глобальный u64: ${val}")
```

### 4.4 rand_u32

Возвращает случайное 32-битное беззнаковое число из глобального генератора.

```yukino
public fun rand_u32() -> uint32
```

Возвращает случайное значение `uint32`.

Функция никогда не паникует.

```yukino
load std.random: rand_u32
load std.io: println

set val = rand_u32()
println("Глобальный u32: ${val}")
```

### 4.5 rand_int

Возвращает равномерно распределенное целое число в закрытом отрезке `[min, max]` из глобального генератора.

```yukino
public fun rand_int(min: int64, max: int64) -> int64
```

Возвращает случайное целое число `int64` между `min` и `max` включительно.

В случае если `min > max`, вызывает панику со структурой `RandomError` (`code = 1`).

```yukino
load std.random: rand_int, RandomError
load std.io: println

sus {
    set n = rand_int(1, 100)
    println("Число: ${n}")
} catch err: RandomError {
    println("Ошибка диапазона: ${err.message}")
}
```

### 4.6 rand_bool

Возвращает случайное булево значение с вероятностью 50% из глобального генератора.

```yukino
public fun rand_bool() -> bool
```

Возвращает `true` или `false`.

Функция никогда не паникует.

```yukino
load std.random: rand_bool
load std.io: println

set flag = rand_bool()
println("Флаг: ${flag}")
```

### 4.7 rand_float

Возвращает вещественное число в полуинтервале `[0.0, 1.0)` из глобального генератора.

```yukino
public fun rand_float() -> float64
```

Возвращает вещественное число `float64` в диапазоне от 0.0 (включительно) до 1.0 (не включая).

Функция никогда не паникует.

```yukino
load std.random: rand_float
load std.io: println

set f = rand_float()
println("Случайное float64: ${f}")
```

### 4.8 rand_float32

Возвращает 32-битное вещественное число в полуинтервале `[0.0, 1.0)` из глобального генератора.

```yukino
public fun rand_float32() -> float32
```

Возвращает вещественное число `float32`.

Функция никогда не паникует.

```yukino
load std.random: rand_float32
load std.io: println

set f32 = rand_float32()
println("Случайное float32: ${f32}")
```

### 4.9 rand_range

Возвращает случайное вещественное число в полуинтервале `[min, max)` из глобального генератора.

```yukino
public fun rand_range(min: float64, max: float64) -> float64
```

Возвращает вещественное число `float64` в диапазоне `[min, max)`.

В случае если `min > max`, вызывает панику со структурой `RandomError` (`code = 2`).

```yukino
load std.random: rand_range, RandomError
load std.io: println

sus {
    set speed = rand_range(0.0, 120.0)
    println("Скорость: ${speed}")
} catch err: RandomError {
    println("Ошибка: ${err.message}")
}
```

### 4.10 rand_choice_char

Случайным образом выбирает один символ из переданной строки вариантов.

```yukino
public fun rand_choice_char(chars: string) -> char
```

Возвращает выбранный символ `char`.

В случае если переданная строка пуста, вызывает панику со структурой `RandomError` (`code = 3`).

```yukino
load std.random: rand_choice_char, RandomError
load std.io: println

sus {
    set c = rand_choice_char("0123456789ABCDEF")
    println("Символ: ${c}")
} catch err: RandomError {
    println("Ошибка: ${err.message}")
}
```

### 4.11 rand_uuid_v4

Генерирует канонический уникальный идентификатор UUID версии 4 по стандарту RFC 4122.

```yukino
public fun rand_uuid_v4() -> string
```

Возвращает строку формата `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`.

Функция никогда не паникует.

```yukino
load std.random: rand_uuid_v4
load std.io: println

set session_id = rand_uuid_v4()
println("Session ID: ${session_id}")
```

### 4.12 rand_gaussian

Генерирует случайное вещественное число, приближенно распределенное по нормальному (гауссову) закону, с заданным математическим ожиданием и стандартным отклонением. Значение вычисляется как сумма 12 равномерно распределенных величин из `[0.0, 1.0)` минус 6, поэтому результат всегда лежит в диапазоне `mean ± 6 * std_dev`.

```yukino
public fun rand_gaussian(mean: float64 = 0.0, std_dev: float64 = 1.0) -> float64
```

Возвращает сгенерированное значение `float64`.

Функция никогда не паникует.

```yukino
load std.random: rand_gaussian
load std.io: println

set sample = rand_gaussian(20.0, 2.5)
println("Гауссова величина: ${sample}")
```

### 4.13 rand_bytes

Заполняет переданный буфер памяти случайными байтами системной энтропии. На Unix байты читаются из `/dev/urandom`; если источник недоступен (в том числе на платформах без `/dev/urandom`), байты генерируются программным генератором и не являются криптографически стойкими. Если `buf` нулевой или `len <= 0`, функция ничего не делает.

```yukino
public fun rand_bytes(buf: ptr<uint8>, len: int64)
```

Заполняет `len` байтов по указателю `buf` и не возвращает значений.

Функция никогда не паникует.

```yukino
load std.random: rand_bytes
load std.io: println

set key_buf: uint8[] = []
for i in 0..32 {
    key_buf.push(0)
}
rand_bytes(key_buf.as_ptr(), 32)
println("Буфер энтропии получен")
```

### 4.14 rand_seed

Принудительно устанавливает начальное значение сида для глобального генератора.

```yukino
public fun rand_seed(seed: uint64)
```

Устанавливает внутреннее состояние глобального генератора и не возвращает значений. Сид `0` автоматически заменяется фиксированным значением по умолчанию.

Функция никогда не паникует.

```yukino
load std.random: rand_seed, rand_u64
load std.io: println

rand_seed(123456789u64)
set a = rand_u64()
println("Сидированное число: ${a}")
```

### 4.15 rand_i64

Возвращает случайное 64-битное знаковое число из глобального генератора: результат `rand_u64()`, переинтерпретированный как `int64`.

```yukino
public fun rand_i64() -> int64
```

Возвращает псевдослучайное значение типа `int64`.

Функция никогда не паникует.

```yukino
load std.random: rand_i64
load std.io: println

set val = rand_i64()
println("Глобальный int64: ${val}")
```

### 4.16 rand_i32

Возвращает случайное 32-битное знаковое число из глобального генератора: результат `rand_u32()`, переинтерпретированный как `int32`.

```yukino
public fun rand_i32() -> int32
```

Возвращает псевдослучайное значение типа `int32`.

Функция никогда не паникует.

```yukino
load std.random: rand_i32
load std.io: println

set val = rand_i32()
println("Глобальный int32: ${val}")
```

---

[← Назад: Модуль std.datetime](6-datetime.md) | [Вперед: Модуль std.fs →](8-fs.md)
