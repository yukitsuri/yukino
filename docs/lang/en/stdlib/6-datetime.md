# Module std.datetime

The `std.datetime` module provides tools for working with dates, time, time intervals, system and monotonic clocks, as well as formatting and parsing of string representations according to the ISO 8601 standard.

---

## 1. Import: load std.datetime

Importing the entire package:

```yukino
load std.datetime
```

The following are imported into the current scope:

- **Data structures:**
  - `DateTime` — calendar date and time with a timezone offset;
  - `Duration` — time interval with nanosecond precision;
  - `DateTimeParseError` — date/time string parsing error structure.
- **System and monotonic time:**
  - `now_utc`, `now_local`;
  - `steady_nanos`, `steady_millis`.
- **Calendar calculations:**
  - `is_leap_year`, `days_in_month`, `day_of_week`, `day_of_year`;
  - `make_datetime`, `from_date`.
- **Unix Epoch conversions:**
  - `from_unix_secs`, `from_unix_millis`, `from_unix_nanos`;
  - `to_unix_secs`, `to_unix_millis`, `to_unix_nanos`.
- **Timezones:**
  - `to_utc`, `to_local`, `with_offset`.
- **Time intervals (Duration):**
  - Factories: `duration_from_nanos`, `duration_from_micros`, `duration_from_millis`, `duration_from_secs`, `duration_from_mins`, `duration_from_hours`, `duration_from_days`;
  - Converters: `duration_to_nanos`, `duration_to_micros`, `duration_to_millis`, `duration_to_secs`, `duration_to_mins`, `duration_to_hours`, `duration_to_days`;
  - Arithmetic: `add_duration`, `sub_duration`, `diff`.
- **Classes:**
  - `Stopwatch` — monotonic nanosecond-precision stopwatch with pause, resume, reset, restart, and 64-bit counter overflow protection;
  - `Timer` — countdown timer with monotonic precision, pause, resume, reset, restart, and polling support.
- **Factory functions:**
  - `stopwatch_start` — instantiate and immediately start a `Stopwatch`;
  - `timer_start` — instantiate and immediately start a `Timer` with a given duration in milliseconds.
- **Formatting and parsing:**
  - `to_iso8601`, `format`, `parse_iso8601`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

No global constants are declared in module `std.datetime`.

---

## 3. Data and Error Structures

### 3.1 DateTime

Represents a specific moment of civil time:

```yukino
public struct DateTime {
    year: int32
    month: int32
    day: int32
    hour: int32
    minute: int32
    second: int32
    millisecond: int32
    nanosecond: int32
    utc_offset: int32
}
```

- `year` — the full year (for example, `2026`);
- `month` — the month in the range `1..12`;
- `day` — the day of the month in the range `1..31`;
- `hour` — the hour in the range `0..23`;
- `minute` — the minute in the range `0..59`;
- `second` — the second in the range `0..59`;
- `millisecond` — the millisecond in the range `0..999`;
- `nanosecond` — the nanosecond within the current second in the range `0..999999999`;
- `utc_offset` — the offset from UTC in seconds (for example, `+10800` for UTC+3).

### 2.2 Duration

Represents a difference in time or an interval length:

```yukino
public struct Duration {
    secs: int64
    nanos: int32
}
```

- `secs` — the number of whole seconds in the interval (may be negative);
- `nanos` — the fractional part in nanoseconds `0..999999999`.

### 2.3 DateTimeParseError

Raised via `panic` when parsing a string representation of a date fails:

```yukino
public struct DateTimeParseError {
    message: string
    code: int32
}
```

- `code = 1` — the string is too short, or an unexpected end of string while reading digits;
- `code = 2` — a non-digit character, or a missing `-` separator after the year;
- `code = 3` — a missing `-` separator after the month;
- `code = 4` — a missing `T` or space separator after the date;
- `code = 5` — an incomplete time component;
- `code = 6` — a missing `:` separator after the hour;
- `code = 7` — a missing `:` separator after the minute.

---

## 4. Classes

### 4.1 Stopwatch

Class for measuring elapsed real-time intervals with nanosecond precision based on monotonic system clocks. Supports starting, pausing, resuming, resetting, restarting, and overflow protection for 64-bit unsigned nanosecond counters (`uint64`).

```yukino
public class Stopwatch {
    public elapsed_ms: uint64
    public is_active: bool

    public fun init()
    public fun start()
    public fun pause()
    public fun resume()
    public fun stop()
    public fun reset()
    public fun restart() -> uint64
    public fun is_running() -> bool
    public fun elapsed_nanos() -> uint64
    public fun elapsed_millis() -> uint64
    public fun elapsed_secs() -> float64
}
```

Public fields:
- `elapsed_ms: uint64` — current accumulated time in milliseconds (synchronized during calls to `pause`, `elapsed_millis`, `elapsed_nanos`, `restart`, and `reset`);
- `is_active: bool` — boolean activity flag (`true` while the stopwatch is actively counting time).

#### Stopwatch Methods

##### start

Starts counting time using the monotonic clock. If the stopwatch is already running, repeated calls have no effect.

```yukino
public fun start()
```

Does not return a value. The method never panics.

##### pause

Pauses counting, accumulates elapsed nanoseconds into the internal counter, and updates the public `elapsed_ms` field. If the counter reaches the `uint64` limit, the accumulated value clamps at `0xffffffffffffffffu64`.

```yukino
public fun pause()
```

Does not return a value. The method never panics.

##### resume

Resumes counting after a pause, provided the stopwatch was paused and the `uint64` limit has not been exhausted.

```yukino
public fun resume()
```

Does not return a value. The method never panics.

##### stop

Stops counting and freezes the accumulated time (an alias for `pause()`).

```yukino
public fun stop()
```

Does not return a value. The method never panics.

##### reset

Clears all accumulated measurements and resets the stopwatch to its initial inactive state (`elapsed_ms = 0u64`, `is_active = false`).

```yukino
public fun reset()
```

Does not return a value. The method never panics.

##### restart

Resets accumulated time, immediately begins a new measurement, and returns the accumulated time in milliseconds before reset.

```yukino
public fun restart() -> uint64
```

Returns a `uint64` value. The method never panics.

##### is_running

Checks whether the stopwatch is currently actively counting time.

```yukino
public fun is_running() -> bool
```

Returns a `bool` value. The method never panics.

##### elapsed_nanos

Returns total accumulated time in nanoseconds. If running, the delta from current monotonic time is added. When exceeding 64-bit bounds, it clamps to `0xffffffffffffffffu64`.

```yukino
public fun elapsed_nanos() -> uint64
```

Returns a `uint64` value. The method never panics.

##### elapsed_millis

Returns total accumulated time in milliseconds (`elapsed_nanos() / 1000000u64`).

```yukino
public fun elapsed_millis() -> uint64
```

Returns a `uint64` value. The method never panics.

##### elapsed_secs

Returns total accumulated time in seconds with fractional precision as a 64-bit float.

```yukino
public fun elapsed_secs() -> float64
```

Returns a `float64` value. The method never panics.

#### Stopwatch Thread Example (std.threads)

```yukino
load std.datetime: Stopwatch, stopwatch_start
load std.threads: spawn, join, sleep
load std.io: println

#[thunk]
public fun background_task() {
    set sw: Stopwatch = stopwatch_start()
    sleep(60u64)
    sw.pause()
    sleep(20u64)
    sw.resume()
    sleep(40u64)
    sw.stop()
    println("Background task finished in ${sw.elapsed_ms} ms")
}

fun main() {
    set benchmark = stopwatch_start()
    set handle = spawn(cast(background_task, "ptr<void>"))
    join(handle)
    benchmark.stop()
    println("Total time including join: ${benchmark.elapsed_millis()} ms")
}
```

---

### 4.2 Timer

Monotonic countdown timer with nanosecond precision. Designed for tracking deadlines, timeouts, and polling loops both in the main thread and inside asynchronous thread workers (`std.threads`). Supports setting intervals, starting, pausing, resetting, restarting, and 64-bit overflow protection.

```yukino
public class Timer {
    public remaining_ms: uint64
    public is_active: bool

    public fun init(duration_ms: uint64 = 0u64)
    public fun start(duration_ms: uint64 = 0u64)
    public fun pause()
    public fun resume()
    public fun stop()
    public fun reset()
    public fun restart(duration_ms: uint64 = 0u64)
    public fun is_expired() -> bool
    public fun is_running() -> bool
    public fun remaining_nanos() -> uint64
    public fun remaining_millis() -> uint64
    public fun elapsed_nanos() -> uint64
    public fun elapsed_millis() -> uint64
    public fun tick() -> uint64
}
```

Public fields:
- `remaining_ms: uint64` — remaining time in milliseconds until expiration (decreases towards zero, updated on polling and calls to `tick`, `remaining_millis`, `is_expired`);
- `is_active: bool` — timer activity flag (`true` while actively counting down and not expired; becomes `false` when expired or stopped/paused).

#### Timer Methods

##### init

Initializes the timer with a given duration in milliseconds in an inactive (stopped) state.

```yukino
public fun init(duration_ms: uint64 = 0u64)
```

Does not return a value. The method never panics.

##### start

Starts countdown. If `duration_ms > 0u64` is provided, sets a new target interval. If 0, uses previously preserved remaining duration or initial target duration.

```yukino
public fun start(duration_ms: uint64 = 0u64)
```

Does not return a value. The method never panics.

##### pause

Pauses countdown, calculates elapsed monotonic time, and updates the remaining time in `remaining_ms`.

```yukino
public fun pause()
```

Does not return a value. The method never panics.

##### resume

Resumes countdown if the timer is paused and remaining time is greater than zero (`remaining_nanos > 0`).

```yukino
public fun resume()
```

Does not return a value. The method never panics.

##### stop

Stops the timer and clears remaining time (`remaining_ms = 0u64`, `is_active = false`).

```yukino
public fun stop()
```

Does not return a value. The method never panics.

##### reset

Resets the timer back to its initial target duration in an inactive state.

```yukino
public fun reset()
```

Does not return a value. The method never panics.

##### restart

Resets and immediately starts the timer with either its original duration or a newly specified `duration_ms`.

```yukino
public fun restart(duration_ms: uint64 = 0u64)
```

Does not return a value. The method never panics.

##### is_expired

Checks if the timer has expired (`remaining_nanos == 0`). If expiration is detected during the call, the timer automatically transitions to inactive (`is_active = false`).

```yukino
public fun is_expired() -> bool
```

Returns a `bool` value. The method never panics.

##### is_running

Returns `true` if the timer is actively counting and has not yet expired.

```yukino
public fun is_running() -> bool
```

Returns a `bool` value. The method never panics.

##### remaining_nanos

Returns remaining duration in nanoseconds. If active, computes real-time delta from the monotonic clock; returns `0u64` when expired.

```yukino
public fun remaining_nanos() -> uint64
```

Returns a `uint64` value. The method never panics.

##### remaining_millis

Returns remaining duration in milliseconds (`remaining_nanos() / 1000000u64`) and updates `remaining_ms`.

```yukino
public fun remaining_millis() -> uint64
```

Returns a `uint64` value. The method never panics.

##### elapsed_nanos

Returns elapsed duration from the configured target interval in nanoseconds.

```yukino
public fun elapsed_nanos() -> uint64
```

Returns a `uint64` value. The method never panics.

##### elapsed_millis

Returns elapsed duration from the configured target interval in milliseconds.

```yukino
public fun elapsed_millis() -> uint64
```

Returns a `uint64` value. The method never panics.

##### tick

Polls the timer: updates internal state and returns remaining milliseconds (equivalent to `remaining_millis()`). Convenient for thread sleep polling loops.

```yukino
public fun tick() -> uint64
```

Returns a `uint64` value. The method never panics.

#### Timer Thread Example (std.threads)

```yukino
load std.datetime: Timer, timer_start
load std.threads: spawn, join, sleep
load std.io: println

#[thunk]
public fun timer_worker() {
    set t: Timer = timer_start(150u64)
    while !t.is_expired() {
        sleep(30u64)
    }
    println("Thread timer expired. remaining_ms: ${t.remaining_ms}")
}

fun main() {
    set handle = spawn(cast(timer_worker, "ptr<void>"))
    join(handle)
    println("Background timer worker finished.")
}
```

---

## 5. Function Reference

### 3.1 now_utc

Returns the current system time in the UTC timezone (`utc_offset = 0`).

```yukino
public fun now_utc() -> DateTime
```

Returns a `DateTime` structure with the fields of the current moment in time in UTC.

This function never panics.

```yukino
load std.datetime: now_utc, DateTime
load std.io: println

set dt = now_utc()
println("Current UTC year: ${dt.year}")
```

### 3.2 now_local

Returns the current system time adjusted to the local timezone of the operating system.

```yukino
public fun now_local() -> DateTime
```

Returns a `DateTime` structure with the OS timezone offset in the `utc_offset` field.

This function never panics.

```yukino
load std.datetime: now_local
load std.io: println

set local_time = now_local()
println("Local hour: ${local_time.hour}")
```

### 3.3 steady_nanos

Returns the value of the monotonic hardware timer in nanoseconds from an arbitrary reference point.

```yukino
public fun steady_nanos() -> int64
```

Returns a continuously increasing 64-bit integer of nanoseconds. Not affected by jumps caused by system clock adjustments.

This function never panics.

```yukino
load std.datetime: steady_nanos
load std.io: println

set t1 = steady_nanos()
// Performing the operation being measured
set t2 = steady_nanos()
println("Elapsed: ${t2 - t1} ns")
```

### 3.4 steady_millis

Returns the value of the monotonic hardware timer in milliseconds.

```yukino
public fun steady_millis() -> int64
```

Returns a 64-bit integer of milliseconds as `int64`.

This function never panics.

```yukino
load std.datetime: steady_millis
load std.io: println

set start = steady_millis()
println("Measurement start: ${start} ms")
```

### 3.5 is_leap_year

Determines whether the specified year is a leap year according to the Gregorian calendar.

```yukino
public fun is_leap_year(year: int32) -> bool
```

Returns `true` if the year is divisible by 4 (and at the same time either not divisible by 100, or divisible by 400), otherwise `false`.

This function never panics.

```yukino
load std.datetime: is_leap_year
load std.io: println

set leap = is_leap_year(2024)
println("2024 is leap: ${leap}") // true
```

### 3.6 days_in_month

Returns the number of days in the specified month of the given year, accounting for leap years.

```yukino
public fun days_in_month(year: int32, month: int32) -> int32
```

Returns the number of days in the month (from 28 to 31).

This function never panics.

```yukino
load std.datetime: days_in_month
load std.io: println

set days = days_in_month(2024, 2)
println("Days in February: ${days}") // 29
```

### 3.7 day_of_week

Computes the day of the week for the specified date.

```yukino
public fun day_of_week(ref dt: DateTime) -> int32
```

Returns the ordinal number of the day of the week: from 1 (Monday) to 7 (Sunday).

This function never panics.

```yukino
load std.datetime: now_utc, day_of_week
load std.io: println

set now = now_utc()
set dow = day_of_week(ref now)
println("Day of week: ${dow}")
```

### 3.8 day_of_year

Computes the ordinal number of the day in the year for the specified date.

```yukino
public fun day_of_year(ref dt: DateTime) -> int32
```

Returns an integer from 1 to 366.

This function never panics.

```yukino
load std.datetime: now_utc, day_of_year
load std.io: println

set now = now_utc()
set doy = day_of_year(ref now)
println("Day of year: ${doy}")
```

### 3.9 make_datetime

Constructs a `DateTime` structure with arbitrarily specified date and time components.

```yukino
public fun make_datetime(year: int32, month: int32, day: int32, hour: int32 = 0, minute: int32 = 0, second: int32 = 0, millisecond: int32 = 0, offset: int32 = 0) -> DateTime
```

Returns the constructed `DateTime` structure.

This function never panics.

```yukino
load std.datetime: make_datetime
load std.io: println

set dt = make_datetime(2026, 10, 15, 12, 0, 0, 0, 10800)
println("Date: ${dt.year}-${dt.month}-${dt.day}")
```

### 3.10 from_date

Creates a `DateTime` instance for the start of the specified calendar day (00:00:00 UTC).

```yukino
public fun from_date(year: int32, month: int32, day: int32) -> DateTime
```

Returns a `DateTime` structure with the time 00:00:00 and offset 0.

This function never panics.

```yukino
load std.datetime: from_date
load std.io: println

set start = from_date(2026, 1, 1)
println("Start of year: ${start.year}")
```

### 3.11 from_unix_secs

Creates a `DateTime` structure from the number of seconds elapsed since January 1, 1970 UTC (Unix Epoch).

```yukino
public fun from_unix_secs(secs: int64, offset: int32 = 0) -> DateTime
```

Returns a `DateTime` structure with the specified timezone offset `offset`.

This function never panics.

```yukino
load std.datetime: from_unix_secs
load std.io: println

set dt = from_unix_secs(1700000000, 0)
println("Year: ${dt.year}")
```

### 3.12 from_unix_millis

Creates a `DateTime` structure from the number of milliseconds of the Unix Epoch.

```yukino
public fun from_unix_millis(ms: int64, offset: int32 = 0) -> DateTime
```

Returns a `DateTime` structure.

This function never panics.

```yukino
load std.datetime: from_unix_millis
load std.io: println

set dt = from_unix_millis(1700000000000, 0)
println("Year: ${dt.year}")
```

### 3.13 from_unix_nanos

Creates a `DateTime` structure from the number of nanoseconds of the Unix Epoch.

```yukino
public fun from_unix_nanos(nanos: int64, offset: int32 = 0) -> DateTime
```

Returns a `DateTime` structure.

This function never panics.

```yukino
load std.datetime: from_unix_nanos
load std.io: println

set dt = from_unix_nanos(1700000000000000000, 0)
println("Year: ${dt.year}")
```

### 3.14 to_unix_secs

Computes the number of whole seconds elapsed from the Unix Epoch to the specified moment in time.

```yukino
public fun to_unix_secs(ref dt: DateTime) -> int64
```

Returns the number of Unix Epoch seconds in UTC as `int64`.

This function never panics.

```yukino
load std.datetime: now_utc, to_unix_secs
load std.io: println

set now = now_utc()
set ts = to_unix_secs(ref now)
println("Unix timestamp: ${ts}")
```

### 3.15 to_unix_millis

Computes the number of milliseconds from the Unix Epoch to the specified moment in time.

```yukino
public fun to_unix_millis(ref dt: DateTime) -> int64
```

Returns the number of milliseconds as `int64`.

This function never panics.

```yukino
load std.datetime: now_utc, to_unix_millis
load std.io: println

set now = now_utc()
set ms = to_unix_millis(ref now)
println("Epoch ms: ${ms}")
```

### 3.16 to_unix_nanos

Computes the number of nanoseconds from the Unix Epoch to the specified moment in time.

```yukino
public fun to_unix_nanos(ref dt: DateTime) -> int64
```

Returns the number of nanoseconds as `int64`.

This function never panics.

```yukino
load std.datetime: now_utc, to_unix_nanos
load std.io: println

set now = now_utc()
set ns = to_unix_nanos(ref now)
println("Epoch nanos: ${ns}")
```

### 3.17 to_utc

Converts the passed date and time moment to the UTC timezone (offset 0).

```yukino
public fun to_utc(ref dt: DateTime) -> DateTime
```

Returns a new `DateTime` structure with a zero offset `utc_offset = 0`.

This function never panics.

```yukino
load std.datetime: now_local, to_utc
load std.io: println

set local_dt = now_local()
set utc_dt = to_utc(ref local_dt)
println("UTC hour: ${utc_dt.hour}")
```

### 3.18 to_local

Converts the date and time to the current local timezone of the operating system.

```yukino
public fun to_local(ref dt: DateTime) -> DateTime
```

Returns a `DateTime` with the local timezone offset of the operating system.

This function never panics.

```yukino
load std.datetime: now_utc, to_local
load std.io: println

set utc = now_utc()
set local_time = to_local(ref utc)
println("Local hour: ${local_time.hour}")
```

### 3.19 with_offset

Converts the date and time to the timezone with the specified offset from UTC in seconds.

```yukino
public fun with_offset(ref dt: DateTime, offset_secs: int32) -> DateTime
```

Returns a `DateTime` structure with the updated offset `offset_secs`.

This function never panics.

```yukino
load std.datetime: now_utc, with_offset
load std.io: println

set utc = now_utc()
set tokyo = with_offset(ref utc, 32400) // UTC+9
println("Hour in Tokyo: ${tokyo.hour}")
```

### 3.20 duration_from_nanos

Creates a `Duration` time interval from a number of nanoseconds.

```yukino
public fun duration_from_nanos(nanos: int64) -> Duration
```

Returns a `Duration` structure.

This function never panics.

```yukino
load std.datetime: duration_from_nanos
load std.io: println

set d = duration_from_nanos(500000000)
println("Seconds in interval: ${d.secs}")
```

### 3.21 duration_from_micros

Creates a `Duration` time interval from a number of microseconds.

```yukino
public fun duration_from_micros(us: int64) -> Duration
```

Returns a `Duration` structure.

This function never panics.

```yukino
load std.datetime: duration_from_micros
load std.io: println

set d = duration_from_micros(1500000)
println("Seconds: ${d.secs}")
```

### 3.22 duration_from_millis

Creates a `Duration` time interval from a number of milliseconds.

```yukino
public fun duration_from_millis(ms: int64) -> Duration
```

Returns a `Duration` structure.

This function never panics.

```yukino
load std.datetime: duration_from_millis
load std.io: println

set d = duration_from_millis(2500)
println("Seconds: ${d.secs}")
```

### 3.23 duration_from_secs

Creates a `Duration` time interval from a number of seconds.

```yukino
public fun duration_from_secs(secs: int64) -> Duration
```

Returns a `Duration` structure.

This function never panics.

```yukino
load std.datetime: duration_from_secs
load std.io: println

set d = duration_from_secs(60)
println("Seconds: ${d.secs}")
```

### 3.24 duration_from_mins

Creates a `Duration` time interval from a number of minutes.

```yukino
public fun duration_from_mins(mins: int64) -> Duration
```

Returns a `Duration` structure.

This function never panics.

```yukino
load std.datetime: duration_from_mins
load std.io: println

set d = duration_from_mins(30)
println("Seconds in 30 min: ${d.secs}")
```

### 3.25 duration_from_hours

Creates a `Duration` time interval from a number of hours.

```yukino
public fun duration_from_hours(hours: int64) -> Duration
```

Returns a `Duration` structure.

This function never panics.

```yukino
load std.datetime: duration_from_hours
load std.io: println

set d = duration_from_hours(24)
println("Seconds in a day: ${d.secs}")
```

### 3.26 duration_from_days

Creates a `Duration` time interval from a number of calendar days (of 86400 seconds each).

```yukino
public fun duration_from_days(days: int64) -> Duration
```

Returns a `Duration` structure.

This function never panics.

```yukino
load std.datetime: duration_from_days
load std.io: println

set week = duration_from_days(7)
println("Seconds in a week: ${week.secs}")
```

### 3.27 duration_to_nanos

Converts a `Duration` time interval into the total number of nanoseconds.

```yukino
public fun duration_to_nanos(ref d: Duration) -> int64
```

Returns the total number of nanoseconds of the interval as `int64`.

This function never panics.

```yukino
load std.datetime: duration_from_millis, duration_to_nanos
load std.io: println

set d = duration_from_millis(10)
println("Nanoseconds: ${duration_to_nanos(ref d)}")
```

### 3.28 duration_to_micros

Converts a time interval into microseconds.

```yukino
public fun duration_to_micros(ref d: Duration) -> int64
```

Returns the number of microseconds as `int64`.

This function never panics.

```yukino
load std.datetime: duration_from_millis, duration_to_micros
load std.io: println

set d = duration_from_millis(5)
println("Microseconds: ${duration_to_micros(ref d)}")
```

### 3.29 duration_to_millis

Converts a time interval into milliseconds.

```yukino
public fun duration_to_millis(ref d: Duration) -> int64
```

Returns the number of milliseconds as `int64`.

This function never panics.

```yukino
load std.datetime: duration_from_secs, duration_to_millis
load std.io: println

set d = duration_from_secs(3)
println("Milliseconds: ${duration_to_millis(ref d)}")
```

### 3.30 duration_to_secs

Converts a time interval into seconds as a floating-point number.

```yukino
public fun duration_to_secs(ref d: Duration) -> float64
```

Returns the exact number of seconds as `float64`.

This function never panics.

```yukino
load std.datetime: duration_from_millis, duration_to_secs
load std.io: println

set d = duration_from_millis(1500)
println("Seconds: ${duration_to_secs(ref d)}") // 1.5
```

### 3.31 duration_to_mins

Converts a time interval into minutes as a floating-point number.

```yukino
public fun duration_to_mins(ref d: Duration) -> float64
```

Returns the number of minutes as `float64`.

This function never panics.

```yukino
load std.datetime: duration_from_secs, duration_to_mins
load std.io: println

set d = duration_from_secs(90)
println("Minutes: ${duration_to_mins(ref d)}") // 1.5
```

### 3.32 duration_to_hours

Converts a time interval into hours as a floating-point number.

```yukino
public fun duration_to_hours(ref d: Duration) -> float64
```

Returns the number of hours as `float64`.

This function never panics.

```yukino
load std.datetime: duration_from_mins, duration_to_hours
load std.io: println

set d = duration_from_mins(120)
println("Hours: ${duration_to_hours(ref d)}") // 2.0
```

### 3.33 duration_to_days

Converts a time interval into days as a floating-point number.

```yukino
public fun duration_to_days(ref d: Duration) -> float64
```

Returns the number of days as `float64`.

This function never panics.

```yukino
load std.datetime: duration_from_hours, duration_to_days
load std.io: println

set d = duration_from_hours(36)
println("Days: ${duration_to_days(ref d)}") // 1.5
```

### 3.34 add_duration

Adds a time interval to the specified date and time moment.

```yukino
public fun add_duration(ref dt: DateTime, ref d: Duration) -> DateTime
```

Returns a new `DateTime` structure.

This function never panics.

```yukino
load std.datetime: now_utc, duration_from_days, add_duration
load std.io: println

set now = now_utc()
set d = duration_from_days(1)
set tomorrow = add_duration(ref now, ref d)
println("Tomorrow's day: ${tomorrow.day}")
```

### 3.35 sub_duration

Subtracts a time interval from the specified date and time moment.

```yukino
public fun sub_duration(ref dt: DateTime, ref d: Duration) -> DateTime
```

Returns a new `DateTime` structure.

This function never panics.

```yukino
load std.datetime: now_utc, duration_from_hours, sub_duration
load std.io: println

set now = now_utc()
set d = duration_from_hours(2)
set past = sub_duration(ref now, ref d)
println("Hour 2 hours ago: ${past.hour}")
```

### 3.36 diff

Computes the difference between two moments in time (`dt1 - dt2`).

```yukino
public fun diff(ref dt1: DateTime, ref dt2: DateTime) -> Duration
```

Returns a `Duration` structure representing the difference in time.

This function never panics.

```yukino
load std.datetime: now_utc, duration_from_days, add_duration, diff, duration_to_days
load std.io: println

set now = now_utc()
set delta = duration_from_days(7)
set next_week = add_duration(ref now, ref delta)
set difference = diff(ref next_week, ref now)
println("Difference in days: ${duration_to_days(ref difference)}")
```

### 3.37 to_iso8601

Formats the date and time into a canonical string according to the international ISO 8601 standard.

```yukino
public fun to_iso8601(ref dt: DateTime) -> string
```

Returns the formatted string (for example, `"2026-10-15T12:00:00.000Z"`).

This function never panics.

```yukino
load std.datetime: now_utc, to_iso8601
load std.io: println

set now = now_utc()
println("ISO 8601: " + to_iso8601(ref now))
```

### 3.38 format

Formats the date and time according to a custom template string.

```yukino
public fun format(ref dt: DateTime, fmt: string) -> string
```

Returns the string formatted according to the passed template.

Template specifiers:
- `%Y` — four-digit year (`2026`);
- `%m` — two-digit month (`01`..`12`);
- `%d` — two-digit day (`01`..`31`);
- `%H` — two-digit hour (`00`..`23`);
- `%M` — two-digit minute (`00`..`59`);
- `%S` — two-digit second (`00`..`59`);
- `%f` — three-digit millisecond (`000`..`999`);
- `%z` — timezone offset in the `+HHMM` format without a colon (`+0000` for a zero offset, `+0300` for UTC+3).

The `%%` specifier is not implemented: unrecognized `%` sequences, including `%%`, are output to the result unchanged.

This function never panics.

```yukino
load std.datetime: now_local, format
load std.io: println

set now = now_local()
set pretty = format(ref now, "%d.%m.%Y %H:%M")
println("Date: ${pretty}")
```

### 3.39 parse_iso8601

Parses an ISO 8601 date and time string and constructs a `DateTime` object.

```yukino
public fun parse_iso8601(s: string) -> DateTime
```

Returns the parsed `DateTime` structure.

If the string format is incorrect or contains invalid characters, raises a panic with the `DateTimeParseError` structure:
- `code = 1` — the string is too short, or an unexpected end of string while reading digits;
- `code = 2` — a non-digit character, or a missing `-` separator after the year;
- `code = 3` — a missing `-` separator after the month;
- `code = 4` — a missing `T` or space separator after the date;
- `code = 5` — an incomplete time component;
- `code = 6` — a missing `:` separator after the hour;
- `code = 7` — a missing `:` separator after the minute.

The timezone in the string is not validated: if the character after the time is not `Z`, `z`, `+`, or `-`, it is ignored and the offset remains zero.

```yukino
load std.datetime: parse_iso8601, DateTime, DateTimeParseError
load std.io: println

sus {
    set dt = parse_iso8601("2026-10-15T12:30:00Z")
    println("Parsed year: ${dt.year}")
} catch err: DateTimeParseError {
    println("Parsing error: ${err.message}")
}
```

### 3.40 stopwatch_start

Creates a new `Stopwatch` instance and immediately starts monotonic time counting.

```yukino
public fun stopwatch_start() -> Stopwatch
```

Returns an initialized and running `Stopwatch` object. The returned value cannot be `null`.

The function never panics.

```yukino
load std.datetime: stopwatch_start, Stopwatch
load std.threads: sleep
load std.io: println

set sw: Stopwatch = stopwatch_start()
sleep(50u64)
sw.stop()
println("Elapsed time: ${sw.elapsed_millis()} ms")
```

### 3.41 timer_start

Creates a new `Timer` instance with a given duration in milliseconds and immediately starts countdown.

```yukino
public fun timer_start(duration_ms: uint64) -> Timer
```

Returns an initialized and running `Timer` object. The returned value cannot be `null`.

The function never panics.

```yukino
load std.datetime: timer_start, Timer
load std.threads: sleep
load std.io: println

set timer: Timer = timer_start(100u64)
while !timer.is_expired() {
    sleep(25u64)
}
println("Timer finished, remaining_ms: ${timer.remaining_ms}")
```

---

[← Back: Module std.math](5-math.md) | [Next: Module std.random →](7-random.md)
