# Модуль std.threads

Модуль `std.threads` предоставляет возможности низкоуровневого управления потоками операционной системы и примитивами взаимного исключения (мьютексами): создание фоновых потоков, ожидание завершения (join), управление паузами (`sleep`), уступка времени процессора (`yield_now`), определение параллелизма процессора и синхронизация разделяемых ресурсов.

---

## 1. Подключение: load std.threads

Импорт всего пакета:

```yukino
load std.threads
```

В текущую область видимости импортируются все публичные сущности модуля `std.threads`:

- **Структуры ошибок:**
  - `ThreadError` — ошибки работы с потоками и мьютексами (коды `301`–`307`).
- **Функции жизненного цикла потоков:**
  - `spawn` — запуск новой системной задачи в отдельном потоке;
  - `join` — блокирующее ожидание завершения потока;
  - `is_finished` — неблокирующая проверка завершения потока;
  - `thread_id` — получение идентификатора потока по дескриптору;
  - `current_id` — получение идентификатора текущего активного потока;
  - `sleep` — приостановка текущего потока на заданное число миллисекунд;
  - `yield_now` — добровольная уступка кванта времени планировщику ОС;
  - `hardware_concurrency` — определение числа доступных аппаратных логических ядер процессора.
- **Функции мьютексов (взаимных блокировок):**
  - `mutex_create` — создание нового мьютекса рантайма;
  - `mutex_lock` — захват мьютекса;
  - `mutex_unlock` — освобождение захваченного мьютекса;
  - `mutex_destroy` — удаление мьютекса из внутренней таблицы рантайма.

Все эти сущности также доступны при загрузке всей стандартной библиотеки: `load std`.

---

## 2. Константы

В модуле `std.threads` глобальные константы не объявлены.

---

## 3. Структуры ошибок (паники)

### 3.1 ThreadError

Структура ошибки, генерируемая при нарушении контрактов или системных сбоях в операциях с потоками и блокировками.

```yukino
public struct ThreadError {
    message: string
    code: int32
}
```

- `code: 301` — в функцию `spawn` передан нулевой указатель задачи (`null`);
- `code: 302` — системная ошибка выделения или запуска потока ОС (`spawn`);
- `code: 303` — системная ошибка при попытке присоединить поток (`join`);
- `code: 304` — ошибка создания мьютекса (`mutex_create`);
- `code: 305` — передан невалидный дескриптор мьютекса (0) в `mutex_lock`;
- `code: 306` — передан невалидный дескриптор мьютекса (0) в `mutex_unlock`;
- `code: 307` — передан невалидный дескриптор мьютекса (0) в `mutex_destroy`.

---

## 4. Справочник функций

### 4.1 spawn

Запускает новый поток операционной системы, исполняющий функцию по сырому указателю `task`.

```yukino
public fun spawn(task: ptr<void>, arg: ptr<void> = null) -> uint64
```

Возвращает 64-битный дескриптор созданного потока как `uint64`.

В случае если `task == null`, вызывает панику со структурой `ThreadError` (`code = 301`). Если операционная система не смогла выделить или запустить поток, вызывает панику со структурой `ThreadError` (`code = 302`).

```yukino
load std.threads: spawn, join
load std.io: println

#[thunk]
fun worker_routine() {
    println("Фоновый поток запущен")
}

fun main() {
    set handle = spawn(cast(worker_routine, "ptr<void>"))
    join(handle)
}
```

### 4.2 join

Блокирует выполнение текущего потока до тех пор, пока целевой поток с дескриптором `handle` не завершит работу.

```yukino
public fun join(handle: uint64) -> int32
```

Возвращает `0` при успешном завершении ожидания: любое ненулевое значение статуса приводит к панике, поэтому при обычном возврате значение всегда равно `0`.

В случае ошибки ожидания или передачи невалидного дескриптора вызывает панику со структурой `ThreadError` (`code = 303`).

```yukino
load std.threads: spawn, join
load std.io: println

set h = spawn(task_ptr, null)
set status = join(h)
println("Поток завершен: ${status}")
```

### 4.3 is_finished

Выполняет неблокирующий опрос статуса потока с дескриптором `handle`.

```yukino
public fun is_finished(handle: uint64) -> bool
```

Возвращает `true`, если целевой поток уже завершил свое выполнение, иначе `false`. Если дескриптор отсутствует во внутренней таблице рантайма (например, поток уже присоединен через `join` или дескриптор неизвестен), функция также возвращает `true`.

Функция никогда не паникует.

```yukino
load std.threads: is_finished, sleep
load std.io: println

while !is_finished(handle) {
    sleep(50u64)
}
println("Поток отработал")
```

### 4.4 thread_id

Возвращает числовой идентификатор потока по дескриптору `handle`.

```yukino
public fun thread_id(handle: uint64) -> uint64
```

Возвращает 64-битный идентификатор потока как `uint64`; в текущей реализации он численно совпадает с переданным дескриптором `handle` (функция возвращает аргумент без изменений).

Функция никогда не паникует.

```yukino
load std.threads: thread_id
load std.io: println

set tid = thread_id(handle)
println("TID потока: ${tid}")
```

### 4.5 sleep

Приостанавливает выполнение текущего потока минимум на указанное число миллисекунд `ms`.

```yukino
public fun sleep(ms: uint64)
```

Приостанавливает поток и не возвращает значений. Не нагружает процессор во время ожидания.

Функция никогда не паникует.

```yukino
load std.threads: sleep
load std.io: println

println("Пауза 1 секунда...")
sleep(1000u64)
println("Возобновление работы")
```

### 4.6 yield_now

Добровольно передает текущий квант процессорного времени планировщику операционной системы, позволяя другим готовым к исполнению потокам выполнить работу.

```yukino
public fun yield_now()
```

Уступает квант времени процессора и не возвращает значений.

Функция никогда не паникует.

```yukino
load std.threads: yield_now

yield_now()
```

### 4.7 current_id

Возвращает системный 64-битный идентификатор текущего выполняющегося потока.

```yukino
public fun current_id() -> uint64
```

Возвращает числовой идентификатор потока как `uint64`.

Функция никогда не паникует.

```yukino
load std.threads: current_id
load std.io: println

println("ID текущего потока: ${current_id()}")
```

### 4.8 hardware_concurrency

Возвращает количество логических вычислительных ядер процессора, доступных операционной системе.

```yukino
public fun hardware_concurrency() -> uint32
```

Возвращает количество ядер процессора как `uint32`. Если определить число ядер не удалось, возвращает значение по умолчанию `1`.

Функция никогда не паникует.

```yukino
load std.threads: hardware_concurrency
load std.io: println

set cores = hardware_concurrency()
println("Доступно логических ядер: ${cores}")
```

### 4.9 mutex_create

Создает новый мьютекс рантайма (спин-блокировку) и регистрирует его во внутренней таблице рантайма.

```yukino
public fun mutex_create() -> uint64
```

Возвращает 64-битный дескриптор созданного мьютекса как `uint64`.

В случае сбоя создания мьютекса вызывает панику со структурой `ThreadError` (`code = 304`).

```yukino
load std.threads: mutex_create
load std.io: println

set m = mutex_create()
println("Мьютекс создан: ${m}")
```

### 4.10 mutex_lock

Блокирует мьютекс `m`; если мьютекс уже захвачен другим потоком, вызывающий поток активно ожидает его освобождения, уступая квант процессора планировщику.

```yukino
public fun mutex_lock(m: uint64)
```

Захватывает блокировку мьютекса и не возвращает значений.

В случае передачи нулевого дескриптора (`m == 0`) вызывает панику со структурой `ThreadError` (`code = 305`).

```yukino
load std.threads: mutex_lock, mutex_unlock
load std.io: println

mutex_lock(m)
println("Критическая секция")
mutex_unlock(m)
```

### 4.11 mutex_unlock

Освобождает ранее захваченный мьютекс `m`, предоставляя возможность другим ожидающим потокам захватить ресурс.

```yukino
public fun mutex_unlock(m: uint64)
```

Освобождает блокировку мьютекса и не возвращает значений.

В случае передачи невалидного дескриптора (`m == 0`) вызывает панику со структурой `ThreadError` (`code = 306`).

```yukino
load std.threads: mutex_lock, mutex_unlock
load std.io: println

mutex_lock(m)
defer mutex_unlock(m)
println("Безопасное выполнение в критической секции")
```

### 4.12 mutex_destroy

Уничтожает мьютекс `m`, удаляя его из внутренней таблицы рантайма.

```yukino
public fun mutex_destroy(m: uint64) -> bool
```

Возвращает `true` при успешном уничтожении блокировки.

В случае передачи невалидного дескриптора (`m == 0`) вызывает панику со структурой `ThreadError` (`code = 307`).

```yukino
load std.threads: mutex_create, mutex_destroy
load std.io: println

set m = mutex_create()
set ok = mutex_destroy(m)
println("Мьютекс уничтожен: ${ok}")
```

---

## 5. Модуль std.sync: примитивы синхронизации

Модуль `std.sync` является компаньоном `std.threads` и предоставляет высокоуровневые объектно-ориентированные примитивы синхронизации, разделения данных и межпоточного взаимодействия. Модуль импортируется отдельно и явно:

```yukino
load std.sync
```

> [!NOTE]
> Модуль `std.threads` не реэкспортирует сущности `std.sync`. Чтобы использовать примитивы синхронизации, подключайте `load std.sync`.

### 5.1 Структура ошибки SyncError

```yukino
public struct SyncError {
    message: string
    code: int32
}
```

- `code: 401` — системная ошибка создания примитива или передан невалидный дескриптор (`0`).

### 5.2 Класс Mutex (взаимная блокировка)

Объектно-ориентированная обертка над системным мьютексом:
- `fun init()` — конструктор (`init Mutex()` или фабрика `mutex()`).
- `fun lock()` — захват блокировки.
- `fun try_lock() -> bool` — неблокирующая попытка захвата (`true`, если мьютекс успешно захвачен).
- `fun unlock()` — освобождение блокировки.
- `fun destroy()` — освобождение внутренних ресурсов.
- `fun get_handle() -> uint64` — дескриптор мьютекса.

```yukino
load std.sync: mutex, Mutex
load std.io: println

set m = mutex()
m.lock()
println("Критическая секция под защитой Mutex")
m.unlock()
m.destroy()
```

### 5.3 Класс RwLock (блокировка читателей-писателей)

Предоставляет параллельный доступ для множества читателей либо эксклюзивный доступ для одного писателя:
- `fun init()` / `rwlock() -> RwLock` — создание блокировки.
- `fun read()` — блокирующий захват на чтение.
- `fun try_read() -> bool` — неблокирующая попытка чтения.
- `fun unlock_read()` — освобождение блокировки чтения.
- `fun write()` — блокирующий захват на запись.
- `fun try_write() -> bool` — неблокирующая попытка записи.
- `fun unlock_write()` — освобождение блокировки записи.
- `fun destroy()` — освобождение ресурсов.

```yukino
load std.sync: rwlock
load std.io: println

set rw = rwlock()
rw.read()
println("Параллельное чтение")
rw.unlock_read()

rw.write()
println("Эксклюзивная запись")
rw.unlock_write()
rw.destroy()
```

### 5.4 Класс CondVar (условная переменная)

Позволяет потокам ожидать выполнения условий, освобождая связанный мьютекс на время ожидания:
- `fun init()` / `condvar() -> CondVar` — создание условной переменной.
- `fun wait(ref m: Mutex)` — освобождает мьютекс `m`, ожидает уведомления и снова захватывает `m`.
- `fun wait_handle(mutex_id: uint64)` — вариант ожидания по сырому дескриптору.
- `fun wait_timeout(ref m: Mutex, ms: uint64) -> bool` — ожидание с таймаутом (возвращает `true`, если условие получено до истечения `ms`).
- `fun notify_one()` — пробуждение одного ожидающего потока.
- `fun notify_all()` — пробуждение всех ожидающих потоков.
- `fun destroy()` — освобождение ресурсов.

```yukino
load std.sync: mutex, condvar
load std.io: println

set cv = condvar()
set m = mutex()

m.lock()
set signaled = cv.wait_timeout(ref m, 50u64)
m.unlock()

cv.notify_one()
cv.destroy()
m.destroy()
```

### 5.5 Класс AtomicInt64 и класс AtomicBool (атомарные примитивы)

Предоставляют аппаратные lock-free атомарные операции для счетчиков и флагов:
- `fun init(init_val)` / `atomic_i64(init)` и `atomic_bool(init)` — инициализация.
- `.get() -> int64` / `.get() -> bool` — чтение значения (`load`).
- `.set(val)` — запись значения (`store`).
- `.fetch_add(val: int64) -> int64` — атомарное прибавление (возвращает предыдущее значение).
- `.fetch_sub(val: int64) -> int64` — атомарное вычитание.
- `.swap(val) -> int64 / bool` — атомарный обмен.
- `.compare_exchange(expected, desired) -> bool` — Compare-And-Swap (CAS).
- `.destroy()` — освобождение дескриптора.

```yukino
load std.sync: atomic_i64, atomic_bool
load std.io: println

set counter = atomic_i64(0)
counter.fetch_add(1)
println("Счетчик: " + cast(counter.get(), "string"))

set flag = atomic_bool(false)
set ok = flag.compare_exchange(false, true)
println("CAS успешен: " + cast(ok, "string"))
```

### 5.6 Класс Once (однократная инициализация)

Гарантирует, что переданная подпрограмма будет выполнена ровно один раз, даже если несколько потоков вызывают ее одновременно:
- `fun init()` / `once() -> Once` — создание.
- `fun call_once(task: ptr<void>, arg: ptr<void> = null)` — вызов инициализирующей процедуры.
- `fun is_completed() -> bool` — проверка завершения инициализации.
- `fun destroy()` — освобождение ресурсов.

```yukino
load std.sync: once
load std.io: println

set init_once = once()
if !init_once.is_completed() {
    init_once.call_once(null, null)
}
println("Инициализация завершена: " + cast(init_once.is_completed(), "string"))
```

### 5.7 Класс Channel (межпоточные очереди сообщений)

Предоставляет потокобезопасный канал передачи 64-битных данных (`int64`, числовые дескрипторы или сырые указатели `ptr<void>`):
- `fun init(bound: int64 = 0)` / `channel(bound = 0) -> Channel` — создание канала (`bound = 0` — неограниченный, `bound > 0` — ограниченный).
- `fun send_i64(val: int64) -> bool` — отправка числа в канал.
- `fun recv_i64() -> int64?` — блокирующее чтение (возвращает `null`, если канал закрыт и пуст).
- `fun try_recv_i64() -> int64?` — неблокирующее чтение (`null`, если пуст или закрыт).
- `fun recv_timeout_i64(ms: int64) -> int64?` — чтение с таймаутом.
- `fun send_ptr(p: ptr<void>) -> bool` — отправка указателя.
- `fun recv_ptr() -> ptr<void>?` — чтение указателя.
- `fun try_recv_ptr() -> ptr<void>?` — неблокирующее чтение указателя.
- `fun close()` — закрытие канала для новых отправок.
- `fun is_closed() -> bool` — проверка закрытия.
- `fun len() -> int64` — текущее количество элементов в очереди.
- `fun destroy()` — освобождение ресурсов канала.

```yukino
load std.sync: channel
load std.io: println

set ch = channel(10)
ch.send_i64(100)
ch.send_i64(200)

set item = ch.recv_i64()
if item != null {
    println("Получено значение: " + cast(item ?? 0, "string"))
}
ch.close()
ch.destroy()
```

### 5.8 Класс Semaphore (счетный семафор)

Управляет доступом к пулу из $N$ ресурсов:
- `fun init(permits: int64 = 1)` / `semaphore(permits = 1) -> Semaphore` — создание семафора.
- `fun acquire() -> bool` — блокирующий захват одного разрешения.
- `fun try_acquire() -> bool` — неблокирующая попытка захвата (`true`, если получено).
- `fun release() -> bool` — освобождение разрешения.
- `fun destroy()` — освобождение ресурсов.

```yukino
load std.sync: semaphore
load std.io: println

set sem = semaphore(2)
if sem.acquire() {
    println("Разрешение получено")
    set _ = sem.release()
}
sem.destroy()
```

### 5.9 Класс Barrier (барьер синхронизации)

Блокирует группу из $N$ потоков до тех пор, пока все они не дойдут до общей точки рандеву:
- `fun init(parties: uint32)` / `barrier(parties) -> Barrier` — создание барьера.
- `fun wait() -> bool` — ожидание всех участников. Возвращает `true` ровно для одного потока (лидера фазы) и `false` для остальных.
- `fun destroy()` — освобождение ресурсов.

```yukino
load std.sync: barrier
load std.io: println

set bar = barrier(2u32)
// Потоки вызывают bar.wait()
bar.destroy()
```

---

[← Назад: Модуль std.io](2-io.md) | [Вперед: Модуль std.platformio →](4-platformio.md)
