# Module std.locale

The `std.locale` module provides cross-platform access to operating system language and regional settings, ISO language and country codes, user preferred language lists, and system timezone name.

---

## 1. Import: load std.locale

Importing the entire package:

```yukino
load std.locale
```

When importing the module, the following functions are available:

- **Functions:** `current_language`, `current_locale`, `preferred_languages`, `current_country`, `system_timezone`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

The `std.locale` module does not declare global constants.

---

## 3. Interfaces

The `std.locale` module does not declare interfaces.

---

## 4. Data Structures and Error Types (Panics)

The `std.locale` module does not declare custom data structures or error types.

---

## 5. Classes

The `std.locale` module does not declare classes.

---

## 6. Function Reference

### 6.1 current_language

Returns the primary two-letter language code of the operating system interface under the ISO 639-1 standard (in lowercase, such as `"ru"`, `"en"`, `"de"`, `"zh"`).

```yukino
public fun current_language() -> string
```

Returns a two-letter language code string of type `string`. The return value is non-nullable.

The function never panics. If the system language cannot be determined, it falls back to `"en"`.

```yukino
load std.locale: current_language
load std.io: println

set lang = current_language()
println("Primary language: " + lang)
```

### 6.2 current_locale

Returns the full operating system locale tag in POSIX / BCP 47 format (such as `"ru_RU"`, `"en_US"`, `"zh_CN"`), cleaned of encoding suffixes (such as `.UTF-8`) and modifiers.

```yukino
public fun current_locale() -> string
```

Returns a locale tag string of type `string`. The return value is non-nullable.

The function never panics. If the locale cannot be detected, it returns `"en_US"`.

```yukino
load std.locale: current_locale
load std.io: println

set loc = current_locale()
println("System locale: " + loc)
```

### 6.3 preferred_languages

Returns an ordered list of the user's preferred languages in priority order, retrieved from the operating system configuration or the `LANGUAGE` environment variable.

```yukino
public fun preferred_languages() -> string[]
```

Returns an array of strings of type `string[]`. Elements contain normalized two-letter ISO 639-1 language codes without duplicates. The return value is non-nullable.

The function never panics. If no preferences are set, it returns a single-element array containing `[current_language()]`.

```yukino
load std.locale: preferred_languages
load std.io: println

set langs = preferred_languages()
for i in 0..langs.len() {
    println("Language #" + cast(i + 1, "string") + ": " + langs[i])
}
```

### 6.4 current_country

Returns the two-letter ISO 3166-1 alpha-2 country code (in uppercase, such as `"RU"`, `"US"`, `"DE"`), extracted from the territory part of the active system locale.

```yukino
public fun current_country() -> string?
```

Returns a two-letter country code string of type `string`, or `null` if the active system locale does not specify a region or country. The return value is nullable.

The function never panics.

```yukino
load std.locale: current_country
load std.io: println

set country = current_country()
if country != null {
    println("Country code: " + (country ?? ""))
} else {
    println("No country code in active locale")
}
```

### 6.5 system_timezone

Returns the operating system timezone identifier in IANA Time Zone database format (such as `"Europe/Moscow"`, `"America/New_York"`, `"Asia/Tokyo"`, `"UTC"`).

```yukino
public fun system_timezone() -> string
```

Returns a timezone name string of type `string`. The return value is non-nullable.

The function never panics. If the timezone cannot be resolved, it defaults to `"UTC"`.

```yukino
load std.locale: system_timezone
load std.io: println

set tz = system_timezone()
println("System timezone: " + tz)
```

---

[← Back: Module std.fs.archives](16-archives.md) | [Next: Module std.numf →](18-numf.md)
