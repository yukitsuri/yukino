# Модуль std.simpleurl

Модуль `std.simpleurl` предоставляет высокоуровневый инструментарий для парсинга, валидации, сборки, модификации и кодирования Uniform Resource Locators (URL) в соответствии со стандартом RFC 3986. Модуль включает парсер URL, компонентное экранирование (percent-encoding), работу с параметрами поискового запроса (query parameters), относительное соединение путей (join), а также гибкий построитель `UrlBuilder`.

---

## 1. Подключение: load std.simpleurl

Импорт всего пакета:

```yukino
load std.simpleurl
// либо через псевдоним:
load std.simpleurl
```

При импорте модуля становятся доступны следующие сущности:

- **Структуры данных и ошибок:** `QueryParam`, `UrlParseError`.
- **Классы:** `Url`, `UrlBuilder`.
- **Функции:** `url_parse`, `url_builder`, `parse_query`, `url_encode`, `url_decode`, `url_encode_component`, `url_decode_component`, `default_port_for_scheme`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.simpleurl` глобальные константы не объявлены.

---

## 3. Интерфейсы

В модуле `std.simpleurl` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 QueryParam

Представляет отдельный параметр поисковой строки запроса в виде пары «ключ-значение».

```yukino
public struct QueryParam {
    key: string
    value: string
}
```

### 4.2 UrlParseError

Структура ошибки, инициируемой при передаче синтаксически некорректной или пустой строки URL в парсер.

```yukino
public struct UrlParseError {
    message: string
    input: string
}
```

---

## 5. Классы

### 5.1 Url

Класс, представляющий разобранный URL со структурированным доступом ко всем его компонентам: схеме, учетным данным, хосту, порту, пути, параметрам запроса и фрагменту.

```yukino
public class Url {
    public scheme: string
    public username: string?
    public password: string?
    public host: string
    public port: int32?
    public path: string
    public query_params: QueryParam[]
    public fragment: string?
}
```

#### Методы класса Url

#### effective_port

Возвращает явный порт URL, а если порт опущен — стандартный номер порта для текущей схемы (`80` для `http`/`ws`, `443` для `https`/`wss`, `21` для `ftp`, `22` для `ssh`) либо `0`.

```yukino
public fun effective_port() -> int32
```

Возвращает 32-битное целое число типа `int32`.

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/api")
println("Порт соединения: ${u.effective_port()}")
```

#### origin

Возвращает строку источника (origin), состоящую из схемы, разделителя `://`, имени хоста и нестандартного порта.

```yukino
public fun origin() -> string
```

Возвращает строковое значение источника типа `string`.

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("http://example.com:8080/data?filter=1")
println("Источник: " + u.origin())
```

#### query

Формирует закодированную строку поискового запроса (без ведущего символа `?`) на основе списка параметров `query_params`. Если список параметров пуст, возвращает `null`.

```yukino
public fun query() -> string?
```

Возвращает строку запроса типа `string` либо `null`.

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/search?q=yukino&page=1")
set q = u.query()
if q != null {
    println("Запрос: " + (q ?? ""))
}
```

#### get_query_param

Возвращает значение первого параметра запроса с указанным ключом либо `null`, если ключ отсутствует.

```yukino
public fun get_query_param(key: string) -> string?
```

Возвращает строковое значение типа `string` либо `null`.

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://api.site.org/?token=xyz123")
set tok = u.get_query_param("token")
println("Токен: " + (tok ?? "отсутствует"))
```

#### get_all_query_params

Возвращает массив всех значений, соответствующих указанному ключу (для запросов с повторяющимися ключами, например `?tag=a&tag=b`).

```yukino
public fun get_all_query_params(key: string) -> string[]
```

Возвращает массив строк `string[]`.

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://site.org/?item=1&item=2")
set items = u.get_all_query_params("item")
for i in 0..items.len() {
    println("Элемент: " + items[i])
}
```

#### set_query_param

Устанавливает значение для указанного ключа запроса, удаляя все предыдущие вхождения этого ключа и добавляя новую пару.

```yukino
public fun set_query_param(key: string, value: string)
```

Метод ничего не возвращает (`void`).

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/?page=1")
u.set_query_param("page", "2")
println(u.to_string())
```

#### add_query_param

Добавляет новую пару «ключ-значение» в конец списка параметров запроса без удаления существующих параметров.

```yukino
public fun add_query_param(key: string, value: string)
```

Метод ничего не возвращает (`void`).

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/?filter=active")
u.add_query_param("sort", "desc")
println(u.to_string())
```

#### remove_query_param

Удаляет все параметры запроса, имя ключа которых совпадает с переданным аргументом.

```yukino
public fun remove_query_param(key: string)
```

Метод ничего не возвращает (`void`).

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/?temp=1&stay=2")
u.remove_query_param("temp")
println(u.to_string())
```

#### clear_query

Полностью очищает все параметры поискового запроса URL.

```yukino
public fun clear_query()
```

Метод ничего не возвращает (`void`).

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/test?a=1&b=2")
u.clear_query()
println(u.to_string())
```

#### filter_query

Фильтрует параметры запроса по переданной функции-предикату. Параметры, для которых предикат вернул `false`, удаляются.

```yukino
public fun filter_query(predicate: fun(string, string) -> bool)
```

Метод ничего не возвращает (`void`).

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

fun is_public_param(k: string, v: string) -> bool {
    return k != "secret"
}

set u = url_parse("https://example.com/?secret=123&public=ok")
u.filter_query(is_public_param)
println(u.to_string())
```

#### to_string

Формирует каноническую текстовую строку URL со всеми компонентами, экранированием и параметрами.

```yukino
public fun to_string() -> string
```

Возвращает строковое представление URL типа `string`.

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("http://example.org/index.html")
println("URL: " + u.to_string())
```

#### join

Выполняет объединение текущего базового URL с относительным или абсолютным подпутем с нормализацией сегментов `.` и `..`. Если переданный аргумент уже содержит абсолютную схему (например, `https://...`), возвращает результат парсинга нового URL.

```yukino
public fun join(subpath: string) -> Url
```

Возвращает новый экземпляр класса `Url`.

Метод никогда не вызывает панику.

```yukino
load std.simpleurl: url_parse
load std.io: println

set base = url_parse("https://example.org/v1/api/")
set child = base.join("users")
println(child.to_string())
```

---

### 5.2 UrlBuilder

Плавный построитель (Fluent Builder) для пошагового создания объектов `Url` без необходимости ручного конструирования строк.

```yukino
public class UrlBuilder {
    // Внутренние поля конфигурации
}
```

#### Методы класса UrlBuilder

- `scheme(s: string) -> UrlBuilder` — устанавливает схему протокола (по умолчанию `"http"`).
- `username(u: string) -> UrlBuilder` — устанавливает имя пользователя.
- `password(p: string) -> UrlBuilder` — устанавливает пароль.
- `host(h: string) -> UrlBuilder` — устанавливает имя хоста или IP-адрес.
- `port(p: int32) -> UrlBuilder` — устанавливает порт.
- `path(p: string) -> UrlBuilder` — устанавливает путь ресурса.
- `set_query_param(k: string, v: string) -> UrlBuilder` — устанавливает параметр запроса.
- `add_query_param(k: string, v: string) -> UrlBuilder` — добавляет параметр запроса.
- `remove_query_param(k: string) -> UrlBuilder` — удаляет параметр запроса.
- `clear_query() -> UrlBuilder` — очищает параметры запроса.
- `fragment(f: string) -> UrlBuilder` — задает идентификатор фрагмента.
- `build() -> Url` — создает готовый экземпляр класса `Url`.

```yukino
load std.simpleurl: url_builder
load std.io: println

set u = url_builder()
    .scheme("https")
    .host("api.github.com")
    .path("/repos")
    .set_query_param("per_page", "10")
    .build()

println(u.to_string())
```

---

## 6. Справочник функций

### 6.1 url_parse

Выполняет синтаксический анализ строки URL по спецификации RFC 3986 и возвращает структурированный объект `Url`. Поддерживает валидацию схемы, разбор учетных данных пользователя, извлечение хоста (включая IPv6 в квадратных скобках), порта, пути, параметров запроса и фрагмента.

```yukino
public fun url_parse(input: string) -> Url
```

Возвращает новый экземпляр класса `Url`.

В случае пустой входной строки, отсутствия двоеточия после схемы, недопустимых символов в названии схемы или некорректного значения порта вызывает панику со структурой `UrlParseError`.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://user:pass@example.com:8443/docs?topic=all#intro")
println("Хост: " + u.host)
println("Схема: " + u.scheme)
```

### 6.2 url_builder

Создает новый инициализированный экземпляр построителя `UrlBuilder` с настройками по умолчанию.

```yukino
public fun url_builder() -> UrlBuilder
```

Возвращает новый экземпляр класса `UrlBuilder`.

Функция никогда не вызывает панику.

```yukino
load std.simpleurl: url_builder
load std.io: println

set builder = url_builder()
builder.scheme("https")
builder.host("yukino-lang.org")
set u = builder.build()
println(u.to_string())
```

### 6.3 url_encode

Кодирует специальную строку URL, оставляя неэкранированными алфавитно-цифровые символы, дефис, подчеркивание, точку, тильду, а также допустимые разделители URL (`/`, `?`, `&`, `=`, `:`, `@`, `#`), преобразуя остальные байты в формат `%XX`.

```yukino
public fun url_encode(text: string) -> string
```

Возвращает строку типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.simpleurl: url_encode
load std.io: println

set enc = url_encode("https://example.com/path with spaces")
println(enc)
```

### 6.4 url_decode

Декодирует процентно-кодированную строку URL обратно в исходный текст, преобразуя последовательности `%XX` в соответствующие символы. Символы `+` остаются без изменений (замена `+` на пробел выполняется только в `url_decode_component`).

```yukino
public fun url_decode(encoded: string) -> string
```

Возвращает строку типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.simpleurl: url_decode
load std.io: println

set text = url_decode("hello+world%20again")
println(text)
```

### 6.5 url_encode_component

Строго кодирует отдельный компонент URL (например, ключ или значение поискового параметра). Оставляет неэкранированными только буквы ASCII (`a-z`, `A-Z`), цифры (`0-9`) и символы `-`, `_`, `.`, `~`. Все остальные символы, включая слэши, знаки равенства и амперсанды, кодируются в последовательности `%XX`.

```yukino
public fun url_encode_component(text: string) -> string
```

Возвращает строку типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.simpleurl: url_encode_component
load std.io: println

set comp = url_encode_component("query & data=1/2")
println(comp)
```

### 6.6 url_decode_component

Декодирует компонент URL, восстанавливая символы из последовательностей `%XX` и заменяя символы `+` на пробелы.

```yukino
public fun url_decode_component(encoded: string) -> string
```

Возвращает строку типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.simpleurl: url_decode_component
load std.io: println

set plain = url_decode_component("query%20%26%20data")
println(plain)
```

### 6.7 default_port_for_scheme

Возвращает стандартный номер сетевого порта для переданного названия схемы протокола (без учета регистра): `80` для `http`/`ws`, `443` для `https`/`wss`, `21` для `ftp`, `22` для `ssh`. Для неизвестных схем возвращает `0`.

```yukino
public fun default_port_for_scheme(scheme: string) -> int32
```

Возвращает 32-битное число типа `int32`.

Функция никогда не вызывает панику.

```yukino
load std.simpleurl: default_port_for_scheme
load std.io: println

set port = default_port_for_scheme("https")
println("Порт https: ${port}")
```

### 6.8 parse_query

Разбирает строку поискового запроса (query string) формата `key1=val1&key2=val2` в массив структур `QueryParam[]` с автоматическим процентным декодированием ключей и значений. Начальный символ `?` игнорируется, если присутствует.

```yukino
public fun parse_query(q_str: string) -> QueryParam[]
```

Возвращает массив структур типа `QueryParam[]`.

Функция никогда не вызывает панику.

```yukino
load std.simpleurl: parse_query
load std.io: println

set params = parse_query("name=yukino&tag=lang&page=1")
for i in 0..params.len() {
    println(params[i].key + " = " + params[i].value)
}
```

---

[← Назад: Модуль std.env](13-env.md) | [Вперед: Модуль std.compress →](15-compress.md)
