# Module std.collections

The `std.collections` module provides a comprehensive suite of algorithms, utilities, and data structures for working with slices, arrays, and specialized containers: search routines, functional transformations, in-place slice mutation, string joining, dynamic list `List<T>`, hash table `HashMap<K, V>`, hash set `HashSet<T>`, double-ended queue `Deque<T>`, and FIFO queue `Queue<T>`.

---

## 1. Import: load std.collections

Importing the entire package:

```yukino
load std.collections
```

All public components of `std.collections` are loaded into the current scope:

- **Error structures:**
  - `ListIndexError` — index out of bounds error (code `1`).
- **Containers (Classes):**
  - `List<T>` — dynamically resizable array list;
  - `HashMap<K, V>` — hash-table-backed key-value store;
  - `HashSet<T>` — hash-table-backed set of unique elements;
  - `Deque<T>` — double-ended queue supporting insertion and removal at both ends;
  - `Queue<T>` — first-in, first-out (FIFO) queue.
- **Search and Verification Functions:**
  - `acontains`, `acontains_str`, `aindex_of`, `aindex_of_str`, `afind`.
- **Functional Transformations:**
  - `amap`, `afilter`, `areduce`, `afor_each`, `afor_each_str`.
- **In-Place Mutation:**
  - `afill`, `afill_str`, `areverse`, `areverse_str`, `asort`, `asort_str`, `asort_by`.
- **String Utilities:**
  - `ajoin`.

All of these items are also accessible when importing the whole standard library: `load std`.

---

## 2. Constants

No global constants are declared in `std.collections`. All operations are executed through functions and class methods.

---

## 3. Data and Error Structures

### 3.1 ListIndexError

Thrown when accessing an element of `List<T>` at an index outside the valid range `0 <= index < len`.

```yukino
public struct ListIndexError {
    message: string
    code: int32
}
```

- `message` — human-readable error description (`"list index out of bounds"`).
- `code` — numerical error code: `1`.

---

## 4. Classes (Containers)

### 4.1 List<T>

Dynamically resizable array list with automatic capacity growth and element management.

```yukino
public class List<T>
```

#### push
```yukino
fun push(item: int64)
```
Appends `item` to the end of the list, expanding internal capacity as necessary. This function never panics.

#### pop
```yukino
fun pop() -> int64?
```
Removes and returns the last element of the list, or `null` if the list is empty. This function never panics.

#### del
```yukino
fun del(index: int64)
```
Removes the element at the specified zero-based index `index`, shifting all subsequent elements to the left.

If `index` is out of bounds (`index < 0` or `index >= len`), this method panics with structure `ListIndexError` (code `1`).

#### get
```yukino
fun get(index: int64) -> int64
```
Returns the value of the element at the specified zero-based index `index`.

If `index` is out of bounds (`index < 0` or `index >= len`), this method panics with structure `ListIndexError` (code `1`).

#### set_at
```yukino
fun set_at(index: int64, item: int64)
```
Overwrites the value at the specified zero-based index `index` with `item`.

If `index` is out of bounds (`index < 0` or `index >= len`), this method panics with structure `ListIndexError` (code `1`).

#### len
```yukino
fun len() -> int64
```
Returns the current number of elements in the list as `int64`. This function never panics.

#### is_empty
```yukino
fun is_empty() -> bool
```
Returns `true` if the list contains no elements, otherwise `false`. This function never panics.

#### clear
```yukino
fun clear()
```
Removes all elements from the list, resetting its length to zero. This function never panics.

#### reserve
```yukino
fun reserve(additional: int64)
```
Preallocates memory for at least `additional` more elements. This function never panics.

#### contains
```yukino
fun contains(item: int64) -> bool
```
Returns `true` if `item` is present in the list, otherwise `false`. This function never panics.

#### index_of
```yukino
fun index_of(item: int64) -> int64
```
Returns the zero-based index of the first occurrence of `item`, or `-1` if not found. This function never panics.

#### reverse
```yukino
fun reverse()
```
Reverses the order of elements in the list in place. This function never panics.

#### fill
```yukino
fun fill(val: int64)
```
Sets all elements in the list to the specified value `val`. This function never panics.

#### to_array
```yukino
fun to_array() -> int64[]
```
Returns a copy of all list elements as a native slice. This function never panics.

```yukino
load std.collections.list: List
load std.io: println

set list = init List<int64>()
list.push(10)
list.push(20)
set item = list.get(0)
println("Element: ${item}")
```

---

### 4.2 HashMap<K, V>

Hash table providing amortized O(1) key-value lookups. Supports string keys (`set`, `get`, `contains_key`, `remove`) and integer keys (`set_i64`, `get_i64`, `contains_key_i64`, `remove_i64`).

```yukino
public class HashMap<K, V>
```

#### set
```yukino
fun set(key: string, val: string)
```
Associates `val` with `key`. If the key already exists, its previous value is replaced. This function never panics.

#### get
```yukino
fun get(key: string) -> string?
```
Returns the string value mapped to `key`, or `null` if the key is not found. This function never panics.

#### contains_key
```yukino
fun contains_key(key: string) -> bool
```
Returns `true` if `key` exists in the hash table, otherwise `false`. This function never panics.

#### remove
```yukino
fun remove(key: string) -> bool
```
Removes the entry associated with `key`. Returns `true` if the entry was present and removed, otherwise `false`. This function never panics.

#### len
```yukino
fun len() -> int64
```
Returns the total number of key-value pairs in the table. This function never panics.

#### is_empty
```yukino
fun is_empty() -> bool
```
Returns `true` if the table contains zero entries, otherwise `false`. This function never panics.

#### clear
```yukino
fun clear()
```
Removes all key-value entries from the hash table. This function never panics.

#### set_i64 / get_i64 / contains_key_i64 / remove_i64
```yukino
fun set_i64(key: int64, val: int64)
fun get_i64(key: int64) -> int64?
fun contains_key_i64(key: int64) -> bool
fun remove_i64(key: int64) -> bool
```
Integer key variants for storing and retrieving `int64` key-value pairs. `get_i64` returns `null` when the key is absent. These functions never panic.

```yukino
load std.collections.hashmap: HashMap
load std.io: println

set map = init HashMap<string, string>()
map.set("name", "Yukino")
set name = map.get("name") ?? "Unknown"
println("Name: ${name}")
```

---

### 4.3 HashSet<T>

Hash-table-backed set of unique elements.

```yukino
public class HashSet<T>
```

#### add
```yukino
fun add(item: string) -> bool
```
Inserts `item` into the set. Returns `true` if the element was newly inserted, or `false` if it was already present. This function never panics.

#### contains
```yukino
fun contains(item: string) -> bool
```
Returns `true` if `item` exists in the set, otherwise `false`. This function never panics.

#### remove
```yukino
fun remove(item: string) -> bool
```
Removes `item` from the set. Returns `true` if the item was found and removed, otherwise `false`. This function never panics.

#### len
```yukino
fun len() -> int64
```
Returns the number of unique elements stored in the set. This function never panics.

#### is_empty
```yukino
fun is_empty() -> bool
```
Returns `true` if the set contains no elements, otherwise `false`. This function never panics.

#### clear
```yukino
fun clear()
```
Removes all elements from the set. This function never panics.

#### add_i64 / contains_i64 / remove_i64
```yukino
fun add_i64(val: int64) -> bool
fun contains_i64(val: int64) -> bool
fun remove_i64(val: int64) -> bool
```
Integer element variants for managing sets of `int64` values. These functions never panic.

```yukino
load std.collections.hashset: HashSet
load std.io: println

set tags = init HashSet<string>()
tags.add("system")
if tags.contains("system") {
    println("Tag found")
}
```

---

### 4.4 Deque<T>

Double-ended queue optimized for efficient element insertion and removal at both ends.

```yukino
public class Deque<T>
```

#### push_back / push_front
```yukino
fun push_back(val: int64)
fun push_front(val: int64)
```
Inserts `val` at the back or front of the deque respectively. These functions never panic.

#### pop_front / pop_back
```yukino
fun pop_front() -> int64?
fun pop_back() -> int64?
```
Removes and returns the element from the front or back of the deque, or `null` if the deque is empty. These functions never panic.

#### peek_front / peek_back
```yukino
fun peek_front() -> int64?
fun peek_back() -> int64?
```
Inspects the first or last element without removing it, returning `null` if the deque is empty. These functions never panic.

#### len
```yukino
fun len() -> int64
```
Returns the number of elements in the deque. This function never panics.

#### is_empty
```yukino
fun is_empty() -> bool
```
Returns `true` if the deque is empty, otherwise `false`. This function never panics.

#### clear
```yukino
fun clear()
```
Removes all elements from the deque. This function never panics.

```yukino
load std.collections.queue: Deque
load std.io: println

set dq = init Deque<int64>()
dq.push_back(100)
dq.push_front(200)
set first = dq.pop_front() ?? 0
println("First element: ${first}")
```

---

### 4.5 Queue<T>

First-in, first-out (FIFO) queue container.

```yukino
public class Queue<T>
```

#### push
```yukino
fun push(val: int64)
```
Appends element `val` to the end of the queue. This function never panics.

#### pop
```yukino
fun pop() -> int64?
```
Removes and returns the front element of the queue, or `null` if empty. This function never panics.

#### peek
```yukino
fun peek() -> int64?
```
Inspects the front element of the queue without removing it, or returns `null` if empty. This function never panics.

#### len
```yukino
fun len() -> int64
```
Returns the number of elements in the queue. This function never panics.

#### is_empty
```yukino
fun is_empty() -> bool
```
Returns `true` if the queue is empty, otherwise `false`. This function never panics.

#### clear
```yukino
fun clear()
```
Removes all elements from the queue. This function never panics.

```yukino
load std.collections.queue: Queue
load std.io: println

set q = init Queue<int64>()
q.push(1)
q.push(2)
set val = q.pop() ?? 0
println("Dequeued: ${val}")
```

---

## 5. Function Reference: Search and Verification (search)

### 5.1 acontains

Checks whether integer `item` exists in slice `arr`.

```yukino
public fun acontains(ref arr: int64[], item: int64) -> bool
```

Returns `true` if `item` is found in the slice, otherwise `false`.

This function never panics.

```yukino
load std.collections.search: acontains
load std.io: println

set numbers: int64[] = [10, 20, 30]
set exists = acontains(ref numbers, 20)
println("Found: ${exists}")
```

### 5.2 acontains_str

Checks whether string `item` exists in string slice `arr`.

```yukino
public fun acontains_str(ref arr: string[], item: string) -> bool
```

Returns `true` if the string is present in the slice, otherwise `false`.

This function never panics.

```yukino
load std.collections.search: acontains_str
load std.io: println

set names = ["alice", "bob", "charlie"]
set exists = acontains_str(ref names, "bob")
println("Found: ${exists}")
```

### 5.3 aindex_of

Finds the index of the first occurrence of integer `item` in slice `arr`.

```yukino
public fun aindex_of(ref arr: int64[], item: int64) -> int64
```

Returns the zero-based index `0 <= index < len`, or `-1` if not found.

This function never panics.

```yukino
load std.collections.search: aindex_of
load std.io: println

set values = [100, 200, 300]
set idx = aindex_of(ref values, 200)
println("Index: ${idx}")
```

### 5.4 aindex_of_str

Finds the index of the first occurrence of string `item` in string slice `arr`.

```yukino
public fun aindex_of_str(ref arr: string[], item: string) -> int64
```

Returns the zero-based match index `0 <= index < len`, or `-1` if absent.

This function never panics.

```yukino
load std.collections.search: aindex_of_str
load std.io: println

set fruits = ["apple", "pear", "banana"]
set pos = aindex_of_str(ref fruits, "pear")
println("Position: ${pos}")
```

### 5.5 afind

Finds the first element in slice `arr` for which `predicate` returns `true`.

```yukino
public fun afind(ref arr: int64[], predicate: fun(int64) -> bool) -> int64?
```

Returns the matching integer value, or `null` if no element satisfies the predicate.

This function never panics.

```yukino
load std.collections.search: afind
load std.io: println

fun is_positive(x: int64) -> bool {
    return x > 0
}

set nums: int64[] = [-3, -1, 4, 7]
set found = afind(ref nums, is_positive)
if found != null {
    println("First positive: ${found}")
}
```

---

## 6. Function Reference: Functional Transformations (transform)

### 6.1 amap

Applies transformation function `f` to each element of slice `arr` and returns a new array with the mapped values.

```yukino
public fun amap(ref arr: int64[], f: fun(int64) -> int64) -> int64[]
```

Returns a newly allocated array `int64[]` containing the transformed elements.

This function never panics.

```yukino
load std.collections.transform: amap
load std.io: println

fun double_val(x: int64) -> int64 {
    return x * 2
}

set orig: int64[] = [1, 2, 3]
set doubled = amap(ref orig, double_val)
println("First doubled: ${doubled[0]}")
```

### 6.2 afilter

Selects elements of slice `arr` for which `predicate` returns `true`, collecting them into a new array.

```yukino
public fun afilter(ref arr: int64[], predicate: fun(int64) -> bool) -> int64[]
```

Returns a newly allocated array `int64[]` containing only matching elements.

This function never panics.

```yukino
load std.collections.transform: afilter
load std.io: println

fun is_even(x: int64) -> bool {
    return x % 2 == 0
}

set all: int64[] = [1, 2, 3, 4, 5, 6]
set evens = afilter(ref all, is_even)
println("Even count: ${evens.len()}")
```

### 6.3 areduce

Reduces elements of slice `arr` to a single value using accumulator function `f` and starting with `initial`.

```yukino
public fun areduce(ref arr: int64[], initial: int64, f: fun(int64, int64) -> int64) -> int64
```

Returns the accumulated `int64` value.

This function never panics.

```yukino
load std.collections.transform: areduce
load std.io: println

fun sum(acc: int64, x: int64) -> int64 {
    return acc + x
}

set items: int64[] = [10, 20, 30]
set total = areduce(ref items, 0, sum)
println("Sum: ${total}")
```

### 6.4 afor_each / afor_each_str

Executes function `f` for each element in the given slice.

```yukino
public fun afor_each(ref arr: int64[], f: fun(int64))
public fun afor_each_str(ref arr: string[], f: fun(string))
```

These functions perform side effects over each element and do not return a value.

These functions never panic.

```yukino
load std.collections.transform: afor_each_str
load std.io: println

fun print_item(s: string) {
    println("Item: " + s)
}

set list = ["first", "second"]
afor_each_str(ref list, print_item)
```

---

## 7. Function Reference: In-Place Mutation (mut)

### 7.1 afill / afill_str

Fills all elements of the given mutable slice in place with `val`.

```yukino
public fun afill(ref ~arr: int64[], val: int64)
public fun afill_str(ref ~arr: string[], val: string)
```

These functions modify the slice in place via `ref ~arr` and return no value.

These functions never panic.

```yukino
load std.collections.mut: afill
load std.io: println

set data: int64[] = [1, 2, 3]
afill(ref ~data, 0)
println("Value: ${data[0]}")
```

### 7.2 areverse / areverse_str

Reverses the order of elements in the slice in place.

```yukino
public fun areverse(ref ~arr: int64[])
public fun areverse_str(ref ~arr: string[])
```

These functions reverse the element order in place and return no value.

These functions never panic.

```yukino
load std.collections.mut: areverse
load std.io: println

set order: int64[] = [1, 2, 3]
areverse(ref ~order)
println("First element: ${order[0]}")
```

### 7.3 asort / asort_str

Sorts elements in the slice in place in ascending order. Strings are sorted lexicographically.

```yukino
public fun asort(ref ~arr: int64[])
public fun asort_str(ref ~arr: string[])
```

These functions sort the slice in place and return no value.

These functions never panic.

```yukino
load std.collections.mut: asort
load std.io: println

set scores: int64[] = [45, 12, 89, 3]
asort(ref ~scores)
println("Lowest: ${scores[0]}")
```

### 7.4 asort_by

Sorts elements in the slice in place using custom comparator function `cmp`.

```yukino
public fun asort_by(ref ~arr: int64[], cmp: fun(int64, int64) -> int32)
```

The comparator `cmp(a, b)` must return a negative value if `a < b`, positive if `a > b`, and `0` if equal.

This function never panics.

```yukino
load std.collections.mut: asort_by
load std.io: println

fun cmp_desc(a: int64, b: int64) -> int32 {
    return cast(b - a, "int32")
}

set items: int64[] = [1, 5, 2]
asort_by(ref ~items, cmp_desc)
println("Highest: ${items[0]}")
```

---

## 8. Function Reference: String Utilities (strings)

### 8.1 ajoin

Concatenates the elements of string slice `arr` into a single string using `separator`.

```yukino
public fun ajoin(ref arr: string[], separator: string) -> string
```

Returns the joined `string`. If `arr` is empty, returns an empty string `""`.

This function never panics.

```yukino
load std.collections.str: ajoin
load std.io: println

set words = ["Yukino", "Systems", "Language"]
set title = ajoin(ref words, " ")
println(title)
```

---

[← Previous: Module std.fs](8-fs.md) | [Next: Module std.process →](10-process.md)
