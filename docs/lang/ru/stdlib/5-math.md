# Модуль std.math

Модуль `std.math` предоставляет минимальный ортогональный базис низкоуровневых математических примитивов: побитовые преобразования стандарта IEEE 754, аппаратные инструкции округления и извлечения корня, битовые счетчики процессора, операции расширенной разрядности с флагами переноса/заема и фундаментальные константы чисел с плавающей точкой.

---

## 1. Подключение: load std.math

Импорт всего пакета:

```yukino
load std.math
```

В текущую область видимости импортируются все публичные элементы модуля `std.math`:

- **Константы IEEE 754:**
  - `NAN_BITS`, `INFINITY_BITS`, `NEG_INFINITY_BITS`;
  - `EPSILON`, `MIN_POSITIVE`, `MAX`, `MIN`;
  - `MANTISSA_BITS`, `EXP_BIAS`.
- **Побитовые преобразования (Float Bitcasts):**
  - `to_bits`, `from_bits` (для `float64` / `uint64`);
  - `to_bits32`, `from_bits32` (для `float32` / `uint32`).
- **Специальные значения и классификация:**
  - `nan`, `infinity`, `neg_infinity`;
  - `is_nan`, `is_infinite`, `is_finite`.
- **Аппаратные математические операции:**
  - `sqrt`, `fma`, `abs`, `copysign`;
  - `floor`, `ceil`, `trunc`, `round`;
  - `abs_int`, `min`, `max`, `min_int`, `max_int`.
- **Низкоуровневые битовые счетчики (Bit Twiddling):**
  - `clz`, `ctz`, `popcount` (для `uint64`);
  - `clz32`, `ctz32`, `popcount32` (для `uint32`).
- **Арифметика расширенной точности (Длинная арифметика):**
  - `mul_wide` — умножение двух 64-битных чисел с возвратом пары `(hi, lo)`;
  - `add_carry` — сложение двух 64-битных чисел с флагом переноса `(sum, carry_out)`;
  - `sub_borrow` — вычитание двух 64-битных чисел с флагом заема `(diff, borrow_out)`.

Все перечисленные сущности также автоматически доступны при импорте всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

### 2.1 Константы битовых масок

```yukino
public static NAN_BITS: uint64 = 0x7ff8000000000000u64
public static INFINITY_BITS: uint64 = 0x7ff0000000000000u64
public static NEG_INFINITY_BITS: uint64 = 0xfff0000000000000u64
```

- `NAN_BITS` — канонический битовый паттерн тишины (Quiet NaN) формата IEEE 754 double precision.
- `INFINITY_BITS` — битовое представление положительной бесконечности ($+\infty$).
- `NEG_INFINITY_BITS` — битовое представление отрицательной бесконечности ($-\infty$).

### 2.2 Числовые параметры формата IEEE 754 (float64)

```yukino
public static EPSILON: float64 = 2.2204460492503131e-16
public static MIN_POSITIVE: float64 = 2.2250738585072014e-308
public static MAX: float64 = 1.7976931348623157e+308
public static MIN: float64 = -1.7976931348623157e+308
public static MANTISSA_BITS: int32 = 52
public static EXP_BIAS: int32 = 1023
```

- `EPSILON` — машинный эпсилон: разница между $1.0$ и следующим представимым числом формата `float64`.
- `MIN_POSITIVE` — наименьшее положительное нормализованное число `float64`.
- `MAX` — наибольшее конечное положительное число `float64`.
- `MIN` — наименьшее (наибольшее по модулю отрицательное) конечное число `float64`.
- `MANTISSA_BITS` — количество бит дробной части (мантиссы) в формате `float64`.
- `EXP_BIAS` — экспоненциальное смещение (bias) формата IEEE 754.

---

## 3. Ошибки и паники

В модуле `std.math` структуры ошибок отсутствуют. Функции модуля соответствуют спецификации стандарта IEEE 754: математически неопределенные операции (например, извлечение корня из отрицательного числа) возвращают значение `NaN`, а деление на ноль возвращает `Infinity` или `-Infinity` без прерывания программы паникой.

---

## 4. Справочник функций

### 4.1 to_bits

Выполняет побитовую реинтерпретацию (bitcast) между 64-битным числом с плавающей точкой `float64` и 64-битным беззнаковым целым `uint64` без изменения содержимого памяти.

```yukino
public fun to_bits(x: float64) -> uint64
```

Возвращает сырые 64 бита вещественного числа как `uint64`.

Функция никогда не паникует.

```yukino
load std.math: to_bits
load std.io: println

set bits = to_bits(1.0)
println("Биты 1.0: ${bits}") // 0x3ff0000000000000
```

### 4.2 from_bits

Преобразует 64-битное целое число в число с плавающей точкой `float64` путем побитовой реинтерпретации без арифметического преобразования.

```yukino
public fun from_bits(b: uint64) -> float64
```

Возвращает число `float64` с битовым содержимым `b`.

Функция никогда не паникует.

```yukino
load std.math: from_bits
load std.io: println

set val = from_bits(0x3ff0000000000000u64)
println("Значение: ${val}") // 1.0
```

### 4.3 to_bits32

Выполняет побитовую реинтерпретацию между 32-битным числом с плавающей точкой `float32` и 32-битным беззнаковым целым `uint32`.

```yukino
public fun to_bits32(x: float32) -> uint32
```

Возвращает биты числа как `uint32`.

Функция никогда не паникует.

```yukino
load std.math: to_bits32
load std.io: println

set b = to_bits32(cast(1.0, "float32"))
println("Биты float32: ${b}")
```

### 4.4 from_bits32

Преобразует 32-битное целое `uint32` в `float32` путем побитовой реинтерпретации.

```yukino
public fun from_bits32(b: uint32) -> float32
```

Возвращает число `float32`.

Функция никогда не паникует.

```yukino
load std.math: from_bits32
load std.io: println

set f = from_bits32(0x3f800000u32)
println("Значение: ${f}") // 1.0
```

### 4.5 nan

Возвращает каноническое специальное значение не-число (Quiet NaN) стандарта IEEE 754 double precision.

```yukino
public fun nan() -> float64
```

Возвращает специальное вещественное значение `NaN`.

Функция никогда не паникует.

```yukino
load std.math: nan, is_nan
load std.io: println

set n = nan()
println("Является NaN: ${is_nan(n)}") // true
```

### 4.6 infinity

Возвращает специальное значение положительной бесконечности ($+\infty$) формата IEEE 754.

```yukino
public fun infinity() -> float64
```

Возвращает значение положительной бесконечности.

Функция никогда не паникует.

```yukino
load std.math: infinity, is_infinite
load std.io: println

set inf = infinity()
println("Бесконечность: ${is_infinite(inf)}") // true
```

### 4.7 neg_infinity

Возвращает специальное значение отрицательной бесконечности ($-\infty$) формата IEEE 754.

```yukino
public fun neg_infinity() -> float64
```

Возвращает значение отрицательной бесконечности.

Функция никогда не паникует.

```yukino
load std.math: neg_infinity, is_infinite
load std.io: println

set neg_inf = neg_infinity()
println("Отрицательная бесконечность: ${is_infinite(neg_inf)}") // true
```

### 4.8 is_nan

Проверяет, является ли вещественное число значением `NaN` (Not a Number).

```yukino
public fun is_nan(x: float64) -> bool
```

Возвращает `true`, если число `x` является `NaN`, иначе `false`.

Функция никогда не паникует.

```yukino
load std.math: is_nan, nan
load std.io: println

println("Проверка NaN: ${is_nan(nan())}") // true
```

### 4.9 is_infinite

Проверяет, является ли вещественное число положительной или отрицательной бесконечностью.

```yukino
public fun is_infinite(x: float64) -> bool
```

Возвращает `true`, если число равно $+\infty$ или $-\infty$, иначе `false`.

Функция никогда не паникует.

```yukino
load std.math: is_infinite, infinity
load std.io: println

println("Проверка бесконечности: ${is_infinite(infinity())}") // true
```

### 4.10 is_finite

Проверяет, является ли вещественное число конечным (не является `NaN` и не является бесконечностью).

```yukino
public fun is_finite(x: float64) -> bool
```

Возвращает `true`, если число конечно, иначе `false`.

Функция никогда не паникует.

```yukino
load std.math: is_finite
load std.io: println

println("Число конечно: ${is_finite(3.14)}") // true
```

### 4.11 sqrt

Вычисляет квадратный корень вещественного числа с использованием аппаратной инструкции процессора.

```yukino
public fun sqrt(x: float64) -> float64
```

Возвращает квадратный корень из `x`. Если $x < 0$, возвращает `NaN`.

Функция никогда не паникует.

```yukino
load std.math: sqrt
load std.io: println

set root = sqrt(16.0)
println("Квадратный корень: ${root}") // 4.0
```

### 4.12 fma

Выполняет аппаратную операцию совмещенного умножения-сложения (Fused Multiply-Add: $a \times b + c$) с единственным промежуточным округлением.

```yukino
public fun fma(a: float64, b: float64, c: float64) -> float64
```

Возвращает точный результат вычисления $a \times b + c$ без промежуточной потери разрядов.

Функция никогда не паникует.

```yukino
load std.math: fma
load std.io: println

set res = fma(2.0, 3.0, 4.0)
println("Результат fma: ${res}") // 10.0
```


### 4.13 abs

Возвращает абсолютное значение (модуль) числа с плавающей точкой `float64`.

```yukino
public fun abs(x: float64) -> float64
```

Возвращает неотрицательное число `float64` путем сброса знакового бита в 0.

Функция никогда не паникует.

```yukino
load std.math: abs
load std.io: println

set val = abs(-3.14)
println("Модуль: ${val}") // 3.14
```

### 4.14 abs_int

Возвращает абсолютное значение (модуль) 64-битного знакового целого числа `int64`.

```yukino
public fun abs_int(x: int64) -> int64
```

Возвращает неотрицательное целое число `int64`.

Функция никогда не паникует.

```yukino
load std.math: abs_int
load std.io: println

set val = abs_int(-42)
println("Модуль int: ${val}") // 42
```

### 4.15 copysign

Возвращает величину числа `x` со знаком, заимствованным из числа `sign`.

```yukino
public fun copysign(x: float64, sign: float64) -> float64
```

Возвращает вещественное число `float64`.

Функция никогда не паникует.

```yukino
load std.math: copysign
load std.io: println

set res = copysign(10.0, -1.0)
println("Число со знаком sign: ${res}") // -10.0
```

### 4.16 floor

Округляет вещественное число вниз до ближайшего целого, не превышающего аргумент.

```yukino
public fun floor(x: float64) -> float64
```

Возвращает округленное вниз число `float64`.

Функция никогда не паникует.

```yukino
load std.math: floor
load std.io: println

set res = floor(3.7)
println("Округление вниз: ${res}") // 3.0
```

### 4.17 ceil

Округляет вещественное число вверх до ближайшего целого, не меньшего аргумента.

```yukino
public fun ceil(x: float64) -> float64
```

Возвращает округленное вверх число `float64`.

Функция никогда не паникует.

```yukino
load std.math: ceil
load std.io: println

set res = ceil(3.2)
println("Округление вверх: ${res}") // 4.0
```

### 4.18 trunc

Отбрасывает дробную часть вещественного числа (округление в направлении нуля).

```yukino
public fun trunc(x: float64) -> float64
```

Возвращает усеченное число `float64`.

Функция никогда не паникует.

```yukino
load std.math: trunc
load std.io: println

set res = trunc(-3.7)
println("Усечение: ${res}") // -3.0
```

### 4.19 round

Округляет вещественное число к ближайшему целому числу (полуцелые округляются в сторону от нуля).

```yukino
public fun round(x: float64) -> float64
```

Возвращает округленное число `float64`.

Функция никогда не паникует.

```yukino
load std.math: round
load std.io: println

set res = round(3.5)
println("Округление: ${res}") // 4.0
```


### 4.20 min

Возвращает наименьшее из двух вещественных чисел `float64`.

```yukino
public fun min(a: float64, b: float64) -> float64
```

Возвращает меньшее из двух чисел.

Функция никогда не паникует.

```yukino
load std.math: min
load std.io: println

set m = min(1.5, 2.8)
println("Минимум: ${m}") // 1.5
```

### 4.21 max

Возвращает наибольшее из двух вещественных чисел `float64`.

```yukino
public fun max(a: float64, b: float64) -> float64
```

Возвращает большее из двух чисел.

Функция никогда не паникует.

```yukino
load std.math: max
load std.io: println

set m = max(1.5, 2.8)
println("Максимум: ${m}") // 2.8
```

### 4.22 min_int

Возвращает наименьшее из двух 64-битных знаковых целых чисел `int64`.

```yukino
public fun min_int(a: int64, b: int64) -> int64
```

Возвращает меньшее целое число `int64`.

Функция никогда не паникует.

```yukino
load std.math: min_int
load std.io: println

set m = min_int(10, 20)
println("Минимум int: ${m}") // 10
```

### 4.23 max_int

Возвращает наибольшее из двух 64-битных знаковых целых чисел `int64`.

```yukino
public fun max_int(a: int64, b: int64) -> int64
```

Возвращает большее целое число `int64`.

Функция никогда не паникует.

```yukino
load std.math: max_int
load std.io: println

set m = max_int(10, 20)
println("Максимум int: ${m}") // 20
```

### 4.24 clz

Count Leading Zeros — подсчитывает количество ведущих нулевых битов в 64-битном числе `uint64`.

```yukino
public fun clz(x: uint64) -> int32
```

Возвращает число ведущих нулей от 0 до 64 (для нуля возвращает 64).

Функция никогда не паникует.

```yukino
load std.math: clz
load std.io: println

set count = clz(1u64)
println("Ведущих нулей: ${count}") // 63
```

### 4.25 clz32

Подсчитывает количество ведущих нулевых битов в 32-битном числе `uint32`.

```yukino
public fun clz32(x: uint32) -> int32
```

Возвращает число ведущих нулей от 0 до 32.

Функция никогда не паникует.

```yukino
load std.math: clz32
load std.io: println

set count = clz32(1u32)
println("Ведущих нулей: ${count}") // 31
```

### 4.26 ctz

Count Trailing Zeros — подсчитывает количество замыкающих нулевых битов в 64-битном числе `uint64`.

```yukino
public fun ctz(x: uint64) -> int32
```

Возвращает число замыкающих нулей от 0 до 64.

Функция никогда не паникует.

```yukino
load std.math: ctz
load std.io: println

set count = ctz(8u64)
println("Замыкающих нулей: ${count}") // 3
```

### 4.27 ctz32

Подсчитывает количество замыкающих нулевых битов в 32-битном числе `uint32`.

```yukino
public fun ctz32(x: uint32) -> int32
```

Возвращает число замыкающих нулей от 0 до 32.

Функция никогда не паникует.

```yukino
load std.math: ctz32
load std.io: println

set count = ctz32(16u32)
println("Замыкающих нулей: ${count}") // 4
```

### 4.28 popcount

Population Count — подсчитывает общее число единичных битов (вес Хэмминга) в 64-битном числе `uint64`.

```yukino
public fun popcount(x: uint64) -> int32
```

Возвращает количество единиц от 0 до 64.

Функция никогда не паникует.

```yukino
load std.math: popcount
load std.io: println

set ones = popcount(7u64)
println("Число единичных битов: ${ones}") // 3
```

### 4.29 popcount32

Подсчитывает общее число единичных битов в 32-битном числе `uint32`.

```yukino
public fun popcount32(x: uint32) -> int32
```

Возвращает количество единиц от 0 до 32.

Функция никогда не паникует.

```yukino
load std.math: popcount32
load std.io: println

set ones = popcount32(15u32)
println("Число единичных битов: ${ones}") // 4
```

### 4.30 mul_wide

Выполняет умножение двух 64-битных целых чисел беззнаковой арифметики с сохранением полного 128-битного результата.

```yukino
public fun mul_wide(a: uint64, b: uint64) -> (uint64, uint64)
```

Возвращает пару `(hi, lo)`, где `hi` — старшие 64 бита произведения, а `lo` — младшие 64 бита.

Функция никогда не паникует.

```yukino
load std.math: mul_wide
load std.io: println

set (hi, lo) = mul_wide(0xffffffffffffffffu64, 2u64)
println("hi: ${hi}, lo: ${lo}")
```

### 4.31 add_carry

Складывает два 64-битных числа с учетом входного бита переноса `carry_in`.

```yukino
public fun add_carry(a: uint64, b: uint64, carry_in: uint8 = 0) -> (uint64, uint8)
```

Возвращает пару `(sum, carry_out)`, где `sum` — младшие 64 бита суммы по модулю $2^{64}$, а `carry_out` — бит исходящего переноса (1 при переполнении, иначе 0).

Функция никогда не паникует.

```yukino
load std.math: add_carry
load std.io: println

set (sum, c_out) = add_carry(0xffffffffffffffffu64, 1u64, 0u8)
println("Сумма: ${sum}, перенос: ${c_out}")
```

### 4.32 sub_borrow

Вычитает два 64-битных беззнаковых целых с учетом входного бита заема `borrow_in`.

```yukino
public fun sub_borrow(a: uint64, b: uint64, borrow_in: uint8 = 0) -> (uint64, uint8)
```

Возвращает пару `(diff, borrow_out)`, где `diff` — разность по модулю $2^{64}$, а `borrow_out` — флаг заема (1 при заеме, иначе 0).

Функция никогда не паникует.

```yukino
load std.math: sub_borrow
load std.io: println

set (diff, b_out) = sub_borrow(0u64, 1u64, 0u8)
println("Разность: ${diff}, заем: ${b_out}")
```

---

[← Назад: Модуль std.platformio](4-platformio.md) | [Вперед: Модуль std.datetime →](6-datetime.md)
