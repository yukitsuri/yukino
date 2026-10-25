# Module std.simpleurl

The `std.simpleurl` module (also available via the alias `std.url`) provides a high-level suite for parsing, validating, building, modifying, and encoding Uniform Resource Locators (URLs) in compliance with RFC 3986. The module includes a full URL parser, percent-encoding utilities, query parameter inspection and mutation, path normalization and joining, and a fluent `UrlBuilder`.

---

## 1. Import: load std.simpleurl

Importing the entire package:

```yukino
load std.simpleurl
// or via alias:
load std.url
```

When imported, the following entities become available:

- **Data and Error Structures:** `QueryParam`, `UrlParseError`.
- **Classes:** `Url`, `UrlBuilder`.
- **Functions:** `url_parse`, `url_builder`, `parse_query`, `url_encode`, `url_decode`, `url_encode_component`, `url_decode_component`, `default_port_for_scheme`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

No global constants are declared in module `std.simpleurl`.

---

## 3. Interfaces

No interfaces are declared in module `std.simpleurl`.

---

## 4. Data and Error Structures (Panic)

### 4.1 QueryParam

Represents an individual query string parameter as a key-value pair.

```yukino
public struct QueryParam {
    key: string
    value: string
}
```

### 4.2 UrlParseError

Error structure raised when an invalid or empty URL string is passed to the parser.

```yukino
public struct UrlParseError {
    message: string
    input: string
}
```

---

## 5. Classes

### 5.1 Url

Represents a parsed URL providing structured access to its components: scheme, userinfo, host, port, path, query parameters, and fragment.

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

#### Url Class Methods

#### effective_port

Returns the explicit port number of the URL if specified, or the default port for the scheme (`80` for `http`/`ws`, `443` for `https`/`wss`, `21` for `ftp`, `22` for `ssh`), or `0` for unknown schemes.

```yukino
public fun effective_port() -> int32
```

Returns a 32-bit integer of type `int32`.

This method never panics.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/api")
println("Port: ${u.effective_port()}")
```

#### origin

Returns the origin string composed of the scheme, `://`, host, and optional non-standard port.

```yukino
public fun origin() -> string
```

Returns a string of type `string`.

This method never panics.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("http://example.com:8080/data?filter=1")
println("Origin: " + u.origin())
```

#### query

Builds the percent-encoded query string (without the leading `?`) from `query_params`. Returns `null` if the parameter list is empty.

```yukino
public fun query() -> string?
```

Returns a query string of type `string`, or `null`.

This method never panics.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/search?q=yukino&page=1")
set q = u.query()
if q != null {
    println("Query: " + (q ?? ""))
}
```

#### get_query_param

Returns the value of the first query parameter matching the specified key, or `null` if absent.

```yukino
public fun get_query_param(key: string) -> string?
```

Returns an optional string value of type `string?`.

This method never panics.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://api.site.org/?token=xyz123")
set tok = u.get_query_param("token")
println("Token: " + (tok ?? "none"))
```

#### get_all_query_params

Returns an array of all values corresponding to the specified key.

```yukino
public fun get_all_query_params(key: string) -> string[]
```

Returns an array of strings `string[]`.

This method never panics.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://site.org/?item=1&item=2")
set items = u.get_all_query_params("item")
for i in 0..items.len() {
    println("Item: " + items[i])
}
```

#### set_query_param

Sets the value for the specified query key, replacing any existing entries for that key.

```yukino
public fun set_query_param(key: string, value: string)
```

Returns `void`.

This method never panics.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/?page=1")
u.set_query_param("page", "2")
println(u.to_string())
```

#### add_query_param

Appends a new query parameter to the end of the query list without removing existing keys.

```yukino
public fun add_query_param(key: string, value: string)
```

Returns `void`.

This method never panics.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/?filter=active")
u.add_query_param("sort", "desc")
println(u.to_string())
```

#### remove_query_param

Removes all query parameters matching the specified key.

```yukino
public fun remove_query_param(key: string)
```

Returns `void`.

This method never panics.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/?temp=1&stay=2")
u.remove_query_param("temp")
println(u.to_string())
```

#### clear_query

Removes all query parameters from the URL.

```yukino
public fun clear_query()
```

Returns `void`.

This method never panics.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://example.com/test?a=1&b=2")
u.clear_query()
println(u.to_string())
```

#### filter_query

Filters query parameters using the provided predicate function. Parameters for which the predicate returns `false` are removed.

```yukino
public fun filter_query(predicate: fun(string, string) -> bool)
```

Returns `void`.

This method never panics.

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

Serializes the URL object into its canonical string representation according to RFC 3986.

```yukino
public fun to_string() -> string
```

Returns a string of type `string`.

This method never panics.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("http://example.org/index.html")
println("URL: " + u.to_string())
```

#### join

Resolves a relative or absolute subpath against the current URL, normalizing `.` and `..` segments. If `subpath` contains an absolute scheme, returns the parsed new URL.

```yukino
public fun join(subpath: string) -> Url
```

Returns a new instance of `Url`.

This method never panics.

```yukino
load std.simpleurl: url_parse
load std.io: println

set base = url_parse("https://example.org/v1/api/")
set child = base.join("users")
println(child.to_string())
```

---

### 5.2 UrlBuilder

Fluent builder class for programmatically constructing `Url` objects.

```yukino
public class UrlBuilder {
    // Internal state
}
```

#### UrlBuilder Methods

- `scheme(s: string) -> UrlBuilder` — sets protocol scheme (default `"http"`).
- `username(u: string) -> UrlBuilder` — sets username.
- `password(p: string) -> UrlBuilder` — sets password.
- `host(h: string) -> UrlBuilder` — sets host or IP address.
- `port(p: int32) -> UrlBuilder` — sets port.
- `path(p: string) -> UrlBuilder` — sets resource path.
- `set_query_param(k: string, v: string) -> UrlBuilder` — sets a query parameter.
- `add_query_param(k: string, v: string) -> UrlBuilder` — appends a query parameter.
- `remove_query_param(k: string) -> UrlBuilder` — removes query parameters by key.
- `clear_query() -> UrlBuilder` — clears query parameters.
- `fragment(f: string) -> UrlBuilder` — sets fragment identifier.
- `build() -> Url` — creates a configured `Url` instance.

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

## 6. Function Reference

### 6.1 url_parse

Parses an RFC 3986 URL string into a structured `Url` object, validating the scheme and extracting userinfo, host (including IPv6 in brackets), port, path, query parameters, and fragment.

```yukino
public fun url_parse(input: string) -> Url
```

Returns a `Url` instance.

Panics with `UrlParseError` if the input is empty, lacks a scheme delimiter, contains illegal scheme characters, or has an invalid port number.

```yukino
load std.simpleurl: url_parse
load std.io: println

set u = url_parse("https://user:pass@example.com:8443/docs?topic=all#intro")
println("Host: " + u.host)
println("Scheme: " + u.scheme)
```

### 6.2 url_builder

Creates an initialized `UrlBuilder` instance with default settings.

```yukino
public fun url_builder() -> UrlBuilder
```

Returns a `UrlBuilder` instance.

This function never panics.

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

Encodes a URL string, preserving unreserved characters (`a-z`, `A-Z`, `0-9`, `-`, `_`, `.`, `~`) and standard URL delimiters (`/`, `?`, `&`, `=`, `:`, `@`, `#`), while converting other bytes to `%XX`.

```yukino
public fun url_encode(text: string) -> string
```

Returns a string of type `string`.

This function never panics.

```yukino
load std.simpleurl: url_encode
load std.io: println

set enc = url_encode("https://example.com/path with spaces")
println(enc)
```

### 6.4 url_decode

Decodes percent-encoded sequences in a URL string, converting `%XX` to characters. The `+` character is left unchanged (replacing `+` with a space is only performed by `url_decode_component`).

```yukino
public fun url_decode(encoded: string) -> string
```

Returns a string of type `string`.

This function never panics.

```yukino
load std.simpleurl: url_decode
load std.io: println

set text = url_decode("hello+world%20again")
println(text)
```

### 6.5 url_encode_component

Strictly percent-encodes an individual URL component (such as a query key or value), encoding all characters except unreserved ASCII (`a-z`, `A-Z`, `0-9`, `-`, `_`, `.`, `~`).

```yukino
public fun url_encode_component(text: string) -> string
```

Returns a string of type `string`.

This function never panics.

```yukino
load std.simpleurl: url_encode_component
load std.io: println

set comp = url_encode_component("query & data=1/2")
println(comp)
```

### 6.6 url_decode_component

Decodes a percent-encoded component, restoring characters from `%XX` and replacing `+` with spaces.

```yukino
public fun url_decode_component(encoded: string) -> string
```

Returns a string of type `string`.

This function never panics.

```yukino
load std.simpleurl: url_decode_component
load std.io: println

set plain = url_decode_component("query%20%26%20data")
println(plain)
```

### 6.7 default_port_for_scheme

Returns the default port number for a given protocol scheme (case-insensitive): `80` for `http`/`ws`, `443` for `https`/`wss`, `21` for `ftp`, `22` for `ssh`. Returns `0` for unknown schemes.

```yukino
public fun default_port_for_scheme(scheme: string) -> int32
```

Returns a 32-bit integer of type `int32`.

This function never panics.

```yukino
load std.simpleurl: default_port_for_scheme
load std.io: println

set port = default_port_for_scheme("https")
println("Port https: ${port}")
```

### 6.8 parse_query

Parses a query string of the form `key1=val1&key2=val2` into an array of `QueryParam[]` with automatic percent-decoding of keys and values. A leading `?` is ignored if present.

```yukino
public fun parse_query(q_str: string) -> QueryParam[]
```

Returns an array of `QueryParam[]`.

This function never panics.

```yukino
load std.simpleurl: parse_query
load std.io: println

set params = parse_query("name=yukino&tag=lang&page=1")
for i in 0..params.len() {
    println(params[i].key + " = " + params[i].value)
}
```

---

[← Previous: Module std.env](13-env.md) | [Next: Module std.compress →](15-compress.md)
