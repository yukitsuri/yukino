# Модуль std.sysinfo

Модуль `std.sysinfo` предоставляет интерфейс для получения характеристик операционной системы, архитектуры процессора, аппаратных ресурсов (ядра, тактовая частота, оперативная память, swap), средней нагрузки (load average), дисковых разделов, информации о текущем пользователе и состоянии батареи.

---

## 1. Подключение: load std.sysinfo

Импорт всего пакета:

```yukino
load std.sysinfo
```

В текущую область видимости импортируются все публичные элементы модуля `std.sysinfo`:

- **Структуры данных:**
  - `LoadAverage` — метрики средней загрузки системы за 1, 5 и 15 минут;
  - `DiskSpace` — общий и доступный объем дискового пространства;
  - `DiskInfo` — детальная информация о смонтированном дисковом разделе;
  - `NetworkInterfaceInfo` — информация о сетевом адаптере, MAC-адресе и счетчиках трафика;
  - `ThermalSensorInfo` — показания аппаратных температурных датчиков и критические лимиты;
  - `ProcessInfo` — детальная информация о запущенном системном процессе.
- **Хост, операционная система и ядро:**
  - `host_name`, `os_name`, `os_version`, `kernel_version`, `distribution_id`, `cpu_arch`, `uptime`, `boot_time`.
- **Пользователи и привилегии:**
  - `current_user_name`, `current_user_id`, `is_elevated`.
- **Процессор, ядра и нагрузка:**
  - `logical_core_count`, `physical_core_count`, `cpu_brand`, `cpu_frequency`, `load_average`.
- **Оперативная память и Swap:**
  - `total_memory`, `free_memory`, `available_memory`, `used_memory`, `total_swap`, `free_swap`, `used_swap`.
- **Накопители и дисковые разделы:**
  - `disk_space`, `disks`.
- **Состояние батареи:**
  - `battery_percent`, `is_battery_charging`.
- **Сетевые интерфейсы и трафик (std.sysinfox):**
  - `network_interfaces`, `network_interface`.
- **Температурные датчики оборудования:**
  - `thermal_sensors`.
- **Таблица процессов операционной системы:**
  - `process_list`, `process_by_pid`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.sysinfo` глобальные константы не объявлены. Все системные параметры и метрики считываются динамически во время выполнения через специализированные функции.

---

## 3. Структуры данных

### 3.1 LoadAverage

Метрики средней загрузки операционной системы за скользящие интервалы времени (1, 5 и 15 минут).

```yukino
public struct LoadAverage {
    one: float64
    five: float64
    fifteen: float64
}
```

- `one` — средняя нагрузка за последнюю 1 минуту.
- `five` — средняя нагрузка за последние 5 минут.
- `fifteen` — средняя нагрузка за последние 15 минут.

### 3.2 DiskSpace

Сводные метрики общего и доступного объема дискового пространства по указанной точке файловой системы.

```yukino
public struct DiskSpace {
    total_space: uint64
    available_space: uint64
}
```

- `total_space` — полный объем файловой системы в байтах.
- `available_space` — объем доступного свободного места в байтах.

### 3.3 DiskInfo

Описывает смонтированный блочный накопитель или файловую систему.

```yukino
public struct DiskInfo {
    name: string
    mount_point: string
    file_system: string
    total_space: uint64
    available_space: uint64
    is_removable: bool
    is_read_only: bool
}
```

- `name` — имя системного устройства (например, `"/dev/nvme0n1p2"` или `"C:"`).
- `mount_point` — путь монтирования в дереве каталогов (например, `"/"` или `"/home"`).
- `file_system` — тип файловой системы (например, `"btrfs"`, `"ext4"`, `"ntfs"`, `"apfs"`).
- `total_space` — суммарный объем раздела в байтах.
- `available_space` — свободное дисковое пространство, доступное непривилегированным пользователям.
- `is_removable` — флаг съемного носителя (USB-флеш-карта, SD-накопитель).
- `is_read_only` — признак режима "только чтение".

### 3.4 NetworkInterfaceInfo

Детальные характеристики сетевого адаптера, его операционный статус и счетчики переданных и принятых пакетов и байтов.

```yukino
public struct NetworkInterfaceInfo {
    name: string
    mac_address: string
    is_up: bool
    is_loopback: bool
    rx_bytes: uint64
    tx_bytes: uint64
    rx_packets: uint64
    tx_packets: uint64
    rx_errors: uint64
    tx_errors: uint64
}
```

- `name` — системное имя интерфейса (например, `"eth0"`, `"wlan0"`, `"lo"`).
- `mac_address` — аппаратный MAC-адрес устройства (например, `"9c:c7:d3:eb:e0:38"`).
- `is_up` — флаг активности интерфейса (`true`, если интерфейс включен и поднят).
- `is_loopback` — признак петлевого интерфейса локального хоста (`lo`).
- `rx_bytes` — суммарное количество принятых байтов.
- `tx_bytes` — суммарное количество отправленных байтов.
- `rx_packets` — количество успешно принятых сетевых пакетов.
- `tx_packets` — количество успешно отправленных сетевых пакетов.
- `rx_errors` — количество ошибок приема пакетов.
- `tx_errors` — количество ошибок передачи пакетов.

### 3.5 ThermalSensorInfo

Показания аппаратного температурного датчика материнской платы, процессора или видеокарты.

```yukino
public struct ThermalSensorInfo {
    label: string
    temperature_celsius: float32
    critical_temp: float32?
}
```

- `label` — идентификатор или метка датчика (например, `"k10temp Tctl"`, `"acpitz"`, `"coretemp Package id 0"`).
- `temperature_celsius` — текущая температура в градусах Цельсия.
- `critical_temp` — предельно допустимая температура в градусах Цельсия либо `null`, если порог не определен драйвером оборудования.

### 3.6 ProcessInfo

Информация о запущенном системном процессе из таблицы процессов операционной системы.

```yukino
public struct ProcessInfo {
    pid: uint32
    parent_pid: uint32
    name: string
    cmdline: string
    cpu_usage: float32
    memory_rss: uint64
    memory_virtual: uint64
}
```

- `pid` — уникальный идентификатор процесса (PID).
- `parent_pid` — идентификатор родительского процесса (PPID).
- `name` — имя исполняемого файла или процесса (comm).
- `cmdline` — полная строка запуска с аргументами командной строки.
- `cpu_usage` — расчетный процент использования ресурсов ЦП.
- `memory_rss` — физическая оперативная память, занятая процессом (Resident Set Size в байтах).
- `memory_virtual` — общий объем выделенной виртуальной памяти процесса (в байтах).

---

## 4. Справочник функций: Хост, операционная система и ядро

### 4.1 host_name (nullable)

Возвращает сетевое имя текущего компьютера (hostname).

```yukino
public fun host_name() -> string?
```

Возвращает строковое имя узла либо `null`, если операционная система не возвращает имя хоста.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: host_name
load std.io: println

set host = host_name() ?? "localhost"
println("Имя компьютера: " + host)
```

### 4.2 os_name

Возвращает название базовой операционной системы (`"Linux"`, `"Windows"`, `"macOS"`, `"Android"`, `"FreeBSD"`).

```yukino
public fun os_name() -> string
```

Возвращает имя операционной системы типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: os_name
load std.io: println

set os = os_name()
println("Операционная система: " + os)
```

### 4.3 os_version (nullable)

Возвращает номер версии или идентификатор сборки операционной системы (например, `"24.04"`, `"11"`, `"14.5"`).

```yukino
public fun os_version() -> string?
```

Возвращает строковое значение версии либо `null`, если сведения отсутствуют в системе.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: os_version
load std.io: println

set ver = os_version()
if ver != null {
    println("Версия ОС: " + ver)
}
```

### 4.4 kernel_version (nullable)

Возвращает версию ядра операционной системы (например, `"6.8.0-45-generic"` на Linux или сборку ядра NT на Windows).

```yukino
public fun kernel_version() -> string?
```

Возвращает строку версии ядра либо `null`, если информация недоступна.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: kernel_version
load std.io: println

set kern = kernel_version()
if kern != null {
    println("Ядро: " + kern)
}
```

### 4.5 distribution_id

Возвращает строковый идентификатор дистрибутива операционной системы (например, `"ubuntu"`, `"cachyos"`, `"arch"`, `"debian"`, `"fedora"`, `"windows"`).

```yukino
public fun distribution_id() -> string
```

Возвращает строковый идентификатор типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: distribution_id
load std.io: println

set dist = distribution_id()
println("Дистрибутив: " + dist)
```

### 4.6 cpu_arch

Возвращает аппаратную архитектуру центрального процессора, для которой скомпилирована программа (`"x86_64"`, `"aarch64"`, `"arm"`, `"riscv64"`).

```yukino
public fun cpu_arch() -> string
```

Возвращает наименование архитектуры типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: cpu_arch
load std.io: println

set arch = cpu_arch()
println("Архитектура CPU: " + arch)
```

### 4.7 uptime

Возвращает количество секунд непрерывной работы операционной системы с момента последней загрузки.

```yukino
public fun uptime() -> uint64
```

Возвращает числовое значение секунд типа `uint64`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: uptime
load std.io: println

set up = uptime()
println("Время работы системы (сек): ${up}")
```

### 4.8 boot_time

Возвращает Unix timestamp (в секундах с 1 января 1970 года UTC) точного момента старта операционной системы.

```yukino
public fun boot_time() -> uint64
```

Возвращает временную метку типа `uint64`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: boot_time
load std.io: println

set boot = boot_time()
println("Время старта системы: ${boot}")
```

---

## 5. Справочник функций: Пользователи и привилегии

### 5.1 current_user_name (nullable)

Возвращает имя пользователя (логин учетной записи), запустившего текущий процесс.

```yukino
public fun current_user_name() -> string?
```

Возвращает строковое имя пользователя либо `null`, если имя не удалось определить.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: current_user_name
load std.io: println

set user = current_user_name() ?? "anonymous"
println("Пользователь: " + user)
```

### 5.2 current_user_id (nullable)

Возвращает строковый идентификатор пользователя (числовой эффективный UID на Unix или имя учетной записи из переменной окружения `USERNAME` на Windows).

```yukino
public fun current_user_id() -> string?
```

Возвращает строковый идентификатор либо `null`, если идентификатор недоступен.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: current_user_id
load std.io: println

set uid = current_user_id()
if uid != null {
    println("Идентификатор пользователя (UID): " + uid)
}
```

### 5.3 is_elevated

Проверяет, запущен ли текущий процесс с повышенными привилегиями: на Unix — суперпользователь root с эффективным `uid == 0`. На Windows функция всегда возвращает `false`.

```yukino
public fun is_elevated() -> bool
```

Возвращает логическое значение `true`, если процесс обладает повышенными привилегиями, иначе `false`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: is_elevated
load std.io: println

if is_elevated() {
    println("Процесс запущен с правами администратора (root)")
} else {
    println("Обычный пользовательский процесс")
}
```

---

## 6. Справочник функций: Процессор, ядра и средняя нагрузка

### 6.1 logical_core_count

Возвращает общее количество доступных логических ядер процессора (включая вычислительные потоки Hyper-Threading / SMT).

```yukino
public fun logical_core_count() -> uint32
```

Возвращает числовое количество ядер типа `uint32`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: logical_core_count
load std.io: println

set cores = logical_core_count()
println("Логических ядер: ${cores}")
```

### 6.2 physical_core_count (nullable)

Возвращает количество реальных физических ядер центрального процессора без учета виртуальных потоков SMT.

```yukino
public fun physical_core_count() -> uint32?
```

Возвращает количество физических ядер типа `uint32`. Если определить физическую топологию процессора не удалось, функция возвращает количество логических ядер, поэтому на практике `null` не возвращается.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: physical_core_count
load std.io: println

set phys = physical_core_count()
if phys != null {
    println("Физических ядер: ${phys}")
}
```

### 6.3 cpu_brand (nullable)

Возвращает официальное маркетинговое наименование модели центрального процессора (например, `"AMD Ryzen 7 7445HS w/ Radeon 740M Graphics"` или `"13th Gen Intel(R) Core(TM) i7-13700K"`).

```yukino
public fun cpu_brand() -> string?
```

Возвращает строку с названием процессора либо `null`, если модель не определена.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: cpu_brand
load std.io: println

set brand = cpu_brand()
if brand != null {
    println("Модель CPU: " + brand)
}
```

### 6.4 cpu_frequency

Возвращает текущую тактовую частоту процессора в мегагерцах (МГц).

```yukino
public fun cpu_frequency() -> uint64
```

Возвращает целое число мегагерц типа `uint64`. При невозможности прочесть частоту возвращает `0`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: cpu_frequency
load std.io: println

set freq = cpu_frequency()
println("Частота CPU: ${freq} МГц")
```

### 6.5 load_average (nullable)

Возвращает снимок скользящей средней загрузки операционной системы (load average) за 1, 5 и 15 минут.

```yukino
public fun load_average() -> LoadAverage?
```

Возвращает структуру `LoadAverage` либо `null` на операционных системах, не поддерживающих расчет метрики (например, базовые сборки Windows).

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: load_average
load std.io: println

set load_avg = load_average()
if load_avg != null {
    set one = load_avg?.one ?? 0.0
    println("Загрузка за 1 мин: " + cast(one, "string"))
}
```

---

## 7. Справочник функций: Оперативная память и Swap

### 7.1 total_memory

Возвращает суммарный объем установленной в системе физической оперативной памяти (RAM) в байтах.

```yukino
public fun total_memory() -> uint64
```

Возвращает объем памяти типа `uint64`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: total_memory
load std.io: println

set total = total_memory()
println("Всего RAM (байт): ${total}")
```

### 7.2 free_memory

Возвращает объем полностью неиспользуемой оперативной памяти (в байтах), не занятой ни процессами, ни системными дисковыми кэшами.

```yukino
public fun free_memory() -> uint64
```

Возвращает число свободных байт типа `uint64`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: free_memory
load std.io: println

set free_ram = free_memory()
println("Свободная память: ${free_ram}")
```

### 7.3 available_memory

Возвращает объем реально доступной оперативной памяти для запуска новых приложений без необходимости сброса страниц в раздел подкачки (включает освобождаемые кэши ядра и буферы).

```yukino
public fun available_memory() -> uint64
```

Возвращает доступный объем памяти типа `uint64`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: available_memory
load std.io: println

set avail = available_memory()
println("Доступно для приложений: ${avail}")
```

### 7.4 used_memory

Возвращает объем занятой оперативной памяти в байтах, рассчитываемый как разница между суммарным и реально доступным объемом памяти (`total_memory - available_memory`).

```yukino
public fun used_memory() -> uint64
```

Возвращает объем занятой памяти типа `uint64`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: used_memory
load std.io: println

set used = used_memory()
println("Используется RAM: ${used}")
```

### 7.5 total_swap

Возвращает общий размер системного пространства подкачки (Swap) в байтах.

```yukino
public fun total_swap() -> uint64
```

Возвращает объем swap типа `uint64` (возвращает `0`, если файл или раздел swap не настроен).

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: total_swap
load std.io: println

set swap = total_swap()
println("Общий Swap: ${swap}")
```

### 7.6 free_swap

Возвращает неиспользуемый (свободный) объем пространства подкачки в байтах.

```yukino
public fun free_swap() -> uint64
```

Возвращает число свободных байт swap типа `uint64`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: free_swap
load std.io: println

set fswap = free_swap()
println("Свободный Swap: ${fswap}")
```

### 7.7 used_swap

Возвращает занятый процессами объем пространства подкачки в байтах (`total_swap - free_swap`).

```yukino
public fun used_swap() -> uint64
```

Возвращает объем используемого swap типа `uint64`.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: used_swap
load std.io: println

set uswap = used_swap()
println("Используется Swap: ${uswap}")
```

---

## 8. Справочник функций: Накопители и дисковое пространство

### 8.1 disk_space (nullable)

Определяет общий и доступный объем дискового пространства файловой системы, в которой расположен указанный путь `path`.

```yukino
public fun disk_space(path: string) -> DiskSpace?
```

Возвращает структуру `DiskSpace` либо `null`, если указанный путь не существует или файловая система недоступна.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: disk_space
load std.io: println

set space = disk_space("/")
if space != null {
    println("Доступно на корневом разделе: ${space?.available_space}")
}
```

### 8.2 disks

Возвращает срез всех физических и логических смонтированных блочных накопителей и файловых систем.

```yukino
public fun disks() -> DiskInfo[]
```

Возвращает срез структур `DiskInfo[]`. Если накопители не обнаружены, возвращает пустой срез.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: disks
load std.io: println

set all_disks = disks()
for i in 0..all_disks.len() {
    set d = all_disks[i]
    println("Диск: ${d.name} смонтирован в ${d.mount_point} (${d.file_system})")
}
```

---

## 9. Справочник функций: Состояние батареи и электропитания

### 9.1 battery_percent (nullable)

Возвращает текущий процент уровня заряда аккумуляторной батареи в диапазоне от `0.0` до `100.0`.

```yukino
public fun battery_percent() -> float32?
```

Возвращает числовое значение процента заряда типа `float32` либо `null`, если устройство не имеет батареи (стационарный настольный компьютер, сервер или виртуальная машина).

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: battery_percent
load std.io: println

set bat = battery_percent()
if bat != null {
    set level = bat ?? 0.0
    println("Заряд аккумулятора: " + cast(level, "string") + "%")
} else {
    println("Батарея не обнаружена")
}
```

### 9.2 is_battery_charging (nullable)

Определяет, подключено ли внешнее сетевое зарядное устройство и происходит ли зарядка аккумулятора.

```yukino
public fun is_battery_charging() -> bool?
```

Возвращает логическое значение `true`, если батарея заряжается, `false`, если питание работает от аккумулятора, либо `null`, если батарея отсутствует в оборудовании.

Функция никогда не вызывает панику.

```yukino
load std.sysinfo: is_battery_charging
load std.io: println

set charging = is_battery_charging()
if charging != null {
    println("Питание от сети: ${charging}")
}
```

---

## 10. Справочник функций: Сетевые интерфейсы и трафик

### 10.1 network_interfaces

Возвращает список всех обнаруженных физических и виртуальных сетевых адаптеров и счетчиков переданного трафика.

```yukino
public fun network_interfaces() -> NetworkInterfaceInfo[]
```

Возвращает срез структур `NetworkInterfaceInfo[]`. Если сетевые интерфейсы не найдены, возвращает пустой срез.

Функция никогда не вызывает панику.

```yukino
load std.sysinfox: network_interfaces
load std.io: println

set ifaces = network_interfaces()
for i in 0..ifaces.len() {
    set iface = ifaces[i]
    println("Интерфейс: ${iface.name}, MAC: ${iface.mac_address}, Up: ${iface.is_up}, RX: ${iface.rx_bytes} байт")
}
```

### 10.2 network_interface (nullable)

Находит сетевой адаптер по его точному системному имени.

```yukino
public fun network_interface(name: string) -> NetworkInterfaceInfo?
```

Возвращает структуру `NetworkInterfaceInfo` либо `null`, если интерфейс с указанным именем отсутствует в системе.

Функция никогда не вызывает панику.

```yukino
load std.sysinfox: network_interface
load std.io: println

set eth = network_interface("eth0")
if eth != null {
    println("Найден eth0 с адресом: " + (eth?.mac_address ?? "нет MAC"))
} else {
    println("Интерфейс eth0 не обнаружен")
}
```

---

## 11. Справочник функций: Температурные датчики оборудования

### 11.1 thermal_sensors

Считывает показатели всех доступных аппаратных термодатчиков процессора, чипсета и системных контроллеров.

```yukino
public fun thermal_sensors() -> ThermalSensorInfo[]
```

Возвращает срез структур `ThermalSensorInfo[]`. Если термодатчики не поддерживаются платформой или недоступны, возвращает пустой срез.

Функция никогда не вызывает панику.

```yukino
load std.sysinfox: thermal_sensors
load std.io: println

set sensors = thermal_sensors()
for i in 0..sensors.len() {
    set s = sensors[i]
    println("Датчик [${s.label}]: " + cast(s.temperature_celsius, "string") + " °C")
}
```

---

## 12. Справочник функций: Таблица процессов операционной системы

### 12.1 process_list

Возвращает снимок всех запущенных в операционной системе процессов.

```yukino
public fun process_list() -> ProcessInfo[]
```

Возвращает срез структур `ProcessInfo[]`. Если доступ к таблице процессов ограничен, возвращает пустой срез.

Функция никогда не вызывает панику.

```yukino
load std.sysinfox: process_list
load std.io: println

set procs = process_list()
for i in 0..procs.len() {
    set p = procs[i]
    println("PID ${p.pid} (${p.name}): RSS = ${p.memory_rss} байт")
}
```

### 12.2 process_by_pid (nullable)

Находит запись о процессе в системной таблице по его уникальному идентификатору (PID).

```yukino
public fun process_by_pid(pid: uint32) -> ProcessInfo?
```

Возвращает структуру `ProcessInfo` либо `null`, если процесс с указанным PID не найден или завершил выполнение.

Функция никогда не вызывает панику.

```yukino
load std.sysinfox: process_by_pid
load std.io: println

set proc = process_by_pid(1u32)
if proc != null {
    println("Корневой процесс инициализации: ${proc?.name} (${proc?.cmdline})")
} else {
    println("Процесс PID 1 не найден")
}
```

---

[← Назад: Модуль std.os](11-os.md) | [Вперед: Модуль std.env →](13-env.md)
