# Модуль std.net

Модуль `std.net` предоставляет полный высокоуровневый сетевой стек для языка Yukino: парсинг и разрешение сетевых адресов IPv4/IPv6, надежную потоковую передачу данных через TCP (`TcpStream`, `TcpListener`), датаграммный обмен через UDP (`UdpSocket`), кроссплатформенное мультиплексирование ввода-вывода с неблокирующими сокетами (`SocketPoller`), криптографическое шифрование TLS/HTTPS (`TlsStream`, `TlsServerConfig`), кодек протокола HTTP/1.1 (`HttpRequest`, `HttpResponse`, `HttpHeaders`), встроенный HTTP/HTTPS сервер (`HttpServer`, `HttpsServer`) и клиент сетевых HTTP-запросов (`HttpClient`, `get`, `post`, `post_json`).

---

## 1. Подключение: load std.net

Импорт всего пакета:

```yukino
load std.net
```

При импорте модуля становятся доступны следующие компоненты:

- **Константы:**
  - Методы HTTP: `METHOD_GET`, `METHOD_POST`, `METHOD_PUT`, `METHOD_DELETE`, `METHOD_PATCH`, `METHOD_HEAD`
  - Статусы HTTP: `STATUS_OK`, `STATUS_CREATED`, `STATUS_ACCEPTED`, `STATUS_NO_CONTENT`, `STATUS_MOVED_PERMANENTLY`, `STATUS_FOUND`, `STATUS_SEE_OTHER`, `STATUS_NOT_MODIFIED`, `STATUS_BAD_REQUEST`, `STATUS_UNAUTHORIZED`, `STATUS_FORBIDDEN`, `STATUS_NOT_FOUND`, `STATUS_METHOD_NOT_ALLOWED`, `STATUS_CONFLICT`, `STATUS_PAYLOAD_TOO_LARGE`, `STATUS_INTERNAL_SERVER_ERROR`, `STATUS_NOT_IMPLEMENTED`, `STATUS_BAD_GATEWAY`, `STATUS_SERVICE_UNAVAILABLE`
  - События поллера: `POLL_READABLE`, `POLL_WRITABLE`, `POLL_BOTH`
- **Структуры данных и ошибок:**
  - `SocketAddr` — разобранный сетевой адрес (хост и порт)
  - `NetError` — ошибка сетевых сокетов и системных вызовов
  - `TlsError` — ошибка криптографического рукопожатия и сертификатов TLS
  - `HttpError` — ошибка HTTP-клиента и сервера
  - `UdpPacket` — полученный UDP пакет с данными и адресом отправителя
  - `PollEvent` — событие готовности дескриптора сокета
  - `TlsServerConfig` — пути к сертификату и приватному ключу сервера
  - `HeaderEntry` — пара «ключ-значение» HTTP-заголовка
- **Классы:**
  - `TcpStream` — клиентский TCP поток
  - `TcpListener` — слушающий серверный TCP сокет
  - `UdpSocket` — датаграммный сокет UDP
  - `SocketPoller` — кроссплатформенный мультиплексор готовности сокетов
  - `TlsStream` — зашифрованный TLS сеанс
  - `HttpHeaders` — коллекция HTTP-заголовков с регистронезависимым доступом
  - `HttpRequest` — разобранный HTTP-запрос
  - `HttpResponse` — сформированный HTTP-ответ
  - `HttpServer` — легковесный HTTP сервер с поддержкой Keep-Alive
  - `HttpsServer` — защищенный HTTPS сервер с TLS терминацией
  - `HttpClient` — настраиваемый HTTP/HTTPS клиент
- **Функции:**
  - Адресация и DNS: `parse_socket_addr`, `format_socket_addr`, `resolve`
  - TCP фабрики: `tcp_connect`, `tcp_listen`
  - UDP фабрика: `udp_bind`
  - TLS фабрики: `tls_connect`, `tls_accept`
  - HTTP фабрики ответов: `response_status`, `response_ok`, `response_text`, `response_json_str`, `response_html`, `response_json`, `response_not_found`, `response_error`, `status_reason`, `request_new`
  - Кодек HTTP/1.1: `parse_request_str`, `parse_response_str`, `format_request`, `format_response`, `parse_request`, `parse_tls_request`, `parse_response`, `parse_tls_response`
  - Запросы requests: `get`, `http_get`, `post`, `http_post`, `post_json`, `http_post_json`, `put`, `http_put`, `delete`, `http_delete`, `patch`, `http_patch`, `head`, `http_head`, `http_client_default`

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

### 2.1 Методы HTTP

Строковые константы стандартных глаголов HTTP/1.1:

```yukino
public static METHOD_GET: string = "GET"
public static METHOD_POST: string = "POST"
public static METHOD_PUT: string = "PUT"
public static METHOD_DELETE: string = "DELETE"
public static METHOD_PATCH: string = "PATCH"
public static METHOD_HEAD: string = "HEAD"
```

### 2.2 Коды статусов HTTP

Числовые коды наиболее распространенных ответов протокола HTTP/1.1:

```yukino
public static STATUS_OK: int32 = 200
public static STATUS_CREATED: int32 = 201
public static STATUS_ACCEPTED: int32 = 202
public static STATUS_NO_CONTENT: int32 = 204
public static STATUS_MOVED_PERMANENTLY: int32 = 301
public static STATUS_FOUND: int32 = 302
public static STATUS_SEE_OTHER: int32 = 303
public static STATUS_NOT_MODIFIED: int32 = 304
public static STATUS_BAD_REQUEST: int32 = 400
public static STATUS_UNAUTHORIZED: int32 = 401
public static STATUS_FORBIDDEN: int32 = 403
public static STATUS_NOT_FOUND: int32 = 404
public static STATUS_METHOD_NOT_ALLOWED: int32 = 405
public static STATUS_CONFLICT: int32 = 409
public static STATUS_PAYLOAD_TOO_LARGE: int32 = 413
public static STATUS_INTERNAL_SERVER_ERROR: int32 = 500
public static STATUS_NOT_IMPLEMENTED: int32 = 501
public static STATUS_BAD_GATEWAY: int32 = 502
public static STATUS_SERVICE_UNAVAILABLE: int32 = 503
```

### 2.3 Маски событий поллера

Битовые маски для регистрации интереса к событиям на сокетах:

```yukino
public static POLL_READABLE: uint32 = 1
public static POLL_WRITABLE: uint32 = 2
public static POLL_BOTH: uint32 = 3
```

---

## 3. Интерфейсы

В модуле `std.net` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 SocketAddr

Представляет разобранный сетевой адрес в виде комбинации IP-хоста (или доменного имени) и числового номера порта.

```yukino
public struct SocketAddr {
    host: string
    port: int32
}
```

### 4.2 NetError

Структура ошибки операционной системы при выполнении сетевых операций ввода-вывода (TCP, UDP, Poller, DNS).

```yukino
public struct NetError {
    message: string
    code: int32
}
```

Коды ошибок `NetError`:
- `1` — ресурс не найден (NotFound)
- `2` — доступ запрещен (PermissionDenied)
- `3` — соединение отклонено (ConnectionRefused)
- `4` — соединение сброшено (ConnectionReset)
- `5` — соединение разорвано (ConnectionAborted)
- `6` — сокет не подключен (NotConnected)
- `7` — адрес уже используется (AddrInUse)
- `8` — адрес недоступен (AddrNotAvailable)
- `9` — превышен интервал ожидания (TimedOut)
- `10` — операция заблокирована в неблокирующем режиме (WouldBlock)
- `11` — некорректный аргумент или ввод (InvalidInput)
- `12` — невалидные данные UTF-8 (InvalidData)
- `20` — прочие системные ошибки сокетов

### 4.3 TlsError

Структура ошибки криптографического уровня TLS/HTTPS при проверке сертификатов, рукопожатии или дешифровании.

```yukino
public struct TlsError {
    message: string
    code: int32
}
```

Коды ошибок `TlsError`:
- `1` — ошибка разбора PEM-сертификата
- `2` — ошибка разбора PEM-ключа
- `3` — ошибка верификации цепочки доверия X.509
- `4` — некорректное имя домена SNI
- `5` — сбой TLS рукопожатия (handshake)
- `6` — ошибка чтения/записи зашифрованного сеанса

### 4.4 HttpError

Структура ошибки HTTP-уровня при превышении таймаутов, ошибках протокола или невалидных заголовках.

```yukino
public struct HttpError {
    message: string
    code: int32
}
```

### 4.5 UdpPacket

Содержит полученные через сокет UDP полезные данные и адрес удаленного отправителя.

```yukino
public struct UdpPacket {
    data: string
    sender: SocketAddr
}
```

### 4.6 PollEvent

Описывает событие готовности, возвращаемое методом `SocketPoller.wait`.

```yukino
public struct PollEvent {
    key: uint64
    readable: bool
    writable: bool
    is_error: bool
}
```

### 4.7 TlsServerConfig

Конфигурация TLS сервера, задающая файловые пути к цепочке сертификатов и закрытому ключу в формате PEM.

```yukino
public struct TlsServerConfig {
    cert_path: string
    key_path: string
}
```

### 4.8 HeaderEntry

Описывает отдельный заголовок HTTP в виде пары ключа и значения.

```yukino
public struct HeaderEntry {
    key: string
    value: string
}
```

---

## 5. Классы

### 5.1 TcpStream

Полнодуплексный сетевой поток TCP. Реализует чтение, запись, управление неблокирующим режимом, таймаутами и плавным закрытием соединения.

```yukino
public class TcpStream {
    id: uint64
}
```

#### Методы TcpStream

- `read(max_len: int64 = 65536) -> string` — читает текстовые данные до `max_len` байт. Возвращает строку `string`. В случае системного сбоя вызывает панику со структурой `NetError`.
- `read_bytes(buf: ptr<uint8>, max_len: int64) -> int64` — читает до `max_len` байт в предоставленный буфер. Возвращает количество прочитанных байт типа `int64`. В случае ошибки вызывает панику с `NetError`.
- `write(data: string) -> int64` — отправляет строку `data` в сокет. Возвращает количество переданных байт типа `int64`. В случае ошибки вызывает панику с `NetError`.
- `write_bytes(buf: ptr<uint8>, len: int64) -> int64` — отправляет сырые байты из буфера. Возвращает число байт типа `int64`. При ошибке вызывает панику с `NetError`.
- `flush() -> bool` — сбрасывает внутренний буфер сокета на уровень ОС. Возвращает `true`. В случае ошибки вызывает панику с `NetError`.
- `set_timeouts(read_ms: int64, write_ms: int64) -> bool` — настраивает таймауты чтения и записи в миллисекундах. Значение `0` отключает таймаут. Возвращает `true`.
- `set_nodelay(nodelay: bool) -> bool` — включает или выключает алгоритм Нагла (`TCP_NODELAY`). Возвращает `true`.
- `set_nonblocking(nonblocking: bool) -> bool` — переключает сокет в неблокирующий режим. Возвращает `true`.
- `shutdown(how: int32 = 2) -> bool` — завершает чтение (`0`), запись (`1`) или оба направления (`2`). Возвращает `true`.
- `peer_addr() -> string` — возвращает строковый адрес удаленной стороны (`"IP:port"`).
- `local_addr() -> string` — возвращает строковый локальный адрес сокета.
- `close() -> bool` — закрывает сокет и освобождает системный дескриптор.

```yukino
load std.net.tcp: tcp_connect
load std.io: println

set stream = tcp_connect("127.0.0.1:8080", 5000)
stream.write("GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
stream.flush()
set resp = stream.read(4096)
println(resp)
stream.close()
```

### 5.2 TcpListener

Серверный TCP сокет, ожидающий входящие клиентские подключения на заданном локальном адресе.

```yukino
public class TcpListener {
    id: uint64
}
```

#### Методы TcpListener

- `accept() -> TcpStream` — ожидает и принимает новое входящее клиентское соединение. Возвращает не-nullable объект `TcpStream`. При системном сбое вызывает панику со структурой `NetError`.
- `set_nonblocking(nonblocking: bool) -> bool` — переключает слушающий сокет в неблокирующий режим. Возвращает `true`.
- `local_addr() -> string` — возвращает локальный адрес привязки сокета.
- `close() -> bool` — прекращает прослушивание и закрывает системный сокет.

```yukino
load std.net.tcp: tcp_listen
load std.io: println

set listener = tcp_listen("127.0.0.1:8080")
println("Сервер слушает на: " + listener.local_addr())
set client = listener.accept()
client.write("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK")
client.close()
listener.close()
```

### 5.3 UdpSocket

Сетевой датаграммный сокет UDP для отправки и приема сообщений без установления постоянного соединения.

```yukino
public class UdpSocket {
    id: uint64
}
```

#### Методы UdpSocket

- `send_to(data: string, addr: string) -> int64` — отправляет датаграмму `data` по указанному адресу `addr` (`"IP:port"`). Возвращает число отправленных байт типа `int64`. При ошибке вызывает панику с `NetError`.
- `recv_from(max_len: int64 = 65536) -> UdpPacket` — ожидает и принимает входящую датаграмму. Возвращает структуру `UdpPacket`, содержащую полезную нагрузку и разобранный адрес отправителя. При ошибке вызывает панику со структурой `NetError`.
- `connect(addr: string) -> bool` — связывает UDP сокет с удаленным адресом для использования вызовов `send`/`recv`. Возвращает `true`.
- `send(data: string) -> int64` — отправляет датаграмму на подключенный адрес. Возвращает число байт типа `int64`.
- `recv(max_len: int64 = 65536) -> string` — читает датаграмму с подключенного адреса. Возвращает строку `string`.
- `set_broadcast(broadcast: bool) -> bool` — включает или выключает широковещательный режим (`SO_BROADCAST`). Возвращает `true`.
- `set_nonblocking(nonblocking: bool) -> bool` — переключает UDP сокет в неблокирующий режим. Возвращает `true`.
- `set_timeouts(read_ms: int64, write_ms: int64) -> bool` — задает таймауты чтения и отправки датаграмм в миллисекундах.
- `last_sender() -> string` — возвращает строковый адрес отправителя последней принятой датаграммы.
- `local_addr() -> string` — возвращает локальный адрес сокета.
- `close() -> bool` — закрывает UDP сокет.

```yukino
load std.net.udp: udp_bind
load std.io: println

set socket = udp_bind("127.0.0.1:9000")
socket.send_to("ping", "127.0.0.1:9001")
set pkt = socket.recv_from(1024)
println("Получено от " + pkt.sender.host + ": " + pkt.data)
socket.close()
```

### 5.4 SocketPoller

Кроссплатформенный мультиплексор готовности дескрипторов ввода-вывода (на базе `epoll` на Linux, `kqueue` на macOS/BSD, IOCP/wevent на Windows). Позволяет эффективно обрабатывать тысячи одновременных TCP соединений в неблокирующем режиме в одном потоке.

```yukino
public class SocketPoller {
    id: uint64
}
```

#### Методы SocketPoller

- `add_stream(ref stream: TcpStream, interest: uint32, key: uint64) -> bool` — регистрирует `TcpStream` в поллере с указанным ключом `key` и маской интереса (`POLL_READABLE`, `POLL_WRITABLE`, `POLL_BOTH`). Возвращает `true`.
- `add_listener(ref listener: TcpListener, interest: uint32, key: uint64) -> bool` — регистрирует серверный `TcpListener` для отслеживания входящих соединений. Возвращает `true`.
- `modify_stream(ref stream: TcpStream, interest: uint32, key: uint64) -> bool` — изменяет маску отслеживаемых событий для зарегистрированного потока. Возвращает `true`.
- `remove_stream(ref stream: TcpStream) -> bool` — удаляет поток из отслеживания поллером. Возвращает `true`.
- `remove_listener(ref listener: TcpListener) -> bool` — удаляет слушатель из отслеживания. Возвращает `true`.
- `wait(timeout_ms: int32 = -1) -> PollEvent[]` — ожидает возникновения событий готовности до `timeout_ms` миллисекунд (значение `-1` означает бесконечное ожидание). Возвращает массив структур `PollEvent[]`. При ошибке вызывает панику с `NetError`.
- `close() -> bool` — уничтожает поллер и освобождает ресурсы ядра ОС.

```yukino
load std.net.poll: SocketPoller, POLL_READABLE
load std.net.tcp: tcp_listen
load std.io: println

set poller = init SocketPoller()
set listener = tcp_listen("127.0.0.1:8080")
listener.set_nonblocking(true)
poller.add_listener(ref listener, POLL_READABLE, 1u64)

set events = poller.wait(1000)
for i in 0..events.len() {
    if events[i].key == 1u64 && events[i].readable {
        set client = listener.accept()
        println("Принято входящее подключение!")
        client.close()
    }
}
listener.close()
poller.close()
```

### 5.5 TlsStream

Зашифрованный TLS сеанс поверх базового транспортного потока `TcpStream`. Предоставляет прозрачное шифрование и дешифрование данных по протоколу TLS 1.2 / TLS 1.3 с использованием современной криптографии Rustls.

```yukino
public class TlsStream {
    id: uint64
}
```

#### Методы TlsStream

- `read(max_len: int64 = 65536) -> string` — читает дешифрованные данные из TLS сеанса до `max_len` байт. Возвращает строку `string`. В случае криптографической ошибки вызывает панику со структурой `TlsError`.
- `read_bytes(buf: ptr<uint8>, max_len: int64) -> int64` — читает дешифрованные данные в сырой байтовый буфер. Возвращает число байт `int64`. При ошибке вызывает панику с `TlsError`.
- `write(data: string) -> int64` — шифрует и отправляет строку `data`. Возвращает число байт `int64`. При ошибке вызывает панику с `TlsError`.
- `write_bytes(buf: ptr<uint8>, len: int64) -> int64` — шифрует и отправляет байты из буфера. Возвращает число байт `int64`.
- `flush() -> bool` — принудительно передает накопленные зашифрованные записи в базовый сокет. Возвращает `true`.
- `alpn_protocol() -> string?` — возвращает согласованный в процессе рукопожатия протокол ALPN (например, `"http/1.1"`) либо `null`, если согласование не производилось.
- `close() -> bool` — отправляет уведомление `close_notify` и завершает TLS сеанс.

```yukino
load std.net.tcp: tcp_connect
load std.net.tls: tls_connect
load std.io: println

set tcp = tcp_connect("example.com:443", 5000)
set tls = tls_connect(ref tcp, "example.com", false)
tls.write("GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n")
tls.flush()
set response = tls.read(4096)
println(response)
tls.close()
tcp.close()
```

### 5.6 HttpHeaders

Коллекция HTTP-заголовков с регистронезависимым поиском ключей по RFC 7230, добавлением, заменой и форматированием в проводной формат протокола.

```yukino
public class HttpHeaders {
    entries: HeaderEntry[]
}
```

#### Методы HttpHeaders

- `set(key: string, value: string) -> HttpHeaders` — устанавливает значение заголовка `key`, заменяя все ранее добавленные заголовки с таким именем (без учета регистра). Возвращает `this` для fluent-цепочек.
- `add(key: string, value: string) -> HttpHeaders` — добавляет заголовок `key` со значением `value` без удаления существующих дубликатов. Возвращает `this`.
- `get(key: string) -> string?` — выполняет регистронезависимый поиск значения заголовка по имени `key`. Возвращает строку `string` либо `null`, если заголовок отсутствует.
- `has(key: string) -> bool` — проверяет наличие заголовка с указанным именем (без учета регистра). Возвращает `true` или `false`.
- `remove(key: string) -> HttpHeaders` — удаляет все заголовки с указанным именем `key`. Возвращает `this`.
- `count() -> int32` — возвращает общее количество заголовков в коллекции.
- `format() -> string` — сериализует заголовки в формат протокола HTTP (`"Key: Value\r\n"` для каждой записи).

```yukino
load std.net.http_types: HttpHeaders
load std.io: println

set h = init HttpHeaders()
h.set("Content-Type", "application/json").set("Authorization", "Bearer token123")
if h.has("content-type") {
    println("Тип контента: " + (h.get("Content-Type") ?? ""))
}
println(h.format())
```

### 5.7 HttpRequest

Описывает входящий или исходящий HTTP-запрос, включая метод, путь URL, коллекцию заголовков `HttpHeaders`, тело сообщения и предварительно разобранные параметры строки запроса.

```yukino
public class HttpRequest {
    method: string
    path: string
    headers: HttpHeaders
    body: string
    query: QueryParam[]
}
```

#### Методы HttpRequest

- `query_param(key: string) -> string?` — возвращает значение параметра строки запроса по имени `key` либо `null`, если параметр не найден.
- `header(key: string) -> string?` — возвращает значение HTTP-заголовка по имени `key` (без учета регистра).
- `text() -> string` — возвращает тело запроса в виде строки.

### 5.8 HttpResponse

Описывает HTTP-ответ, содержащий трехзначный числовой статус-код, заголовки `HttpHeaders` и тело ответа.

```yukino
public class HttpResponse {
    status: int32
    headers: HttpHeaders
    body: string
}
```

#### Методы HttpResponse

- `header(key: string) -> string?` — возвращает значение заголовка ответа по имени `key`.
- `text() -> string` — возвращает тело ответа в виде строки.
- `json() -> JsonValue?` — производит автоматический парсинг тела ответа как JSON, возвращая объект `JsonValue` либо `null`, если тело пустое или парсинг не удался.

### 5.9 HttpServer

Встраиваемый HTTP/1.1 сервер, поддерживающий постоянные соединения (HTTP Keep-Alive), конвейерную обработку запросов и передачу управления функции-обработчику.

```yukino
public class HttpServer {
    addr: string
}
```

#### Методы HttpServer

- `listen(handler: fun(HttpRequest) -> HttpResponse)` — запускает прослушивание указанного адреса и цикл обработки входящих соединений. Для каждого запроса вызывается переданная функция `handler`.
- `handle_connection(stream: TcpStream, handler: fun(HttpRequest) -> HttpResponse)` — обрабатывает последовательность HTTP-запросов на уже принятом клиентском потоке `stream`.
- `close()` — останавливает сервер и освобождает слушающий сетевой сокет.

```yukino
load std.net.server: HttpServer
load std.net.http_types: response_ok, response_not_found
load std.io: println

set server = init HttpServer("127.0.0.1:8080")
println("HTTP сервер запущен на http://127.0.0.1:8080")
server.listen(fun(req: HttpRequest) -> HttpResponse {
    if req.path == "/hello" {
        return response_ok("Привет из Yukino!")
    }
    return response_not_found("Страница не найдена")
})
```

### 5.10 HttpsServer

Защищенный HTTPS сервер со встроенной терминацией TLS на базе сертификатов X.509 и приватных ключей PKCS#8/RSA.

```yukino
public class HttpsServer {
    addr: string
    config: TlsServerConfig
}
```

#### Методы HttpsServer

- `listen(handler: fun(HttpRequest) -> HttpResponse)` — начинает прием зашифрованных клиентских подключений, автоматически производит TLS рукопожатие и передает расшифрованные HTTP-запросы в `handler`.
- `handle_tls_connection(tls: TlsStream, handler: fun(HttpRequest) -> HttpResponse)` — обрабатывает защищенное соединение.
- `close()` — останавливает HTTPS сервер.

### 5.11 HttpClient

Настраиваемый клиент HTTP/1.1 и HTTPS с поддержкой таймаутов, установки пользовательских заголовков и опционального пропуска проверки сертификатов TLS.

```yukino
public class HttpClient {
    timeout_ms: int64
    insecure_skip_verify: bool
}
```

#### Методы HttpClient

- `request(method: string, url_str: string, body: string, headers: HttpHeaders?) -> HttpResponse` — отправляет произвольный HTTP/HTTPS запрос и возвращает `HttpResponse`. При сетевом сбое вызывает панику со структурой `HttpError`.
- `get(url_str: string, headers: HttpHeaders? = null) -> HttpResponse` — отправляет запрос метода GET.
- `post(url_str: string, body: string, headers: HttpHeaders? = null) -> HttpResponse` — отправляет запрос метода POST с телом `body`.
- `post_json(url_str: string, json_val: JsonValue, headers: HttpHeaders? = null) -> HttpResponse` — сериализует `json_val` и отправляет запрос POST с заголовком `Content-Type: application/json; charset=utf-8`.
- `put(url_str: string, body: string, headers: HttpHeaders? = null) -> HttpResponse` — отправляет запрос метода PUT.
- `delete(url_str: string, headers: HttpHeaders? = null) -> HttpResponse` — отправляет запрос метода DELETE.
- `patch(url_str: string, body: string, headers: HttpHeaders? = null) -> HttpResponse` — отправляет запрос метода PATCH.
- `head(url_str: string, headers: HttpHeaders? = null) -> HttpResponse` — отправляет запрос метода HEAD.

---

## 6. Справочник функций

### 6.1 parse_socket_addr

Разбирает строку вида `"IP:порт"` или `"хост:порт"` в типизированную структуру `SocketAddr`.

```yukino
public fun parse_socket_addr(addr_str: string) -> SocketAddr
```

Возвращает разобранный сетевой адрес типа `SocketAddr`.

В случае отсутствия разделителя двоеточия или некорректного формата номера порта вызывает панику со структурой `NetError` (код 11).

```yukino
load std.net.address: parse_socket_addr
load std.io: println

set addr = parse_socket_addr("127.0.0.1:8080")
println("Хост: " + addr.host + ", Порт: " + cast(addr.port, "string"))
```

### 6.2 format_socket_addr

Форматирует структуру `SocketAddr` в стандартную сетевую строку вида `"хост:порт"`.

```yukino
public fun format_socket_addr(addr: SocketAddr) -> string
```

Возвращает строковое представление адреса типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.net.address: format_socket_addr, SocketAddr
load std.io: println

set formatted = format_socket_addr(SocketAddr { host: "0.0.0.0", port: 3000 })
println(formatted)
```

### 6.3 resolve

Выполняет синхронное разрешение доменного имени через системный DNS в список IP-адресов.

```yukino
public fun resolve(hostname: string) -> string[]
```

Возвращает массив строк IP-адресов типа `string[]`.

В случае ошибки DNS-резолвинга вызывает панику со структурой `NetError` (код 1).

```yukino
load std.net.address: resolve
load std.io: println

set ips = resolve("localhost")
for i in 0..ips.len() {
    println("IP: " + ips[i])
}
```

### 6.4 tcp_connect

Устанавливает клиентское TCP соединение с удаленным узлом с ограничением по времени ожидания в миллисекундах.

```yukino
public fun tcp_connect(addr: string, timeout_ms: int64 = 15000) -> TcpStream
```

Возвращает подключенный сетевой поток типа `TcpStream`.

В случае отказа в подключении, недоступности узла или истечения таймаута вызывает панику со структурой `NetError`.

```yukino
load std.net.tcp: tcp_connect
load std.io: println

set client = tcp_connect("127.0.0.1:8080", 5000)
client.write("ping")
client.close()
```

### 6.5 tcp_listen

Создает и привязывает слушающий серверный TCP сокет к указанному локальному адресу.

```yukino
public fun tcp_listen(addr: string) -> TcpListener
```

Возвращает слушающий сетевой сокет типа `TcpListener`.

В случае ошибки привязки (например, адрес уже занят) вызывает панику со структурой `NetError` (код 7).

```yukino
load std.net.tcp: tcp_listen
load std.io: println

set listener = tcp_listen("127.0.0.1:9090")
println("Сервер ожидает подключений...")
listener.close()
```

### 6.6 udp_bind

Создает и привязывает датаграммный сокет UDP к локальному адресу.

```yukino
public fun udp_bind(addr: string) -> UdpSocket
```

Возвращает привязанный датаграммный сокет типа `UdpSocket`.

В случае ошибки привязки к порту вызывает панику со структурой `NetError`.

```yukino
load std.net.udp: udp_bind
load std.io: println

set sock = udp_bind("127.0.0.1:9091")
println("UDP сокет привязан к: " + sock.local_addr())
sock.close()
```

### 6.7 tls_connect

Устанавливает зашифрованный клиентский TLS сеанс поверх существующего потока `TcpStream`. Производит криптографическое рукопожатие с передачей доменного имени через расширение SNI и валидацией сертификата сервера через системное хранилище WebPKI.

```yukino
public fun tls_connect(ref tcp: TcpStream, hostname: string, skip_verify: bool = false) -> TlsStream
```

Возвращает зашифрованный сеанс типа `TlsStream`.

В случае сбоя проверки сертификата, недопустимого имени домена или нарушения протокола TLS вызывает панику со структурой `TlsError`.

```yukino
load std.net.tcp: tcp_connect
load std.net.tls: tls_connect
load std.io: println

set tcp = tcp_connect("google.com:443", 5000)
set tls = tls_connect(ref tcp, "google.com", false)
println("TLS рукопожатие успешно завершено!")
tls.close()
tcp.close()
```

### 6.8 tls_accept

Инициализирует серверный TLS сеанс поверх входящего принятого клиентского потока `TcpStream`, используя переданную конфигурацию сертификатов `TlsServerConfig`.

```yukino
public fun tls_accept(ref tcp: TcpStream, config: TlsServerConfig) -> TlsStream
```

Возвращает зашифрованный поток типа `TlsStream`.

В случае некорректного файла сертификата, закрытого ключа или ошибки рукопожатия вызывает панику со структурой `TlsError`.

### 6.9 status_reason

Возвращает канонический текст пояснения (Reason-Phrase) для переданного числового HTTP статус-кода по стандарту RFC 7231 (например, `"OK"` для `200`, `"Not Found"` для `404`). Для нестандартных кодов возвращает сам числовой код в строковом виде.

```yukino
public fun status_reason(code: int32) -> string
```

Возвращает строковое пояснение типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.net.http_types: status_reason
load std.io: println

println("200: " + status_reason(200))
println("404: " + status_reason(404))
```

### 6.10 response_status

Создает базовый объект `HttpResponse` с указанным кодом ответа и пустым телом.

```yukino
public fun response_status(status: int32) -> HttpResponse
```

Возвращает объект ответа типа `HttpResponse`.

Функция никогда не вызывает панику.

### 6.11 response_ok

Создает успешный HTTP-ответ со статусом 200 OK, телом `body` и автоматически выставленным заголовком `Content-Type: text/plain; charset=utf-8`.

```yukino
public fun response_ok(body: string) -> HttpResponse
```

Возвращает сформированный ответ типа `HttpResponse`.

Функция никогда не вызывает панику.

```yukino
load std.net.http_types: response_ok
load std.io: println

set resp = response_ok("Операция успешно завершена")
println("Статус: " + cast(resp.status, "string"))
```

### 6.12 response_text

Создает HTTP-ответ с произвольным текстовым содержимым, настраиваемым статусом ответа и заголовком `Content-Type: text/plain; charset=utf-8`.

```yukino
public fun response_text(content: string, status: int32) -> HttpResponse
```

Возвращает объект ответа типа `HttpResponse`.

Функция никогда не вызывает панику.

### 6.13 response_json_str

Создает HTTP-ответ со статусом `status`, телом из готовой строки JSON и заголовком `Content-Type: application/json; charset=utf-8`.

```yukino
public fun response_json_str(content: string, status: int32) -> HttpResponse
```

Возвращает объект ответа типа `HttpResponse`.

Функция никогда не вызывает панику.

### 6.14 response_json

Создает HTTP-ответ из типизированного узла `JsonValue`, автоматически производя его сериализацию в строку и выставляя заголовок `Content-Type: application/json; charset=utf-8`.

```yukino
public fun response_json(value: JsonValue, status: int32) -> HttpResponse
```

Возвращает объект ответа типа `HttpResponse`.

Функция никогда не вызывает панику.

### 6.15 response_not_found

Создает стандартный HTTP-ответ 404 Not Found с переданным сообщением об ошибке.

```yukino
public fun response_not_found(message: string) -> HttpResponse
```

Возвращает объект ответа типа `HttpResponse`.

Функция никогда не вызывает панику.

### 6.16 response_error

Создает HTTP-ответ серверной ошибки со статусом `status` (по умолчанию 500) и поясняющим текстом.

```yukino
public fun response_error(message: string, status: int32) -> HttpResponse
```

Возвращает объект ответа типа `HttpResponse`.

Функция никогда не вызывает панику.

### 6.17 parse_request_str

Разбирает сырую строковую последовательность байт HTTP-запроса в объект `HttpRequest`.

```yukino
public fun parse_request_str(raw: string) -> HttpRequest
```

Возвращает разобранный запрос типа `HttpRequest`.

Функция никогда не вызывает панику.

```yukino
load std.net.http_codec: parse_request_str
load std.io: println

set req = parse_request_str("GET /api/v1/users?page=2 HTTP/1.1\r\nHost: api.example.com\r\n\r\n")
println("Метод: " + req.method + ", Путь: " + req.path)
```

### 6.18 parse_response_str

Разбирает сырую строковую последовательность HTTP-ответа в объект `HttpResponse`.

```yukino
public fun parse_response_str(raw: string) -> HttpResponse
```

Возвращает разобранный ответ типа `HttpResponse`.

Функция никогда не вызывает панику.

### 6.19 parse_request

Читает и разбирает входящий HTTP-запрос непосредственно из потока `TcpStream`. Корректно считывает тело запроса фиксированной длины согласно заголовку `Content-Length`.

```yukino
public fun parse_request(ref stream: TcpStream) -> HttpRequest?
```

Возвращает разобранный `HttpRequest` либо `null`, если клиент закрыл соединение без передачи данных.

Функция никогда не вызывает панику.

### 6.20 parse_tls_request

Читает и разбирает входящий защищенный HTTP-запрос из сеанса `TlsStream`.

```yukino
public fun parse_tls_request(ref stream: TlsStream) -> HttpRequest?
```

Возвращает разобранный `HttpRequest` либо `null` при завершении соединения.

Функция никогда не вызывает панику.

### 6.21 parse_response

Читает входящий HTTP-ответ из сетевого потока `TcpStream`.

```yukino
public fun parse_response(ref stream: TcpStream) -> HttpResponse?
```

Возвращает прочитанный объект `HttpResponse` либо `null`, если удаленный сервер разорвал соединение.

Функция никогда не вызывает панику.

### 6.22 parse_tls_response

Читает входящий HTTP-ответ из защищенного сеанса `TlsStream`.

```yukino
public fun parse_tls_response(ref stream: TlsStream) -> HttpResponse?
```

Возвращает прочитанный объект `HttpResponse` либо `null`, если сервер разорвал сеанс.

Функция никогда не вызывает панику.

### 6.23 format_request

Сериализует объект `HttpRequest` в канонический проводной формат HTTP/1.1 с автоматическим вычислением заголовков `Host` и `Content-Length`.

```yukino
public fun format_request(req: HttpRequest, host: string = "") -> string
```

Возвращает проводную строку запроса типа `string`.

Функция никогда не вызывает панику.

### 6.24 format_response

Сериализует объект `HttpResponse` в проводной формат HTTP/1.1, формируя строку статуса с соответствующим `Reason-Phrase` и выставляя заголовок `Content-Length`.

```yukino
public fun format_response(res: HttpResponse) -> string
```

Возвращает проводную строку ответа типа `string`.

Функция никогда не вызывает панику.

### 6.25 get / http_get

Выполняет высокоуровневый сетевой запрос метода GET по указанному URL (поддерживаются протоколы `http://` и `https://`).

```yukino
public fun get(url_str: string) -> HttpResponse
public fun http_get(url_str: string) -> HttpResponse
```

Возвращает полученный ответ сервера типа `HttpResponse`.

В случае ошибки соединения, сетевого таймаута или недоступности хоста вызывает панику со структурой `HttpError`.

```yukino
load std.net.requests: get
load std.io: println

set resp = get("http://example.com/")
println("Статус ответа: " + cast(resp.status, "string"))
println("Тело:\n" + resp.body)
```

### 6.26 post / http_post

Выполняет высокоуровневый сетевой запрос метода POST с передачей текстового тела `body`.

```yukino
public fun post(url_str: string, body: string) -> HttpResponse
public fun http_post(url_str: string, body: string) -> HttpResponse
```

Возвращает объект ответа типа `HttpResponse`.

В случае сетевого сбоя вызывает панику со структурой `HttpError`.

```yukino
load std.net.requests: post
load std.io: println

set resp = post("http://httpbin.org/post", "hello from yukino")
println("Код ответа: " + cast(resp.status, "string"))
```

### 6.27 post_json / http_post_json

Сериализует переданное значение `JsonValue` и выполняет POST-запрос с автоматическим выставлением заголовка `Content-Type: application/json; charset=utf-8`.

```yukino
public fun post_json(url_str: string, json_val: JsonValue) -> HttpResponse
public fun http_post_json(url_str: string, json_val: JsonValue) -> HttpResponse
```

Возвращает объект ответа типа `HttpResponse`.

При ошибке соединения вызывает панику со структурой `HttpError`.

### 6.28 put / http_put

Выполняет HTTP-запрос метода PUT по указанному URL с телом `body`.

```yukino
public fun put(url_str: string, body: string) -> HttpResponse
public fun http_put(url_str: string, body: string) -> HttpResponse
```

Возвращает ответ сервера типа `HttpResponse`.

При сетевом сбое вызывает панику со структурой `HttpError`.

### 6.29 delete / http_delete

Выполняет HTTP-запрос метода DELETE по указанному URL.

```yukino
public fun delete(url_str: string) -> HttpResponse
public fun http_delete(url_str: string) -> HttpResponse
```

Возвращает ответ сервера типа `HttpResponse`.

При ошибке сети вызывает панику со структурой `HttpError`.

### 6.30 patch / http_patch

Выполняет HTTP-запрос метода PATCH с телом `body`.

```yukino
public fun patch(url_str: string, body: string) -> HttpResponse
public fun http_patch(url_str: string, body: string) -> HttpResponse
```

Возвращает ответ сервера типа `HttpResponse`.

При ошибке сети вызывает панику со структурой `HttpError`.

### 6.31 head / http_head

Выполняет легковесный HTTP-запрос метода HEAD для проверки существования ресурса и считывания заголовков без передачи тела.

```yukino
public fun head(url_str: string) -> HttpResponse
public fun http_head(url_str: string) -> HttpResponse
```

Возвращает ответ сервера типа `HttpResponse`.

При ошибке сети вызывает панику со структурой `HttpError`.

### 6.32 http_client_default

Создает экземпляр клиента `HttpClient` со стандартным таймаутом 15 000 миллисекунд и включенной верификацией TLS-сертификатов.

```yukino
public fun http_client_default() -> HttpClient
```

Возвращает настроенный клиент типа `HttpClient`.

Функция никогда не вызывает панику.

---

[← Назад: Модуль std.crypto](27-crypto.md) | [Вперед: Модуль std.regex →](29-regex.md)
