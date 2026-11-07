# Модуль std.locale

Модуль `std.locale` предоставляет кроссплатформенный доступ к языковым и региональным настройкам операционной системы, кодам языка и страны, списку предпочтительных языков пользователя и системному часовому поясу.

---

## 1. Подключение: load std.locale

Импорт всего пакета:

```yukino
load std.locale
```

При импорте модуля доступны следующие функции:

- **Функции:** `current_language`, `current_locale`, `preferred_languages`, `current_country`, `system_timezone`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.locale` глобальные константы не объявлены.

---

## 3. Интерфейсы

В модуле `std.locale` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

В модуле `std.locale` пользовательские структуры данных и структуры ошибок не объявлены.

---

## 5. Классы

В модуле `std.locale` классы не объявлены.

---

## 6. Справочник функций

### 6.1 current_language

Определяет основной двухбуквенный код языка интерфейса операционной системы по стандарту ISO 639-1 (в нижнем регистре, например `"ru"`, `"en"`, `"de"`, `"zh"`).

```yukino
public fun current_language() -> string
```

Возвращает двухбуквенную строку с кодом языка типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику. Если язык в системе не определен, возвращает `"en"`.

```yukino
load std.locale: current_language
load std.io: println

set lang = current_language()
println("Основной язык: " + lang)
```

### 6.2 current_locale

Возвращает полный системный тег локали в формате POSIX / BCP 47 (например, `"ru_RU"`, `"en_US"`, `"zh_CN"`), очищенный от указаний кодировок (таких как `.UTF-8`) и модификаторов.

```yukino
public fun current_locale() -> string
```

Возвращает строковое значение тега локали типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику. При невозможности определить локаль возвращает значение по умолчанию `"en_US"`.

```yukino
load std.locale: current_locale
load std.io: println

set loc = current_locale()
println("Текущая локаль системы: " + loc)
```

### 6.3 preferred_languages

Возвращает упорядоченный список предпочтительных языков пользователя в порядке приоритета, полученный из настроек операционной системы или переменной окружения `LANGUAGE`.

```yukino
public fun preferred_languages() -> string[]
```

Возвращает массив строк типа `string[]`. Элементы списка содержат нормализованные двухбуквенные коды языков по стандарту ISO 639-1 без дубликатов. Возвращаемое значение не nullable.

Функция никогда не вызывает панику. Если список предпочтений пуст, возвращает массив из одного элемента с текущим языком `[current_language()]`.

```yukino
load std.locale: preferred_languages
load std.io: println

set langs = preferred_languages()
for i in 0..langs.len() {
    println("Язык #" + cast(i + 1, "string") + ": " + langs[i])
}
```

### 6.4 current_country

Определяет двухбуквенный код страны по стандарту ISO 3166-1 (в верхнем регистре, например `"RU"`, `"US"`, `"DE"`), извлеченный из региональной части текущей локали системы.

```yukino
public fun current_country() -> string?
```

Возвращает двухбуквенную строку с кодом страны типа `string` либо `null`, если текущая локаль системы не содержит указания региона. Возвращаемое значение nullable.

Функция никогда не вызывает панику.

```yukino
load std.locale: current_country
load std.io: println

set country = current_country()
if country != null {
    println("Страна пользователя: " + (country ?? ""))
} else {
    println("Регион не указан в системной локали")
}
```

### 6.5 system_timezone

Определяет имя системного часового пояса операционной системы в формате базы данных IANA Time Zone (например, `"Europe/Moscow"`, `"America/New_York"`, `"Asia/Tokyo"`, `"UTC"`).

```yukino
public fun system_timezone() -> string
```

Возвращает строковое имя часового пояса типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику. При невозможности определить системную временную зону возвращает `"UTC"`.

```yukino
load std.locale: system_timezone
load std.io: println

set tz = system_timezone()
println("Системный часовой пояс: " + tz)
```

---

[← Назад: Модуль std.fs.archives](16-archives.md) | [Вперед: Модуль std.numf →](18-numf.md)
