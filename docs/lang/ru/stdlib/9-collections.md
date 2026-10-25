# Модуль std.collections

Модуль `std.collections` предоставляет набор утилит, алгоритмов и структур данных для работы со срезами, массивами и специализированными контейнерами: поиск, функциональная трансформация, модификация на месте, объединение строк, динамический список `List<T>`, ассоциативный массив `HashMap<K, V>`, множество `HashSet<T>`, двусторонняя очередь `Deque<T>` и очередь FIFO `Queue<T>`.

---

## 1. Подключение: load std.collections

Импорт всего пакета:

```yukino
load std.collections
```

В текущую область видимости импортируются все публичные элементы модуля `std.collections`:

- **Структуры ошибок:**
  - `ListIndexError` — ошибка выхода индекса за границы списка (код `1`).
- **Контейнеры (классы):**
  - `List<T>` — динамический расширяемый список элементов;
  - `HashMap<K, V>` — ассоциативный массив ключей и значений на основе хеш-таблицы;
  - `HashSet<T>` — множество уникальных элементов на основе хеш-таблицы;
  - `Deque<T>` — двусторонняя очередь с операциями вставки и удаления с обоих концов;
  - `Queue<T>` — очередь по принципу FIFO (первым пришел — первым ушел).
- **Функции поиска и проверок:**
  - `acontains`, `acontains_str`, `aindex_of`, `aindex_of_str`, `afind`.
- **Функциональные трансформации:**
  - `amap`, `afilter`, `areduce`, `afor_each`, `afor_each_str`.
- **Модификация срезов на месте:**
  - `afill`, `afill_str`, `areverse`, `areverse_str`, `asort`, `asort_str`, `asort_by`.
- **Строковые утилиты:**
  - `ajoin`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.collections` глобальные константы не объявлены. Все операции выполняются через функции и методы классов.

---

## 3. Структуры данных и ошибок (паники)

### 3.1 ListIndexError

Вызывается при попытке доступа к элементу списка `List<T>` по индексу за пределами допустимого диапазона `0 <= index < len`.

```yukino
public struct ListIndexError {
    message: string
    code: int32
}
```

- `message` — текстовое описание ошибки (`"list index out of bounds"`).
- `code` — числовой код ошибки: `1`.

---

## 4. Классы (Контейнеры)

### 4.1 List<T>

Динамический расширяемый массив элементов с автоматическим управлением памятью и емкостью.

```yukino
public class List<T>
```

#### push
```yukino
fun push(item: int64)
```
Добавляет элемент `item` в конец списка, при необходимости увеличивая его емкость. Функция никогда не вызывает панику.

#### pop
```yukino
fun pop() -> int64?
```
Удаляет и возвращает последний элемент списка. Возвращает `null`, если список пуст. Функция никогда не вызывает панику.

#### del
```yukino
fun del(index: int64)
```
Удаляет элемент по указанному индексу `index`, сдвигая все последующие элементы влево.

В случае выхода индекса за пределы `0 <= index < len` вызывает панику со структурой `ListIndexError` (код `1`).

#### get
```yukino
fun get(index: int64) -> int64
```
Возвращает значение элемента по указанному индексу `index`.

В случае выхода индекса за границы списка вызывает панику со структурой `ListIndexError` (код `1`).

#### set_at
```yukino
fun set_at(index: int64, item: int64)
```
Перезаписывает значение элемента по указанному индексу `index`.

В случае выхода индекса за границы списка вызывает панику со структурой `ListIndexError` (код `1`).

#### len
```yukino
fun len() -> int64
```
Возвращает текущее количество элементов в списке как `int64`. Функция никогда не вызывает панику.

#### is_empty
```yukino
fun is_empty() -> bool
```
Возвращает `true`, если список пуст, иначе `false`. Функция никогда не вызывает панику.

#### clear
```yukino
fun clear()
```
Очищает список, сбрасывая количество элементов в ноль. Функция никогда не вызывает панику.

#### reserve
```yukino
fun reserve(additional: int64)
```
Выделяет память под как минимум `additional` дополнительных элементов. Функция никогда не вызывает панику.

#### contains
```yukino
fun contains(item: int64) -> bool
```
Возвращает `true`, если элемент `item` содержится в списке, иначе `false`. Функция никогда не вызывает панику.

#### index_of
```yukino
fun index_of(item: int64) -> int64
```
Возвращает нулевой индекс первого вхождения элемента `item` или `-1`, если элемент не найден. Функция никогда не вызывает панику.

#### reverse
```yukino
fun reverse()
```
Разворачивает порядок элементов списка на месте. Функция никогда не вызывает панику.

#### fill
```yukino
fun fill(val: int64)
```
Заполняет все элементы списка значением `val`. Функция никогда не вызывает панику.

#### to_array
```yukino
fun to_array() -> int64[]
```
Возвращает копию всех элементов списка в виде обычного среза. Функция никогда не вызывает панику.

```yukino
load std.collections.list: List
load std.io: println

set list = init List<int64>()
list.push(10)
list.push(20)
set item = list.get(0)
println("Элемент: ${item}")
```

---

### 4.2 HashMap<K, V>

Хеш-таблица ассоциативных пар ключ-значение с амортизированным временем доступа O(1). Поддерживает строковые ключи (`set`, `get`, `contains_key`, `remove`) и целочисленные ключи (`set_i64`, `get_i64`, `contains_key_i64`, `remove_i64`).

```yukino
public class HashMap<K, V>
```

#### set
```yukino
fun set(key: string, val: string)
```
Сохраняет значение `val` по ключу `key`. Если ключ уже существовал, его значение обновляется. Функция никогда не вызывает панику.

#### get
```yukino
fun get(key: string) -> string?
```
Возвращает строковое значение по ключу `key` либо `null`, если ключ отсутствует в таблице. Функция никогда не вызывает панику.

#### contains_key
```yukino
fun contains_key(key: string) -> bool
```
Возвращает `true`, если ключ `key` присутствует в таблице, иначе `false`. Функция никогда не вызывает панику.

#### remove
```yukino
fun remove(key: string) -> bool
```
Удаляет запись с ключом `key`. Возвращает `true`, если ключ был найден и удален, иначе `false`. Функция никогда не вызывает панику.

#### len
```yukino
fun len() -> int64
```
Возвращает общее количество пар ключ-значение в таблице. Функция никогда не вызывает панику.

#### is_empty
```yukino
fun is_empty() -> bool
```
Возвращает `true`, если таблица пуста, иначе `false`. Функция никогда не вызывает панику.

#### clear
```yukino
fun clear()
```
Удаляет все записи из хеш-таблицы. Функция никогда не вызывает панику.

#### set_i64 / get_i64 / contains_key_i64 / remove_i64
```yukino
fun set_i64(key: int64, val: int64)
fun get_i64(key: int64) -> int64?
fun contains_key_i64(key: int64) -> bool
fun remove_i64(key: int64) -> bool
```
Целочисленные аналоги методов для работы с ключами и значениями типа `int64`. Возвращают соответствующие значения либо `null` при отсутствии ключа. Функции никогда не вызывают панику.

```yukino
load std.collections.hashmap: HashMap
load std.io: println

set map = init HashMap<string, string>()
map.set("name", "Yukino")
set name = map.get("name") ?? "Unknown"
println("Имя: ${name}")
```

---

### 4.3 HashSet<T>

Множество уникальных элементов на основе хеш-таблицы.

```yukino
public class HashSet<T>
```

#### add
```yukino
fun add(item: string) -> bool
```
Добавляет строковый элемент `item` в множество. Возвращает `true`, если элемент был успешно добавлен (ранее отсутствовал), или `false`, если элемент уже присутствовал. Функция никогда не вызывает панику.

#### contains
```yukino
fun contains(item: string) -> bool
```
Возвращает `true`, если строковый элемент `item` присутствует в множестве, иначе `false`. Функция никогда не вызывает панику.

#### remove
```yukino
fun remove(item: string) -> bool
```
Удаляет элемент из множества. Возвращает `true`, если элемент был удален, иначе `false`. Функция никогда не вызывает панику.

#### len
```yukino
fun len() -> int64
```
Возвращает количество уникальных элементов в множестве. Функция никогда не вызывает панику.

#### is_empty
```yukino
fun is_empty() -> bool
```
Возвращает `true`, если множество пусто, иначе `false`. Функция никогда не вызывает панику.

#### clear
```yukino
fun clear()
```
Удаляет все элементы из множества. Функция никогда не вызывает панику.

#### add_i64 / contains_i64 / remove_i64
```yukino
fun add_i64(val: int64) -> bool
fun contains_i64(val: int64) -> bool
fun remove_i64(val: int64) -> bool
```
Целочисленные аналоги методов множества для работы со значениями типа `int64`. Функции никогда не вызывают панику.

```yukino
load std.collections.hashset: HashSet
load std.io: println

set tags = init HashSet<string>()
tags.add("system")
if tags.contains("system") {
    println("Тег найден")
}
```

---

### 4.4 Deque<T>

Двусторонняя очередь, оптимизированная для эффективной вставки и удаления элементов как с начала, так и с конца.

```yukino
public class Deque<T>
```

#### push_back / push_front
```yukino
fun push_back(val: int64)
fun push_front(val: int64)
```
Добавляет элемент `val` в конец или начало очереди соответственно. Функции никогда не вызывают панику.

#### pop_front / pop_back
```yukino
fun pop_front() -> int64?
fun pop_back() -> int64?
```
Извлекает и возвращает элемент из начала или конца очереди соответственно. Возвращает `null`, если очередь пуста. Функции никогда не вызывают панику.

#### peek_front / peek_back
```yukino
fun peek_front() -> int64?
fun peek_back() -> int64?
```
Возвращает первый или последний элемент очереди без его удаления. Возвращает `null`, если очередь пуста. Функции никогда не вызывают панику.

#### len
```yukino
fun len() -> int64
```
Возвращает количество элементов в очереди. Функция никогда не вызывает панику.

#### is_empty
```yukino
fun is_empty() -> bool
```
Возвращает `true`, если очередь пуста, иначе `false`. Функция никогда не вызывает панику.

#### clear
```yukino
fun clear()
```
Очищает очередь от всех элементов. Функция никогда не вызывает панику.

```yukino
load std.collections.queue: Deque
load std.io: println

set dq = init Deque<int64>()
dq.push_back(100)
dq.push_front(200)
set first = dq.pop_front() ?? 0
println("Первый элемент: ${first}")
```

---

### 4.5 Queue<T>

Очередь по принципу FIFO (First In, First Out).

```yukino
public class Queue<T>
```

#### push
```yukino
fun push(val: int64)
```
Помещает элемент `val` в конец очереди. Функция никогда не вызывает панику.

#### pop
```yukino
fun pop() -> int64?
```
Извлекает и возвращает первый элемент очереди. Возвращает `null`, если очередь пуста. Функция никогда не вызывает панику.

#### peek
```yukino
fun peek() -> int64?
```
Возвращает первый элемент очереди без извлечения. Возвращает `null`, если очередь пуста. Функция никогда не вызывает панику.

#### len
```yukino
fun len() -> int64
```
Возвращает количество элементов в очереди. Функция никогда не вызывает панику.

#### is_empty
```yukino
fun is_empty() -> bool
```
Возвращает `true`, если очередь пуста, иначе `false`. Функция никогда не вызывает панику.

#### clear
```yukino
fun clear()
```
Удаляет все элементы из очереди. Функция никогда не вызывает панику.

```yukino
load std.collections.queue: Queue
load std.io: println

set q = init Queue<int64>()
q.push(1)
q.push(2)
set val = q.pop() ?? 0
println("Извлечено: ${val}")
```

---

## 5. Справочник функций: Поиск и проверки (search)

### 5.1 acontains

Проверяет, содержится ли заданное значение `item` в целочисленном срезе `arr`.

```yukino
public fun acontains(ref arr: int64[], item: int64) -> bool
```

Возвращает `true`, если элемент найден в срезе, иначе `false`.

Функция никогда не вызывает панику.

```yukino
load std.collections.search: acontains
load std.io: println

set numbers: int64[] = [10, 20, 30]
set exists = acontains(ref numbers, 20)
println("Найдено: ${exists}")
```

### 5.2 acontains_str

Проверяет, содержится ли заданная строка `item` в строковом срезе `arr`.

```yukino
public fun acontains_str(ref arr: string[], item: string) -> bool
```

Возвращает `true`, если строка найдена в срезе, иначе `false`.

Функция никогда не вызывает панику.

```yukino
load std.collections.search: acontains_str
load std.io: println

set names = ["alice", "bob", "charlie"]
set exists = acontains_str(ref names, "bob")
println("Найден: ${exists}")
```

### 5.3 aindex_of

Возвращает индекс первого вхождения значения `item` в целочисленном срезе `arr`.

```yukino
public fun aindex_of(ref arr: int64[], item: int64) -> int64
```

Возвращает индекс найденного элемента `0 <= index < len` или `-1`, если элемент не найден.

Функция никогда не вызывает панику.

```yukino
load std.collections.search: aindex_of
load std.io: println

set values = [100, 200, 300]
set idx = aindex_of(ref values, 200)
println("Индекс: ${idx}")
```

### 5.4 aindex_of_str

Возвращает индекс первого вхождения строки `item` в строковом срезе `arr`.

```yukino
public fun aindex_of_str(ref arr: string[], item: string) -> int64
```

Возвращает индекс совпадения `0 <= index < len` или `-1`, если совпадений нет.

Функция никогда не вызывает панику.

```yukino
load std.collections.search: aindex_of_str
load std.io: println

set fruits = ["яблоко", "груша", "банан"]
set pos = aindex_of_str(ref fruits, "груша")
println("Позиция: ${pos}")
```

### 5.5 afind

Находит первый элемент в срезе `arr`, для которого предикат `predicate` возвращает `true`.

```yukino
public fun afind(ref arr: int64[], predicate: fun(int64) -> bool) -> int64?
```

Возвращает найденное числовое значение либо `null`, если ни один элемент не удовлетворяет условию предиката.

Функция никогда не вызывает панику.

```yukino
load std.collections.search: afind
load std.io: println

fun is_positive(x: int64) -> bool {
    return x > 0
}

set nums: int64[] = [-3, -1, 4, 7]
set found = afind(ref nums, is_positive)
if found != null {
    println("Первое положительное: ${found}")
}
```

---

## 6. Справочник функций: Функциональная трансформация (transform)

### 6.1 amap

Применяет функцию преобразования `f` к каждому элементу среза `arr` и возвращает новый массив результатов.

```yukino
public fun amap(ref arr: int64[], f: fun(int64) -> int64) -> int64[]
```

Возвращает новый массив `int64[]` той же длины, содержащий результаты применения функции `f`.

Функция никогда не вызывает панику.

```yukino
load std.collections.transform: amap
load std.io: println

fun double_val(x: int64) -> int64 {
    return x * 2
}

set orig: int64[] = [1, 2, 3]
set doubled = amap(ref orig, double_val)
println("Первый удвоенный: ${doubled[0]}")
```

### 6.2 afilter

Отбирает элементы среза `arr`, для которых предикат `predicate` возвращает `true`, и собирает их в новый массив.

```yukino
public fun afilter(ref arr: int64[], predicate: fun(int64) -> bool) -> int64[]
```

Возвращает новый срез `int64[]`, содержащий только прошедшие фильтрацию элементы.

Функция никогда не вызывает панику.

```yukino
load std.collections.transform: afilter
load std.io: println

fun is_even(x: int64) -> bool {
    return x % 2 == 0
}

set all: int64[] = [1, 2, 3, 4, 5, 6]
set evens = afilter(ref all, is_even)
println("Четных элементов: ${evens.len()}")
```

### 6.3 areduce

Вычисляет свертку элементов среза с использованием функции-аккумулятора `f` и начального значения `initial`.

```yukino
public fun areduce(ref arr: int64[], initial: int64, f: fun(int64, int64) -> int64) -> int64
```

Возвращает накопленный результат свертки как `int64`.

Функция никогда не вызывает панику.

```yukino
load std.collections.transform: areduce
load std.io: println

fun sum(acc: int64, x: int64) -> int64 {
    return acc + x
}

set items: int64[] = [10, 20, 30]
set total = areduce(ref items, 0, sum)
println("Сумма: ${total}")
```

### 6.4 afor_each / afor_each_str

Выполняет функцию `f` для каждого элемента переданного среза.

```yukino
public fun afor_each(ref arr: int64[], f: fun(int64))
public fun afor_each_str(ref arr: string[], f: fun(string))
```

Функции выполняют действие над элементами и не возвращают значений.

Функции никогда не вызывают панику.

```yukino
load std.collections.transform: afor_each_str
load std.io: println

fun print_item(s: string) {
    println("Элемент: " + s)
}

set list = ["first", "second"]
afor_each_str(ref list, print_item)
```

---

## 7. Справочник функций: Модификация срезов на месте (mut)

### 7.1 afill / afill_str

Заполняет все элементы среза значением `val` на месте.

```yukino
public fun afill(ref ~arr: int64[], val: int64)
public fun afill_str(ref ~arr: string[], val: string)
```

Функции модифицируют переданный срез по мутабельной ссылке `ref ~arr` и не возвращают значений.

Функции никогда не вызывают панику.

```yukino
load std.collections.mut: afill
load std.io: println

set data: int64[] = [1, 2, 3]
afill(ref ~data, 0)
println("Значение: ${data[0]}")
```

### 7.2 areverse / areverse_str

Разворачивает порядок элементов среза на месте.

```yukino
public fun areverse(ref ~arr: int64[])
public fun areverse_str(ref ~arr: string[])
```

Функции обращают порядок следования элементов на месте и не возвращают значений.

Функции никогда не вызывают панику.

```yukino
load std.collections.mut: areverse
load std.io: println

set order: int64[] = [1, 2, 3]
areverse(ref ~order)
println("Первый элемент: ${order[0]}")
```

### 7.3 asort / asort_str

Сортирует элементы среза на месте по возрастанию. Для строк применяется лексикографический порядок.

```yukino
public fun asort(ref ~arr: int64[])
public fun asort_str(ref ~arr: string[])
```

Функции сортируют срез на месте и не возвращают значений.

Функции никогда не вызывают панику.

```yukino
load std.collections.mut: asort
load std.io: println

set scores: int64[] = [45, 12, 89, 3]
asort(ref ~scores)
println("Наименьший: ${scores[0]}")
```

### 7.4 asort_by

Сортирует элементы среза на месте с использованием пользовательской функции сравнения `cmp`.

```yukino
public fun asort_by(ref ~arr: int64[], cmp: fun(int64, int64) -> int32)
```

Функция сравнения `cmp(a, b)` должна возвращать отрицательное число, если `a < b`, положительное число, если `a > b`, и ноль, если элементы равны.

Функция никогда не вызывает панику.

```yukino
load std.collections.mut: asort_by
load std.io: println

fun cmp_desc(a: int64, b: int64) -> int32 {
    return cast(b - a, "int32")
}

set items: int64[] = [1, 5, 2]
asort_by(ref ~items, cmp_desc)
println("Максимальный: ${items[0]}")
```

---

## 8. Справочник функций: Строковые утилиты (strings)

### 8.1 ajoin

Объединяет строковые элементы среза `arr` в одну строку с использованием строкового разделителя `separator`.

```yukino
public fun ajoin(ref arr: string[], separator: string) -> string
```

Возвращает объединенную строку типа `string`. Если переданный срез пуст, возвращается пустая строка `""`.

Функция никогда не вызывает панику.

```yukino
load std.collections.str: ajoin
load std.io: println

set words = ["Yukino", "Systems", "Language"]
set title = ajoin(ref words, " ")
println(title)
```

---

[← Назад: Модуль std.fs](8-fs.md) | [Вперед: Модуль std.process →](10-process.md)
