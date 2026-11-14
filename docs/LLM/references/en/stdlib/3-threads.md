# Module std.threads

The `std.threads` module provides low-level control over operating system threads and mutual exclusion primitives (mutexes): spawning background threads, waiting for completion (join), pause management (`sleep`), yielding processor time (`yield_now`), detecting processor concurrency, and synchronizing shared resources.

---

## 1. Import: load std.threads

Importing the entire package:

```yukino
load std.threads
```

All public entities of the `std.threads` module are imported into the current scope:

- **Error structures:**
  - `ThreadError` — thread and mutex operation errors (codes `301`–`307`).
- **Thread lifecycle functions:**
  - `spawn` — launching a new system task in a separate thread;
  - `join` — blocking wait for thread completion;
  - `is_finished` — non-blocking check of thread completion;
  - `thread_id` — obtaining the identifier of a thread by its handle;
  - `current_id` — obtaining the identifier of the currently active thread;
  - `sleep` — suspending the current thread for a given number of milliseconds;
  - `yield_now` — voluntarily yielding the time slice to the OS scheduler;
  - `hardware_concurrency` — detecting the number of available hardware logical processor cores.
- **Mutex (mutual exclusion) functions:**
  - `mutex_create` — creating a new runtime mutex;
  - `mutex_lock` — acquiring a mutex;
  - `mutex_unlock` — releasing an acquired mutex;
  - `mutex_destroy` — removing a mutex from the runtime's internal table.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

The `std.threads` module does not declare global constants.

---

## 3. Error Structures (Panics)

### 3.1 ThreadError

The error structure raised on contract violations or system failures in thread and lock operations.

```yukino
public struct ThreadError {
    message: string
    code: int32
}
```

- `code: 301` — a null task pointer (`null`) was passed to the `spawn` function;
- `code: 302` — system error allocating or starting an OS thread (`spawn`);
- `code: 303` — system error when attempting to join a thread (`join`);
- `code: 304` — mutex creation error (`mutex_create`);
- `code: 305` — an invalid mutex handle (0) was passed to `mutex_lock`;
- `code: 306` — an invalid mutex handle (0) was passed to `mutex_unlock`;
- `code: 307` — an invalid mutex handle (0) was passed to `mutex_destroy`.

---

## 4. Function Reference

### 4.1 spawn

Starts a new operating system thread executing a function via the raw pointer `task`.

```yukino
public fun spawn(task: ptr<void>, arg: ptr<void> = null) -> uint64
```

Returns the 64-bit handle of the created thread as `uint64`.

If `task == null`, panics with the `ThreadError` structure (`code = 301`). If the operating system failed to allocate or start the thread, panics with the `ThreadError` structure (`code = 302`).

```yukino
load std.threads: spawn, join
load std.io: println

#[thunk]
fun worker_routine() {
    println("Background thread started")
}

fun main() {
    set handle = spawn(cast(worker_routine, "ptr<void>"))
    join(handle)
}
```

### 4.2 join

Blocks execution of the current thread until the target thread with handle `handle` finishes its work.

```yukino
public fun join(handle: uint64) -> int32
```

Returns `0` on successful completion of the wait: any non-zero status value causes a panic, so on a normal return the value is always `0`.

On a wait error or an invalid handle, panics with the `ThreadError` structure (`code = 303`).

```yukino
load std.threads: spawn, join
load std.io: println

set h = spawn(task_ptr, null)
set status = join(h)
println("Thread finished: ${status}")
```

### 4.3 is_finished

Performs a non-blocking poll of the status of the thread with handle `handle`.

```yukino
public fun is_finished(handle: uint64) -> bool
```

Returns `true` if the target thread has already finished its execution, otherwise `false`. If the handle is missing from the runtime's internal table (for example, the thread has already been joined or the handle is unknown), the function also returns `true`.

The function never panics.

```yukino
load std.threads: is_finished, sleep
load std.io: println

while !is_finished(handle) {
    sleep(50u64)
}
println("Thread completed")
```

### 4.4 thread_id

Returns the numeric thread identifier for the handle `handle`.

```yukino
public fun thread_id(handle: uint64) -> uint64
```

Returns the 64-bit thread identifier as `uint64`; in the current implementation it is numerically equal to the passed handle `handle` (the function returns the argument unchanged).

The function never panics.

```yukino
load std.threads: thread_id
load std.io: println

set tid = thread_id(handle)
println("Thread TID: ${tid}")
```

### 4.5 sleep

Suspends execution of the current thread for at least the specified number of milliseconds `ms`.

```yukino
public fun sleep(ms: uint64)
```

Suspends the thread and returns no value. It does not load the processor while waiting.

The function never panics.

```yukino
load std.threads: sleep
load std.io: println

println("Pausing for 1 second...")
sleep(1000u64)
println("Resuming work")
```

### 4.6 yield_now

Voluntarily yields the current processor time slice to the operating system scheduler, allowing other threads that are ready to execute to do their work.

```yukino
public fun yield_now()
```

Yields the processor time slice and returns no value.

The function never panics.

```yukino
load std.threads: yield_now

yield_now()
```

### 4.7 current_id

Returns the system 64-bit identifier of the currently executing thread.

```yukino
public fun current_id() -> uint64
```

Returns the numeric thread identifier as `uint64`.

The function never panics.

```yukino
load std.threads: current_id
load std.io: println

println("Current thread ID: ${current_id()}")
```

### 4.8 hardware_concurrency

Returns the number of logical processor cores available to the operating system.

```yukino
public fun hardware_concurrency() -> uint32
```

Returns the number of processor cores as `uint32`. If the core count cannot be determined, returns the default value `1`.

The function never panics.

```yukino
load std.threads: hardware_concurrency
load std.io: println

set cores = hardware_concurrency()
println("Logical cores available: ${cores}")
```

### 4.9 mutex_create

Creates a new runtime mutex (a spinlock) and registers it in the runtime's internal table.

```yukino
public fun mutex_create() -> uint64
```

Returns the 64-bit handle of the created mutex as `uint64`.

On a mutex creation failure, panics with the `ThreadError` structure (`code = 304`).

```yukino
load std.threads: mutex_create
load std.io: println

set m = mutex_create()
println("Mutex created: ${m}")
```

### 4.10 mutex_lock

Locks the mutex `m`; if the mutex is already held by another thread, the calling thread actively waits for its release, yielding its processor time slice to the scheduler.

```yukino
public fun mutex_lock(m: uint64)
```

Acquires the mutex lock and returns no value.

If a null handle is passed (`m == 0`), panics with the `ThreadError` structure (`code = 305`).

```yukino
load std.threads: mutex_lock, mutex_unlock
load std.io: println

mutex_lock(m)
println("Critical section")
mutex_unlock(m)
```

### 4.11 mutex_unlock

Releases the previously acquired mutex `m`, giving other waiting threads the opportunity to acquire the resource.

```yukino
public fun mutex_unlock(m: uint64)
```

Releases the mutex lock and returns no value.

If an invalid handle is passed (`m == 0`), panics with the `ThreadError` structure (`code = 306`).

```yukino
load std.threads: mutex_lock, mutex_unlock
load std.io: println

mutex_lock(m)
defer mutex_unlock(m)
println("Safe execution in a critical section")
```

### 4.12 mutex_destroy

Destroys the mutex `m` by removing it from the runtime's internal table.

```yukino
public fun mutex_destroy(m: uint64) -> bool
```

Returns `true` on successful destruction of the lock.

If an invalid handle is passed (`m == 0`), panics with the `ThreadError` structure (`code = 307`).

```yukino
load std.threads: mutex_create, mutex_destroy
load std.io: println

set m = mutex_create()
set ok = mutex_destroy(m)
println("Mutex destroyed: ${ok}")
```

---

## 5. Module std.sync: Synchronization Primitives

The `std.sync` module serves as a companion to `std.threads`, delivering high-level object-oriented synchronization primitives, data-sharing utilities, and cross-thread communication tools. The module is imported independently and explicitly:

```yukino
load std.sync
```

> [!NOTE]
> The `std.threads` module does not re-export `std.sync`. To use synchronization primitives, import `load std.sync`.

### 5.1 Error Structure: SyncError

```yukino
public struct SyncError {
    message: string
    code: int32
}
```

- `code: 401` — runtime error creating a primitive or invalid handle (`0`) passed.

### 5.2 Class Mutex (Mutual Exclusion Lock)

An object-oriented wrapper around the system mutex:
- `fun init()` — constructor (`init Mutex()` or factory `mutex()`).
- `fun lock()` — acquires the lock.
- `fun try_lock() -> bool` — non-blocking lock attempt (`true` if successfully acquired).
- `fun unlock()` — releases the lock.
- `fun destroy()` — frees internal resources.
- `fun get_handle() -> uint64` — raw mutex handle.

```yukino
load std.sync: mutex, Mutex
load std.io: println

set m = mutex()
m.lock()
println("Critical section protected by Mutex")
m.unlock()
m.destroy()
```

### 5.3 Class RwLock (Reader-Writer Lock)

Allows concurrent read access for multiple readers, or exclusive access for a single writer:
- `fun init()` / `rwlock() -> RwLock` — creates the lock.
- `fun read()` — blocking read acquisition.
- `fun try_read() -> bool` — non-blocking read attempt.
- `fun unlock_read()` — releases the read lock.
- `fun write()` — blocking write acquisition.
- `fun try_write() -> bool` — non-blocking write attempt.
- `fun unlock_write()` — releases the write lock.
- `fun destroy()` — frees resources.

```yukino
load std.sync: rwlock
load std.io: println

set rw = rwlock()
rw.read()
println("Concurrent read")
rw.unlock_read()

rw.write()
println("Exclusive write")
rw.unlock_write()
rw.destroy()
```

### 5.4 Class CondVar (Condition Variable)

Enables threads to wait for conditions while releasing the associated mutex during the wait:
- `fun init()` / `condvar() -> CondVar` — creates the condition variable.
- `fun wait(ref m: Mutex)` — releases mutex `m`, waits for notification, and re-acquires `m`.
- `fun wait_handle(mutex_id: uint64)` — wait variant accepting a raw mutex handle.
- `fun wait_timeout(ref m: Mutex, ms: uint64) -> bool` — timed wait (returns `true` if signaled before `ms` elapsed).
- `fun notify_one()` — wakes up one waiting thread.
- `fun notify_all()` — wakes up all waiting threads.
- `fun destroy()` — frees resources.

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

### 5.5 Class AtomicInt64 and Class AtomicBool (Atomic Primitives)

Provide hardware lock-free atomic operations for counters and flags:
- `fun init(init_val)` / `atomic_i64(init)` and `atomic_bool(init)` — initialization.
- `.get() -> int64` / `.get() -> bool` — reads value (`load`).
- `.set(val)` — writes value (`store`).
- `.fetch_add(val: int64) -> int64` — atomic addition (returns previous value).
- `.fetch_sub(val: int64) -> int64` — atomic subtraction.
- `.swap(val) -> int64 / bool` — atomic swap.
- `.compare_exchange(expected, desired) -> bool` — Compare-And-Swap (CAS).
- `.destroy()` — frees handle.

```yukino
load std.sync: atomic_i64, atomic_bool
load std.io: println

set counter = atomic_i64(0)
counter.fetch_add(1)
println("Counter: " + cast(counter.get(), "string"))

set flag = atomic_bool(false)
set ok = flag.compare_exchange(false, true)
println("CAS succeeded: " + cast(ok, "string"))
```

### 5.6 Class Once (One-Time Initialization)

Guarantees that an initialization routine runs exactly once, even if multiple threads invoke it concurrently:
- `fun init()` / `once() -> Once` — creation.
- `fun call_once(task: ptr<void>, arg: ptr<void> = null)` — executes the procedure once.
- `fun is_completed() -> bool` — checks whether initialization completed.
- `fun destroy()` — frees resources.

```yukino
load std.sync: once
load std.io: println

set init_once = once()
if !init_once.is_completed() {
    init_once.call_once(null, null)
}
println("Initialization done: " + cast(init_once.is_completed(), "string"))
```

### 5.7 Class Channel (Cross-Thread Message Queues)

Provides a thread-safe message passing channel for 64-bit values (`int64`, descriptors, or raw pointers `ptr<void>`):
- `fun init(bound: int64 = 0)` / `channel(bound = 0) -> Channel` — creates a channel (`bound = 0` unbounded, `bound > 0` bounded).
- `fun send_i64(val: int64) -> bool` — sends an integer into the channel.
- `fun recv_i64() -> int64?` — blocking receive (`null` if closed and empty).
- `fun try_recv_i64() -> int64?` — non-blocking receive (`null` if empty or closed).
- `fun recv_timeout_i64(ms: int64) -> int64?` — receive with timeout.
- `fun send_ptr(p: ptr<void>) -> bool` — sends a pointer into the channel.
- `fun recv_ptr() -> ptr<void>?` — receives a pointer.
- `fun try_recv_ptr() -> ptr<void>?` — non-blocking pointer receive.
- `fun close()` — closes channel for sending.
- `fun is_closed() -> bool` — checks closure.
- `fun len() -> int64` — current item count.
- `fun destroy()` — frees channel resources.

```yukino
load std.sync: channel
load std.io: println

set ch = channel(10)
ch.send_i64(100)
ch.send_i64(200)

set item = ch.recv_i64()
if item != null {
    println("Received: " + cast(item ?? 0, "string"))
}
ch.close()
ch.destroy()
```

### 5.8 Class Semaphore (Counting Semaphore)

Controls access to a pool of $N$ shared resources:
- `fun init(permits: int64 = 1)` / `semaphore(permits = 1) -> Semaphore` — creates semaphore.
- `fun acquire() -> bool` — blocking permit acquisition.
- `fun try_acquire() -> bool` — non-blocking permit attempt (`true` if granted).
- `fun release() -> bool` — releases a permit.
- `fun destroy()` — frees resources.

```yukino
load std.sync: semaphore
load std.io: println

set sem = semaphore(2)
if sem.acquire() {
    println("Permit acquired")
    set _ = sem.release()
}
sem.destroy()
```

### 5.9 Class Barrier (Synchronization Barrier)

Blocks a set of $N$ threads until all threads reach the common rendezvous point:
- `fun init(parties: uint32)` / `barrier(parties) -> Barrier` — creates barrier.
- `fun wait() -> bool` — waits for all participants. Returns `true` for exactly one thread (phase leader) and `false` for others.
- `fun destroy()` — frees resources.

```yukino
load std.sync: barrier
load std.io: println

set bar = barrier(2u32)
// Worker threads call bar.wait()
bar.destroy()
```

---

[← Back: Module std.io](2-io.md) | [Next: Module std.platformio →](4-platformio.md)
