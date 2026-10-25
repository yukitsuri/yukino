# Модуль std.fs

Модуль `std.fs` предоставляет полный набор кроссплатформенных функций, классов и структур для манипуляции файловой системой: работа с путями, создание и удаление директорий, рекурсивный обход, чтение и запись файлов, метаданные, потоковый ввод-вывод через `class File`, а также определение стандартных пользовательских и системных путей (XDG, переменные окружения Windows, Temp). Работа с архивами (zip, tar) выделена в подмодуль `std.fs.archives` и описана в документе [16-archives.md](16-archives.md).

---

## 1. Подключение: load std.fs

Импорт всего пакета:

```yukino
load std.fs
```

В текущую область видимости импортируются все компоненты подмодулей `paths`, `dirs`, `files`, `defaults` и `archives` (архивы описаны в [16-archives.md](16-archives.md)):

- **Структуры ошибок и данных:**
  - `FsError` — структура ошибки файловой системы с текстовым описанием и числовым кодом ошибки;
  - `FileMetadata` — метаданные файла или директории (размер, флаги, временные метки, права доступа);
  - `DirEntry` — запись об элементе директории.
- **Потоковый ввод-вывод:**
  - `class File` — объектно-ориентированный потоковый файловый дескриптор.
- **Функции работы с путями (`paths`):**
  - Разделители: `path_separator`, `path_is_separator`
  - Проверка пути: `path_is_absolute`, `path_is_relative`, `path_exists`, `path_is_file`, `path_is_dir`, `path_is_symlink`
  - Объединение: `path_join`, `path_join3`
  - Анализ компонентов: `path_parent`, `path_file_name`, `path_file_stem`, `path_extension`, `path_has_extension`
  - Модификация: `path_with_extension`
  - Канонизация: `path_canonicalize`
- **Функции работы с директориями (`dirs`):**
  - Создание: `dir_create`, `dir_create_all`
  - Удаление и копирование: `dir_remove`, `dir_remove_all`, `dir_copy_all`
  - Состояние: `dir_exists`, `dir_is_empty`
  - Перечисление: `dir_list`, `dir_walk`
  - Рабочая директория: `dir_current`, `dir_set_current`
- **Функции работы с файлами (`files`):**
  - Проверка: `file_exists`, `file_size`
  - Манипуляция: `file_copy`, `file_rename`, `file_remove`, `file_touch`, `file_hard_link`, `file_symlink`, `file_set_readonly`, `file_set_permissions`
  - Текстовый ввод-вывод: `file_read_text`, `file_write_text`, `file_append_text`, `file_read_lines`
  - Байтовый ввод-вывод: `file_read_bytes`
  - Метаданные: `file_metadata`
  - Конструкторы потоков: `open`, `open_read`, `open_write`, `open_append`, `create`, `create_file` (пары: `file_open_read`, `file_open_write`, `file_open_append`, `file_create`)
- **Стандартные системные пути (`defaults`):**
  - Пользовательские директории: `dir_home`, `dir_config`, `dir_data`, `dir_data_local`, `dir_cache`, `dir_temp`, `dir_state`, `dir_runtime`, `dir_desktop`, `dir_documents`, `dir_downloads`, `dir_music`, `dir_pictures`, `dir_videos`
  - Каталоги приложений: `dir_app_config`, `dir_app_data`, `dir_app_cache`
  - Исполняемый файл: `exe_path`, `exe_dir`

Все эти функции также доступны при импорте всей стандартной библиотеки: `load std`.

---

## 2. Структуры данных и ошибок

### 2.1 FsError

Структура ошибки, передаваемая в `panic` при системных сбоях файловых операций.

```yukino
public struct FsError {
    message: string
    code: int32
}
```

- `message` — текстовое описание ошибки операционной системы;
- `code` — числовой код ошибки.

#### Коды ошибок FsError

| Код (`code`) | Категория | Описание и условия возникновения |
| :--- | :--- | :--- |
| `1` | `NotFound` | Файл или директория не найдены по указанному пути. |
| `2` | `PermissionDenied` | Отказано в доступе: у процесса недостаточно прав для выполнения операции. |
| `3` | `AlreadyExists` | Файл или каталог уже существует при попытке эксклюзивного создания. |
| `5` | `DirectoryNotEmpty` | Попытка удаления непустой директории через `dir_remove` (требуется `dir_remove_all`). |
| `6` | `IoError` | Общая ошибка ввода-вывода ОС или невалидный/закрытый дескриптор файла. |
| `7` | `InvalidInput` | Некорректный путь, невалидный режим открытия (`mode`) или некорректный параметр `whence` в `seek`. |
| `8` | `Unsupported` | Операция не поддерживается файловой системой или платформой. |

### 2.2 FileMetadata

Сведения о свойствах файла или каталога.

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

- `size` — размер файла в байтах (для каталогов возвращается размер структуры каталога в ОС);
- `is_file` — `true`, если объект является обычным файлом;
- `is_dir` — `true`, если объект является директорией;
- `is_symlink` — `true`, если объект является символической ссылкой;
- `readonly` — `true`, если атрибуты разрешают только чтение;
- `modified_nanos` — время последней модификации в наносекундах (Unix Epoch);
- `created_nanos` — время создания в наносекундах;
- `accessed_nanos` — время последнего доступа в наносекундах;
- `mode` — права доступа к объекту в виде целого `uint32` (интерпретируется как набор бит прав доступа Unix).

### 2.3 DirEntry

Запись об элементе директории. Структура объявлена публично, однако функции перечисления `dir_list` и `dir_walk` возвращают не массив структур, а массив строк-путей (`string[]`).

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

- `path` — полный путь к элементу;
- `file_name` — имя файла или каталога без пути;
- `is_file` — флаг обычного файла;
- `is_dir` — флаг директории;
- `is_symlink` — флаг символической ссылки;
- `size` — размер файла в байтах.

---

## 3. Класс File (Потоковый ввод-вывод)

Класс `File` реализует непрерывную потоковую работу с файлами через файловый дескриптор операционной системы. Экземпляры `File` обычно создаются функциями-конструкторами `open_read`, `open_write`, `open_append` и `create_file` (см. раздел 6), а не прямым вызовом `init`.

### 3.1 init

Конструирует объект `File` на основе уже открытого числового дескриптора и пути к файлу.

```yukino
public fun init(fd: int32, path: string)
```

Аргумент `fd` — число-дескриптор, полученный от функции открытия (используется конструкторами `open_read` и другими), `path` — путь к файлу, сохраняемый в поле экземпляра для сообщений об ошибках. Конструктор инициализирует поля `fd` и `path` и не возвращает значений. Функция никогда не паникует и не открывает файл сама.

```yukino
load std.fs.files: File, open_read
load std.io: println

set file = open_read("app.log")
println("Открыт файл: ${file.path}")
file.close()
```

### 3.2 read

Считывает не более `max_len` байт из текущей позиции открытого файла в буфер `buf`.

```yukino
public fun read(buf: ptr<uint8>, max_len: int64) -> int64
```

Возвращает количество фактически прочитанных байт как `int64`. При достижении конца файла возвращает 0. Если `buf` равен `null` или `max_len <= 0`, возвращает 0 без обращения к ОС. Функция никогда не возвращает null.

В случае системной ошибки ввода-вывода или попытки чтения из закрытого дескриптора вызывает панику со структурой `FsError` (код 6).

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
println("Прочитано байт: ${n}")
```

### 3.3 write

Записывает `len` байт из буфера `buf` в файл, начиная с текущей позиции указателя.

```yukino
public fun write(buf: ptr<uint8>, len: int64) -> int64
```

Возвращает фактическое количество успешно записанных байт как `int64`. Если `buf` равен `null` или `len <= 0`, возвращает 0 без обращения к ОС. Функция никогда не возвращает null.

В случае ошибки записи операционной системы или записи в закрытый дескриптор вызывает панику со структурой `FsError` (код 6).

```yukino
load std.fs.files: open_write
load std.io: println

set file = open_write("output.bin")
set written = file.write_text("Yukino standard library\n")
file.close()
println("Записано байт: ${written}")
```

### 3.4 write_text

Записывает UTF-8 строку в файл, начиная с текущей позиции файлового дескриптора. Реализована поверх метода `write` и передаёт ему байты строки.

```yukino
public fun write_text(text: string) -> int64
```

Возвращает число записанных байт как `int64`. Функция никогда не возвращает null.

В случае ошибки ввода-вывода на диск вызывает панику со структурой `FsError` (код 6).

```yukino
load std.fs.files: open_write
load std.io: println

set file = open_write("note.txt")
set written = file.write_text("Yukino standard library\n")
file.close()
println("Записано байт: ${written}")
```

### 3.5 seek

Перемещает внутренний указатель позиции чтения/записи файла на заданное смещение.

```yukino
public fun seek(offset: int64, whence: int32) -> int64
```

Возвращает итоговое смещение указателя в байтах от начала файла. Функция никогда не возвращает null.

В случае некорректного значения `whence` (код 7) или системной ошибки перемещения (код 6) вызывает панику со структурой `FsError`.

Параметр `whence` принимает следующие значения:
- `0` (`SEEK_SET`) — смещение от начала файла;
- `1` (`SEEK_CUR`) — смещение относительно текущей позиции;
- `2` (`SEEK_END`) — смещение относительно конца файла.

```yukino
load std.fs.files: open_read
load std.io: println

set file = open_read("archive.dat")
set file_size = file.seek(0, 2)
file.close()
println("Полный размер: ${file_size} байт")
```

### 3.6 tell

Возвращает текущую позицию указателя чтения/записи в открытом файле.

```yukino
public fun tell() -> int64
```

Возвращает 64-битное смещение в байтах от начала файла. Функция никогда не возвращает null.

В случае ошибки опроса позиции дескриптора вызывает панику со структурой `FsError` (код 6).

```yukino
load std.fs.files: open_read
load std.io: println

set file = open_read("data.bin")
file.seek(32, 0)
set current_offset = file.tell()
file.close()
println("Текущее смещение: ${current_offset}")
```

### 3.7 flush

Принудительно сбрасывает внутренние системные буферы записи на физический диск.

```yukino
public fun flush() -> bool
```

Возвращает `true`, если операция сброса буферов завершилась успешно. Функция никогда не возвращает null.

В случае сбоя дискового ввода-вывода вызывает панику со структурой `FsError` (код 6).

```yukino
load std.fs.files: open_append
load std.io: println

set file = open_append("audit.log")
file.write_text("checkpoint\n")
set flushed = file.flush()
file.close()
println("Буфер сброшен: ${flushed}")
```

### 3.8 close

Закрывает дескриптор файла и освобождает связанные с ним системные ресурсы операционной системы.

```yukino
public fun close() -> bool
```

Возвращает `true` при успешном закрытии. Повторный вызов `close` для уже закрытого файла также возвращает `true`: закрытие идемпотентно. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.files: create_file
load std.io: println

set file = create_file("test.txt")
file.write_text("data\n")
set is_closed = file.close()
println("Файл закрыт: ${is_closed}")
```

---

## 4. Функции работы с путями (std.fs.paths)

### 4.1 path_separator

Возвращает разделитель компонентов пути, используемый стандартной библиотекой.

```yukino
public fun path_separator() -> string
```

Возвращает строку `"/"`. В текущей реализации стандартная библиотека использует прямой слэш в качестве разделителя на всех платформах, включая Windows. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_separator
load std.io: println

println("Разделитель пути: " + path_separator())
```

### 4.2 path_is_separator

Проверяет, является ли переданный символ допустимым символом разделения компонентов пути.

```yukino
public fun path_is_separator(c: char) -> bool
```

Возвращает `true`, если символ равен `/` или `\`, иначе `false`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_is_separator
load std.io: println

set ok = path_is_separator('/')
println("Является разделителем: ${ok}")
```

### 4.3 path_is_absolute

Проверяет, является ли переданный путь абсолютным.

```yukino
public fun path_is_absolute(path: string) -> bool
```

Возвращает `true`, если путь начинается с корневого слэша `/` или обратного слэша `\` (Unix/UNC), либо состоит из буквы диска с двоеточием и разделителем, например `C:\`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_is_absolute
load std.io: println

set is_abs = path_is_absolute("/etc/hosts")
println("Абсолютный путь: ${is_abs}")
```

### 4.4 path_is_relative

Проверяет, является ли переданный путь относительным.

```yukino
public fun path_is_relative(path: string) -> bool
```

Возвращает `true`, если путь не является абсолютным (см. `path_is_absolute`) и отсчитывается от текущего рабочего каталога. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_is_relative
load std.io: println

set is_rel = path_is_relative("docs/api.md")
println("Относительный путь: ${is_rel}")
```

### 4.5 path_exists

Проверяет, существует ли в файловой системе какой-либо объект (файл, директория, ссылка) по указанному пути.

```yukino
public fun path_exists(path: string) -> bool
```

Возвращает `true`, если объект по указанному пути существует, иначе `false`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_exists
load std.io: println

set exists = path_exists("config.toml")
println("Путь существует: ${exists}")
```

### 4.6 path_is_file

Проверяет, указывает ли путь на обычный файл (не директорию и не символическую ссылку).

```yukino
public fun path_is_file(path: string) -> bool
```

Возвращает `true`, если путь существует и является регулярным файлом, иначе `false`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_is_file
load std.io: println

set regular = path_is_file("build.sh")
println("Обычный файл: ${regular}")
```

### 4.7 path_is_dir

Проверяет, указывает ли путь на директорию.

```yukino
public fun path_is_dir(path: string) -> bool
```

Возвращает `true`, если путь существует и указывает на каталог, иначе `false`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_is_dir
load std.io: println

set is_directory = path_is_dir("src")
println("Каталог: ${is_directory}")
```

### 4.8 path_is_symlink

Проверяет, указывает ли путь на символическую ссылку.

```yukino
public fun path_is_symlink(path: string) -> bool
```

Возвращает `true`, если путь существует и является символической ссылкой, иначе `false`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_is_symlink
load std.io: println

set is_link = path_is_symlink("current")
println("Символическая ссылка: ${is_link}")
```

### 4.9 path_join

Объединяет два сегмента файлового пути, добавляя разделитель `/` при необходимости.

```yukino
public fun path_join(a: string, b: string) -> string
```

Возвращает объединенный путь. Если `b` является абсолютным путем, возвращается сам `b`. Если одна из строк пуста, возвращается вторая строка без изменений. Повторяющийся разделитель на стыке не дублируется. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_join
load std.io: println

set joined = path_join("src", "main.ykn")
println(joined) // "src/main.ykn"
```

### 4.10 path_join3

Объединяет три последовательных сегмента файлового пути в единую строку, последовательно применяя `path_join`.

```yukino
public fun path_join3(a: string, b: string, c: string) -> string
```

Возвращает результирующий объединенный путь как строку `string`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_join3
load std.io: println

set full = path_join3("var", "log", "audit.log")
println(full) // "var/log/audit.log"
```

### 4.11 path_parent

Определяет путь к родительской директории для указанного пути.

```yukino
public fun path_parent(path: string) -> string
```

Возвращает строку с родительским каталогом. Для корневых путей (`/`, `\`) возвращает `"/"`. Для пустого пути или пути из одного сегмента без родителя возвращает пустую строку `""`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_parent
load std.io: println

set parent = path_parent("/home/user/document.txt")
println("Родительский каталог: ${parent}") // "/home/user"
```

### 4.12 path_file_name

Извлекает имя файла или последнего сегмента пути.

```yukino
public fun path_file_name(path: string) -> string
```

Возвращает строку с завершающим сегментом пути. Для корневых путей возвращает `"/"`, для пустого пути — пустую строку `""`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_file_name
load std.io: println

set name = path_file_name("/home/user/report.pdf")
println("Имя файла: ${name}") // "report.pdf"
```

### 4.13 path_file_stem

Извлекает имя файла без завершающего расширения и точки.

```yukino
public fun path_file_stem(path: string) -> string
```

Возвращает основу имени файла (сегмент до последней точки). Если имя файла отсутствует или пусто, возвращает пустую строку `""`. Для скрытых файлов вида `.gitignore` точка в начале считается частью имени, и возвращается имя целиком. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_file_stem
load std.io: println

set stem = path_file_stem("archive.tar.gz")
println("Основа: ${stem}") // "archive.tar"
```

### 4.14 path_extension

Извлекает расширение файла (все символы после последней точки).

```yukino
public fun path_extension(path: string) -> string
```

Возвращает расширение файла без точки. Если расширение отсутствует (нет точки или точка идет в начале скрытого файла), возвращает пустую строку `""`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_extension
load std.io: println

set ext = path_extension("document.docx")
println("Расширение: ${ext}") // "docx"
```

### 4.15 path_has_extension

Проверяет наличие расширения у имени файла в указанном пути.

```yukino
public fun path_has_extension(path: string) -> bool
```

Возвращает `true`, если путь имеет непустое расширение после точки, иначе `false`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_has_extension
load std.io: println

set has_ext = path_has_extension("build.sh")
println("Имеет расширение: ${has_ext}") // true
```

### 4.16 path_with_extension

Заменяет существующее расширение файла на новое либо добавляет его, если расширение отсутствовало.

```yukino
public fun path_with_extension(path: string, new_ext: string) -> string
```

Возвращает путь с обновленным расширением (имя заменяется на `path_file_stem` + `.` + `new_ext`, результат объединяется с `path_parent`). Если параметр `new_ext` передан как пустая строка `""`, текущее расширение удаляется. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.paths: path_with_extension
load std.io: println

set updated = path_with_extension("image.jpeg", "png")
println(updated) // "image.png"
```

### 4.17 path_canonicalize

Преобразует путь в абсолютный канонический, обращаясь к файловой системе (разрешает символические ссылки и сегменты `.`/`..`).

```yukino
public fun path_canonicalize(path: string) -> string
```

Возвращает канонический путь как строку `string`. Функция никогда не возвращает null.

В случае, если путь не существует или системный вызов завершился неудачей, вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 1).

```yukino
load std.fs.paths: path_canonicalize, FsError
load std.io: println

sus {
    set real = path_canonicalize("docs/../config")
    println("Канонический путь: ${real}")
} catch err: FsError {
    println("Ошибка канонизации: ${err.message}")
}
```

---

## 5. Функции работы с директориями (std.fs.dirs)

Для каждой функции с префиксом `dir_` существует идентичная пара без префикса (`dir_create`/`create_dir`, `dir_remove`/`remove_dir`, `dir_list`/`list_dir`, `dir_walk`/`walk_dir`, `dir_current`/`current_dir`, `dir_set_current`/`set_current_dir`, `dir_is_empty`/`is_dir_empty`, `dir_copy_all`/`copy_dir_all` и т.д.); обе формы экспортируются модулем.

### 5.1 dir_create

Создает одну новую директорию по указанному пути.

```yukino
public fun dir_create(path: string) -> bool
```

Возвращает `true` при успешном создании каталога. Родительский каталог обязан существовать. Функция никогда не возвращает null.

В случае сбоя ОС (отсутствие родительского каталога, отказ в доступе или каталог уже существует) вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.dirs: dir_create
load std.fs.paths: FsError
load std.io: println

sus {
    set ok = dir_create("logs")
    println("Каталог создан: ${ok}")
} catch err: FsError {
    println("Ошибка создания: ${err.message}")
}
```

### 5.2 dir_create_all

Рекурсивно создает директорию и все недостающие промежуточные родительские каталоги.

```yukino
public fun dir_create_all(path: string) -> bool
```

Возвращает `true` при успешном создании целевого каталога и всей цепочки родителей. Если каталог уже существовал, возвращает `true` без ошибки. Функция никогда не возвращает null.

В случае ошибки файловой системы или отказа в доступе вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.dirs: dir_create_all
load std.io: println

set ok = dir_create_all("build/output/bin")
println("Дерево каталогов создано: ${ok}")
```

### 5.3 dir_remove

Удаляет пустую директорию по указанному пути.

```yukino
public fun dir_remove(path: string) -> bool
```

Возвращает `true` при успешном удалении каталога. Функция никогда не возвращает null.

В случае если директория не пуста (`code = 5`), не существует (`code = 1`) или недостаточно прав (`code = 2`), вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.dirs: dir_remove
load std.fs.paths: FsError
load std.io: println

sus {
    set ok = dir_remove("temp_dir")
    println("Удалено: ${ok}")
} catch err: FsError {
    println("Ошибка удаления: ${err.message}")
}
```

### 5.4 dir_remove_all

Рекурсивно удаляет каталог со всем его содержимым, включая все вложенные файлы и поддиректории.

```yukino
public fun dir_remove_all(path: string) -> bool
```

Возвращает `true` при успешном удалении дерева каталогов. Если каталог не существует, также возвращает `true` без ошибки. Функция никогда не возвращает null.

В случае отказа в доступе или системной ошибки файловой системы вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.dirs: dir_remove_all
load std.io: println

set ok = dir_remove_all("cache/temp")
println("Дерево полностью очищено: ${ok}")
```

### 5.5 dir_exists

Проверяет, существует ли объект по указанному пути и является ли он директорией.

```yukino
public fun dir_exists(path: string) -> bool
```

Возвращает `true`, если путь существует и указывает на директорию, иначе `false`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.dirs: dir_exists
load std.io: println

set exists = dir_exists("src")
println("Каталог существует: ${exists}")
```

### 5.6 dir_is_empty

Проверяет, не содержит ли директория вложенных файлов и папок.

```yukino
public fun dir_is_empty(path: string) -> bool
```

Возвращает `true`, если директория пуста, и `false`, если в ней найден хотя бы один элемент, а также если каталог не существует или недоступен для чтения. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.dirs: dir_is_empty
load std.io: println

set empty = dir_is_empty("empty_folder")
println("Каталог пуст: ${empty}")
```

### 5.7 dir_list

Возвращает список путей элементов, находящихся непосредственно внутри указанной директории (без рекурсии).

```yukino
public fun dir_list(path: string) -> string[]
```

Возвращает массив строк `string[]` с путями к каждому элементу каталога. Для пустого каталога возвращается массив нулевой длины. Функция никогда не возвращает null.

В случае отсутствия каталога или отказа в чтении содержимого вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 1).

```yukino
load std.fs.dirs: dir_list
load std.io: println

set entries = dir_list("src")
for entry in entries {
    println(entry)
}
```

### 5.8 dir_walk

Выполняет полный рекурсивный обход дерева каталогов, возвращая список путей всех найденных файлов и поддиректорий.

```yukino
public fun dir_walk(path: string) -> string[]
```

Возвращает массив строк `string[]` с путями всех вложенных сущностей на любой глубине вложенности. Функция никогда не возвращает null.

В случае системной ошибки доступа к дереву директорий вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 1).

```yukino
load std.fs.dirs: dir_walk
load std.io: println

set all_files = dir_walk("docs")
println("Всего элементов найдено: ${all_files.len()}")
```

### 5.9 dir_copy_all

Рекурсивно копирует директорию со всем содержимым из `from_path` в `to_path`.

```yukino
public fun dir_copy_all(from_path: string, to_path: string) -> bool
```

Возвращает `true` при успешном копировании дерева каталогов. Функция никогда не возвращает null.

В случае отсутствия исходного каталога или ошибки записи вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.dirs: dir_copy_all
load std.io: println

set ok = dir_copy_all("assets", "backup/assets")
println("Дерево скопировано: ${ok}")
```

### 5.10 dir_current

Определяет абсолютный путь к текущей рабочей директории запущенного процесса.

```yukino
public fun dir_current() -> string
```

Возвращает абсолютный путь к рабочей директории в виде строки `string`. Функция никогда не возвращает null.

В случае ошибки получения рабочего каталога от операционной системы вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.dirs: dir_current
load std.io: println

set cwd = dir_current()
println("Текущая директория: ${cwd}")
```

### 5.11 dir_set_current

Устанавливает новую текущую рабочую директорию для запущенного процесса.

```yukino
public fun dir_set_current(path: string) -> bool
```

Возвращает `true` при успешной смене рабочей директории. Функция никогда не возвращает null.

В случае отсутствия целевого каталога или недостаточных прав вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 1).

```yukino
load std.fs.dirs: dir_set_current
load std.fs.paths: FsError
load std.io: println

sus {
    set ok = dir_set_current("/tmp")
    println("Директория изменена: ${ok}")
} catch err: FsError {
    println("Не удалось сменить директорию: ${err.message}")
}
```

---

## 6. Функции работы с файлами (std.fs.files)

Для большинства функций с префиксом `file_` существует идентичная пара без префикса (`file_read_text`/`read_text`, `file_remove`/`remove_file`, `file_copy`/`copy_file`, `file_metadata`/`metadata`, `file_open_read`/`open_read`, `file_create`/`create_file` и т.д.); обе формы экспортируются модулем.

### 6.1 file_exists

Проверяет, указывает ли путь на обычный файл.

```yukino
public fun file_exists(path: string) -> bool
```

Возвращает `true`, если путь существует и является регулярным файлом (не директорией), иначе `false`. Для проверки существования объекта любого типа используйте `path_exists`, для директорий — `dir_exists`. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.files: file_exists
load std.io: println

set exists = file_exists("config.toml")
println("Файл существует: ${exists}")
```

### 6.2 file_size

Определяет размер файла в байтах.

```yukino
public fun file_size(path: string) -> int64
```

Возвращает размер файла в байтах как 64-битное целое число `int64`. Функция никогда не возвращает null.

В случае если файл не найден или недоступен для чтения, вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 1).

```yukino
load std.fs.files: file_size
load std.io: println

set bytes = file_size("data.bin")
println("Размер: ${bytes} байт")
```

### 6.3 file_copy

Копирует содержимое файла из исходного пути `from_path` в целевой путь `to_path`.

```yukino
public fun file_copy(from_path: string, to_path: string) -> int64
```

Возвращает фактическое число скопированных байт как `int64`. Функция никогда не возвращает null.

В случае отсутствия исходного файла или ошибки записи вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: file_copy
load std.io: println

set copied = file_copy("config.toml", "config.toml.bak")
println("Скопировано байт: ${copied}")
```

### 6.4 file_rename

Переименовывает или перемещает файл или директорию из `from_path` в `to_path`.

```yukino
public fun file_rename(from_path: string, to_path: string) -> bool
```

Возвращает `true` при успешном перемещении или переименовании. Функция никогда не возвращает null.

В случае если исходный объект не существует или целевой путь недоступен, вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: file_rename
load std.io: println

set ok = file_rename("temp.log", "archive.log")
println("Переименовано: ${ok}")
```

### 6.5 file_remove

Удаляет указанный файл из файловой системы.

```yukino
public fun file_remove(path: string) -> bool
```

Возвращает `true` при успешном удалении файла. Функция никогда не возвращает null.

В случае если файл не найден (`code = 1`) или недостаточно прав для удаления (`code = 2`), вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 1).

```yukino
load std.fs.files: file_remove
load std.io: println

set removed = file_remove("cache.tmp")
println("Файл удален: ${removed}")
```

### 6.6 file_touch

Создает пустой файл по указанному пути, если он не существует, либо обновляет время последнего доступа и модификации существующего файла.

```yukino
public fun file_touch(path: string) -> bool
```

Возвращает `true` при успешном выполнении операции. Функция никогда не возвращает null.

В случае системной ошибки вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: file_touch
load std.io: println

set ok = file_touch("marker.tmp")
println("Файл создан или обновлен: ${ok}")
```

### 6.7 file_hard_link

Создает жесткую ссылку `link` на существующий файл `original`.

```yukino
public fun file_hard_link(original: string, link: string) -> bool
```

Возвращает `true` при успешном создании ссылки. Функция никогда не возвращает null.

В случае если исходный файл не существует, ссылка уже существует или файловая система не поддерживает жесткие ссылки, вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: file_hard_link
load std.io: println

set ok = file_hard_link("data.bin", "data_alias.bin")
println("Жесткая ссылка создана: ${ok}")
```

### 6.8 file_symlink

Создает символическую ссылку `link`, указывающую на путь `original`.

```yukino
public fun file_symlink(original: string, link: string) -> bool
```

Возвращает `true` при успешном создании ссылки. Функция никогда не возвращает null.

В случае если ссылка уже существует или платформа отказала в создании символической ссылки, вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: file_symlink
load std.io: println

set ok = file_symlink("releases/1.2", "current")
println("Символическая ссылка создана: ${ok}")
```

### 6.9 file_set_readonly

Устанавливает или снимает флаг «только для чтения» у файла по указанному пути.

```yukino
public fun file_set_readonly(path: string, readonly: bool) -> bool
```

Возвращает `true` при успешном изменении флага. Функция никогда не возвращает null.

В случае если файл не найден или операция отклонена ОС, вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: file_set_readonly
load std.io: println

set ok = file_set_readonly("system.dat", true)
println("Флаг установлен: ${ok}")
```

### 6.10 file_set_permissions

Устанавливает права доступа (mode) к файлу в виде целого числа `uint32`.

```yukino
public fun file_set_permissions(path: string, mode: uint32) -> bool
```

Возвращает `true` при успешном изменении прав. Формат `mode` зависит от платформы (на Unix это стандартные битовые права). Функция никогда не возвращает null.

В случае если файл не найден или операция отклонена ОС, вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: file_set_permissions
load std.io: println

set ok = file_set_permissions("script.sh", 0o755u32)
println("Права установлены: ${ok}")
```

### 6.11 file_read_text

Считывает всё содержимое файла по указанному пути как единую UTF-8 строку за одну операцию.

```yukino
public fun file_read_text(path: string) -> string
```

Возвращает полное текстовое содержимое файла в виде строки `string`. Функция никогда не возвращает null.

В случае отсутствия файла или ошибки чтения вызывает панику со структурой `FsError`:
- `code = 1` (`NotFound`) — файл не найден;
- `code = 2` (`PermissionDenied`) — недостаточно прав для чтения;
- `code = 6` (`IoError`) — системная ошибка ввода-вывода.

```yukino
load std.fs.files: file_read_text
load std.fs.paths: FsError
load std.io: println

sus {
    set text = file_read_text("config.toml")
    println(text)
} catch err: FsError {
    println("Ошибка чтения: ${err.message}")
}
```

### 6.12 file_write_text

Создает новый файл или перезаписывает существующий переданной строкой.

```yukino
public fun file_write_text(path: string, content: string) -> bool
```

Возвращает `true` при успешной записи текста в файл. Функция никогда не возвращает null.

В случае ошибки создания файла или записи на диск вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: file_write_text
load std.io: println

set ok = file_write_text("greeting.txt", "Hello from Yukino!\n")
println("Записано: ${ok}")
```

### 6.13 file_append_text

Добавляет строку текста в конец файла, автоматически создавая файл, если он еще не существует.

```yukino
public fun file_append_text(path: string, content: string) -> bool
```

Возвращает `true` при успешном добавлении содержимого в файл. Функция никогда не возвращает null.

В случае ошибки открытия или записи файла вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: file_append_text
load std.io: println

set ok = file_append_text("events.log", "event: user_login\n")
println("Добавлено: ${ok}")
```

### 6.14 file_read_lines

Считывает текстовый файл и разделяет его содержимое на массив строк по символам перевода строки `\n` или `\r\n`.

```yukino
public fun file_read_lines(path: string) -> string[]
```

Возвращает массив строк `string[]`. Разделители строк автоматически отсекаются. Функция никогда не возвращает null.

В случае ошибки чтения или отсутствия файла вызывает панику со структурой `FsError` (коды как у `file_read_text`).

```yukino
load std.fs.files: file_read_lines
load std.io: println

set lines = file_read_lines("todo.txt")
for line in lines {
    println("- ${line}")
}
```

### 6.15 file_read_bytes

Считывает бинарные данные файла в заранее выделенный буфер `buf` размером не менее `max_len` байт.

```yukino
public fun file_read_bytes(path: string, buf: ptr<uint8>, max_len: int64) -> int64
```

Возвращает количество фактически прочитанных байт как `int64`. Если `buf` равен `null` или `max_len <= 0`, возвращает 0 без обращения к ОС. Функция никогда не возвращает null.

В случае отсутствия файла или ошибки дискового ввода-вывода вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: file_read_bytes
load std.io: println

set buf: uint8[] = []
for i in 0..1024 {
    buf.push(0)
}
set n = file_read_bytes("image.png", buf.as_ptr(), 1024)
println("Считано байт: ${n}")
```

### 6.16 file_metadata

Запрашивает системные метаданные файла или каталога по указанному пути.

```yukino
public fun file_metadata(path: string) -> FileMetadata
```

Возвращает структуру `FileMetadata` (см. раздел 2.2) с размером, атрибутами, правами и временными метками объекта. Функция никогда не возвращает null.

В случае если объект не найден или доступ запрещен, вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 1).

```yukino
load std.fs.files: file_metadata
load std.io: println

set meta = file_metadata("main.ykn")
println("Размер: ${meta.size} байт, каталог: ${meta.is_dir}")
```

### 6.17 open_read

Открывает существующий файл только для чтения и возвращает потоковый объект `File`.

```yukino
public fun open_read(path: string) -> File
```

Возвращает экземпляр `File` с открытым дескриптором. Функция никогда не возвращает null.

В случае если файл не найден или недоступен, вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 1).

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
println("Прочитано байт: ${n}")
```

### 6.18 open_write

Открывает файл для записи: создает новый файл или усекает существующий до нулевой длины.

```yukino
public fun open_write(path: string) -> File
```

Возвращает экземпляр `File` с открытым дескриптором. Функция никогда не возвращает null.

В случае ошибки открытия или создания файла вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: open_write
load std.io: println

set file = open_write("output.bin")
set written = file.write_text("Yukino")
file.close()
println("Записано байт: ${written}")
```

### 6.19 open_append

Открывает файл для добавления данных в конец, создавая его при отсутствии.

```yukino
public fun open_append(path: string) -> File
```

Возвращает экземпляр `File` с открытым дескриптором. Функция никогда не возвращает null.

В случае ошибки открытия или создания файла вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 6).

```yukino
load std.fs.files: open_append
load std.fs.paths: FsError
load std.io: println

sus {
    set file = open_append("app.log")
    file.write_text("Application started\n")
    file.close()
} catch err: FsError {
    println("Не удалось открыть файл: ${err.message}")
}
```

### 6.20 create_file

Создает новый файл в эксклюзивном режиме: если файл уже существует, операция завершается ошибкой `AlreadyExists` (`code = 3`).

```yukino
public fun create_file(path: string) -> File
```

Возвращает экземпляр `File` с открытым дескриптором, доступным для записи. Функция никогда не возвращает null.

В случае если файл уже существует или создание отклонено ОС, вызывает панику со структурой `FsError` (код ошибки берется у ОС, резервный код 3).

```yukino
load std.fs.files: create_file
load std.fs.paths: FsError
load std.io: println

sus {
    set file = create_file("session.tmp")
    file.write_text("created\n")
    file.close()
} catch err: FsError {
    println("Не удалось создать файл: ${err.message}")
}
```

Помимо перечисленных, модуль экспортирует пары-синонимы: `open` (эквивалент `open_read`), `create` (эквивалент `create_file`), `file_open_read`, `file_open_write`, `file_open_append` и `file_create` — они ведут себя идентично соответствующим функциям выше.

---

## 7. Стандартные системные пути (std.fs.defaults)

Все функции модуля `defaults` являются кроссплатформенными. На Linux они следуют спецификации XDG Base Directory, на Windows используют переменные окружения (`APPDATA`, `LOCALAPPDATA`, `USERPROFILE` и т.п.), а на macOS — стандартные системные каталоги. Все функции возвращают `string`, никогда не возвращают null, и, за исключением `exe_path`, никогда не паникуют: если путь не удалось определить, возвращается пустая строка `""` либо запасной путь (временный каталог). Для каждой функции вида `dir_x` существует пара `x_dir` (`dir_home`/`home_dir` и т.д.).

### 7.1 dir_home

Возвращает путь к домашней директории текущего пользователя системы.

```yukino
public fun dir_home() -> string
```

Возвращает путь к домашнему каталогу (например, `"/home/username"` на Linux или `"C:\Users\username"` на Windows; на Unix берется из `HOME`, на Windows — из `USERPROFILE` или `HOMEDRIVE` + `HOMEPATH`). Функция никогда не возвращает null.

Функция никогда не паникует: если домашний каталог определить не удалось, возвращает пустую строку `""`.

```yukino
load std.fs.defaults: dir_home
load std.io: println

set home = dir_home()
println("Домашний каталог: ${home}")
```

### 7.2 dir_config

Возвращает путь к каталогу хранения конфигурационных файлов пользователя.

```yukino
public fun dir_config() -> string
```

Возвращает путь к каталогу конфигураций (`"~/.config"` на Linux с учетом `XDG_CONFIG_HOME`, `%APPDATA%` на Windows). Функция никогда не возвращает null.

Функция никогда не паникует: при невозможности определить каталог возвращается запасной путь (временный каталог).

```yukino
load std.fs.defaults: dir_config
load std.io: println

set cfg = dir_config()
println("Каталог конфигураций: ${cfg}")
```

### 7.3 dir_data

Возвращает путь к каталогу постоянных данных пользователя для приложений.

```yukino
public fun dir_data() -> string
```

Возвращает путь к каталогу пользовательских данных (`"~/.local/share"` на Linux с учетом `XDG_DATA_HOME`, `%LOCALAPPDATA%` на Windows). Функция никогда не возвращает null.

Функция никогда не паникует: при невозможности определить каталог возвращается запасной путь (временный каталог).

```yukino
load std.fs.defaults: dir_data
load std.io: println

set data_dir = dir_data()
println("Каталог данных: ${data_dir}")
```

### 7.4 dir_data_local

Возвращает путь к локальному каталогу данных пользователя. В текущей реализации совпадает с `dir_data`.

```yukino
public fun dir_data_local() -> string
```

Возвращает путь к каталогу локальных пользовательских данных (`"~/.local/share"` на Linux, `%LOCALAPPDATA%` на Windows). Функция никогда не возвращает null.

Функция никогда не паникует: при невозможности определить каталог возвращается запасной путь (временный каталог).

```yukino
load std.fs.defaults: dir_data_local
load std.io: println

set local_data = dir_data_local()
println("Локальный каталог данных: ${local_data}")
```

### 7.5 dir_cache

Возвращает путь к каталогу временного кэша пользователя.

```yukino
public fun dir_cache() -> string
```

Возвращает путь к каталогу кэша (`"~/.cache"` на Linux с учетом `XDG_CACHE_HOME`, `%LOCALAPPDATA%\cache` на Windows). Функция никогда не возвращает null.

Функция никогда не паникует: при невозможности определить каталог возвращается запасной путь (временный каталог).

```yukino
load std.fs.defaults: dir_cache
load std.io: println

set cache_dir = dir_cache()
println("Каталог кэша: ${cache_dir}")
```

### 7.6 dir_temp

Возвращает путь к стандартному каталогу временных файлов системы.

```yukino
public fun dir_temp() -> string
```

Возвращает строку пути к системной временной папке (`"/tmp"` на Linux, `%TEMP%` на Windows). Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.defaults: dir_temp
load std.io: println

set tmp = dir_temp()
println("Временный каталог системы: ${tmp}")
```

### 7.7 dir_state

Возвращает путь к каталогу сохранения состояния приложений пользователя (история, логи, сессии).

```yukino
public fun dir_state() -> string
```

Возвращает путь к каталогу состояния (`"~/.local/state"` на Linux с учетом `XDG_STATE_HOME`, `%LOCALAPPDATA%` на Windows). Функция никогда не возвращает null.

Функция никогда не паникует: при невозможности определить каталог возвращается запасной путь (временный каталог).

```yukino
load std.fs.defaults: dir_state
load std.io: println

set state_dir = dir_state()
println("Каталог состояния: ${state_dir}")
```

### 7.8 dir_runtime

Возвращает путь к каталогу среды исполнения (runtime) пользователя.

```yukino
public fun dir_runtime() -> string
```

Возвращает путь из `XDG_RUNTIME_DIR` на Unix-системах; если переменная не задана — временный каталог ОС. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.defaults: dir_runtime
load std.io: println

set rt = dir_runtime()
println("Runtime-каталог: ${rt}")
```

### 7.9 dir_desktop

Возвращает путь к подкаталогу `Desktop` домашнего каталога текущего пользователя.

```yukino
public fun dir_desktop() -> string
```

Возвращает путь к папке Рабочего стола (`"~/Desktop"` на Linux/macOS). Функция никогда не возвращает null.

Функция никогда не паникует: если домашний каталог не определен, возвращает пустую строку `""`.

```yukino
load std.fs.defaults: dir_desktop
load std.io: println

set dt = dir_desktop()
println("Рабочий стол: ${dt}")
```

### 7.10 dir_documents

Возвращает путь к подкаталогу `Documents` домашнего каталога пользователя.

```yukino
public fun dir_documents() -> string
```

Возвращает путь к каталогу `"~/Documents"`. Функция никогда не возвращает null.

Функция никогда не паникует: если домашний каталог не определен, возвращает пустую строку `""`.

```yukino
load std.fs.defaults: dir_documents
load std.io: println

set docs = dir_documents()
println("Документы: ${docs}")
```

### 7.11 dir_downloads

Возвращает путь к подкаталогу `Downloads` домашнего каталога пользователя.

```yukino
public fun dir_downloads() -> string
```

Возвращает путь к каталогу `"~/Downloads"`. Функция никогда не возвращает null.

Функция никогда не паникует: если домашний каталог не определен, возвращает пустую строку `""`.

```yukino
load std.fs.defaults: dir_downloads
load std.io: println

set dls = dir_downloads()
println("Загрузки: ${dls}")
```

### 7.12 dir_music

Возвращает путь к подкаталогу `Music` домашнего каталога пользователя.

```yukino
public fun dir_music() -> string
```

Возвращает путь к каталогу `"~/Music"`. Функция никогда не возвращает null.

Функция никогда не паникует: если домашний каталог не определен, возвращает пустую строку `""`.

```yukino
load std.fs.defaults: dir_music
load std.io: println

set music = dir_music()
println("Музыка: ${music}")
```

### 7.13 dir_pictures

Возвращает путь к подкаталогу `Pictures` домашнего каталога пользователя.

```yukino
public fun dir_pictures() -> string
```

Возвращает путь к каталогу `"~/Pictures"`. Функция никогда не возвращает null.

Функция никогда не паникует: если домашний каталог не определен, возвращает пустую строку `""`.

```yukino
load std.fs.defaults: dir_pictures
load std.io: println

set pics = dir_pictures()
println("Изображения: ${pics}")
```

### 7.14 dir_videos

Возвращает путь к подкаталогу `Videos` домашнего каталога пользователя.

```yukino
public fun dir_videos() -> string
```

Возвращает путь к каталогу `"~/Videos"`. Функция никогда не возвращает null.

Функция никогда не паникует: если домашний каталог не определен, возвращает пустую строку `""`.

```yukino
load std.fs.defaults: dir_videos
load std.io: println

set vids = dir_videos()
println("Видео: ${vids}")
```

### 7.15 dir_app_config

Возвращает каталог конфигураций конкретного приложения: `path_join(dir_config(), app_name)`.

```yukino
public fun dir_app_config(app_name: string) -> string
```

Возвращает объединенный путь. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.defaults: dir_app_config
load std.io: println

set cfg = dir_app_config("myapp")
println("Конфигурация приложения: ${cfg}")
```

### 7.16 dir_app_data

Возвращает каталог данных конкретного приложения: `path_join(dir_data(), app_name)`.

```yukino
public fun dir_app_data(app_name: string) -> string
```

Возвращает объединенный путь. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.defaults: dir_app_data
load std.io: println

set data = dir_app_data("myapp")
println("Данные приложения: ${data}")
```

### 7.17 dir_app_cache

Возвращает каталог кэша конкретного приложения: `path_join(dir_cache(), app_name)`.

```yukino
public fun dir_app_cache(app_name: string) -> string
```

Возвращает объединенный путь. Функция никогда не возвращает null.

Функция никогда не паникует.

```yukino
load std.fs.defaults: dir_app_cache
load std.io: println

set cache = dir_app_cache("myapp")
println("Кэш приложения: ${cache}")
```

### 7.18 exe_path

Возвращает полный абсолютный путь к запущенному исполняемому бинарному файлу текущего процесса.

```yukino
public fun exe_path() -> string
```

Возвращает абсолютный путь к бинарнику процесса (например, `"/usr/bin/my_app"`). Функция никогда не возвращает null.

В случае ошибки ОС при определении пути к бинарнику процесса вызывает панику со структурой `FsError` (код 6).

```yukino
load std.fs.defaults: exe_path
load std.io: println

set path = exe_path()
println("Исполняемый файл: ${path}")
```

### 7.19 exe_dir

Возвращает путь к директории, в которой размещен исполняемый файл текущего процесса: `path_parent(exe_path())`.

```yukino
public fun exe_dir() -> string
```

Возвращает путь к папке размещения бинарного файла процесса. Функция никогда не возвращает null.

Паникует только в случае сбоя внутренней функции `exe_path` — со структурой `FsError` (код 6).

```yukino
load std.fs.defaults: exe_dir
load std.io: println

set bin_dir = exe_dir()
println("Директория бинарника: ${bin_dir}")
```

---

[← Назад: Модуль std.random](7-random.md) | [Вперед: Модуль std.collections →](9-collections.md)
