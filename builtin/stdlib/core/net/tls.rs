use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::{
    ClientConfig, ClientConnection, DigitallySignedStruct, RootCertStore, ServerConfig,
    ServerConnection, SignatureScheme, StreamOwned,
};

use super::error::{clear_last_error, record_io_error, set_last_error, set_net_str_result};
use super::tcp::get_tcp_stream;

#[derive(Debug)]
struct InsecureCertVerifier;

impl ServerCertVerifier for InsecureCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer,
        _intermediates: &[CertificateDer],
        _server_name: &ServerName,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

pub enum TlsStreamWrapper {
    Client(StreamOwned<ClientConnection, TcpStream>),
    Server(StreamOwned<ServerConnection, TcpStream>),
}

impl Read for TlsStreamWrapper {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            TlsStreamWrapper::Client(s) => s.read(buf),
            TlsStreamWrapper::Server(s) => s.read(buf),
        }
    }
}

impl Write for TlsStreamWrapper {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            TlsStreamWrapper::Client(s) => s.write(buf),
            TlsStreamWrapper::Server(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            TlsStreamWrapper::Client(s) => s.flush(),
            TlsStreamWrapper::Server(s) => s.flush(),
        }
    }
}

static NEXT_TLS_ID: AtomicU64 = AtomicU64::new(1);
static TLS_SESSIONS: Mutex<Option<HashMap<u64, Arc<Mutex<TlsStreamWrapper>>>>> = Mutex::new(None);

fn get_tls_session(id: u64) -> Option<Arc<Mutex<TlsStreamWrapper>>> {
    let guard = match TLS_SESSIONS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.as_ref().and_then(|map| map.get(&id).cloned())
}

fn parse_str<'a>(p: *const u8, len: i64) -> Result<&'a str, i32> {
    if p.is_null() || len <= 0 {
        set_last_error(11, "null or empty string pointer".to_string());
        return Err(11);
    }
    let slice = unsafe { std::slice::from_raw_parts(p, len as usize) };
    std::str::from_utf8(slice).map_err(|e| {
        set_last_error(12, format!("invalid utf-8 argument: {e}"));
        12
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tls_client_connect(
    tcp_stream_id: u64,
    host_p: *const u8,
    host_len: i64,
    insecure_skip_verify: bool,
) -> u64 {
    clear_last_error();
    let host_str = match parse_str(host_p, host_len) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let tcp_arc = match get_tcp_stream(tcp_stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {tcp_stream_id}"));
            return 0;
        }
    };
    let cloned_tcp = match tcp_arc.try_clone() {
        Ok(t) => t,
        Err(e) => {
            record_io_error(e);
            return 0;
        }
    };
    let client_config = if insecure_skip_verify {
        ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(InsecureCertVerifier))
            .with_no_client_auth()
    } else {
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth()
    };
    let server_name = match ServerName::try_from(host_str.to_string()) {
        Ok(sn) => sn,
        Err(e) => {
            set_last_error(11, format!("invalid server name '{host_str}': {e}"));
            return 0;
        }
    };
    let conn = match ClientConnection::new(Arc::new(client_config), server_name) {
        Ok(c) => c,
        Err(e) => {
            set_last_error(20, format!("tls client connection error: {e}"));
            return 0;
        }
    };
    let stream_owned = StreamOwned::new(conn, cloned_tcp);
    let wrapper = TlsStreamWrapper::Client(stream_owned);
    let id = NEXT_TLS_ID.fetch_add(1, Ordering::Relaxed);
    let mut guard = match TLS_SESSIONS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.get_or_insert_with(HashMap::new).insert(id, Arc::new(Mutex::new(wrapper)));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tls_server_accept(
    tcp_stream_id: u64,
    cert_pem_p: *const u8,
    cert_pem_len: i64,
    key_pem_p: *const u8,
    key_pem_len: i64,
) -> u64 {
    clear_last_error();
    let cert_str = match parse_str(cert_pem_p, cert_pem_len) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let key_str = match parse_str(key_pem_p, key_pem_len) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let tcp_arc = match get_tcp_stream(tcp_stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {tcp_stream_id}"));
            return 0;
        }
    };
    let cloned_tcp = match tcp_arc.try_clone() {
        Ok(t) => t,
        Err(e) => {
            record_io_error(e);
            return 0;
        }
    };
    let mut cert_reader = cert_str.as_bytes();
    let certs: Vec<CertificateDer> = match rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, _>>() {
        Ok(c) if !c.is_empty() => c,
        Ok(_) => {
            set_last_error(11, "no certificates found in cert_pem".to_string());
            return 0;
        }
        Err(e) => {
            record_io_error(e);
            return 0;
        }
    };
    let mut key_reader = key_str.as_bytes();
    let key: PrivateKeyDer = match rustls_pemfile::private_key(&mut key_reader) {
        Ok(Some(k)) => k,
        Ok(None) => {
            set_last_error(11, "no private key found in key_pem".to_string());
            return 0;
        }
        Err(e) => {
            record_io_error(e);
            return 0;
        }
    };
    let server_config = match ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
    {
        Ok(cfg) => cfg,
        Err(e) => {
            set_last_error(20, format!("invalid tls server configuration: {e}"));
            return 0;
        }
    };
    let conn = match ServerConnection::new(Arc::new(server_config)) {
        Ok(c) => c,
        Err(e) => {
            set_last_error(20, format!("tls server connection error: {e}"));
            return 0;
        }
    };
    let stream_owned = StreamOwned::new(conn, cloned_tcp);
    let wrapper = TlsStreamWrapper::Server(stream_owned);
    let id = NEXT_TLS_ID.fetch_add(1, Ordering::Relaxed);
    let mut guard = match TLS_SESSIONS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.get_or_insert_with(HashMap::new).insert(id, Arc::new(Mutex::new(wrapper)));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tls_read_text(
    tls_id: u64,
    max_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let session = match get_tls_session(tls_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tls session id {tls_id}"));
            return std::ptr::null();
        }
    };
    let cap = if max_len <= 0 { 65536 } else { max_len as usize };
    let mut buf = vec![0u8; cap];
    let mut guard = match session.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    match guard.read(&mut buf) {
        Ok(0) => {
            unsafe {
                if !out_len.is_null() {
                    *out_len = 0;
                }
            }
            b"".as_ptr()
        }
        Ok(n) => {
            buf.truncate(n);
            set_net_str_result(&buf, out_len)
        }
        Err(e) => {
            record_io_error(e);
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tls_read(tls_id: u64, buf_p: *mut u8, max_len: i64) -> i64 {
    clear_last_error();
    if buf_p.is_null() || max_len <= 0 {
        set_last_error(11, "invalid buffer pointer or max_len <= 0".to_string());
        return -1;
    }
    let session = match get_tls_session(tls_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tls session id {tls_id}"));
            return -1;
        }
    };
    let mut guard = match session.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let slice = unsafe { std::slice::from_raw_parts_mut(buf_p, max_len as usize) };
    match guard.read(slice) {
        Ok(n) => n as i64,
        Err(e) => {
            record_io_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tls_write(tls_id: u64, buf_p: *const u8, len: i64) -> i64 {
    clear_last_error();
    if buf_p.is_null() || len < 0 {
        set_last_error(11, "invalid buffer pointer or len < 0".to_string());
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let session = match get_tls_session(tls_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tls session id {tls_id}"));
            return -1;
        }
    };
    let mut guard = match session.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let slice = unsafe { std::slice::from_raw_parts(buf_p, len as usize) };
    match guard.write(slice) {
        Ok(n) => n as i64,
        Err(e) => {
            record_io_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tls_flush(tls_id: u64) -> bool {
    clear_last_error();
    let session = match get_tls_session(tls_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tls session id {tls_id}"));
            return false;
        }
    };
    let mut guard = match session.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    match guard.flush() {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tls_close(tls_id: u64) -> bool {
    clear_last_error();
    let mut guard = match TLS_SESSIONS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&tls_id).is_some()
    } else {
        false
    }
}
