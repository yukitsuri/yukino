# Module std.fs

The `std.fs` module provides a comprehensive suite of cross-platform functions, classes, and structures for file system interaction: path manipulation, directory creation and removal, recursive traversal, file reading and writing, metadata queries, streaming I/O via `class File`, and resolving standard user and system paths (XDG, Windows environment variables, Temp). Archive handling (zip, tar) is separated into the `std.fs.archives` submodule and documented in [16-archives.md](16-archives.md).

---

## 1. Import: load std.fs

Importing the entire package:

```yukino
load std.fs
```

All public components from submodules `paths`, `dirs`, `files`, `defaults`, and `archives` (archives are documented in [16-archives.md](16-archives.md)) are loaded into the current scope:

- **Error and Data Structures:**
  - `FsError` - file system error with message and numerical error code;
  - `FileMetadata` - file or directory metadata (size, flags, timestamps, permissions);
  - `DirEntry` - directory entry record.
- **Stream I/O:**
  - `class File` - object-oriented streaming file handle.
- **Path Operations (`paths`):**
  - Separators: `path_separator`, `path_is_separator`
  - Validation: `path_is_absolute`, `path_is_relative`, `path_exists`, `path_is_file`, `path_is_dir`, `path_is_symlink`
  - Joining: `path_join`
  - Component analysis: `path_parent`, `path_file_name`, `path_file_stem`, `path_extension`, `path_has_extension`
  - Modification: `path_with_extension`
  - Canonicalization: `path_canonicalize`
- **Directory Operations (`dirs`):**
  - Creation: `dir_create`, `dir_create_all`
  - Deletion and copying: `dir_remove`, `dir_remove_all`, `dir_copy_all`
  - State: `dir_exists`, `dir_is_empty`
  - Enumeration: `dir_list`, `dir_walk`
  - Working directory: `dir_current`, `dir_set_current`
- **File Operations (`files`):**
  - Query: `file_exists`, `file_size`
  - Manipulation: `file_copy`, `file_rename`, `file_remove`, `file_touch`, `file_hard_link`, `file_symlink`, `file_set_readonly`, `file_set_permissions`
  - Text I/O: `file_read_text`, `file_write_text`, `file_append_text`, `file_read_lines`
  - Binary I/O: `file_read_bytes`
  - Metadata: `file_metadata`
  - Stream constructors: `open`, `open_read`, `open_write`, `open_append`, `create`, `create_file` (pairs: `file_open_read`, `file_open_write`, `file_open_append`, `file_create`)
- **Standard Paths (`defaults`):**
  - User directories: `dir_home`, `dir_config`, `dir_data`, `dir_data_local`, `dir_cache`, `dir_temp`, `dir_state`, `dir_runtime`, `dir_desktop`, `dir_documents`, `dir_downloads`, `dir_music`, `dir_pictures`, `dir_videos`
  - Application directories: `dir_app_config`, `dir_app_data`, `dir_app_cache`
  - Executable path: `exe_path`, `exe_dir`

All of these items are also accessible when importing the whole standard library: `load std`.

---

## 2. Data and Error Structures

### 2.1 FsError

Error structure raised via `panic` upon system I/O failures.

```yukino
public struct FsError {
    message: string
    code: int32
}
```

- `message` - human-readable operating system error message;
- `code` - numerical error code.

#### FsError Error Codes

| Code (`code`) | Category | Description |
| :--- | :--- | :--- |
| `1` | `NotFound` | File or directory does not exist at specified path. |
| `2` | `PermissionDenied` | Process lacks permissions to perform the requested operation. |
| `3` | `AlreadyExists` | File or directory already exists during exclusive creation. |
| `5` | `DirectoryNotEmpty` | Attempted to remove a non-empty directory via `dir_remove` (use `dir_remove_all` instead). |
| `6` | `IoError` | Generic OS I/O error or invalid/closed file handle. |
| `7` | `InvalidInput` | Invalid path, invalid open mode (`mode`), or invalid `whence` parameter in `seek`. |
| `8` | `Unsupported` | Requested operation is not supported by the filesystem or platform. |

### 2.2 FileMetadata

Metadata describing file or directory attributes, permissions, and timestamps.

```yukino
public struct FileMetadata {
    size: int64
    is_file: bool
    is_dir: bool
    is_symlink: bool
    readonly: bool
    modified_nanos: int64
    created_nanos: int64
    accessed_nanos: int64
    mode: uint32
}
```

- `size` - file size in bytes (for directories, reports OS directory metadata size);
- `is_file` - `true` if entity is a regular file;
- `is_dir` - `true` if entity is a directory;
- `is_symlink` - `true` if entity is a symbolic link;
- `readonly` - `true` if file attributes prevent write access;
- `modified_nanos` - last modification timestamp in nanoseconds since Unix Epoch;
- `created_nanos` - creation timestamp in nanoseconds since Unix Epoch;
- `accessed_nanos` - last access timestamp in nanoseconds since Unix Epoch;
- `mode` - access permissions as an `uint32` integer (interpreted as Unix permission bits).

### 2.3 DirEntry

Record describing a directory entry. The structure is declared publicly, but the enumeration functions `dir_list` and `dir_walk` do not return an array of structures; they return an array of path strings (`string[]`).

```yukino
public struct DirEntry {
    path: string
    file_name: string
    is_file: bool
    is_dir: bool
    is_symlink: bool
    size: int64
}
```

- `path` - full path to entry;
- `file_name` - entry name without leading directory path;
- `is_file` - regular file flag;
- `is_dir` - directory flag;
- `is_symlink` - symbolic link flag;
- `size` - file size in bytes.

---

## 3. Class File (Streaming I/O)

The `File` class wraps an operating system file descriptor to provide continuous streaming byte and text input/output operations. `File` instances are normally created by the constructor functions `open_read`, `open_write`, `open_append`, and `create_file` (see section 6) rather than by calling `init` directly.

### 3.1 init

Constructs a `File` object from an already opened numeric descriptor and the file path.

```yukino
public fun init(fd: int32, path: string)
```

The `fd` argument is a descriptor number obtained from an open function (used internally by `open_read` and the other constructors); `path` is the file path stored in the instance field for error messages. Initializes the instance fields `fd` and `path` and returns no value. This function never panics and does not open any file itself.

```yukino
load std.fs.files: File, open_read
load std.io: println

set file = open_read("app.log")
println("Opened file: ${file.path}")
file.close()
```

### 3.2 read

Reads up to `max_len` bytes from the current file position into the buffer `buf`.

```yukino
public fun read(buf: ptr<uint8>, max_len: int64) -> int64
```

Returns the number of bytes actually read as an `int64`. Returns 0 at end-of-file. If `buf` is `null` or `max_len <= 0`, returns 0 without touching the OS. This function never returns null.

Panics with an `FsError` structure (code 6) on I/O error or if called on a closed file descriptor.

```yukino
load std.fs.files: open_read
load std.io: println

set file = open_read("data.bin")
set buf: uint8[] = []
for i in 0..64 {
    buf.push(0)
}
set n = file.read(buf.as_ptr(), 64)
file.close()
println("Bytes read: ${n}")
```

### 3.3 write

Writes `len` bytes from the buffer `buf` to the file starting at the current cursor position.

```yukino
public fun write(buf: ptr<uint8>, len: int64) -> int64
```

Returns the number of bytes successfully written as an `int64`. If `buf` is `null` or `len <= 0`, returns 0 without touching the OS. This function never returns null.

Panics with an `FsError` structure (code 6) on OS write failure or when writing to a closed descriptor.

```yukino
load std.fs.files: open_write
load std.io: println

set file = open_write("output.bin")
set written = file.write_text("Yukino standard library\n")
file.close()
println("Bytes written: ${written}")
```

### 3.4 write_text

Writes a UTF-8 string to the file at the current cursor position. Implemented on top of the `write` method, passing it the string bytes.

```yukino
public fun write_text(text: string) -> int64
```

Returns the number of bytes written as `int64`. This function never returns null.

Panics with an `FsError` structure (code 6) on disk write failure.

```yukino
load std.fs.files: open_write
load std.io: println

set file = open_write("note.txt")
set written = file.write_text("Yukino standard library\n")
file.close()
println("Bytes written: ${written}")
```

### 3.5 seek

Repositions the read/write cursor inside the file to a specified offset.

```yukino
public fun seek(offset: int64, whence: int32) -> int64
```

Returns the new absolute cursor offset in bytes from the start of the file. This function never returns null.

Panics with an `FsError` structure if `whence` is invalid (code 7) or on system seek failure (code 6).

Values for `whence`:
- `0` (`SEEK_SET`) - offset relative to the beginning of the file;
- `1` (`SEEK_CUR`) - offset relative to the current cursor position;
- `2` (`SEEK_END`) - offset relative to the end of the file.

```yukino
load std.fs.files: open_read
load std.io: println

set file = open_read("archive.dat")
set file_size = file.seek(0, 2)
file.close()
println("Total size: ${file_size} bytes")
```

### 3.6 tell

Returns the current read/write cursor position in the file.

```yukino
public fun tell() -> int64
```

Returns a 64-bit integer offset in bytes from the start of the file. This function never returns null.

Panics with an `FsError` structure (code 6) on system query failure.

```yukino
load std.fs.files: open_read
load std.io: println

set file = open_read("data.bin")
file.seek(32, 0)
set offset = file.tell()
file.close()
println("Current offset: ${offset}")
```

### 3.7 flush

Flushes unwritten internal buffers to physical disk.

```yukino
public fun flush() -> bool
```

Returns `true` when the flush operation succeeds. This function never returns null.

Panics with an `FsError` structure (code 6) on system I/O error.

```yukino
load std.fs.files: open_append
load std.io: println

set file = open_append("audit.log")
file.write_text("checkpoint\n")
set ok = file.flush()
file.close()
println("Flushed: ${ok}")
```

### 3.8 close

Closes the operating system file descriptor and releases all associated resources.

```yukino
public fun close() -> bool
```

Returns `true` upon successful closure. Calling `close` again on an already closed file also returns `true`: closing is idempotent. This function never returns null.

This function never panics.

```yukino
load std.fs.files: create_file
load std.io: println

set file = create_file("test.txt")
file.write_text("data\n")
set closed = file.close()
println("Closed: ${closed}")
```

---

## 4. Path Operations (std.fs.paths)

### 4.1 path_separator

Returns the path separator string used by the standard library.

```yukino
public fun path_separator() -> string
```

Returns the platform-specific path component separator for the target platform (the string `"\\"` on Windows and `"/"` on all other platforms). Resolved at compile time via the `get_target!()` macro. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_separator
load std.io: println

println("Path separator: " + path_separator())
```

### 4.2 path_is_separator

Determines whether the given character is a valid path separator.

```yukino
public fun path_is_separator(c: char) -> bool
```

Returns `true` if the character equals `/` or `\`, otherwise `false`. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_is_separator
load std.io: println

set ok = path_is_separator('/')
println("Is separator: ${ok}")
```

### 4.3 path_is_absolute

Determines whether the specified path is absolute.

```yukino
public fun path_is_absolute(path: string) -> bool
```

Returns `true` if path starts with a root slash `/` or backslash `\` (Unix/UNC), or consists of a drive letter with a colon and a separator, such as `C:\`. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_is_absolute
load std.io: println

set is_abs = path_is_absolute("/etc/hosts")
println("Is absolute: ${is_abs}")
```

### 4.4 path_is_relative

Determines whether the specified path is relative.

```yukino
public fun path_is_relative(path: string) -> bool
```

Returns `true` if the path is not absolute (see `path_is_absolute`) and is evaluated relative to the current working directory. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_is_relative
load std.io: println

set is_rel = path_is_relative("docs/api.md")
println("Is relative: ${is_rel}")
```

### 4.5 path_exists

Checks whether any filesystem entity (file, directory, link) exists at the specified path.

```yukino
public fun path_exists(path: string) -> bool
```

Returns `true` if an entity exists at the specified path, otherwise `false`. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_exists
load std.io: println

set exists = path_exists("config.toml")
println("Path exists: ${exists}")
```

### 4.6 path_is_file

Checks whether the path points to a regular file (not a directory or symbolic link).

```yukino
public fun path_is_file(path: string) -> bool
```

Returns `true` if the path exists and is a regular file, otherwise `false`. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_is_file
load std.io: println

set regular = path_is_file("build.sh")
println("Is regular file: ${regular}")
```

### 4.7 path_is_dir

Checks whether the path points to a directory.

```yukino
public fun path_is_dir(path: string) -> bool
```

Returns `true` if the path exists and is a directory, otherwise `false`. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_is_dir
load std.io: println

set is_directory = path_is_dir("src")
println("Is directory: ${is_directory}")
```

### 4.8 path_is_symlink

Checks whether the path points to a symbolic link.

```yukino
public fun path_is_symlink(path: string) -> bool
```

Returns `true` if the path exists and is a symbolic link, otherwise `false`. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_is_symlink
load std.io: println

set is_link = path_is_symlink("current")
println("Is symlink: ${is_link}")
```

### 4.9 path_join

Joins an arbitrary number of file path segments into a unified path string, correctly normalizing separators.

```yukino
public fun path_join(...parts: string) -> string
```

Accepts a variadic number of path segments (`...parts: string`). If no arguments are passed, returns an empty string `""`. When passed a single argument, returns it. Empty segments are skipped automatically. If any subsequent segment is an absolute path (`path_is_absolute`), all previously accumulated segments are discarded and joining starts anew from that absolute path. Redundant separators and extraneous relative markers (`./`) at boundaries are cleaned without repeating slashes, preserving root paths (`/`, `\`, `C:\`, `C:/`). This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_join
load std.io: println

set joined = path_join("src", "main.ykn")
println(joined) // "src/main.ykn"

set full = path_join("var", "log", "audit.log")
println(full) // "var/log/audit.log"

set reset = path_join("base", "dir", "/etc/hosts")
println(reset) // "/etc/hosts"
```

### 4.10 path_parent

Calculates the parent directory path for the given path string.

```yukino
public fun path_parent(path: string) -> string
```

Returns the parent directory string. For root paths (`/`, `\`) returns `"/"`. For an empty path or a single-segment path without a parent, returns an empty string `""`. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_parent
load std.io: println

set parent = path_parent("/home/user/document.txt")
println("Parent directory: ${parent}") // "/home/user"
```

### 4.11 path_file_name

Extracts the filename or trailing segment of the path.

```yukino
public fun path_file_name(path: string) -> string
```

Returns the trailing segment string. For root paths returns `"/"`; for an empty path returns an empty string `""`. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_file_name
load std.io: println

set name = path_file_name("/home/user/report.pdf")
println("Filename: ${name}") // "report.pdf"
```

### 4.12 path_file_stem

Extracts the filename stem excluding the trailing extension and dot.

```yukino
public fun path_file_stem(path: string) -> string
```

Returns the stem of the filename (the segment before the last dot). If the filename is absent or empty, returns an empty string `""`. For hidden files such as `.gitignore` the leading dot is treated as part of the name and the whole name is returned. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_file_stem
load std.io: println

set stem = path_file_stem("archive.tar.gz")
println("Stem: ${stem}") // "archive.tar"
```

### 4.13 path_extension

Extracts the file extension (all characters following the final dot).

```yukino
public fun path_extension(path: string) -> string
```

Returns the extension string without leading dot. If the extension is absent (no dot, or a dot at the start of a hidden file), returns an empty string `""`. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_extension
load std.io: println

set ext = path_extension("document.docx")
println("Extension: ${ext}") // "docx"
```

### 4.14 path_has_extension

Checks whether the filename in the specified path contains an extension.

```yukino
public fun path_has_extension(path: string) -> bool
```

Returns `true` if path contains a non-empty extension following a dot, otherwise `false`. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_has_extension
load std.io: println

set has_ext = path_has_extension("build.sh")
println("Has extension: ${has_ext}") // true
```

### 4.15 path_with_extension

Replaces an existing file extension or adds one if missing.

```yukino
public fun path_with_extension(path: string, new_ext: string) -> string
```

Returns the path with updated extension (the name becomes `path_file_stem` + `.` + `new_ext`, joined with `path_parent`). If `new_ext` is the empty string `""`, the current extension is removed. This function never returns null.

This function never panics.

```yukino
load std.fs.paths: path_with_extension
load std.io: println

set updated = path_with_extension("image.jpeg", "png")
println(updated) // "image.png"
```

### 4.16 path_canonicalize

Resolves the path to its absolute canonical form by querying the filesystem (resolving symlinks and `.`/`..` segments).

```yukino
public fun path_canonicalize(path: string) -> string
```

Returns the canonical path as a `string`. This function never returns null.

Panics with an `FsError` structure if the path does not exist or the system call fails (the error code is taken from the OS; fallback code 1).

```yukino
load std.fs.paths: path_canonicalize, FsError
load std.io: println

sus {
    set real = path_canonicalize("docs/../config")
    println("Canonical path: ${real}")
} catch err: FsError {
    println("Canonicalization failed: ${err.message}")
}
```

---

## 5. Directory Operations (std.fs.dirs)

For every `dir_`-prefixed function there is an identical pair without the prefix (`dir_create`/`create_dir`, `dir_remove`/`remove_dir`, `dir_list`/`list_dir`, `dir_walk`/`walk_dir`, `dir_current`/`current_dir`, `dir_set_current`/`set_current_dir`, `dir_is_empty`/`is_dir_empty`, `dir_copy_all`/`copy_dir_all`, etc.); both forms are exported by the module.

### 5.1 dir_create

Creates a single new directory at the specified path.

```yukino
public fun dir_create(path: string) -> bool
```

Returns `true` upon successful directory creation. The parent directory must already exist. This function never returns null.

Panics with an `FsError` structure on OS failure (missing parent directory, permission denied, or already exists; the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.dirs: dir_create
load std.fs.paths: FsError
load std.io: println

sus {
    set ok = dir_create("logs")
    println("Created: ${ok}")
} catch err: FsError {
    println("Failed to create directory: ${err.message}")
}
```

### 5.2 dir_create_all

Recursively creates a directory and all missing parent directories.

```yukino
public fun dir_create_all(path: string) -> bool
```

Returns `true` when target directory and all intermediate directories are successfully created. If directory already exists, returns `true` without error. This function never returns null.

Panics with an `FsError` structure on permission failure or filesystem error (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.dirs: dir_create_all
load std.io: println

set ok = dir_create_all("build/output/bin")
println("Directory tree created: ${ok}")
```

### 5.3 dir_remove

Removes an empty directory at the specified path.

```yukino
public fun dir_remove(path: string) -> bool
```

Returns `true` upon successful removal. This function never returns null.

Panics with an `FsError` structure if directory is not empty (`code = 5`), not found (`code = 1`), or permission denied (`code = 2`); the error code is taken from the OS, fallback code 6.

```yukino
load std.fs.dirs: dir_remove
load std.fs.paths: FsError
load std.io: println

sus {
    set ok = dir_remove("temp_dir")
    println("Removed: ${ok}")
} catch err: FsError {
    println("Removal error: ${err.message}")
}
```

### 5.4 dir_remove_all

Recursively removes a directory along with all its files and subdirectories.

```yukino
public fun dir_remove_all(path: string) -> bool
```

Returns `true` upon complete deletion of the directory tree. If the directory does not exist, also returns `true` without error. This function never returns null.

Panics with an `FsError` structure on permission error or OS failure (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.dirs: dir_remove_all
load std.io: println

set ok = dir_remove_all("cache/temp")
println("Tree removed: ${ok}")
```

### 5.5 dir_exists

Checks whether an entity exists at the specified path and is a directory.

```yukino
public fun dir_exists(path: string) -> bool
```

Returns `true` if path exists and points to a directory, otherwise `false`. This function never returns null.

This function never panics.

```yukino
load std.fs.dirs: dir_exists
load std.io: println

set exists = dir_exists("src")
println("Directory exists: ${exists}")
```

### 5.6 dir_is_empty

Checks whether a directory contains zero files or subdirectories.

```yukino
public fun dir_is_empty(path: string) -> bool
```

Returns `true` if directory is empty, or `false` if it contains at least one entry, as well as when the directory does not exist or is not readable. This function never returns null.

This function never panics.

```yukino
load std.fs.dirs: dir_is_empty
load std.io: println

set empty = dir_is_empty("empty_folder")
println("Directory is empty: ${empty}")
```

### 5.7 dir_list

Returns a list of paths of entries located directly inside the specified directory (non-recursive).

```yukino
public fun dir_list(path: string) -> string[]
```

Returns an array of strings `string[]` holding the path of each directory entry. An empty directory yields a zero-length array. This function never returns null.

Panics with an `FsError` structure if directory is missing or read permission is denied (the error code is taken from the OS, fallback code 1).

```yukino
load std.fs.dirs: dir_list
load std.io: println

set entries = dir_list("src")
for entry in entries {
    println(entry)
}
```

### 5.8 dir_walk

Recursively walks the entire directory tree, returning a flat list of paths of all nested files and subdirectories.

```yukino
public fun dir_walk(path: string) -> string[]
```

Returns an array of strings `string[]` holding the paths of all nested entities at any depth. This function never returns null.

Panics with an `FsError` structure on directory access failure (the error code is taken from the OS, fallback code 1).

```yukino
load std.fs.dirs: dir_walk
load std.io: println

set all_files = dir_walk("docs")
println("Total entries found: ${all_files.len()}")
```

### 5.9 dir_copy_all

Recursively copies a directory with all its contents from `from_path` to `to_path`.

```yukino
public fun dir_copy_all(from_path: string, to_path: string) -> bool
```

Returns `true` upon successful copy of the directory tree. This function never returns null.

Panics with an `FsError` structure if the source directory is missing or the write fails (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.dirs: dir_copy_all
load std.io: println

set ok = dir_copy_all("assets", "backup/assets")
println("Tree copied: ${ok}")
```

### 5.10 dir_current

Returns the absolute path to the current working directory of the process.

```yukino
public fun dir_current() -> string
```

Returns the working directory path as a `string`. This function never returns null.

Panics with an `FsError` structure on operating system query failure (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.dirs: dir_current
load std.io: println

set cwd = dir_current()
println("Current working directory: ${cwd}")
```

### 5.11 dir_set_current

Sets the current working directory for the active process.

```yukino
public fun dir_set_current(path: string) -> bool
```

Returns `true` upon successful change of working directory. This function never returns null.

Panics with an `FsError` structure if target directory does not exist or access is denied (the error code is taken from the OS, fallback code 1).

```yukino
load std.fs.dirs: dir_set_current
load std.fs.paths: FsError
load std.io: println

sus {
    set ok = dir_set_current("/tmp")
    println("Changed directory: ${ok}")
} catch err: FsError {
    println("Failed to change directory: ${err.message}")
}
```

---

## 6. File Operations (std.fs.files)

For most `file_`-prefixed functions there is an identical pair without the prefix (`file_read_text`/`read_text`, `file_remove`/`remove_file`, `file_copy`/`copy_file`, `file_metadata`/`metadata`, `file_open_read`/`open_read`, `file_create`/`create_file`, etc.); both forms are exported by the module.

### 6.1 file_exists

Checks whether the path points to a regular file.

```yukino
public fun file_exists(path: string) -> bool
```

Returns `true` if the path exists and is a regular file (not a directory), otherwise `false`. To check for the existence of any entity use `path_exists`; for directories use `dir_exists`. This function never returns null.

This function never panics.

```yukino
load std.fs.files: file_exists
load std.io: println

set exists = file_exists("config.toml")
println("File exists: ${exists}")
```

### 6.2 file_size

Returns the size of the file in bytes.

```yukino
public fun file_size(path: string) -> int64
```

Returns file size in bytes as an `int64`. This function never returns null.

Panics with an `FsError` structure if file does not exist or cannot be accessed (the error code is taken from the OS, fallback code 1).

```yukino
load std.fs.files: file_size
load std.io: println

set bytes = file_size("data.bin")
println("File size: ${bytes} bytes")
```

### 6.3 file_copy

Copies a file from source path `from_path` to destination path `to_path`.

```yukino
public fun file_copy(from_path: string, to_path: string) -> int64
```

Returns the number of bytes copied as an `int64`. This function never returns null.

Panics with an `FsError` structure if source file is missing or write access fails (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: file_copy
load std.io: println

set copied = file_copy("config.toml", "config.toml.bak")
println("Bytes copied: ${copied}")
```

### 6.4 file_rename

Renames or moves a file or directory from `from_path` to `to_path`.

```yukino
public fun file_rename(from_path: string, to_path: string) -> bool
```

Returns `true` upon successful move or rename. This function never returns null.

Panics with an `FsError` structure if source does not exist or destination path is inaccessible (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: file_rename
load std.io: println

set ok = file_rename("temp.log", "archive.log")
println("Renamed: ${ok}")
```

### 6.5 file_remove

Deletes the specified file from the filesystem.

```yukino
public fun file_remove(path: string) -> bool
```

Returns `true` upon successful removal. This function never returns null.

Panics with an `FsError` structure if file is not found (`code = 1`) or access is denied (`code = 2`); the error code is taken from the OS, fallback code 1.

```yukino
load std.fs.files: file_remove
load std.io: println

set removed = file_remove("cache.tmp")
println("Removed: ${removed}")
```

### 6.6 file_touch

Creates an empty file at the specified path if it does not exist, or updates the access and modification times of the existing file.

```yukino
public fun file_touch(path: string) -> bool
```

Returns `true` upon successful completion of the operation. This function never returns null.

Panics with an `FsError` structure on system failure (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: file_touch
load std.io: println

set ok = file_touch("marker.tmp")
println("File created or updated: ${ok}")
```

### 6.7 file_hard_link

Creates a hard link `link` pointing to the existing file `original`.

```yukino
public fun file_hard_link(original: string, link: string) -> bool
```

Returns `true` upon successful link creation. This function never returns null.

Panics with an `FsError` structure if the original file does not exist, the link already exists, or the filesystem does not support hard links (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: file_hard_link
load std.io: println

set ok = file_hard_link("data.bin", "data_alias.bin")
println("Hard link created: ${ok}")
```

### 6.8 file_symlink

Creates a symbolic link `link` pointing to the path `original`.

```yukino
public fun file_symlink(original: string, link: string) -> bool
```

Returns `true` upon successful link creation. This function never returns null.

Panics with an `FsError` structure if the link already exists or the platform refuses to create a symbolic link (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: file_symlink
load std.io: println

set ok = file_symlink("releases/1.2", "current")
println("Symlink created: ${ok}")
```

### 6.9 file_set_readonly

Sets or clears the read-only flag of the file at the specified path.

```yukino
public fun file_set_readonly(path: string, readonly: bool) -> bool
```

Returns `true` upon successful flag change. This function never returns null.

Panics with an `FsError` structure if the file is not found or the operation is rejected by the OS (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: file_set_readonly
load std.io: println

set ok = file_set_readonly("system.dat", true)
println("Flag set: ${ok}")
```

### 6.10 file_set_permissions

Sets the access permissions (mode) of the file as an `uint32` integer.

```yukino
public fun file_set_permissions(path: string, mode: uint32) -> bool
```

Returns `true` upon successful permission change. The `mode` format is platform-dependent (on Unix it is the standard permission bits). This function never returns null.

Panics with an `FsError` structure if the file is not found or the operation is rejected by the OS (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: file_set_permissions
load std.io: println

set ok = file_set_permissions("script.sh", 0o755u32)
println("Permissions set: ${ok}")
```

### 6.11 file_read_text

Reads the entire file contents at specified path into a single UTF-8 string.

```yukino
public fun file_read_text(path: string) -> string
```

Returns full file content as a `string`. This function never returns null.

Panics with an `FsError` structure if file does not exist or read fails:
- `code = 1` (`NotFound`) - file not found;
- `code = 2` (`PermissionDenied`) - insufficient permissions;
- `code = 6` (`IoError`) - system I/O error.

```yukino
load std.fs.files: file_read_text
load std.fs.paths: FsError
load std.io: println

sus {
    set text = file_read_text("config.toml")
    println(text)
} catch err: FsError {
    println("Read error: ${err.message}")
}
```

### 6.12 file_write_text

Creates a new file or overwrites an existing file with the provided text string.

```yukino
public fun file_write_text(path: string, content: string) -> bool
```

Returns `true` upon successful write. This function never returns null.

Panics with an `FsError` structure on file creation or write failure (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: file_write_text
load std.io: println

set ok = file_write_text("greeting.txt", "Hello from Yukino!\n")
println("Written: ${ok}")
```

### 6.13 file_append_text

Appends text to the end of a file, automatically creating it if missing.

```yukino
public fun file_append_text(path: string, content: string) -> bool
```

Returns `true` upon successful append. This function never returns null.

Panics with an `FsError` structure on file open or write failure (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: file_append_text
load std.io: println

set ok = file_append_text("events.log", "event: user_login\n")
println("Appended: ${ok}")
```

### 6.14 file_read_lines

Reads a text file and splits its contents into an array of lines on newline boundaries (`\n` or `\r\n`).

```yukino
public fun file_read_lines(path: string) -> string[]
```

Returns an array of strings `string[]` with newline terminators stripped. This function never returns null.

Panics with an `FsError` structure on read error or if file is missing (error codes as in `file_read_text`).

```yukino
load std.fs.files: file_read_lines
load std.io: println

set lines = file_read_lines("todo.txt")
for line in lines {
    println("- ${line}")
}
```

### 6.15 file_read_bytes

Reads binary data from a file into the pre-allocated buffer `buf` of at least `max_len` bytes.

```yukino
public fun file_read_bytes(path: string, buf: ptr<uint8>, max_len: int64) -> int64
```

Returns the number of bytes actually read as an `int64`. If `buf` is `null` or `max_len <= 0`, returns 0 without touching the OS. This function never returns null.

Panics with an `FsError` structure if file is missing or on I/O error (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: file_read_bytes
load std.io: println

set buf: uint8[] = []
for i in 0..1024 {
    buf.push(0)
}
set n = file_read_bytes("image.png", buf.as_ptr(), 1024)
println("Bytes read: ${n}")
```

### 6.16 file_metadata

Queries filesystem metadata for the specified file or directory.

```yukino
public fun file_metadata(path: string) -> FileMetadata
```

Returns a `FileMetadata` structure (see section 2.2) containing size, flags, permissions, and timestamps. This function never returns null.

Panics with an `FsError` structure if target entity does not exist or access is denied (the error code is taken from the OS, fallback code 1).

```yukino
load std.fs.files: file_metadata
load std.io: println

set meta = file_metadata("main.ykn")
println("Size: ${meta.size} bytes, is_dir: ${meta.is_dir}")
```

### 6.17 open_read

Opens an existing file for reading only and returns a streaming `File` object.

```yukino
public fun open_read(path: string) -> File
```

Returns a `File` instance with an open descriptor. This function never returns null.

Panics with an `FsError` structure if the file is not found or inaccessible (the error code is taken from the OS, fallback code 1).

```yukino
load std.fs.files: open_read
load std.io: println

set file = open_read("data.bin")
set buf: uint8[] = []
for i in 0..64 {
    buf.push(0)
}
set n = file.read(buf.as_ptr(), 64)
file.close()
println("Bytes read: ${n}")
```

### 6.18 open_write

Opens a file for writing: creates a new file or truncates an existing one to zero length.

```yukino
public fun open_write(path: string) -> File
```

Returns a `File` instance with an open descriptor. This function never returns null.

Panics with an `FsError` structure on open or creation failure (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: open_write
load std.io: println

set file = open_write("output.bin")
set written = file.write_text("Yukino")
file.close()
println("Bytes written: ${written}")
```

### 6.19 open_append

Opens a file for appending data to the end, creating it if missing.

```yukino
public fun open_append(path: string) -> File
```

Returns a `File` instance with an open descriptor. This function never returns null.

Panics with an `FsError` structure on open or creation failure (the error code is taken from the OS, fallback code 6).

```yukino
load std.fs.files: open_append
load std.fs.paths: FsError
load std.io: println

sus {
    set file = open_append("app.log")
    file.write_text("Application started\n")
    file.close()
} catch err: FsError {
    println("Failed to open file: ${err.message}")
}
```

### 6.20 create_file

Creates a new file in exclusive mode: if the file already exists, the operation fails with `AlreadyExists` (`code = 3`).

```yukino
public fun create_file(path: string) -> File
```

Returns a `File` instance with an open descriptor available for writing. This function never returns null.

Panics with an `FsError` structure if the file already exists or creation is rejected by the OS (the error code is taken from the OS, fallback code 3).

```yukino
load std.fs.files: create_file
load std.fs.paths: FsError
load std.io: println

sus {
    set file = create_file("session.tmp")
    file.write_text("created\n")
    file.close()
} catch err: FsError {
    println("Failed to create file: ${err.message}")
}
```

In addition to the functions above, the module exports synonym pairs: `open` (equivalent to `open_read`), `create` (equivalent to `create_file`), `file_open_read`, `file_open_write`, `file_open_append`, and `file_create` - they behave identically to the corresponding functions above.

---

## 7. Standard System Paths (std.fs.defaults)

All functions in the `defaults` submodule are cross-platform. On Linux they adhere to the XDG Base Directory specification, on Windows they query environment variables (`APPDATA`, `LOCALAPPDATA`, `USERPROFILE`, etc.), and on macOS they use standard system directories. Every function returns a `string`, never returns null, and - with the exception of `exe_path` - never panics: if the path cannot be resolved, an empty string `""` or a fallback path (the temporary directory) is returned. For every `dir_x` function there is an `x_dir` pair (`dir_home`/`home_dir`, etc.).

### 7.1 dir_home

Returns the path to the home directory of the current user.

```yukino
public fun dir_home() -> string
```

Returns home directory path (e.g. `"/home/username"` on Linux or `"C:\Users\username"` on Windows; taken from `HOME` on Unix, or from `USERPROFILE` / `HOMEDRIVE` + `HOMEPATH` on Windows). This function never returns null.

This function never panics: if the home directory cannot be resolved, an empty string `""` is returned.

```yukino
load std.fs.defaults: dir_home
load std.io: println

set home = dir_home()
println("Home directory: ${home}")
```

### 7.2 dir_config

Returns the path to the user configuration directory.

```yukino
public fun dir_config() -> string
```

Returns configuration directory path (`"~/.config"` on Linux honoring `XDG_CONFIG_HOME`, `%APPDATA%` on Windows). This function never returns null.

This function never panics: if the directory cannot be resolved, a fallback path (the temporary directory) is returned.

```yukino
load std.fs.defaults: dir_config
load std.io: println

set cfg = dir_config()
println("Config directory: ${cfg}")
```

### 7.3 dir_data

Returns the path to the user data directory for applications.

```yukino
public fun dir_data() -> string
```

Returns user data directory path (`"~/.local/share"` on Linux honoring `XDG_DATA_HOME`, `%LOCALAPPDATA%` on Windows). This function never returns null.

This function never panics: if the directory cannot be resolved, a fallback path (the temporary directory) is returned.

```yukino
load std.fs.defaults: dir_data
load std.io: println

set data_dir = dir_data()
println("Data directory: ${data_dir}")
```

### 7.4 dir_data_local

Returns the path to the user local data directory. In the current implementation it is identical to `dir_data`.

```yukino
public fun dir_data_local() -> string
```

Returns local user data directory path (`"~/.local/share"` on Linux, `%LOCALAPPDATA%` on Windows). This function never returns null.

This function never panics: if the directory cannot be resolved, a fallback path (the temporary directory) is returned.

```yukino
load std.fs.defaults: dir_data_local
load std.io: println

set local_data = dir_data_local()
println("Local data directory: ${local_data}")
```

### 7.5 dir_cache

Returns the path to the user temporary cache directory.

```yukino
public fun dir_cache() -> string
```

Returns cache directory path (`"~/.cache"` on Linux honoring `XDG_CACHE_HOME`, `%LOCALAPPDATA%\cache` on Windows). This function never returns null.

This function never panics: if the directory cannot be resolved, a fallback path (the temporary directory) is returned.

```yukino
load std.fs.defaults: dir_cache
load std.io: println

set cache_dir = dir_cache()
println("Cache directory: ${cache_dir}")
```

### 7.6 dir_temp

Returns the path to the standard system temporary directory.

```yukino
public fun dir_temp() -> string
```

Returns temporary directory path string (`"/tmp"` on Linux, `%TEMP%` on Windows). This function never returns null.

This function never panics.

```yukino
load std.fs.defaults: dir_temp
load std.io: println

set tmp = dir_temp()
println("Temp directory: ${tmp}")
```

### 7.7 dir_state

Returns the path to the user application state directory (history, logs, sessions).

```yukino
public fun dir_state() -> string
```

Returns application state directory path (`"~/.local/state"` on Linux honoring `XDG_STATE_HOME`, `%LOCALAPPDATA%` on Windows). This function never returns null.

This function never panics: if the directory cannot be resolved, a fallback path (the temporary directory) is returned.

```yukino
load std.fs.defaults: dir_state
load std.io: println

set state_dir = dir_state()
println("State directory: ${state_dir}")
```

### 7.8 dir_runtime

Returns the path to the user runtime directory.

```yukino
public fun dir_runtime() -> string
```

Returns the path from `XDG_RUNTIME_DIR` on Unix systems; if the variable is unset, the OS temporary directory is returned. This function never returns null.

This function never panics.

```yukino
load std.fs.defaults: dir_runtime
load std.io: println

set rt = dir_runtime()
println("Runtime directory: ${rt}")
```

### 7.9 dir_desktop

Returns the path to the `Desktop` subdirectory of the current user home directory.

```yukino
public fun dir_desktop() -> string
```

Returns path to Desktop folder (`"~/Desktop"` on Linux/macOS). This function never returns null.

This function never panics: if the home directory is unresolved, an empty string `""` is returned.

```yukino
load std.fs.defaults: dir_desktop
load std.io: println

set dt = dir_desktop()
println("Desktop directory: ${dt}")
```

### 7.10 dir_documents

Returns the path to the `Documents` subdirectory of the user home directory.

```yukino
public fun dir_documents() -> string
```

Returns path to Documents directory (`"~/Documents"`). This function never returns null.

This function never panics: if the home directory is unresolved, an empty string `""` is returned.

```yukino
load std.fs.defaults: dir_documents
load std.io: println

set docs = dir_documents()
println("Documents directory: ${docs}")
```

### 7.11 dir_downloads

Returns the path to the `Downloads` subdirectory of the user home directory.

```yukino
public fun dir_downloads() -> string
```

Returns path to Downloads directory (`"~/Downloads"`). This function never returns null.

This function never panics: if the home directory is unresolved, an empty string `""` is returned.

```yukino
load std.fs.defaults: dir_downloads
load std.io: println

set dls = dir_downloads()
println("Downloads directory: ${dls}")
```

### 7.12 dir_music

Returns the path to the `Music` subdirectory of the user home directory.

```yukino
public fun dir_music() -> string
```

Returns path to Music directory (`"~/Music"`). This function never returns null.

This function never panics: if the home directory is unresolved, an empty string `""` is returned.

```yukino
load std.fs.defaults: dir_music
load std.io: println

set music = dir_music()
println("Music directory: ${music}")
```

### 7.13 dir_pictures

Returns the path to the `Pictures` subdirectory of the user home directory.

```yukino
public fun dir_pictures() -> string
```

Returns path to Pictures directory (`"~/Pictures"`). This function never returns null.

This function never panics: if the home directory is unresolved, an empty string `""` is returned.

```yukino
load std.fs.defaults: dir_pictures
load std.io: println

set pics = dir_pictures()
println("Pictures directory: ${pics}")
```

### 7.14 dir_videos

Returns the path to the `Videos` subdirectory of the user home directory.

```yukino
public fun dir_videos() -> string
```

Returns path to Videos directory (`"~/Videos"`). This function never returns null.

This function never panics: if the home directory is unresolved, an empty string `""` is returned.

```yukino
load std.fs.defaults: dir_videos
load std.io: println

set vids = dir_videos()
println("Videos directory: ${vids}")
```

### 7.15 dir_app_config

Returns the configuration directory of a specific application: `path_join(dir_config(), app_name)`.

```yukino
public fun dir_app_config(app_name: string) -> string
```

Returns the joined path. This function never returns null.

This function never panics.

```yukino
load std.fs.defaults: dir_app_config
load std.io: println

set cfg = dir_app_config("myapp")
println("Application config directory: ${cfg}")
```

### 7.16 dir_app_data

Returns the data directory of a specific application: `path_join(dir_data(), app_name)`.

```yukino
public fun dir_app_data(app_name: string) -> string
```

Returns the joined path. This function never returns null.

This function never panics.

```yukino
load std.fs.defaults: dir_app_data
load std.io: println

set data = dir_app_data("myapp")
println("Application data directory: ${data}")
```

### 7.17 dir_app_cache

Returns the cache directory of a specific application: `path_join(dir_cache(), app_name)`.

```yukino
public fun dir_app_cache(app_name: string) -> string
```

Returns the joined path. This function never returns null.

This function never panics.

```yukino
load std.fs.defaults: dir_app_cache
load std.io: println

set cache = dir_app_cache("myapp")
println("Application cache directory: ${cache}")
```

### 7.18 exe_path

Returns the full absolute path to the running executable binary of the current process.

```yukino
public fun exe_path() -> string
```

Returns absolute executable path (e.g. `"/usr/bin/my_app"`). This function never returns null.

Panics with an `FsError` structure (code 6) if executable path cannot be resolved from the OS.

```yukino
load std.fs.defaults: exe_path
load std.io: println

set path = exe_path()
println("Executable path: ${path}")
```

### 7.19 exe_dir

Returns the path to the directory containing the running executable of the current process: `path_parent(exe_path())`.

```yukino
public fun exe_dir() -> string
```

Returns directory path string where the current process binary resides. This function never returns null.

Panics only when the inner `exe_path` call fails, with an `FsError` structure (code 6).

```yukino
load std.fs.defaults: exe_dir
load std.io: println

set bin_dir = exe_dir()
println("Binary directory: ${bin_dir}")
```

---

[← Back: Module std.random](7-random.md) | [Next: Module std.collections →](9-collections.md)
