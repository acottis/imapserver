use std::net::{TcpListener, TcpStream};
//use native_tls::{Identity, TlsAcceptor, Tls}

mod stream;
use stream::Stream;

mod types;
use types::{Command, Response};

mod error;
use error::{Result, Error};

mod session;
use session::{UserSession};

mod email;

#[cfg(test)]
mod test;

static BIND_ADDRESS: &str = "0.0.0.0:143";
static MAX_BAD_ATTEMPTS: u8 = 3;

/// Main entry point, calls the TCP listener INIT [listen]
/// 
fn main() {
    listen().expect("Could not start Server");
}
/// Listens for incomming connections and then spawns a thread and deals with the transaction in [imap_main]
/// 
fn listen() -> Result<()> {
    println!("Starting IMAP Server...");
    let listener = TcpListener::bind(BIND_ADDRESS).map_err(Error::IO)?;
    println!("Listening on {}", listener.local_addr().map_err(Error::IO)?);

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                std::thread::spawn(|| -> Result<()> {
                    println!("Recieved connection from: {}", &s.peer_addr().map_err(Error::IO)?);
                    s.set_read_timeout(Some(std::time::Duration::from_secs(120))).unwrap();
                    s.set_write_timeout(Some(std::time::Duration::from_secs(15))).unwrap();
                    imap_main(s)?;
                    Ok(())
                });
            },
            Err(e) => panic!("Encountered IO error: {}", e),   
        }
    }
    Ok(())
}
///Function takes the network buffer from the client and splits it into parts
/// 
fn parse_response(res: String) -> Result<(Command, Option<String>, String)>{

    let tmp = res.replace("\r\n", "").to_owned();
    let mut split = tmp.splitn(3, " ");

    let tag = Some(split.next().unwrap().to_string());
    let cmd = Command::try_from(split.next().unwrap().to_string()).unwrap(); 
    let msg = split.next().unwrap_or("");

    Ok((cmd, tag, msg.to_owned()))
}
/// Main program Loop, imap logic is here
/// 
fn imap_main(stream: TcpStream) -> Result<()> {

    let mut stream: Stream = Stream::new(stream);

    let mut session = UserSession::new();

    stream.write(None, Response::Ok, "IMAP4 Service Ready.\r\n".into())?;

    loop{
        std::thread::sleep(std::time::Duration::from_millis(500));
        let res = stream.read()?;
        let (cmd, tag, msg) = parse_response(res)?;
        //println!("CMD: {:?}, TAG: {:?}, MSG: {}", cmd, tag, msg);

        match cmd {
            Command::Capability => {
                stream.write(None, Response::None, "CAPABILITY IMAP4 IMAP4rev1 AUTH=PLAIN\r\n".into())?;
                stream.write(tag, Response::Ok, "CAPABILITY completed.\r\n".into())?;
            }
            Command::Noop => {
                stream.write(tag, Response::Ok, "NOOP COMPLETED\r\n".into())?;
            }
            Command::Authenticate => {
                todo!();
            }
            Command::Login => {
                session.authenticate(&msg);
                println!("{:?}", session);
                if session.authenticated {
                    stream.write(tag, Response::Ok, "LOGIN completed.\r\n".into())?;
                    session.count_emails()?;
                }else{
                    stream.write(tag, Response::Bad, "LOGIN failed.\r\n".into())?;
                }
            }
            Command::List => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                let folders = session.list(&msg).unwrap();
                for folder in folders{
                    if folder == r#""""#{
                        stream.write(None, Response::None, format!("LIST (\\Noselect \\HasChildren) \"/\" {}\r\n", folder))?;
                    }else{
                        stream.write(None, Response::None, format!("LIST (\\Marked \\HasNoChildren) \"/\" {}\r\n", folder))?;
                    }
                }
                stream.write(tag, Response::Ok, "LIST completed.\r\n".into())?;
            }
            Command::Select => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                let mailbox = msg.split(" ").last().unwrap().to_uppercase().replace("\"", "");
                match mailbox.as_str() {
                    "INBOX" => {
                        stream.write(None, Response::None, format!("{} EXISTS\r\n", session.mail_count))?; // Number of mail items
                        stream.write(None, Response::None, format!("{} RECENT\r\n", session.mail_count))?; // Number of unread
                        //stream.write(None, Response::None, format!("{} RECENT\r\n", "0"))?; // Number of unread
                        stream.write(None, Response::None, "FLAGS (\\Seen \\Answered \\Flagged \\Deleted \\Draft)\r\n".into())?;
                        stream.write(None, Response::Ok, "[PERMANENTFLAGS (\\Seen \\Answered \\Flagged \\Deleted \\Draft)] Permanent flags\r\n".into())?;
                        stream.write(None, Response::Ok, "[UIDVALIDITY 1]\r\n".into())?;
                        stream.write(None, Response::Ok, format!("[UNSEEN {}]\r\n", session.mail_count))?;
                        //stream.write(None, Response::Ok, format!("[UIDNEXT {}] The next unique identifier value\r\n", session.mail_count))?;
                    }
                    _=>{
                        todo!()
                    }
                }
                stream.write(tag, Response::Ok, "[READ-WRITE] SELECT completed.\r\n".into())?;
            }
            Command::Lsub => {
                stream.write(tag, Response::Ok, "LSUB completed.\r\n".into())?;
            }
            Command::Status => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                todo!();
                // stream.write(None, Response::None, 
                //     format!("STATUS Inbox (UNSEEN 1 MESSAGES {} RECENT 1 UIDNEXT {} UIDVALIDITY 999)\r\n", session.mail_count, session.mail_count+1))?;
                // stream.write(tag, Response::Ok, "STATUS completed.\r\n".into())?;
            }
            Command::Fetch => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                // Client asking for full message
                let responses = session.fetch_seq(&msg).unwrap();
                for response in responses{
                    stream.write(None, Response::None, response)?;
                }
                stream.write(tag, Response::Ok, "FETCH completed.\r\n".into())?; 
            }
            Command::Create => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                stream.write(tag, Response::No, "Not implemented\r\n".into())?;
            }
            Command::Uid => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                let mut split = msg.splitn(2, " ");
                let cmd = split.next().unwrap();
                let msg = split.next().unwrap();
                match cmd {
                    "SEARCH" => {
                        let uids = session.search(&msg).unwrap();
                        let mut uid_string = String::new();
                        for uid in uids{
                            uid_string.push_str(&(uid.to_owned()+" "));
                        }
                        stream.write(None, Response::None, format!("SEARCH {}\r\n", uid_string))?;
                        stream.write(tag, Response::Ok, "Search completed\r\n".into())?;
                    }
                    "FETCH" => { 
                        let responses = session.fetch_uid(&msg).unwrap();
                        for response in responses{
                            stream.write(None, Response::None, response)?;
                        }
                        stream.write(tag, Response::Ok, "FETCH completed.\r\n".into())?; 
                    },
                    "COPY" => {
                        match session.copy(msg) {
                            Ok(_) => stream.write(tag, Response::Ok, "COPY Completed\r\n".into())?,
                            Err(e) => stream.write(tag, Response::No, format!("COPY error: {:?}\r\n",e))?,
                        }
                             
                    }
                    _ => {
                        stream.write(tag, Response::Ok, "FETCH Completed\r\n".into())?;
                    }
                }
            }
            Command::Subscribe => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                stream.write(tag, Response::Ok, "SUBSCRIBE Completed\r\n".into())?;
            }
            Command::Logout => {
                stream.write(None, Response::None, "BYE\r\n".into())?;
                stream.write(tag, Response::Ok, "LOGOUT completed.\r\n".into())?;
                break
            }
            _ => { 
                session.bad_attempts += 1;
                println!("{:?} found", cmd);
                stream.write(tag, Response::Bad, format!("Command Unrecognised, Attempts Remaining: {}\r\n", (MAX_BAD_ATTEMPTS - session.bad_attempts)))?;
                if session.bad_attempts > 3 { 
                    let _ = stream.shutdown();
                    break;
                }
            }
        } 
    }  
    Ok(())
}