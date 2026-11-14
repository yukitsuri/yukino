# Module std.sysinfo

The `std.sysinfo` module provides cross-platform facilities for querying operating system attributes, CPU architecture, processor topology, memory statistics (physical RAM, swap), system load average, mounted storage filesystems, user account identification, and battery state.

---

## 1. Import: load std.sysinfo

Importing the entire package:

```yukino
load std.sysinfo
```

All public components of `std.sysinfo` are loaded into the current scope:

- **Data Structures:**
  - `LoadAverage` — system load average metrics for 1, 5, and 15 minutes;
  - `DiskSpace` — total and available filesystem storage space;
  - `DiskInfo` — mounted block device and filesystem attributes;
  - `NetworkInterfaceInfo` — network interface details, MAC addresses, and packet/byte counters;
  - `ThermalSensorInfo` — hardware thermal sensor readings and critical trip temperatures;
  - `ProcessInfo` — detailed metadata for running system processes.
- **Host, OS, and Kernel:**
  - `host_name`, `os_name`, `os_version`, `kernel_version`, `distribution_id`, `cpu_arch`, `uptime`, `boot_time`.
- **Users and Privileges:**
  - `current_user_name`, `current_user_id`, `is_elevated`.
- **CPU, Cores, and Load:**
  - `logical_core_count`, `physical_core_count`, `cpu_brand`, `cpu_frequency`, `load_average`.
- **Memory and Swap:**
  - `total_memory`, `free_memory`, `available_memory`, `used_memory`, `total_swap`, `free_swap`, `used_swap`.
- **Storage and Disks:**
  - `disk_space`, `disks`.
- **Battery and Power:**
  - `battery_percent`, `is_battery_charging`.
- **Network Interfaces & Traffic (std.sysinfox):**
  - `network_interfaces`, `network_interface`.
- **Hardware Thermal Sensors:**
  - `thermal_sensors`.
- **System Process Table:**
  - `process_list`, `process_by_pid`.

All of these items are also accessible when importing the whole standard library: `load std`.

---

## 2. Constants

No global constants are declared in `std.sysinfo`. All metrics and parameters are queried dynamically at runtime via functions.

---

## 3. Data Structures

### 3.1 LoadAverage

Represents moving load average metrics calculated by the operating system kernel over 1, 5, and 15 minute intervals.

```yukino
public struct LoadAverage {
    one: float64
    five: float64
    fifteen: float64
}
```

- `one` — average system load over the past 1 minute.
- `five` — average system load over the past 5 minutes.
- `fifteen` — average system load over the past 15 minutes.

### 3.2 DiskSpace

Aggregated capacity and free space metrics for a filesystem path.

```yukino
public struct DiskSpace {
    total_space: uint64
    available_space: uint64
}
```

- `total_space` — total filesystem capacity in bytes.
- `available_space` — storage capacity in bytes available to non-privileged processes.

### 3.3 DiskInfo

Describes a mounted storage volume or filesystem partition.

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

- `name` — device node path or volume identifier (e.g. `"/dev/nvme0n1p2"` or `"C:"`).
- `mount_point` — directory tree mount location (e.g. `"/"` or `"/home"`).
- `file_system` — filesystem type identifier (e.g. `"btrfs"`, `"ext4"`, `"ntfs"`, `"apfs"`).
- `total_space` — total capacity in bytes.
- `available_space` — remaining available space in bytes.
- `is_removable` — `true` if media is removable (such as USB flash or SD cards).
- `is_read_only` — `true` if volume is mounted in read-only mode.

### 3.4 NetworkInterfaceInfo

Contains network interface attributes, link status, MAC address, and traffic packet/byte counters.

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

- `name` — interface system identifier (e.g. `"eth0"`, `"wlan0"`, `"lo"`).
- `mac_address` — hardware MAC address string (e.g. `"9c:c7:d3:eb:e0:38"`).
- `is_up` — `true` if interface link is operational and active.
- `is_loopback` — `true` if loopback device (`lo`).
- `rx_bytes` — cumulative received payload bytes.
- `tx_bytes` — cumulative transmitted payload bytes.
- `rx_packets` — cumulative received packets.
- `tx_packets` — cumulative transmitted packets.
- `rx_errors` — cumulative receive error count.
- `tx_errors` — cumulative transmit error count.

### 3.5 ThermalSensorInfo

Represents hardware thermal zone or onboard sensor temperature metrics.

```yukino
public struct ThermalSensorInfo {
    label: string
    temperature_celsius: float32
    critical_temp: float32?
}
```

- `label` — device or sensor label identifier (e.g. `"k10temp Tctl"`, `"acpitz"`, `"coretemp Package id 0"`).
- `temperature_celsius` — current temperature in degrees Celsius.
- `critical_temp` — critical threshold temperature in degrees Celsius, or `null` if unspecified by hardware drivers.

### 3.6 ProcessInfo

Represents a running operating system process inspectable from the system process table.

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

- `pid` — unique process identifier (PID).
- `parent_pid` — parent process identifier (PPID).
- `name` — executable name or process command moniker.
- `cmdline` — full command-line invocation string with arguments.
- `cpu_usage` — estimated processor utilization percentage.
- `memory_rss` — resident set size (physical RAM occupied by process, in bytes).
- `memory_virtual` — total allocated virtual address space, in bytes.

---

## 4. Function Reference: Host, OS, and Kernel

### 4.1 host_name (nullable)

Returns the network node hostname of the current computer.

```yukino
public fun host_name() -> string?
```

Returns a hostname `string`, or `null` if the operating system does not supply one.

This function never panics.

```yukino
load std.sysinfo: host_name
load std.io: println

set host = host_name() ?? "localhost"
println("Hostname: " + host)
```

### 4.2 os_name

Returns the canonical name of the operating system platform (`"Linux"`, `"Windows"`, `"macOS"`, `"Android"`, `"FreeBSD"`).

```yukino
public fun os_name() -> string
```

Returns an operating system name `string`.

This function never panics.

```yukino
load std.sysinfo: os_name
load std.io: println

set os = os_name()
println("Operating System: " + os)
```

### 4.3 os_version (nullable)

Returns the operating system version or release build identifier (e.g. `"24.04"`, `"11"`, `"14.5"`).

```yukino
public fun os_version() -> string?
```

Returns a version `string`, or `null` if version information cannot be resolved.

This function never panics.

```yukino
load std.sysinfo: os_version
load std.io: println

set ver = os_version()
if ver != null {
    println("OS Version: " + ver)
}
```

### 4.4 kernel_version (nullable)

Returns the operating system kernel release version (e.g. `"6.8.0-45-generic"` on Linux or NT build version on Windows).

```yukino
public fun kernel_version() -> string?
```

Returns a kernel version `string`, or `null` if not available.

This function never panics.

```yukino
load std.sysinfo: kernel_version
load std.io: println

set kern = kernel_version()
if kern != null {
    println("Kernel: " + kern)
}
```

### 4.5 distribution_id

Returns the lowercase distribution identifier (e.g. `"ubuntu"`, `"cachyos"`, `"arch"`, `"debian"`, `"fedora"`, `"windows"`).

```yukino
public fun distribution_id() -> string
```

Returns a distribution identifier `string`.

This function never panics.

```yukino
load std.sysinfo: distribution_id
load std.io: println

set dist = distribution_id()
println("Distribution: " + dist)
```

### 4.6 cpu_arch

Returns the target processor architecture for which the binary was compiled (`"x86_64"`, `"aarch64"`, `"arm"`, `"riscv64"`).

```yukino
public fun cpu_arch() -> string
```

Returns an architecture name `string`.

This function never panics.

```yukino
load std.sysinfo: cpu_arch
load std.io: println

set arch = cpu_arch()
println("Architecture: " + arch)
```

### 4.7 uptime

Returns the total continuous running time of the system in seconds since last boot.

```yukino
public fun uptime() -> uint64
```

Returns seconds count as `uint64`.

This function never panics.

```yukino
load std.sysinfo: uptime
load std.io: println

set up = uptime()
println("System uptime (seconds): ${up}")
```

### 4.8 boot_time

Returns the Unix timestamp in seconds since January 1, 1970 UTC when the system booted.

```yukino
public fun boot_time() -> uint64
```

Returns a Unix timestamp as `uint64`.

This function never panics.

```yukino
load std.sysinfo: boot_time
load std.io: println

set boot = boot_time()
println("System boot time: ${boot}")
```

---

## 5. Function Reference: Users and Privileges

### 5.1 current_user_name (nullable)

Returns the login username of the account executing the current process.

```yukino
public fun current_user_name() -> string?
```

Returns a username `string`, or `null` if the user cannot be identified.

This function never panics.

```yukino
load std.sysinfo: current_user_name
load std.io: println

set user = current_user_name() ?? "anonymous"
println("User: " + user)
```

### 5.2 current_user_id (nullable)

Returns the unique user identifier (numeric effective UID on Unix, or the account name from the `USERNAME` environment variable on Windows).

```yukino
public fun current_user_id() -> string?
```

Returns an identifier `string`, or `null` if unavailable.

This function never panics.

```yukino
load std.sysinfo: current_user_id
load std.io: println

set uid = current_user_id()
if uid != null {
    println("User ID: " + uid)
}
```

### 5.3 is_elevated

Checks whether the current process is running with elevated privileges: on Unix, whether the effective user is the superuser with `uid == 0`. On Windows this function always returns `false`.

```yukino
public fun is_elevated() -> bool
```

Returns `true` if the process has elevated rights, otherwise `false`.

This function never panics.

```yukino
load std.sysinfo: is_elevated
load std.io: println

if is_elevated() {
    println("Running with elevated privileges")
} else {
    println("Standard user execution")
}
```

---

## 6. Function Reference: CPU, Cores, and Load Average

### 6.1 logical_core_count

Returns the total number of available logical processor execution threads (including SMT / Hyper-Threading).

```yukino
public fun logical_core_count() -> uint32
```

Returns logical core count as `uint32`.

This function never panics.

```yukino
load std.sysinfo: logical_core_count
load std.io: println

set cores = logical_core_count()
println("Logical cores: ${cores}")
```

### 6.2 physical_core_count (nullable)

Returns the number of physical hardware CPU cores excluding hyper-threaded execution pipelines.

```yukino
public fun physical_core_count() -> uint32?
```

Returns physical core count as `uint32`. If the physical topology cannot be resolved, the logical core count is returned instead, so `null` is never returned in practice.

This function never panics.

```yukino
load std.sysinfo: physical_core_count
load std.io: println

set phys = physical_core_count()
if phys != null {
    println("Physical cores: ${phys}")
}
```

### 6.3 cpu_brand (nullable)

Returns the commercial model name of the central processor (e.g. `"AMD Ryzen 7 7445HS w/ Radeon 740M Graphics"` or `"13th Gen Intel(R) Core(TM) i7-13700K"`).

```yukino
public fun cpu_brand() -> string?
```

Returns a model name `string`, or `null` if unresolved.

This function never panics.

```yukino
load std.sysinfo: cpu_brand
load std.io: println

set brand = cpu_brand()
if brand != null {
    println("Processor Model: " + brand)
}
```

### 6.4 cpu_frequency

Returns the current operating clock frequency of the processor in megahertz (MHz).

```yukino
public fun cpu_frequency() -> uint64
```

Returns frequency in MHz as `uint64`. Returns `0` if clock rate cannot be measured.

This function never panics.

```yukino
load std.sysinfo: cpu_frequency
load std.io: println

set freq = cpu_frequency()
println("Clock rate: ${freq} MHz")
```

### 6.5 load_average (nullable)

Returns a snapshot of the operating system moving load averages over 1, 5, and 15 minute windows.

```yukino
public fun load_average() -> LoadAverage?
```

Returns a `LoadAverage` structure, or `null` on operating systems without standard loadavg support.

This function never panics.

```yukino
load std.sysinfo: load_average
load std.io: println

set load_avg = load_average()
if load_avg != null {
    set one = load_avg?.one ?? 0.0
    println("1-minute load average: " + cast(one, "string"))
}
```

---

## 7. Function Reference: Memory and Swap

### 7.1 total_memory

Returns the total amount of physical random access memory (RAM) installed in bytes.

```yukino
public fun total_memory() -> uint64
```

Returns byte count as `uint64`.

This function never panics.

```yukino
load std.sysinfo: total_memory
load std.io: println

set total = total_memory()
println("Total RAM (bytes): ${total}")
```

### 7.2 free_memory

Returns the amount of completely unallocated physical memory in bytes.

```yukino
public fun free_memory() -> uint64
```

Returns byte count as `uint64`.

This function never panics.

```yukino
load std.sysinfo: free_memory
load std.io: println

set free_ram = free_memory()
println("Free RAM: ${free_ram}")
```

### 7.3 available_memory

Returns the amount of memory in bytes readily available for launching new applications without swapping (accounting for reclaimable kernel caches and buffers).

```yukino
public fun available_memory() -> uint64
```

Returns available byte count as `uint64`.

This function never panics.

```yukino
load std.sysinfo: available_memory
load std.io: println

set avail = available_memory()
println("Available RAM: ${avail}")
```

### 7.4 used_memory

Returns the amount of physical memory currently in active use in bytes (`total_memory - available_memory`).

```yukino
public fun used_memory() -> uint64
```

Returns used memory count as `uint64`.

This function never panics.

```yukino
load std.sysinfo: used_memory
load std.io: println

set used = used_memory()
println("Used RAM: ${used}")
```

### 7.5 total_swap

Returns the total size of configured swap space in bytes.

```yukino
public fun total_swap() -> uint64
```

Returns total swap size as `uint64` (or `0` if swap is disabled).

This function never panics.

```yukino
load std.sysinfo: total_swap
load std.io: println

set swap = total_swap()
println("Total Swap: ${swap}")
```

### 7.6 free_swap

Returns the unused portion of swap space in bytes.

```yukino
public fun free_swap() -> uint64
```

Returns free swap count as `uint64`.

This function never panics.

```yukino
load std.sysinfo: free_swap
load std.io: println

set fswap = free_swap()
println("Free Swap: ${fswap}")
```

### 7.7 used_swap

Returns the portion of swap space currently held by memory pages in bytes (`total_swap - free_swap`).

```yukino
public fun used_swap() -> uint64
```

Returns used swap count as `uint64`.

This function never panics.

```yukino
load std.sysinfo: used_swap
load std.io: println

set uswap = used_swap()
println("Used Swap: ${uswap}")
```

---

## 8. Function Reference: Storage and Disk Space

### 8.1 disk_space (nullable)

Calculates total and available storage space for the filesystem containing the given `path`.

```yukino
public fun disk_space(path: string) -> DiskSpace?
```

Returns a `DiskSpace` structure, or `null` if the path does not exist or cannot be accessed.

This function never panics.

```yukino
load std.sysinfo: disk_space
load std.io: println

set space = disk_space("/")
if space != null {
    println("Root available space: ${space?.available_space}")
}
```

### 8.2 disks

Enumerates all mounted physical and logical partitions and filesystems.

```yukino
public fun disks() -> DiskInfo[]
```

Returns a slice of `DiskInfo` structures. Returns an empty slice if no volumes are detected.

This function never panics.

```yukino
load std.sysinfo: disks
load std.io: println

set all_disks = disks()
for i in 0..all_disks.len() {
    set d = all_disks[i]
    println("Volume: ${d.name} mounted at ${d.mount_point} (${d.file_system})")
}
```

---

## 9. Function Reference: Battery and Power State

### 9.1 battery_percent (nullable)

Returns the current battery state of charge as a percentage between `0.0` and `100.0`.

```yukino
public fun battery_percent() -> float32?
```

Returns a `float32` percentage, or `null` if no battery is present in the system (desktop workstation, server, virtual machine).

This function never panics.

```yukino
load std.sysinfo: battery_percent
load std.io: println

set bat = battery_percent()
if bat != null {
    set level = bat ?? 0.0
    println("Battery level: " + cast(level, "string") + "%")
} else {
    println("No battery detected")
}
```

### 9.2 is_battery_charging (nullable)

Checks whether an external AC power adapter is connected and actively charging the battery.

```yukino
public fun is_battery_charging() -> bool?
```

Returns `true` if charging, `false` if discharging, or `null` if no battery is present.

This function never panics.

```yukino
load std.sysinfo: is_battery_charging
load std.io: println

set charging = is_battery_charging()
if charging != null {
    println("AC power connected: ${charging}")
}
```

---

## 10. Function Reference: Network Interfaces & Traffic

### 10.1 network_interfaces

Returns a list of all detected physical and virtual network interfaces along with traffic telemetry.

```yukino
public fun network_interfaces() -> NetworkInterfaceInfo[]
```

Returns a slice of `NetworkInterfaceInfo[]`. If no interfaces are found, returns an empty slice.

This function never panics.

```yukino
load std.sysinfox: network_interfaces
load std.io: println

set ifaces = network_interfaces()
for i in 0..ifaces.len() {
    set iface = ifaces[i]
    println("Interface: ${iface.name}, MAC: ${iface.mac_address}, Up: ${iface.is_up}, RX: ${iface.rx_bytes} bytes")
}
```

### 10.2 network_interface (nullable)

Finds a network interface by its exact system name.

```yukino
public fun network_interface(name: string) -> NetworkInterfaceInfo?
```

Returns a `NetworkInterfaceInfo` structure or `null` if no interface with the specified name exists.

This function never panics.

```yukino
load std.sysinfox: network_interface
load std.io: println

set eth = network_interface("eth0")
if eth != null {
    println("Found eth0 with MAC: " + (eth?.mac_address ?? "no MAC"))
} else {
    println("Interface eth0 not found")
}
```

---

## 11. Function Reference: Hardware Thermal Sensors

### 11.1 thermal_sensors

Queries readings from available hardware thermal sensors across the CPU, GPU, motherboard, and drives.

```yukino
public fun thermal_sensors() -> ThermalSensorInfo[]
```

Returns a slice of `ThermalSensorInfo[]`. If thermal sensors are unsupported or unavailable on the current system, returns an empty slice.

This function never panics.

```yukino
load std.sysinfox: thermal_sensors
load std.io: println

set sensors = thermal_sensors()
for i in 0..sensors.len() {
    set s = sensors[i]
    println("Sensor [${s.label}]: " + cast(s.temperature_celsius, "string") + " °C")
}
```

---

## 12. Function Reference: System Process Table

### 12.1 process_list

Returns a snapshot of currently running operating system processes.

```yukino
public fun process_list() -> ProcessInfo[]
```

Returns a slice of `ProcessInfo[]`. If access to the process table is restricted, returns an empty slice.

This function never panics.

```yukino
load std.sysinfox: process_list
load std.io: println

set procs = process_list()
for i in 0..procs.len() {
    set p = procs[i]
    println("PID ${p.pid} (${p.name}): RSS = ${p.memory_rss} bytes")
}
```

### 12.2 process_by_pid (nullable)

Finds a process entry in the system process table by its unique process identifier (PID).

```yukino
public fun process_by_pid(pid: uint32) -> ProcessInfo?
```

Returns a `ProcessInfo` structure or `null` if no process matches the specified PID.

This function never panics.

```yukino
load std.sysinfox: process_by_pid
load std.io: println

set proc = process_by_pid(1u32)
if proc != null {
    println("Init root process: ${proc?.name} (${proc?.cmdline})")
} else {
    println("PID 1 not found")
}
```

---

[← Previous: Module std.os](11-os.md) | [Next: Module std.env →](13-env.md)
