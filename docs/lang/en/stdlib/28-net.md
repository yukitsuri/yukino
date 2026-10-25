# Module std.net

The `std.net` module provides a comprehensive, high-level networking stack for the Yukino programming language: IPv4/IPv6 address parsing and DNS resolution, reliable stream communication via TCP (`TcpStream`, `TcpListener`), datagram messaging via UDP (`UdpSocket`), cross-platform I/O readiness multiplexing with non-blocking sockets (`SocketPoller`), TLS/HTTPS encryption (`TlsStream`, `TlsServerConfig`), an HTTP/1.1 protocol codec (`HttpRequest`, `HttpResponse`, `HttpHeaders`), an embedded HTTP/HTTPS server (`HttpServer`, `HttpsServer`), and a high-level requests client (`HttpClient`, `get`, `post`, `post_json`).

---

## 1. Import: load std.net

Import the entire package:

```yukino
load std.net
```

Importing the module exposes the following entities:

- **Constants:**
  - HTTP Methods: `METHOD_GET`, `METHOD_POST`, `METHOD_PUT`, `METHOD_DELETE`, `METHOD_PATCH`, `METHOD_HEAD`
  - HTTP Status Codes: `STATUS_OK`, `STATUS_CREATED`, `STATUS_ACCEPTED`, `STATUS_NO_CONTENT`, `STATUS_MOVED_PERMANENTLY`, `STATUS_FOUND`, `STATUS_SEE_OTHER`, `STATUS_NOT_MODIFIED`, `STATUS_BAD_REQUEST`, `STATUS_UNAUTHORIZED`, `STATUS_FORBIDDEN`, `STATUS_NOT_FOUND`, `STATUS_METHOD_NOT_ALLOWED`, `STATUS_CONFLICT`, `STATUS_PAYLOAD_TOO_LARGE`, `STATUS_INTERNAL_SERVER_ERROR`, `STATUS_NOT_IMPLEMENTED`, `STATUS_BAD_GATEWAY`, `STATUS_SERVICE_UNAVAILABLE`
  - Poller Event Masks: `POLL_READABLE`, `POLL_WRITABLE`, `POLL_BOTH`
- **Data and Panic Structures:**
  - `SocketAddr` - parsed host and port network address
  - `NetError` - socket and OS syscall error
  - `TlsError` - TLS handshake, certificate, and cryptography error
  - `HttpError` - HTTP client and server protocol error
  - `UdpPacket` - received UDP payload with sender address
  - `PollEvent` - socket readiness event
  - `TlsServerConfig` - paths to server PEM certificate and private key
  - `HeaderEntry` - HTTP header key-value entry
- **Classes:**
  - `TcpStream` - client TCP stream
  - `TcpListener` - server TCP listener
  - `UdpSocket` - UDP datagram socket
  - `SocketPoller` - cross-platform readiness event poller
  - `TlsStream` - encrypted TLS session stream
  - `HttpHeaders` - case-insensitive HTTP headers collection
  - `HttpRequest` - parsed HTTP request
  - `HttpResponse` - HTTP response object
  - `HttpServer` - lightweight HTTP server with Keep-Alive support
  - `HttpsServer` - secure HTTPS server with TLS termination
  - `HttpClient` - configurable HTTP/HTTPS client
- **Functions:**
  - Addressing and DNS: `parse_socket_addr`, `format_socket_addr`, `resolve`
  - TCP Factories: `tcp_connect`, `tcp_listen`
  - UDP Factory: `udp_bind`
  - TLS Factories: `tls_connect`, `tls_accept`
  - HTTP Response Factories: `response_status`, `response_ok`, `response_text`, `response_json_str`, `response_html`, `response_json`, `response_not_found`, `response_error`, `status_reason`, `request_new`
  - HTTP/1.1 Codec: `parse_request_str`, `parse_response_str`, `format_request`, `format_response`, `parse_request`, `parse_tls_request`, `parse_response`, `parse_tls_response`
  - Requests Client: `get`, `http_get`, `post`, `http_post`, `post_json`, `http_post_json`, `put`, `http_put`, `delete`, `http_delete`, `patch`, `http_patch`, `head`, `http_head`, `http_client_default`

All listed entities are also automatically made available when loading the entire standard library: `load std`.

---

## 2. Constants

### 2.1 HTTP Methods

Standard HTTP/1.1 method name strings:

```yukino
public static METHOD_GET: string = "GET"
public static METHOD_POST: string = "POST"
public static METHOD_PUT: string = "PUT"
public static METHOD_DELETE: string = "DELETE"
public static METHOD_PATCH: string = "PATCH"
public static METHOD_HEAD: string = "HEAD"
```

### 2.2 HTTP Status Codes

Commonly used HTTP status codes:

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

### 2.3 Poller Event Masks

Bitmasks for registering socket readiness interests:

```yukino
public static POLL_READABLE: uint32 = 1
public static POLL_WRITABLE: uint32 = 2
public static POLL_BOTH: uint32 = 3
```

---

## 3. Interfaces

No global interfaces are declared in the `std.net` module.

---

## 4. Data and Panic Structures

### 4.1 SocketAddr

Represents a network address decomposed into an IP host string (or hostname) and integer port.

```yukino
public struct SocketAddr {
    host: string
    port: int32
}
```

### 4.2 NetError

Error structure raised during operating system socket operations (TCP, UDP, Poller, DNS).

```yukino
public struct NetError {
    message: string
    code: int32
}
```

`NetError` codes:
- `1` - resource not found (NotFound)
- `2` - permission denied (PermissionDenied)
- `3` - connection refused (ConnectionRefused)
- `4` - connection reset (ConnectionReset)
- `5` - connection aborted (ConnectionAborted)
- `6` - socket not connected (NotConnected)
- `7` - address already in use (AddrInUse)
- `8` - address not available (AddrNotAvailable)
- `9` - operation timed out (TimedOut)
- `10` - would block in non-blocking mode (WouldBlock)
- `11` - invalid argument or address input (InvalidInput)
- `12` - invalid UTF-8 data (InvalidData)
- `20` - other operating system socket error

### 4.3 TlsError

Error structure emitted during TLS handshake, certificate verification, or record decryption.

```yukino
public struct TlsError {
    message: string
    code: int32
}
```

`TlsError` codes:
- `1` - failed to parse PEM certificate
- `2` - failed to parse PEM private key
- `3` - X.509 certificate verification failure
- `4` - invalid SNI server name
- `5` - TLS handshake failed
- `6` - TLS record read/write error

### 4.4 HttpError

HTTP protocol error emitted upon network timeout, gateway error, or corrupt header formatting.

```yukino
public struct HttpError {
    message: string
    code: int32
}
```

### 4.5 UdpPacket

Holds the payload received via UDP and the remote sender's parsed socket address.

```yukino
public struct UdpPacket {
    data: string
    sender: SocketAddr
}
```

### 4.6 PollEvent

Describes an I/O readiness event returned by `SocketPoller.wait`.

```yukino
public struct PollEvent {
    key: uint64
    readable: bool
    writable: bool
    is_error: bool
}
```

### 4.7 TlsServerConfig

Server TLS configuration specifying file paths to the PEM certificate chain and private key.

```yukino
public struct TlsServerConfig {
    cert_path: string
    key_path: string
}
```

### 4.8 HeaderEntry

Represents a single HTTP header key-value pair.

```yukino
public struct HeaderEntry {
    key: string
    value: string
}
```

---

## 5. Classes

### 5.1 TcpStream

Full-duplex TCP stream supporting reading, writing, timeout configuration, TCP_NODELAY, and non-blocking modes.

```yukino
public class TcpStream {
    id: uint64
}
```

#### Methods of TcpStream

- `read(max_len: int64 = 65536) -> string` - reads up to `max_len` bytes as text. Returns a string of type `string`. In case of socket error, panics with `NetError`.
- `read_bytes(buf: ptr<uint8>, max_len: int64) -> int64` - reads up to `max_len` bytes into a raw buffer. Returns the byte count of type `int64`. Panics with `NetError` on failure.
- `write(data: string) -> int64` - writes `data` to the socket. Returns the number of written bytes of type `int64`. Panics with `NetError` on failure.
- `write_bytes(buf: ptr<uint8>, len: int64) -> int64` - writes raw bytes from buffer. Returns the byte count of type `int64`.
- `flush() -> bool` - flushes buffered data to the operating system socket. Returns `true`. Panics with `NetError` on failure.
- `set_timeouts(read_ms: int64, write_ms: int64) -> bool` - sets socket read and write timeouts in milliseconds (`0` disables timeout). Returns `true`.
- `set_nodelay(nodelay: bool) -> bool` - enables or disables Nagle's algorithm (`TCP_NODELAY`). Returns `true`.
- `set_nonblocking(nonblocking: bool) -> bool` - switches the socket to non-blocking mode. Returns `true`.
- `shutdown(how: int32 = 2) -> bool` - shuts down read (`0`), write (`1`), or both channels (`2`). Returns `true`.
- `peer_addr() -> string` - returns the remote socket address string (`"IP:port"`).
- `local_addr() -> string` - returns the local socket address string.
- `close() -> bool` - closes the socket descriptor.

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

Server TCP socket listening for incoming connections.

```yukino
public class TcpListener {
    id: uint64
}
```

#### Methods of TcpListener

- `accept() -> TcpStream` - blocks waiting for an incoming connection and returns a non-nullable `TcpStream`. Panics with `NetError` on OS error.
- `set_nonblocking(nonblocking: bool) -> bool` - switches the listener to non-blocking mode. Returns `true`.
- `local_addr() -> string` - returns the listening address string.
- `close() -> bool` - closes the listener socket.

```yukino
load std.net.tcp: tcp_listen
load std.io: println

set listener = tcp_listen("127.0.0.1:8080")
println("Server listening on " + listener.local_addr())
set client = listener.accept()
client.write("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK")
client.close()
listener.close()
```

### 5.3 UdpSocket

UDP datagram socket for connectionless messaging.

```yukino
public class UdpSocket {
    id: uint64
}
```

#### Methods of UdpSocket

- `send_to(data: string, addr: string) -> int64` - sends `data` to `addr` (`"IP:port"`). Returns sent byte count of type `int64`. Panics with `NetError` on failure.
- `recv_from(max_len: int64 = 65536) -> UdpPacket` - receives a datagram packet. Returns `UdpPacket` containing data and parsed sender address. Panics with `NetError` on failure.
- `connect(addr: string) -> bool` - connects UDP socket to a remote address for subsequent `send`/`recv` calls. Returns `true`.
- `send(data: string) -> int64` - sends data to connected address. Returns byte count of type `int64`.
- `recv(max_len: int64 = 65536) -> string` - receives data from connected address. Returns a string of type `string`.
- `set_broadcast(broadcast: bool) -> bool` - enables or disables broadcast mode (`SO_BROADCAST`). Returns `true`.
- `set_nonblocking(nonblocking: bool) -> bool` - toggles non-blocking socket mode. Returns `true`.
- `set_timeouts(read_ms: int64, write_ms: int64) -> bool` - configures read and write timeouts in milliseconds.
- `last_sender() -> string` - returns the address string of the last datagram sender.
- `local_addr() -> string` - returns the bound local address string.
- `close() -> bool` - closes the socket.

```yukino
load std.net.udp: udp_bind
load std.io: println

set socket = udp_bind("127.0.0.1:9000")
socket.send_to("ping", "127.0.0.1:9001")
set pkt = socket.recv_from(1024)
println("Received from " + pkt.sender.host + ": " + pkt.data)
socket.close()
```

### 5.4 SocketPoller

Cross-platform I/O event multiplexer (`epoll` on Linux, `kqueue` on macOS/BSD, IOCP/wevent on Windows) allowing thousands of non-blocking TCP connections to be handled concurrently in a single thread.

```yukino
public class SocketPoller {
    id: uint64
}
```

#### Methods of SocketPoller

- `add_stream(ref stream: TcpStream, interest: uint32, key: uint64) -> bool` - registers `TcpStream` with interest mask (`POLL_READABLE`, `POLL_WRITABLE`, `POLL_BOTH`) and identifier `key`. Returns `true`.
- `add_listener(ref listener: TcpListener, interest: uint32, key: uint64) -> bool` - registers server `TcpListener`. Returns `true`.
- `modify_stream(ref stream: TcpStream, interest: uint32, key: uint64) -> bool` - updates interest mask for registered stream. Returns `true`.
- `remove_stream(ref stream: TcpStream) -> bool` - deregisters stream from poller. Returns `true`.
- `remove_listener(ref listener: TcpListener) -> bool` - deregisters listener from poller. Returns `true`.
- `wait(timeout_ms: int32 = -1) -> PollEvent[]` - waits for I/O events up to `timeout_ms` milliseconds (`-1` means indefinite). Returns an array of `PollEvent[]`. Panics with `NetError` on failure.
- `close() -> bool` - destroys poller instance.

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
        println("Accepted client connection!")
        client.close()
    }
}
listener.close()
poller.close()
```

### 5.5 TlsStream

Encrypted TLS session over underlying `TcpStream` transport using Rustls cryptography for TLS 1.2 / TLS 1.3.

```yukino
public class TlsStream {
    id: uint64
}
```

#### Methods of TlsStream

- `read(max_len: int64 = 65536) -> string` - reads decrypted payload up to `max_len` bytes. Returns a string of type `string`. Panics with `TlsError` on failure.
- `read_bytes(buf: ptr<uint8>, max_len: int64) -> int64` - reads decrypted payload into a byte buffer. Returns byte count of type `int64`. Panics with `TlsError` on failure.
- `write(data: string) -> int64` - encrypts and sends `data`. Returns byte count of type `int64`. Panics with `TlsError` on failure.
- `write_bytes(buf: ptr<uint8>, len: int64) -> int64` - encrypts and sends raw bytes. Returns byte count of type `int64`.
- `flush() -> bool` - flushes encrypted records to the socket. Returns `true`.
- `alpn_protocol() -> string?` - returns negotiated ALPN protocol string (e.g. `"http/1.1"`) or `null` if none negotiated.
- `close() -> bool` - emits `close_notify` and shuts down session.

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

Collection of HTTP headers with case-insensitive lookup (RFC 7230), modification, and serialization.

```yukino
public class HttpHeaders {
    entries: HeaderEntry[]
}
```

#### Methods of HttpHeaders

- `set(key: string, value: string) -> HttpHeaders` - sets header `key` to `value`, replacing previous occurrences. Returns `this` for chaining.
- `add(key: string, value: string) -> HttpHeaders` - appends header without replacing existing keys. Returns `this`.
- `get(key: string) -> string?` - case-insensitive header lookup. Returns string value or `null` if missing.
- `has(key: string) -> bool` - checks if header exists (case-insensitive). Returns `true` or `false`.
- `remove(key: string) -> HttpHeaders` - removes all headers matching `key`. Returns `this`.
- `count() -> int32` - returns the total number of header entries.
- `format() -> string` - serializes headers into wire HTTP format (`"Key: Value\r\n"` for each line).

```yukino
load std.net.http_types: HttpHeaders
load std.io: println

set h = init HttpHeaders()
h.set("Content-Type", "application/json").set("Authorization", "Bearer token123")
if h.has("content-type") {
    println("Content-Type: " + (h.get("Content-Type") ?? ""))
}
println(h.format())
```

### 5.7 HttpRequest

HTTP request container holding method, URL path, `HttpHeaders`, body, and parsed query parameters.

```yukino
public class HttpRequest {
    method: string
    path: string
    headers: HttpHeaders
    body: string
    query: QueryParam[]
}
```

#### Methods of HttpRequest

- `query_param(key: string) -> string?` - returns query parameter value for `key` or `null` if absent.
- `header(key: string) -> string?` - case-insensitive header lookup.
- `text() -> string` - returns request body as string.

### 5.8 HttpResponse

HTTP response container holding status code, `HttpHeaders`, and response body.

```yukino
public class HttpResponse {
    status: int32
    headers: HttpHeaders
    body: string
}
```

#### Methods of HttpResponse

- `header(key: string) -> string?` - returns response header value.
- `text() -> string` - returns response body.
- `json() -> JsonValue?` - automatically parses body as JSON, returning `JsonValue` or `null` if parsing fails.

### 5.9 HttpServer

Embedded HTTP/1.1 server supporting Keep-Alive persistent connections and dispatching to a request handler callback.

```yukino
public class HttpServer {
    addr: string
}
```

#### Methods of HttpServer

- `listen(handler: fun(HttpRequest) -> HttpResponse)` - binds address and runs connection loop, dispatching requests to `handler`.
- `handle_connection(stream: TcpStream, handler: fun(HttpRequest) -> HttpResponse)` - processes HTTP requests on an accepted stream.
- `close()` - stops the server and releases listening socket.

```yukino
load std.net.server: HttpServer
load std.net.http_types: response_ok, response_not_found
load std.io: println

set server = init HttpServer("127.0.0.1:8080")
println("HTTP server listening on http://127.0.0.1:8080")
server.listen(fun(req: HttpRequest) -> HttpResponse {
    if req.path == "/hello" {
        return response_ok("Hello from Yukino!")
    }
    return response_not_found("Not Found")
})
```

### 5.10 HttpsServer

Embedded HTTPS server with TLS termination using X.509 certificates and PKCS#8/RSA keys.

```yukino
public class HttpsServer {
    addr: string
    config: TlsServerConfig
}
```

#### Methods of HttpsServer

- `listen(handler: fun(HttpRequest) -> HttpResponse)` - begins listening, accepts TLS client handshakes, and passes decrypted requests to `handler`.
- `handle_tls_connection(tls: TlsStream, handler: fun(HttpRequest) -> HttpResponse)` - processes an accepted TLS stream.
- `close()` - stops HTTPS server.

### 5.11 HttpClient

Configurable HTTP/1.1 and HTTPS client supporting custom timeouts, headers, and insecure TLS certificate verification override.

```yukino
public class HttpClient {
    timeout_ms: int64
    insecure_skip_verify: bool
}
```

#### Methods of HttpClient

- `request(method: string, url_str: string, body: string, headers: HttpHeaders?) -> HttpResponse` - executes HTTP/HTTPS request. Panics with `HttpError` on network or protocol error.
- `get(url_str: string, headers: HttpHeaders? = null) -> HttpResponse` - sends GET request.
- `post(url_str: string, body: string, headers: HttpHeaders? = null) -> HttpResponse` - sends POST request with body.
- `post_json(url_str: string, json_val: JsonValue, headers: HttpHeaders? = null) -> HttpResponse` - serializes `json_val` and sends POST request with `Content-Type: application/json; charset=utf-8`.
- `put(url_str: string, body: string, headers: HttpHeaders? = null) -> HttpResponse` - sends PUT request.
- `delete(url_str: string, headers: HttpHeaders? = null) -> HttpResponse` - sends DELETE request.
- `patch(url_str: string, body: string, headers: HttpHeaders? = null) -> HttpResponse` - sends PATCH request.
- `head(url_str: string, headers: HttpHeaders? = null) -> HttpResponse` - sends HEAD request.

---

## 6. Function Reference

### 6.1 parse_socket_addr

Parses an address string formatted as `"IP:port"` or `"host:port"` into a typed `SocketAddr`.

```yukino
public fun parse_socket_addr(addr_str: string) -> SocketAddr
```

Returns a parsed `SocketAddr`.

Panics with `NetError` (code 11) if the address string is missing the colon separator or has an invalid port number.

```yukino
load std.net.address: parse_socket_addr
load std.io: println

set addr = parse_socket_addr("127.0.0.1:8080")
println("Host: " + addr.host + ", Port: " + cast(addr.port, "string"))
```

### 6.2 format_socket_addr

Formats a `SocketAddr` into `"host:port"` format.

```yukino
public fun format_socket_addr(addr: SocketAddr) -> string
```

Returns a string of type `string`.

This function never panics.

```yukino
load std.net.address: format_socket_addr, SocketAddr
load std.io: println

set formatted = format_socket_addr(SocketAddr { host: "0.0.0.0", port: 3000 })
println(formatted)
```

### 6.3 resolve

Performs synchronous DNS resolution of a hostname to a list of IP address strings.

```yukino
public fun resolve(hostname: string) -> string[]
```

Returns an array of `string[]`.

Panics with `NetError` (code 1) if name resolution fails.

```yukino
load std.net.address: resolve
load std.io: println

set ips = resolve("localhost")
for i in 0..ips.len() {
    println("IP: " + ips[i])
}
```

### 6.4 tcp_connect

Connects to a remote TCP endpoint within a timeout limit in milliseconds.

```yukino
public fun tcp_connect(addr: string, timeout_ms: int64 = 15000) -> TcpStream
```

Returns a connected `TcpStream`.

Panics with `NetError` if the host is unreachable, connection is refused, or timeout expires.

```yukino
load std.net.tcp: tcp_connect
load std.io: println

set client = tcp_connect("127.0.0.1:8080", 5000)
client.write("ping")
client.close()
```

### 6.5 tcp_listen

Binds and starts a server TCP listener on `addr`.

```yukino
public fun tcp_listen(addr: string) -> TcpListener
```

Returns a listening `TcpListener`.

Panics with `NetError` (code 7) if the port is already in use.

```yukino
load std.net.tcp: tcp_listen
load std.io: println

set listener = tcp_listen("127.0.0.1:9090")
println("Server ready...")
listener.close()
```

### 6.6 udp_bind

Binds a UDP socket to `addr`.

```yukino
public fun udp_bind(addr: string) -> UdpSocket
```

Returns a bound `UdpSocket`.

Panics with `NetError` on failure.

```yukino
load std.net.udp: udp_bind
load std.io: println

set sock = udp_bind("127.0.0.1:9091")
println("UDP bound to " + sock.local_addr())
sock.close()
```

### 6.7 tls_connect

Establishes a client TLS session over an existing `TcpStream` transport, validating the remote certificate against system WebPKI roots and transmitting SNI.

```yukino
public fun tls_connect(ref tcp: TcpStream, hostname: string, skip_verify: bool = false) -> TlsStream
```

Returns an encrypted `TlsStream`.

Panics with `TlsError` on certificate validation error, SNI error, or handshake protocol failure.

```yukino
load std.net.tcp: tcp_connect
load std.net.tls: tls_connect
load std.io: println

set tcp = tcp_connect("google.com:443", 5000)
set tls = tls_connect(ref tcp, "google.com", false)
println("TLS handshake successful!")
tls.close()
tcp.close()
```

### 6.8 tls_accept

Performs server-side TLS handshake over an accepted client `TcpStream` using `TlsServerConfig`.

```yukino
public fun tls_accept(ref tcp: TcpStream, config: TlsServerConfig) -> TlsStream
```

Returns an encrypted `TlsStream`.

Panics with `TlsError` on invalid certificate/key files or handshake error.

### 6.9 status_reason

Returns canonical RFC 7231 Reason-Phrase string for an HTTP status code (e.g. `"OK"` for `200`, `"Not Found"` for `404`). For unknown status codes, returns the integer code itself as a string.

```yukino
public fun status_reason(code: int32) -> string
```

Returns a reason string of type `string`.

This function never panics.

```yukino
load std.net.http_types: status_reason
load std.io: println

println("200: " + status_reason(200))
println("404: " + status_reason(404))
```

### 6.10 response_status

Creates a base `HttpResponse` with the specified status code and empty body.

```yukino
public fun response_status(status: int32) -> HttpResponse
```

Returns an `HttpResponse`.

This function never panics.

### 6.11 response_ok

Creates an HTTP response with status 200 OK, body `body`, and `Content-Type: text/plain; charset=utf-8`.

```yukino
public fun response_ok(body: string) -> HttpResponse
```

Returns an `HttpResponse`.

This function never panics.

```yukino
load std.net.http_types: response_ok
load std.io: println

set resp = response_ok("Success")
println("Status: " + cast(resp.status, "string"))
```

### 6.12 response_text

Creates an HTTP response with text content, status code `status`, and `Content-Type: text/plain; charset=utf-8`.

```yukino
public fun response_text(content: string, status: int32) -> HttpResponse
```

Returns an `HttpResponse`.

This function never panics.

### 6.13 response_json_str

Creates an HTTP response with JSON string content, status code `status`, and `Content-Type: application/json; charset=utf-8`.

```yukino
public fun response_json_str(content: string, status: int32) -> HttpResponse
```

Returns an `HttpResponse`.

This function never panics.

### 6.14 response_json

Creates an HTTP response by serializing a `JsonValue` node and setting `Content-Type: application/json; charset=utf-8`.

```yukino
public fun response_json(value: JsonValue, status: int32) -> HttpResponse
```

Returns an `HttpResponse`.

This function never panics.

### 6.15 response_not_found

Creates an HTTP response with status 404 Not Found and custom error message.

```yukino
public fun response_not_found(message: string) -> HttpResponse
```

Returns an `HttpResponse`.

This function never panics.

### 6.16 response_error

Creates an HTTP server error response with status code `status` and error message.

```yukino
public fun response_error(message: string, status: int32) -> HttpResponse
```

Returns an `HttpResponse`.

This function never panics.

### 6.17 parse_request_str

Parses a raw HTTP request string into an `HttpRequest` object.

```yukino
public fun parse_request_str(raw: string) -> HttpRequest
```

Returns an `HttpRequest`.

This function never panics.

```yukino
load std.net.http_codec: parse_request_str
load std.io: println

set req = parse_request_str("GET /api/v1/users?page=2 HTTP/1.1\r\nHost: api.example.com\r\n\r\n")
println("Method: " + req.method + ", Path: " + req.path)
```

### 6.18 parse_response_str

Parses a raw HTTP response string into an `HttpResponse` object.

```yukino
public fun parse_response_str(raw: string) -> HttpResponse
```

Returns an `HttpResponse`.

This function never panics.

### 6.19 parse_request

Reads and parses an HTTP request directly from a `TcpStream`, reading fixed-length body payloads according to `Content-Length`.

```yukino
public fun parse_request(ref stream: TcpStream) -> HttpRequest?
```

Returns a parsed `HttpRequest` or `null` if the connection is closed without data.

This function never panics.

### 6.20 parse_tls_request

Reads and parses an HTTP request from an encrypted `TlsStream`.

```yukino
public fun parse_tls_request(ref stream: TlsStream) -> HttpRequest?
```

Returns a parsed `HttpRequest` or `null` upon connection termination.

This function never panics.

### 6.21 parse_response

Reads and parses an HTTP response from a `TcpStream`.

```yukino
public fun parse_response(ref stream: TcpStream) -> HttpResponse?
```

Returns an `HttpResponse` or `null` if the remote server closed the connection.

This function never panics.

### 6.22 parse_tls_response

Reads and parses an HTTP response from an encrypted `TlsStream`.

```yukino
public fun parse_tls_response(ref stream: TlsStream) -> HttpResponse?
```

Returns an `HttpResponse` or `null` if the session was closed.

This function never panics.

### 6.23 format_request

Serializes an `HttpRequest` to HTTP/1.1 wire format, calculating `Host` and `Content-Length` headers.

```yukino
public fun format_request(req: HttpRequest, host: string = "") -> string
```

Returns a request wire string of type `string`.

This function never panics.

### 6.24 format_response

Serializes an `HttpResponse` to HTTP/1.1 wire format with status reason phrase and `Content-Length`.

```yukino
public fun format_response(res: HttpResponse) -> string
```

Returns a response wire string of type `string`.

This function never panics.

### 6.25 get / http_get

Sends an HTTP GET request to `url_str` (supports both `http://` and `https://`).

```yukino
public fun get(url_str: string) -> HttpResponse
public fun http_get(url_str: string) -> HttpResponse
```

Returns server `HttpResponse`.

Panics with `HttpError` on connection failure, timeout, or host error.

```yukino
load std.net.requests: get
load std.io: println

set resp = get("http://example.com/")
println("Status: " + cast(resp.status, "string"))
println("Body:\n" + resp.body)
```

### 6.26 post / http_post

Sends an HTTP POST request with string body `body`.

```yukino
public fun post(url_str: string, body: string) -> HttpResponse
public fun http_post(url_str: string, body: string) -> HttpResponse
```

Returns server `HttpResponse`.

Panics with `HttpError` on failure.

```yukino
load std.net.requests: post
load std.io: println

set resp = post("http://httpbin.org/post", "hello from yukino")
println("Response status: " + cast(resp.status, "string"))
```

### 6.27 post_json / http_post_json

Serializes `JsonValue` and sends a POST request with `Content-Type: application/json; charset=utf-8`.

```yukino
public fun post_json(url_str: string, json_val: JsonValue) -> HttpResponse
public fun http_post_json(url_str: string, json_val: JsonValue) -> HttpResponse
```

Returns server `HttpResponse`.

Panics with `HttpError` on failure.

### 6.28 put / http_put

Sends an HTTP PUT request with string body `body`.

```yukino
public fun put(url_str: string, body: string) -> HttpResponse
public fun http_put(url_str: string, body: string) -> HttpResponse
```

Returns server `HttpResponse`.

Panics with `HttpError` on failure.

### 6.29 delete / http_delete

Sends an HTTP DELETE request.

```yukino
public fun delete(url_str: string) -> HttpResponse
public fun http_delete(url_str: string) -> HttpResponse
```

Returns server `HttpResponse`.

Panics with `HttpError` on failure.

### 6.30 patch / http_patch

Sends an HTTP PATCH request with body `body`.

```yukino
public fun patch(url_str: string, body: string) -> HttpResponse
public fun http_patch(url_str: string, body: string) -> HttpResponse
```

Returns server `HttpResponse`.

Panics with `HttpError` on failure.

### 6.31 head / http_head

Sends a lightweight HTTP HEAD request to check resource existence without downloading the body.

```yukino
public fun head(url_str: string) -> HttpResponse
public fun http_head(url_str: string) -> HttpResponse
```

Returns server `HttpResponse`.

Panics with `HttpError` on failure.

### 6.32 http_client_default

Creates a new `HttpClient` with a default 15 000 ms timeout and TLS certificate verification enabled.

```yukino
public fun http_client_default() -> HttpClient
```

Returns a configured `HttpClient`.

This function never panics.

---

[← Back: Module std.crypto](27-crypto.md) | [Next: Module std.regex →](29-regex.md)
