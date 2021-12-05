use std::net::{TcpListener, TcpStream};
use std::io::{Read};
use std::fs::metadata;
use chrono::{DateTime, Utc};
//use native_tls::{Identity, TlsAcceptor, Tls}

mod stream;
use stream::Stream;

mod types;
use types::{Command, Response};

mod error;
use error::{Result, Error};

mod parse_email;
use parse_email::ParseEmail;

#[cfg(test)]
mod test;

static BIND_ADDRESS: &str = "0.0.0.0:143";
static MAX_BAD_ATTEMPTS: u8 = 3;

#[derive(Debug)]
struct UserSession{
    email: Option<String>,
    username: Option<String>,
    mail_count: usize,
    authenticated: bool,
    bad_attempts: u8,
}

/// This struct handles information about the connected session and methods that modify, controls access
/// 
impl UserSession{
    fn authenticate(&mut self, creds: String) {
        let tmp = creds.replace(&['\"', '\r','\n'][..], "");
        let mut sc = tmp.rsplit(" ");   
        let pass = sc.next().unwrap_or("");
        let user = sc.next().unwrap_or("");
        println!("User: {}, Password: {}", user, pass);
        self.email = Some(user.to_string());
        self.username = Some(user.split("@").next().unwrap().to_string());
        self.authenticated = true;
    }

    fn count_emails(&mut self) -> Result<()>{
        let user =  self.username.as_ref().ok_or(Error::FolderLookup("Username Invalid"))?;
        let path_str = &format!("mail/{}/Inbox", user);
        let path = std::path::Path::new(path_str);
        self.mail_count = std::fs::read_dir(path).map_err(Error::IO)?.count();
        Ok(())
    }
}
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
    // listener.set_nonblocking(true).expect("Cannot set non-blocking"); Dont need this?
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

    let mut session = UserSession{
        email: None,
        username: None,
        mail_count: 0,
        authenticated: false,
        bad_attempts: 0,
    };

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
                session.authenticate(msg.to_owned());
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
                println!("{}", msg);
                if msg == "\"\" \"\""{
                    stream.write(None, Response::None, "LIST (\\Noselect \\HasChildren) \"/\" \"\"\r\n".into())?;  
                    stream.write(tag, Response::Ok, "LIST completed.\r\n".into())?;
                }else if msg == "\"\" \"*\""{       
                    stream.write(None, Response::None, "LIST (\\Marked \\HasNoChildren) \"/\" Inbox\r\n".into())?;
                    stream.write(None, Response::None, "LIST (\\HasNoChildren \\Drafts) \"/\" Drafts\r\n".into())?;
                    stream.write(None, Response::None, "LIST (\\HasNoChildren \\Sent) \"/\" Sent\r\n".into())?;
                    stream.write(None, Response::None, "LIST (\\Marked \\HasNoChildren \\Trash) \"/\" Deleted\r\n".into())?;
                    stream.write(tag, Response::Ok, "LIST completed.\r\n".into())?;
                }else{
                    todo!();
                }
            }
            Command::Select => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                stream.write(None, Response::None, format!("{} EXISTS\r\n", session.mail_count))?; // Number of mail items
                stream.write(None, Response::None, format!("{} RECENT\r\n", session.mail_count))?; // Number of unread
                stream.write(None, Response::None, "FLAGS (\\Seen \\Answered \\Flagged \\Deleted \\Draft)\r\n".into())?;
                stream.write(None, Response::Ok, "[PERMANENTFLAGS (\\Seen \\Answered \\Flagged \\Deleted \\Draft)] Permanent flags\r\n".into())?;
                stream.write(None, Response::Ok, "[UIDVALIDITY 0]\r\n".into())?;
                stream.write(None, Response::Ok, format!("[UIDNEXT {}] The next unique identifier value\r\n", session.mail_count))?;
                stream.write(tag, Response::Ok, "[READ-WRITE] SELECT completed.\r\n".into())?;
            }
            Command::Status => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                stream.write(None, Response::None, 
                    format!("STATUS Inbox (UNSEEN 1 MESSAGES {} RECENT 1 UIDNEXT {} UIDVALIDITY 999)\r\n", session.mail_count, session.mail_count+1))?;
                stream.write(tag, Response::Ok, "STATUS completed.\r\n".into())?;
            }
            Command::Fetch => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                stream.write(None, Response::None, "1 FETCH (UID 1)\r\n".into())?;
                stream.write(tag, Response::Ok, "FETCH completed\r\n".into())?;
            }
            Command::Create => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                stream.write(tag, Response::Ok, "Create completed\r\n".into())?;
            }
            Command::Uid => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                let cmd = msg.split(" ").nth(0).unwrap();
                match cmd {
                    "SEARCH" => {
                        stream.write(None, Response::None, "SEARCH 1\r\n".into())?;
                        stream.write(tag, Response::Ok, "Search completed\r\n".into())?;
                    }
                    "FETCH" => {
                        // This is what the client intially asks for
                        if msg.contains("FLAGS") {
                            let res = fetch_info(msg).unwrap();
                            stream.write(None, Response::None, res)?;
                            stream.write(tag, Response::Ok, "FETCH completed.\r\n".into())?;
                        // Client asking for full message
                        }else if msg.contains("BODY.PEEK[]"){
                            let res = fetch_all(msg).unwrap();
                            stream.write(None, Response::None, res)?;
                            stream.write(tag, Response::Ok, "FETCH completed.\r\n".into())?;  
                        // Hack needs investigated
                        }else{
                            stream.write(None, Response::None, "1 FETCH (UID 1 FLAGS (\\Recent))\r\n".into())?;
                            stream.write(tag, Response::Ok, "FETCH completed.\r\n".into())?;  
                        }
                    }
                    "COPY" => {
                        match copy(&session, msg) {
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


fn copy(session: &UserSession, msg: String) -> Result<()>{
    Err(Error::IMAPCopyErr)
}

fn fetch_all(msg: String) -> Result<String>{
    let mut b = vec![];
    let mut f = std::fs::File::open("mail/test/inbox/test.eml").unwrap();
    f.read_to_end(&mut b).unwrap();
    let res = format!("1 FETCH (BODY[] {{{}}}\r\n{}\r\n)\r\n", 
        b.len()+2, String::from_utf8(b).map_err(Error::UTF8)?);
    Ok(res)
}

fn fetch_info(msg: String) -> Result<String>{
    let parser: ParseEmail = ParseEmail::new("mail/test/inbox/test.eml")?;
    let (to_user, to_domain, to_display_name) = parser.to_header()?;
    let (from_user, from_domain, from_display_name) = parser.from_header()?;
    let date = parser.date_header()?;
    let subject = parser.subject_header()?;
    let internal_date = internal_date("mail/test/inbox/test.eml")?;

    // TODO FIELDS
    //let message_id = "<CADkb2rHmFeg1D01=a=n9xFZ5LxqH5FsWHT_dPxMyK7O4v1EKUA@mail.gmail.com>";
    let message_id = "NIL";
    let data = ""; // This is what the bytes refer to, the new line adds +2 though
    let seq_num = 1;
    let uid = 1;

    let bytes = data.len()+2;

    Ok(format!(
        "{seq_num} FETCH (UID {uid} FLAGS (\\RECENT) ENVELOPE (\"{date}\" \"{subject}\" \
        ((\"{from_display_name}\" NIL \"{from_user}\" \"{from_domain}\")) NIL NIL ((\"{to_display_name}\" NIL \
        \"{to_user}\" \"{to_domain}\")) NIL NIL NIL {message_id}) INTERNALDATE \"{internal_date}\" \
        BODY[HEADER.FIELDS (References)] {{{bytes}}}\r\n{data}\r\n)\r\n",
        seq_num = seq_num,
        uid = uid,
        date = date,
        subject = subject,
        from_display_name = from_display_name,
        from_user = from_user,
        from_domain = from_domain,
        to_display_name = to_display_name,
        to_user = to_user,
        to_domain = to_domain,
        message_id = message_id,
        internal_date = internal_date,
        bytes = bytes,
        data = data,
    ))
}

/// Takes the creation date of the file as the creation date and converts to an IMAP friendly format [String]
/// 
fn internal_date(path: impl AsRef<std::path::Path>) -> Result<String> {
    let metadata = metadata(path).map_err(Error::IO)?;
    let created_date = metadata.created().map_err(Error::IO)?;
    let dt: DateTime<Utc> = created_date.into();
    Ok(dt.format("%Y-%b-%d %H:%M:%S %z").to_string())
}

