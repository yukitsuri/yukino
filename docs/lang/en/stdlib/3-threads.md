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

[← Back: Module std.io](2-io.md) | [Next: Module std.platformio →](4-platformio.md)
