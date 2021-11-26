use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, Read};
use std::io::{Write, BufRead};
use std::fs::metadata;
use chrono::{DateTime, Utc};
//use native_tls::{Identity, TlsAcceptor, Tls}

pub mod error;
use error::{Result, Error};

mod parse_email;
use parse_email::ParseEmail;

#[cfg(test)]
mod test;

static BIND_ADDRESS: &str = "0.0.0.0:143";
static MAX_BAD_ATTEMPTS: u8 = 3;

#[allow(dead_code)]
#[derive(PartialEq)]
enum Response{
    Continuation,
    Ok,
    No,
    Bad,
    None,
}

impl std::fmt::Display for Response{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result{
        let s = match self{
            Response::Ok => "OK",
            Response::Bad => "BAD",
            Response::Continuation => "+",
            Response::No => "NO",
            Response::None => "",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug)]
enum Command{
    Capability,
    Authenticate,
    Fetch,
    Unrecognised,
    Login,
    List,
    Select,
    Status,
    Logout,
    Noop,
    Subscribe,
    Uid,
    Create,
}

impl From<String> for Command{
    fn from(s: String) -> Self {
        let cmd = s.to_uppercase();
        match cmd.as_str() {
            "CAPABILITY" => Command::Capability,
            "AUTHENTICATE" => Command::Authenticate,
            "FETCH" => Command::Fetch,
            "LOGIN" => Command::Login,
            "LIST" => Command::List,
            "SELECT" => Command::Select,
            "STATUS" => Command::Status,
            "SUBSCRIBE" => Command::Subscribe,
            "NOOP" => Command::Noop,
            "LOGOUT" => Command::Logout,
            "UID" => Command::Uid,
            "CREATE" => Command::Create,
            _ => Command::Unrecognised,
        }   
    }
}

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

    let mut session = UserSession{
        email: None,
        username: None,
        mail_count: 0,
        authenticated: false,
        bad_attempts: 0,
    };

    write(&stream, None, Response::Ok, "IMAP4 Service Ready.\r\n".into())?;

    loop{
        std::thread::sleep(std::time::Duration::from_millis(500));
        let res = read(&stream)?;
        let (cmd, tag, msg) = parse_response(res)?;
        //println!("CMD: {:?}, TAG: {:?}, MSG: {}", cmd, tag, msg);

        match cmd {
            Command::Capability => {
                write(&stream, None, Response::None, "CAPABILITY IMAP4 IMAP4rev1 AUTH=PLAIN\r\n".into())?;
                write(&stream, tag, Response::Ok, "CAPABILITY completed.\r\n".into())?;
            }
            Command::Noop => {
                write(&stream, tag, Response::Ok, "NOOP COMPLETED\r\n".into())?;
            }
            Command::Authenticate => {
                todo!();
            }
            Command::Login => {
                session.authenticate(msg.to_owned());
                println!("{:?}", session);
                if session.authenticated {
                    write(&stream, tag, Response::Ok, "LOGIN completed.\r\n".into())?;
                    session.count_emails()?;
                }else{
                    write(&stream, tag, Response::Bad, "LOGIN failed.\r\n".into())?;
                }
            }
            Command::List => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                println!("{}", msg);
                if msg == "\"\" \"\""{
                    write(&stream, None, Response::None, "LIST (\\Noselect \\HasChildren) \"/\" \"\"\r\n".into())?;  
                    write(&stream, tag, Response::Ok, "LIST completed.\r\n".into())?;
                }else if msg == "\"\" \"*\""{       
                    write(&stream, None, Response::None, "LIST (\\Marked \\HasNoChildren) \"/\" Inbox\r\n".into())?;
                    write(&stream, None, Response::None, "LIST (\\HasNoChildren \\Drafts) \"/\" Drafts\r\n".into())?;
                    write(&stream, None, Response::None, "LIST (\\HasNoChildren \\Sent) \"/\" Sent\r\n".into())?;
                    write(&stream, None, Response::None, "LIST (\\Marked \\HasNoChildren \\Trash) \"/\" Deleted\r\n".into())?;
                    write(&stream, tag, Response::Ok, "LIST completed.\r\n".into())?;
                }else{
                    todo!();
                }
            }
            Command::Select => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                write(&stream, None, Response::None, format!("{} EXISTS\r\n", session.mail_count))?; // Number of mail items
                write(&stream, None, Response::None, format!("{} RECENT\r\n", session.mail_count))?; // Number of unread
                write(&stream, None, Response::None, "FLAGS (\\Seen \\Answered \\Flagged \\Deleted \\Draft)\r\n".into())?;
                write(&stream, None, Response::Ok, "[PERMANENTFLAGS (\\Seen \\Answered \\Flagged \\Deleted \\Draft)] Permanent flags\r\n".into())?;
                write(&stream, None, Response::Ok, "[UIDVALIDITY 0]\r\n".into())?;
                write(&stream, None, Response::Ok, format!("[UIDNEXT {}] The next unique identifier value\r\n", session.mail_count))?;
                write(&stream, tag, Response::Ok, "[READ-WRITE] SELECT completed.\r\n".into())?;
            }
            Command::Status => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                write(&stream, None, Response::None, 
                    format!("STATUS Inbox (UNSEEN 1 MESSAGES {} RECENT 1 UIDNEXT {} UIDVALIDITY 999)\r\n", session.mail_count, session.mail_count+1))?;
                write(&stream, tag, Response::Ok, "STATUS completed.\r\n".into())?;
            }
            Command::Fetch => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                write(&stream, None, Response::None, "1 FETCH (UID 1)\r\n".into())?;
                write(&stream, tag, Response::Ok, "FETCH completed\r\n".into())?;
            }
            Command::Create => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                write(&stream, tag, Response::Ok, "Create completed\r\n".into())?;
            }
            Command::Uid => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                let cmd = msg.split(" ").nth(0).unwrap();
                match cmd {
                    "SEARCH" => {
                        write(&stream, None, Response::None, "SEARCH 1\r\n".into())?;
                        write(&stream, tag, Response::Ok, "Search completed\r\n".into())?;
                    }
                    "FETCH" => {
                        // This is what the client intially asks for
                        if msg.contains("FLAGS") {
                            let res = fetch_info(msg).unwrap();
                            write(&stream, None, Response::None, res)?;
                            write(&stream, tag, Response::Ok, "FETCH completed.\r\n".into())?;
                        // Client asking for full message
                        }else if msg.contains("BODY.PEEK[]"){
                            let res = fetch_all(msg).unwrap();
                            write(&stream, None, Response::None, res)?;
                            write(&stream, tag, Response::Ok, "FETCH completed.\r\n".into())?;  
                        // Hack needs investigated
                        }else{
                            write(&stream, None, Response::None, "1 FETCH (UID 1 FLAGS (\\Recent))\r\n".into())?;
                            write(&stream, tag, Response::Ok, "FETCH completed.\r\n".into())?;  
                        }
                    }
                    "COPY" => {
                        match copy(&session, msg) {
                            Ok(_) => write(&stream, tag, Response::Ok, "COPY Completed\r\n".into())?,
                            Err(e) => write(&stream, tag, Response::No, "COPY error\r\n".into())?,
                        }
                             
                    }
                    _ => {
                        write(&stream, tag, Response::Ok, "FETCH Completed\r\n".into())?;
                    }
                }
            }
            Command::Subscribe => {
                if !session.authenticated { println!("Not Authenticated yet"); continue }
                write(&stream, tag, Response::Ok, "SUBSCRIBE Completed\r\n".into())?;
            }
            Command::Logout => {
                write(&stream, None, Response::None, "BYE\r\n".into())?;
                write(&stream, tag, Response::Ok, "LOGOUT completed.\r\n".into())?;
                break
            }
            _ => { 
                session.bad_attempts += 1;
                println!("{:?} found", cmd);
                write(&stream, tag, Response::Bad, format!("Command Unrecognised, Attempts Remaining: {}\r\n", (MAX_BAD_ATTEMPTS - session.bad_attempts)))?;
                if session.bad_attempts > 3 { 
                    let _ = &stream.shutdown(std::net::Shutdown::Both);
                    break;
                }
            }
        } 
    }  
    Ok(())
}

/// This function reads a TCP stream until a CLRF `[13, 10]` is sent then collects into a [Vec]
fn read<T>(stream: T) -> Result<String> where T: std::io::Read {
    
    let mut reader = BufReader::new(stream);
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
    }
    //println!("Data from client: {:?}", data);
    let res = String::from_utf8_lossy(&data);
    print!("C: {}", res);
    Ok(res.to_string())
}

fn write(mut stream: &TcpStream, tag: Option<String>, response: Response, msg: String) -> Result<()> {

    let tag = tag.unwrap_or("*".to_owned());
    let mut res: String;
    if response == Response::None{
        res = format!("{} {}", tag, msg);
    }else{
        res = format!("{} {} {}", tag, response, msg);
    }
    print!("S: {}", res);
    //print!("{:?}", res.as_bytes());
    stream.write(res.as_bytes()).map_err(Error::IO)?;

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

