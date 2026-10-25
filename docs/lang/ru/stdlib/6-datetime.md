# Модуль std.datetime

Модуль `std.datetime` предоставляет инструменты для работы с датой, временем, временными интервалами, системными и монотонными часами, а также форматированием и разбором строковых представлений по стандарту ISO 8601.

---

## 1. Подключение: load std.datetime

Импорт всего пакета:

```yukino
load std.datetime
```

В область видимости импортируются:

- **Структуры данных:**
  - `DateTime` — календарная дата и время с учетом смещения часового пояса;
  - `Duration` — временной интервал с наносекундной точностью;
  - `DateTimeParseError` — структура ошибки разбора строки даты/времени.
- **Получение системного и монотонного времени:**
  - `now_utc`, `now_local`;
  - `steady_nanos`, `steady_millis`.
- **Календарные расчеты:**
  - `is_leap_year`, `days_in_month`, `day_of_week`, `day_of_year`;
  - `make_datetime`, `from_date`.
- **Преобразования Unix Epoch:**
  - `from_unix_secs`, `from_unix_millis`, `from_unix_nanos`;
  - `to_unix_secs`, `to_unix_millis`, `to_unix_nanos`.
- **Часовые пояса:**
  - `to_utc`, `to_local`, `with_offset`.
- **Временные интервалы (Duration):**
  - Фабрики: `duration_from_nanos`, `duration_from_micros`, `duration_from_millis`, `duration_from_secs`, `duration_from_mins`, `duration_from_hours`, `duration_from_days`;
  - Конвертеры: `duration_to_nanos`, `duration_to_micros`, `duration_to_millis`, `duration_to_secs`, `duration_to_mins`, `duration_to_hours`, `duration_to_days`;
  - Арифметика: `add_duration`, `sub_duration`, `diff`.
- **Классы:**
  - `Stopwatch` — секундомер с наносекундной точностью, поддержкой паузы, возобновления, сброса, перезапуска и защитой от переполнения 64-битного счетчика;
  - `Timer` — таймер обратного отсчета с монотонным отсчетом, поддержкой паузы, возобновления, сброса, перезапуска и поллинга оставшегося времени.
- **Вспомогательные фабричные функции:**
  - `stopwatch_start` — создание и немедленный запуск секундомера `Stopwatch`;
  - `timer_start` — создание и немедленный запуск таймера `Timer` на заданное число миллисекунд.
- **Форматирование и парсинг:**
  - `to_iso8601`, `format`, `parse_iso8601`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.datetime` глобальные константы не объявлены.

---

## 3. Структуры данных и ошибок (паники)

### 3.1 DateTime

Представляет конкретный момент гражданского времени:

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

- `year` — полный год (например, `2026`);
- `month` — месяц в диапазоне `1..12`;
- `day` — день месяца в диапазоне `1..31`;
- `hour` — час в диапазоне `0..23`;
- `minute` — минута в диапазоне `0..59`;
- `second` — секунда в диапазоне `0..59`;
- `millisecond` — миллисекунда в диапазоне `0..999`;
- `nanosecond` — наносекунда внутри текущей секунды в диапазоне `0..999999999`;
- `utc_offset` — смещение относительно UTC в секундах (например, `+10800` для UTC+3).

### 2.2 Duration

Представляет разницу во времени или продолжительность интервала:

```yukino
public struct Duration {
    secs: int64
    nanos: int32
}
```

- `secs` — количество полных секунд интервала (может быть отрицательным);
- `nanos` — дробная часть в наносекундах `0..999999999`.

### 2.3 DateTimeParseError

Возбуждается через `panic` при ошибке разбора строкового представления даты:

```yukino
public struct DateTimeParseError {
    message: string
    code: int32
}
```

- `code = 1` — строка слишком коротка либо непредвиденный конец строки при чтении цифр;
- `code = 2` — нецифровой символ либо отсутствует разделитель `-` после года;
- `code = 3` — отсутствует разделитель `-` после месяца;
- `code = 4` — отсутствует разделитель `T` или пробел после даты;
- `code = 5` — неполная компонента времени;
- `code = 6` — отсутствует разделитель `:` после часа;
- `code = 7` — отсутствует разделитель `:` после минуты.

---

## 4. Классы

### 4.1 Stopwatch

Класс для измерения прошедшего физического интервала времени с наносекундной точностью на основе монотонных системных часов. Поддерживает запуск, паузу, возобновление, сброс, перезапуск и защиту от переполнения 64-битного беззнакового счетчика наносекунд (`uint64`).

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

Публичные поля:
- `elapsed_ms: uint64` — текущее накопленное время в миллисекундах (синхронизируется при вызовах `pause`, `elapsed_millis`, `elapsed_nanos`, `restart` и `reset`);
- `is_active: bool` — логический флаг активности (имеет значение `true`, пока секундомер запущен и ведет отсчет).

#### Методы Stopwatch

##### start

Запускает отсчет времени по монотонным часам. Если секундомер уже находится в активном состоянии, повторный вызов игнорируется.

```yukino
public fun start()
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### pause

Приостанавливает отсчет времени, аккумулирует прошедшие наносекунды во внутреннем счетчике и обновляет публичное поле `elapsed_ms`. Если отсчет достигает предела `uint64`, накопленное значение фиксируется на максимальном значении `0xffffffffffffffffu64`.

```yukino
public fun pause()
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### resume

Возобновляет отсчет времени после паузы, если секундомер был остановлен и лимит `uint64` еще не исчерпан.

```yukino
public fun resume()
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### stop

Останавливает отсчет времени и фиксирует накопленное значение (псевдоним для `pause()`).

```yukino
public fun stop()
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### reset

Сбрасывает все накопленные значения и переводит секундомер в начальное неактивное состояние (`elapsed_ms = 0u64`, `is_active = false`).

```yukino
public fun reset()
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### restart

Сбрасывает накопленное время, немедленно запускает новый отсчет и возвращает количество миллисекунд, зафиксированное до момента сброса.

```yukino
public fun restart() -> uint64
```

Возвращает значение типа `uint64`. Метод никогда не вызывает панику.

##### is_running

Проверяет, находится ли секундомер в процессе отсчета времени.

```yukino
public fun is_running() -> bool
```

Возвращает логическое значение типа `bool`. Метод никогда не вызывает панику.

##### elapsed_nanos

Вычисляет общее накопленное время в наносекундах. Если отсчет продолжается, к накопленному времени прибавляется дельта текущего времени монотонных часов. При превышении диапазона 64 бит отсчет останавливается на значении `0xffffffffffffffffu64`.

```yukino
public fun elapsed_nanos() -> uint64
```

Возвращает значение типа `uint64`. Метод никогда не вызывает панику.

##### elapsed_millis

Возвращает общее накопленное время в миллисекундах (`elapsed_nanos() / 1000000u64`).

```yukino
public fun elapsed_millis() -> uint64
```

Возвращает значение типа `uint64`. Метод никогда не вызывает панику.

##### elapsed_secs

Возвращает общее накопленное время в секундах с дробной наносекундной частью в виде числа с плавающей точкой двойной точности.

```yukino
public fun elapsed_secs() -> float64
```

Возвращает значение типа `float64`. Метод никогда не вызывает панику.

#### Пример использования Stopwatch с потоками (std.threads)

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
    println("Фоновая задача выполнена за ${sw.elapsed_ms} мс")
}

fun main() {
    set benchmark = stopwatch_start()
    set handle = spawn(cast(background_task, "ptr<void>"))
    join(handle)
    benchmark.stop()
    println("Полное время выполнения: ${benchmark.elapsed_millis()} мс")
}
```

---

### 4.2 Timer

Класс таймера обратного отсчета с наносекундной монотонной точностью. Разработан для отслеживания таймаутов, дедлайнов и циклов поллинга как в основном потоке, так и в параллельных задачах (`std.threads`). Поддерживает установку интервала, запуск, паузу, сброс, перезапуск и защиту от переполнения 64-битного целого.

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

Публичные поля:
- `remaining_ms: uint64` — оставшееся до истечения таймера время в миллисекундах (уменьшается в сторону нуля, обновляется при опросе и вызовах `tick`, `remaining_millis`, `is_expired`);
- `is_active: bool` — флаг активности таймера (равен `true`, пока отсчет активен и время не истекло; автоматически становится `false` при обнулении или вызове `stop`/`pause`).

#### Методы Timer

##### init

Инициализирует таймер заданной продолжительностью в миллисекундах. Начальное состояние — неактивное (отсчет не запущен).

```yukino
public fun init(duration_ms: uint64 = 0u64)
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### start

Запускает обратный отсчет. Если передан аргумент `duration_ms > 0u64`, таймер настраивается на новую продолжительность. Если аргумент равен 0, используется ранее сохраненное оставшееся время либо исходная целевая продолжительность.

```yukino
public fun start(duration_ms: uint64 = 0u64)
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### pause

Приостанавливает обратный отсчет, вычисляет прошедшую с момента старта дельту времени и сохраняет скорректированный остаток в поле `remaining_ms`.

```yukino
public fun pause()
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### resume

Возобновляет обратный отсчет после паузы, если на таймере еще осталось неизрасходованное время (`remaining_nanos > 0`).

```yukino
public fun resume()
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### stop

Останавливает таймер и принудительно обнуляет оставшееся время (`remaining_ms = 0u64`, `is_active = false`).

```yukino
public fun stop()
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### reset

Сбрасывает таймер в начальное состояние: остаток времени восстанавливается до исходной целевой длительности, флаг активности сбрасывается в `false`.

```yukino
public fun reset()
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### restart

Выполняет `reset()` и немедленно запускает отсчет заново с исходной либо новой продолжительностью `duration_ms`.

```yukino
public fun restart(duration_ms: uint64 = 0u64)
```

Функция ничего не возвращает. Метод никогда не вызывает панику.

##### is_expired

Проверяет, истек ли установленный интервал времени (достиг ли остаток нуля). Если таймер истекает во время вызова, он автоматически переводится в неактивное состояние.

```yukino
public fun is_expired() -> bool
```

Возвращает логическое значение типа `bool`. Метод никогда не вызывает панику.

##### is_running

Возвращает `true`, если таймер активен и отведенное время еще не истекло.

```yukino
public fun is_running() -> bool
```

Возвращает логическое значение типа `bool`. Метод никогда не вызывает панику.

##### remaining_nanos

Возвращает оставшееся время в наносекундах. Если таймер запущен, вычисляет актуальный остаток по монотонным часам; если время истекло, возвращает `0u64`.

```yukino
public fun remaining_nanos() -> uint64
```

Возвращает значение типа `uint64`. Метод никогда не вызывает панику.

##### remaining_millis

Возвращает оставшееся время в миллисекундах (`remaining_nanos() / 1000000u64`) и обновляет публичное поле `remaining_ms`.

```yukino
public fun remaining_millis() -> uint64
```

Возвращает значение типа `uint64`. Метод никогда не вызывает панику.

##### elapsed_nanos

Возвращает наносекунды, прошедшие от целевого интервала с момента запуска.

```yukino
public fun elapsed_nanos() -> uint64
```

Возвращает значение типа `uint64`. Метод никогда не вызывает панику.

##### elapsed_millis

Возвращает миллисекунды, прошедшие от целевого интервала с момента запуска.

```yukino
public fun elapsed_millis() -> uint64
```

Возвращает значение типа `uint64`. Метод никогда не вызывает панику.

##### tick

Метод для поллинга таймера в цикле: обновляет состояние таймера и возвращает оставшееся число миллисекунд (эквивалентно `remaining_millis()`). Удобен для циклов ожидания с `std.threads::sleep`.

```yukino
public fun tick() -> uint64
```

Возвращает значение типа `uint64`. Метод никогда не вызывает панику.

#### Пример использования Timer с потоками (std.threads)

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
    println("Таймер в потоке истек. Остаток мс: ${t.remaining_ms}")
}

fun main() {
    set handle = spawn(cast(timer_worker, "ptr<void>"))
    join(handle)
    println("Фоновый поток с таймером завершил работу.")
}
```

---

## 5. Справочник функций

### 3.1 now_utc

Возвращает текущее системное время в часовом поясе UTC (`utc_offset = 0`).

```yukino
public fun now_utc() -> DateTime
```

Возвращает структуру `DateTime` с заполненными полями текущего момента времени в UTC.

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, DateTime
load std.io: println

set dt = now_utc()
println("Текущий год UTC: ${dt.year}")
```

### 3.2 now_local

Возвращает текущее системное время с учетом локального часового пояса операционной системы.

```yukino
public fun now_local() -> DateTime
```

Возвращает структуру `DateTime` со смещением часового пояса ОС в поле `utc_offset`.

Функция никогда не паникует.

```yukino
load std.datetime: now_local
load std.io: println

set local_time = now_local()
println("Локальный час: ${local_time.hour}")
```

### 3.3 steady_nanos

Возвращает значение монотонного аппаратного таймера в наносекундах от произвольной точки отсчета.

```yukino
public fun steady_nanos() -> int64
```

Возвращает непрерывно возрастающее 64-битное целое число наносекунд. Не подвержено скачкам при переводе системных часов.

Функция никогда не паникует.

```yukino
load std.datetime: steady_nanos
load std.io: println

set t1 = steady_nanos()
// Выполнение измеряемой операции
set t2 = steady_nanos()
println("Затрачено: ${t2 - t1} нс")
```

### 3.4 steady_millis

Возвращает значение монотонного аппаратного таймера в миллисекундах.

```yukino
public fun steady_millis() -> int64
```

Возвращает 64-битное целое число миллисекунд как `int64`.

Функция никогда не паникует.

```yukino
load std.datetime: steady_millis
load std.io: println

set start = steady_millis()
println("Старт измерения: ${start} мс")
```

### 3.5 is_leap_year

Определяет, является ли указанный год високосным по григорианскому календарю.

```yukino
public fun is_leap_year(year: int32) -> bool
```

Возвращает `true`, если год делится на 4 (и при этом не делится на 100, либо делится на 400), иначе `false`.

Функция никогда не паникует.

```yukino
load std.datetime: is_leap_year
load std.io: println

set leap = is_leap_year(2024)
println("2024 високосный: ${leap}") // true
```

### 3.6 days_in_month

Возвращает количество дней в указанном месяце заданного года с учетом високосности.

```yukino
public fun days_in_month(year: int32, month: int32) -> int32
```

Возвращает число дней в месяце (от 28 до 31).

Функция никогда не паникует.

```yukino
load std.datetime: days_in_month
load std.io: println

set days = days_in_month(2024, 2)
println("Дней в феврале: ${days}") // 29
```

### 3.7 day_of_week

Вычисляет день недели для указанной даты.

```yukino
public fun day_of_week(ref dt: DateTime) -> int32
```

Возвращает порядковый номер дня недели: от 1 (понедельник) до 7 (воскресенье).

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, day_of_week
load std.io: println

set now = now_utc()
set dow = day_of_week(ref now)
println("День недели: ${dow}")
```

### 3.8 day_of_year

Вычисляет порядковый номер дня в году для заданной даты.

```yukino
public fun day_of_year(ref dt: DateTime) -> int32
```

Возвращает целое число от 1 до 366.

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, day_of_year
load std.io: println

set now = now_utc()
set doy = day_of_year(ref now)
println("День года: ${doy}")
```

### 3.9 make_datetime

Конструирует структуру `DateTime` с произвольно заданными компонентами даты и времени.

```yukino
public fun make_datetime(year: int32, month: int32, day: int32, hour: int32 = 0, minute: int32 = 0, second: int32 = 0, millisecond: int32 = 0, offset: int32 = 0) -> DateTime
```

Возвращает сконструированную структуру `DateTime`.

Функция никогда не паникует.

```yukino
load std.datetime: make_datetime
load std.io: println

set dt = make_datetime(2026, 10, 15, 12, 0, 0, 0, 10800)
println("Дата: ${dt.year}-${dt.month}-${dt.day}")
```

### 3.10 from_date

Создает экземпляр `DateTime` на начало указанных календарных суток (00:00:00 UTC).

```yukino
public fun from_date(year: int32, month: int32, day: int32) -> DateTime
```

Возвращает структуру `DateTime` со временем 00:00:00 и смещением 0.

Функция никогда не паникует.

```yukino
load std.datetime: from_date
load std.io: println

set start = from_date(2026, 1, 1)
println("Начало года: ${start.year}")
```

### 3.11 from_unix_secs

Создает структуру `DateTime` из количества секунд, прошедших с 1 января 1970 года UTC (Unix Epoch).

```yukino
public fun from_unix_secs(secs: int64, offset: int32 = 0) -> DateTime
```

Возвращает структуру `DateTime` с заданным смещением часового пояса `offset`.

Функция никогда не паникует.

```yukino
load std.datetime: from_unix_secs
load std.io: println

set dt = from_unix_secs(1700000000, 0)
println("Год: ${dt.year}")
```

### 3.12 from_unix_millis

Создает структуру `DateTime` из количества миллисекунд эпохи Unix.

```yukino
public fun from_unix_millis(ms: int64, offset: int32 = 0) -> DateTime
```

Возвращает структуру `DateTime`.

Функция никогда не паникует.

```yukino
load std.datetime: from_unix_millis
load std.io: println

set dt = from_unix_millis(1700000000000, 0)
println("Год: ${dt.year}")
```

### 3.13 from_unix_nanos

Создает структуру `DateTime` из количества наносекунд эпохи Unix.

```yukino
public fun from_unix_nanos(nanos: int64, offset: int32 = 0) -> DateTime
```

Возвращает структуру `DateTime`.

Функция никогда не паникует.

```yukino
load std.datetime: from_unix_nanos
load std.io: println

set dt = from_unix_nanos(1700000000000000000, 0)
println("Год: ${dt.year}")
```

### 3.14 to_unix_secs

Вычисляет количество полных секунд, прошедших от Unix Epoch до указанного момента времени.

```yukino
public fun to_unix_secs(ref dt: DateTime) -> int64
```

Возвращает число секунд эпохи Unix в UTC как `int64`.

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, to_unix_secs
load std.io: println

set now = now_utc()
set ts = to_unix_secs(ref now)
println("Unix timestamp: ${ts}")
```

### 3.15 to_unix_millis

Вычисляет количество миллисекунд от эпохи Unix до указанного момента времени.

```yukino
public fun to_unix_millis(ref dt: DateTime) -> int64
```

Возвращает число миллисекунд как `int64`.

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, to_unix_millis
load std.io: println

set now = now_utc()
set ms = to_unix_millis(ref now)
println("Epoch ms: ${ms}")
```

### 3.16 to_unix_nanos

Вычисляет количество наносекунд от эпохи Unix до указанного момента времени.

```yukino
public fun to_unix_nanos(ref dt: DateTime) -> int64
```

Возвращает число наносекунд как `int64`.

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, to_unix_nanos
load std.io: println

set now = now_utc()
set ns = to_unix_nanos(ref now)
println("Epoch nanos: ${ns}")
```

### 3.17 to_utc

Переводит переданный момент даты и времени в часовой пояс UTC (смещение 0).

```yukino
public fun to_utc(ref dt: DateTime) -> DateTime
```

Возвращает новую структуру `DateTime` с нулевым смещением `utc_offset = 0`.

Функция никогда не паникует.

```yukino
load std.datetime: now_local, to_utc
load std.io: println

set local_dt = now_local()
set utc_dt = to_utc(ref local_dt)
println("Час UTC: ${utc_dt.hour}")
```

### 3.18 to_local

Переводит дату и время в текущий локальный часовой пояс операционной системы.

```yukino
public fun to_local(ref dt: DateTime) -> DateTime
```

Возвращает `DateTime` со смещением локального пояса операционной системы.

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, to_local
load std.io: println

set utc = now_utc()
set local_time = to_local(ref utc)
println("Локальный час: ${local_time.hour}")
```

### 3.19 with_offset

Переводит дату и время в часовой пояс с заданным смещением относительно UTC в секундах.

```yukino
public fun with_offset(ref dt: DateTime, offset_secs: int32) -> DateTime
```

Возвращает структуру `DateTime` с обновленным смещением `offset_secs`.

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, with_offset
load std.io: println

set utc = now_utc()
set tokyo = with_offset(ref utc, 32400) // UTC+9
println("Час в Токио: ${tokyo.hour}")
```

### 3.20 duration_from_nanos

Создает временной интервал `Duration` из количества наносекунд.

```yukino
public fun duration_from_nanos(nanos: int64) -> Duration
```

Возвращает структуру `Duration`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_nanos
load std.io: println

set d = duration_from_nanos(500000000)
println("Секунд в интервале: ${d.secs}")
```

### 3.21 duration_from_micros

Создает временной интервал `Duration` из количества микросекунд.

```yukino
public fun duration_from_micros(us: int64) -> Duration
```

Возвращает структуру `Duration`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_micros
load std.io: println

set d = duration_from_micros(1500000)
println("Секунд: ${d.secs}")
```

### 3.22 duration_from_millis

Создает временной интервал `Duration` из количества миллисекунд.

```yukino
public fun duration_from_millis(ms: int64) -> Duration
```

Возвращает структуру `Duration`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_millis
load std.io: println

set d = duration_from_millis(2500)
println("Секунд: ${d.secs}")
```

### 3.23 duration_from_secs

Создает временной интервал `Duration` из количества секунд.

```yukino
public fun duration_from_secs(secs: int64) -> Duration
```

Возвращает структуру `Duration`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_secs
load std.io: println

set d = duration_from_secs(60)
println("Секунд: ${d.secs}")
```

### 3.24 duration_from_mins

Создает временной интервал `Duration` из количества минут.

```yukino
public fun duration_from_mins(mins: int64) -> Duration
```

Возвращает структуру `Duration`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_mins
load std.io: println

set d = duration_from_mins(30)
println("Секунд в 30 мин: ${d.secs}")
```

### 3.25 duration_from_hours

Создает временной интервал `Duration` из количества часов.

```yukino
public fun duration_from_hours(hours: int64) -> Duration
```

Возвращает структуру `Duration`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_hours
load std.io: println

set d = duration_from_hours(24)
println("Секунд в сутках: ${d.secs}")
```

### 3.26 duration_from_days

Создает временной интервал `Duration` из количества календарных суток (по 86400 секунд).

```yukino
public fun duration_from_days(days: int64) -> Duration
```

Возвращает структуру `Duration`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_days
load std.io: println

set week = duration_from_days(7)
println("Секунд в неделе: ${week.secs}")
```

### 3.27 duration_to_nanos

Переводит временной интервал `Duration` в общее количество наносекунд.

```yukino
public fun duration_to_nanos(ref d: Duration) -> int64
```

Возвращает общее число наносекунд интервала как `int64`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_millis, duration_to_nanos
load std.io: println

set d = duration_from_millis(10)
println("Наносекунд: ${duration_to_nanos(ref d)}")
```

### 3.28 duration_to_micros

Переводит временной интервал в микросекунды.

```yukino
public fun duration_to_micros(ref d: Duration) -> int64
```

Возвращает число микросекунд как `int64`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_millis, duration_to_micros
load std.io: println

set d = duration_from_millis(5)
println("Микросекунд: ${duration_to_micros(ref d)}")
```

### 3.29 duration_to_millis

Переводит временной интервал в миллисекунды.

```yukino
public fun duration_to_millis(ref d: Duration) -> int64
```

Возвращает число миллисекунд как `int64`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_secs, duration_to_millis
load std.io: println

set d = duration_from_secs(3)
println("Миллисекунд: ${duration_to_millis(ref d)}")
```

### 3.30 duration_to_secs

Переводит временной интервал в секунды в виде числа с плавающей точкой.

```yukino
public fun duration_to_secs(ref d: Duration) -> float64
```

Возвращает точное число секунд как `float64`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_millis, duration_to_secs
load std.io: println

set d = duration_from_millis(1500)
println("Секунд: ${duration_to_secs(ref d)}") // 1.5
```

### 3.31 duration_to_mins

Переводит временной интервал в минуты как вещественное число.

```yukino
public fun duration_to_mins(ref d: Duration) -> float64
```

Возвращает число минут как `float64`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_secs, duration_to_mins
load std.io: println

set d = duration_from_secs(90)
println("Минут: ${duration_to_mins(ref d)}") // 1.5
```

### 3.32 duration_to_hours

Переводит временной интервал в часы как вещественное число.

```yukino
public fun duration_to_hours(ref d: Duration) -> float64
```

Возвращает число часов как `float64`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_mins, duration_to_hours
load std.io: println

set d = duration_from_mins(120)
println("Часов: ${duration_to_hours(ref d)}") // 2.0
```

### 3.33 duration_to_days

Переводит временной интервал в дни как вещественное число.

```yukino
public fun duration_to_days(ref d: Duration) -> float64
```

Возвращает число дней как `float64`.

Функция никогда не паникует.

```yukino
load std.datetime: duration_from_hours, duration_to_days
load std.io: println

set d = duration_from_hours(36)
println("Дней: ${duration_to_days(ref d)}") // 1.5
```

### 3.34 add_duration

Прибавляет временной интервал к указанному моменту даты и времени.

```yukino
public fun add_duration(ref dt: DateTime, ref d: Duration) -> DateTime
```

Возвращает новую структуру `DateTime`.

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, duration_from_days, add_duration
load std.io: println

set now = now_utc()
set d = duration_from_days(1)
set tomorrow = add_duration(ref now, ref d)
println("Завтрашний день: ${tomorrow.day}")
```

### 3.35 sub_duration

Вычитает временной интервал из указанного момента даты и времени.

```yukino
public fun sub_duration(ref dt: DateTime, ref d: Duration) -> DateTime
```

Возвращает новую структуру `DateTime`.

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, duration_from_hours, sub_duration
load std.io: println

set now = now_utc()
set d = duration_from_hours(2)
set past = sub_duration(ref now, ref d)
println("Час 2 часа назад: ${past.hour}")
```

### 3.36 diff

Вычисляет разницу между двумя моментами времени (`dt1 - dt2`).

```yukino
public fun diff(ref dt1: DateTime, ref dt2: DateTime) -> Duration
```

Возвращает структуру `Duration`, представляющую разницу во времени.

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, duration_from_days, add_duration, diff, duration_to_days
load std.io: println

set now = now_utc()
set delta = duration_from_days(7)
set next_week = add_duration(ref now, ref delta)
set difference = diff(ref next_week, ref now)
println("Разница в днях: ${duration_to_days(ref difference)}")
```

### 3.37 to_iso8601

Форматирует дату и время в каноническую строку по международному стандарту ISO 8601.

```yukino
public fun to_iso8601(ref dt: DateTime) -> string
```

Возвращает отформатированную строку (например, `"2026-10-15T12:00:00.000Z"`).

Функция никогда не паникует.

```yukino
load std.datetime: now_utc, to_iso8601
load std.io: println

set now = now_utc()
println("ISO 8601: " + to_iso8601(ref now))
```

### 3.38 format

Форматирует дату и время по пользовательской строке шаблона.

```yukino
public fun format(ref dt: DateTime, fmt: string) -> string
```

Возвращает отформатированную строку по переданному шаблону.

Спецификаторы шаблона:
- `%Y` — четырехзначный год (`2026`);
- `%m` — двузначный месяц (`01`..`12`);
- `%d` — двузначный день (`01`..`31`);
- `%H` — двузначный час (`00`..`23`);
- `%M` — двузначная минута (`00`..`59`);
- `%S` — двузначная секунда (`00`..`59`);
- `%f` — трехзначная миллисекунда (`000`..`999`);
- `%z` — смещение часового пояса в формате `+HHMM` без двоеточия (`+0000` для нулевого смещения, `+0300` для UTC+3).

Спецификатор `%%` не реализован: нераспознанные `%`-последовательности, включая `%%`, выводятся в результат без изменений.

Функция никогда не паникует.

```yukino
load std.datetime: now_local, format
load std.io: println

set now = now_local()
set pretty = format(ref now, "%d.%m.%Y %H:%M")
println("Дата: ${pretty}")
```

### 3.39 parse_iso8601

Разбирает строку даты и времени формата ISO 8601 и конструирует объект `DateTime`.

```yukino
public fun parse_iso8601(s: string) -> DateTime
```

Возвращает разобранную структуру `DateTime`.

В случае некорректного формата строки или недопустимых символов вызывает панику со структурой `DateTimeParseError`:
- `code = 1` — строка слишком коротка либо непредвиденный конец строки при чтении цифр;
- `code = 2` — нецифровой символ либо отсутствует разделитель `-` после года;
- `code = 3` — отсутствует разделитель `-` после месяца;
- `code = 4` — отсутствует разделитель `T` или пробел после даты;
- `code = 5` — неполная компонента времени;
- `code = 6` — отсутствует разделитель `:` после часа;
- `code = 7` — отсутствует разделитель `:` после минуты.

Часовой пояс в строке не валидируется: если символ после времени не `Z`, `z`, `+` или `-`, он игнорируется и смещение остается нулевым.

```yukino
load std.datetime: parse_iso8601, DateTime, DateTimeParseError
load std.io: println

sus {
    set dt = parse_iso8601("2026-10-15T12:30:00Z")
    println("Разобранный год: ${dt.year}")
} catch err: DateTimeParseError {
    println("Ошибка парсинга: ${err.message}")
}
```

### 3.40 stopwatch_start

Создает новый экземпляр секундомера `Stopwatch` и немедленно запускает отсчет времени по монотонным часам.

```yukino
public fun stopwatch_start() -> Stopwatch
```

Возвращает инициализированный и запущенный объект класса `Stopwatch`. Значение не может быть `null`.

Функция никогда не вызывает панику.

```yukino
load std.datetime: stopwatch_start, Stopwatch
load std.threads: sleep
load std.io: println

set sw: Stopwatch = stopwatch_start()
sleep(50u64)
sw.stop()
println("Прошло времени: ${sw.elapsed_millis()} мс")
```

### 3.41 timer_start

Создает новый экземпляр таймера `Timer` с заданной продолжительностью в миллисекундах и немедленно запускает обратный отсчет.

```yukino
public fun timer_start(duration_ms: uint64) -> Timer
```

Возвращает инициализированный и запущенный объект класса `Timer`. Значение не может быть `null`.

Функция никогда не вызывает панику.

```yukino
load std.datetime: timer_start, Timer
load std.threads: sleep
load std.io: println

set timer: Timer = timer_start(100u64)
while !timer.is_expired() {
    sleep(25u64)
}
println("Таймер завершен, remaining_ms: ${timer.remaining_ms}")
```

---

[← Назад: Модуль std.math](5-math.md) | [Вперед: Модуль std.random →](7-random.md)
