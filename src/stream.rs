//! Wrapper around stream read and write that handle TCP and then TLS when it starts
//! 
use std::net::TcpStream;
use native_tls::{Identity, TlsAcceptor, TlsStream};
use std::io::{BufReader, BufRead, Write, Read};
use crate::error::{Result, Error};
use crate::types::Response;
use aml;

/// Struct for managing the reading and writing from TLS and TCP streams in a way that abstracts from the rest of the code
///
#[derive(Debug)] 
pub struct Stream{
    tcp_stream: TcpStream,
    tls_stream: Option<TlsStream<TcpStream>>,
}
impl Stream{
    /// Creates a stream object from a TCP Stream
    /// 
    pub fn new(tcp_stream: TcpStream) -> Self {
        Self {
            tcp_stream,
            tls_stream: None,
        }
    }
    /// Shuts down the TCP Stream
    /// 
    pub fn shutdown(&self) -> Result<()>{
        self.tcp_stream.shutdown(std::net::Shutdown::Both).map_err(Error::IO)
    }
    /// Returns Peer Address
    /// 
    pub fn peer_addr(&self) -> std::net::SocketAddr {
        self.tcp_stream.peer_addr().expect("Could not get peer IP Address")
    }
    /// This function reads a TCP stream until a CLRF `[13, 10]` is sent then collects into a [Vec]
    pub fn read(&mut self) -> Result<String> {
        
        let now = std::time::SystemTime::now();
        let mut reader = BufReader::new(Read::by_ref(&mut self.tcp_stream));
        let mut data: Vec<u8> = vec![];
        loop{
            let buffer = reader.fill_buf();      
            match buffer {
                Ok(bytes) => {
                    let length = bytes.len();
                    data.extend_from_slice(bytes); 
                    reader.consume(length);
                    // Okay checks for CLFR if more than one byte is in buffer
                    if (data.len() > 1) && (&data[data.len()-2..] == [13, 10]){
                        break;
                    }
                },
                _ => {}
            }
            if now.elapsed().unwrap() > std::time::Duration::from_secs(119) {
                return Err(Error::TCPReadTimeout)
            }      
        }
        //println!("Data from client: {:?}", data);
        let res = String::from_utf8_lossy(&data);
        print!("C: {}", res);
        Ok(res.to_string())
    }
    /// Wrapper around writing to TCP stream, handles the no whitespace requirement of the HELO response
    /// 
    pub fn write(&mut self, tag: Option<String>, response: Response, msg: String) -> Result<()> {

        let tag = tag.unwrap_or("*".to_owned());
        let res = match response{
            Response::None => format!("{} {}", tag, msg),
            _ => format!("{} {} {}", tag, response, msg),
        };
        print!("S: {}", res);
        //print!("{:?}", res.as_bytes());
        self.tcp_stream.write(res.as_bytes()).map_err(Error::IO)?;
    
        Ok(())
    }
    
    /// Takes a TCP stream and inits a TLS stream if successful
    /// 
    pub fn start_tls(&mut self) -> Result<()> {
        let mut file = std::fs::File::open("cert.pfx").unwrap();
        let config = aml::load("config.aml");
        let mut raw_cert = vec![];
        file.read_to_end(&mut raw_cert).unwrap();
        let identity = Identity::from_pkcs12(&raw_cert, config.get("cert_passphrase").unwrap()).unwrap();
        //let acceptor = TlsAcceptor::builder(identity).min_protocol_version(Some(native_tls::Protocol::Tlsv12)).build().unwrap();
        let acceptor = TlsAcceptor::new(identity).unwrap();
        let tls_stream = acceptor.accept(self.tcp_stream.try_clone().unwrap()).unwrap();
        self.tls_stream = Some(tls_stream);
        Ok(())
    }
}