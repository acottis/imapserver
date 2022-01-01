/// This struct is responsible for opening a email file from the file system then picking out the data that IMAP requires
/// 
use regex::Regex;
use std::path::{PathBuf, Path};
use std::io::Read;
use chrono::{DateTime, Utc};
use std::fs;
use crate::error::{Result,Error};

static MAIL_ROOT: &'static str = "D:/MAILSERVER";
/// Struct containing the email contents
///
#[derive(Debug)] 
pub struct Email{
    uid: String,
    seq: String,
    email_contents: String,
    path: PathBuf,
}

impl Email{
    /// Creates a new struct and gets the contents of an email and stores as a [String]
    /// 
    pub fn new(uid: &str, seq: &str, email_path: impl AsRef<Path>) -> Result<Self> {
        let mut buf = vec![];
        let mut f = fs::File::open(&email_path).map_err(Error::IO)?;
        f.read_to_end(&mut buf).map_err(Error::IO)?;
        let email = String::from_utf8(buf).map_err(Error::UTF8)?;

        Ok(Self{
            uid: uid.to_owned(),
            seq: seq.to_owned(),
            email_contents: email,
            path: PathBuf::from(email_path.as_ref()),
        })
    }
    /// Parses the to field of an email
    /// 
    pub fn to_header(&self) -> Result<(String, String, String)> {

        let to = Regex::new(r"(?mi)^TO: (.*)\r").unwrap();

        let find_to = to.captures(&self.email_contents)
            .ok_or(Error::ToFieldMissing)?;
        let line = find_to.get(1).ok_or(Error::ToFieldMissing)?
            .as_str().replace(&['<','>'][..], "");

        let mut split = line.rsplitn(2, " "); // Remove To: and split email and displayname 

        let mut tmp = split.next().unwrap().split("@"); // Splits email address into user and domain
        let to_user = tmp.next().unwrap().to_owned();
        let to_domain = tmp.next().unwrap().to_owned();
        let to_display_name = split.next().unwrap_or("NIL").to_owned(); // Gets displayname if exists

        Ok((to_user, to_domain, to_display_name))
    }
    /// Parses the from field of an email
    /// 
    pub fn from_header(&self) -> Result<(String, String, String)> {

        let to = Regex::new(r"(?mi)^FROM:.*").unwrap();

        let find_to = to.find(&self.email_contents);
        let line = find_to.map_or("", |m| m.as_str()).replace(&['<','>','\r'][..], "");
        let mut split = line.splitn(2, " ").last().unwrap().rsplitn(2, " "); // Remove To: and split email and displayname 

        let mut tmp = split.next().unwrap().split("@"); // Splits email address into user and domain
        let to_user = tmp.next().unwrap_or("").to_owned();
        let to_domain = tmp.next().unwrap_or("").to_owned();
        let to_display_name = split.next().unwrap_or("NIL").to_owned(); // Gets displayname if exists

        Ok((to_user, to_domain, to_display_name))
    }
    /// Parses the from field of an email and returns it as rfc2822 format
    /// 
    pub fn date_header(&self) -> Result<String> {
    
        let to = Regex::new("(?mi)^DATE:.*").unwrap();
        let find_to = to.find(&self.email_contents);
    
        let line = find_to.map_or("", |m| m.as_str()).replace(&['\r'][..], "");
        let date_string = line.splitn(2, " ").last().unwrap().to_owned(); 
    
        Ok(date_string)
    }
    /// Parses the subject field of an email and returns it
    /// 
    pub fn subject_header(&self) -> Result<String> {
    
        let to = Regex::new(r"(?mi)^SUBJECT:.*").unwrap();
        let find_to = to.find(&self.email_contents);
    
        let line = find_to.map_or("", |m| m.as_str()).replace(&['\r'][..], "");
        let subject = line.splitn(2, " ").last().unwrap().to_owned(); 
    
        Ok(subject)
    }
    /// Takes the creation date of the file as the creation date and converts to an IMAP friendly format [String]
    /// 
    fn internal_date(&self) -> Result<String> {
        let metadata = fs::metadata(&self.path).map_err(Error::IO)?;
        let created_date = metadata.created().map_err(Error::IO)?;
        let dt: DateTime<Utc> = created_date.into();
        Ok(dt.format("%Y-%b-%d %H:%M:%S %z").to_string())
    }
    /// Format a response with just the UID
    /// 
    pub fn format_response(&self, args: &str) -> String{
        //println!("Args: {}", args);

        match args{
            "(UID)" => format!("{} FETCH (UID {})\r\n", self.seq, self.uid),

            "(UID FLAGS RFC822.SIZE BODY.PEEK[] INTERNALDATE)" => {
                self.fetch_uid_flags_RFC822_bodypeek().unwrap()
            },

            "(UID FLAGS)" => { self.fetch_uid_flags().unwrap() },
            _ => {todo!()}
        }
    }

    pub fn fetch_uid_flags(&self) -> Result<String>{
        let data = ""; // This is what the bytes refer to, the new line adds +2 though
        let seq_num = &self.seq;
        let uid = &self.uid;

        let bytes = data.len()+2;
        Ok(format!(
            "{seq_num} FETCH (UID {uid} FLAGS (\\Seen))\r\n",
            seq_num = seq_num,
            uid = uid,
        ))

    }

    // pub fn fetch_info(&self) -> Result<String>{
    //     let (to_user, to_domain, to_display_name) = self.to_header()?;
    //     let (from_user, from_domain, from_display_name) = self.from_header()?;
    //     let date = self.date_header()?;
    //     let subject = self.subject_header()?;
    //     let internal_date = self.internal_date()?;

    //     // TODO FIELDS
    //     //let message_id = "<CADkb2rHmFeg1D01=a=n9xFZ5LxqH5FsWHT_dPxMyK7O4v1EKUA@mail.gmail.com>";
    //     let message_id = "NIL";
    //     let data = ""; // This is what the bytes refer to, the new line adds +2 though
    //     let seq_num = &self.seq;
    //     let uid = &self.uid;

    //     let bytes = data.len()+2;
    //     Ok(format!(
    //         "{seq_num} FETCH (UID {uid} FLAGS (\\RECENT) ENVELOPE (\"{date}\" \"{subject}\" \
    //         ((\"{from_display_name}\" NIL \"{from_user}\" \"{from_domain}\")) NIL NIL ((\"{to_display_name}\" NIL \
    //         \"{to_user}\" \"{to_domain}\")) NIL NIL NIL {message_id}) INTERNALDATE \"{internal_date}\" \
    //         BODY[HEADER.FIELDS (References)] {{{bytes}}}\r\n{data}\r\n)\r\n",
    //         seq_num = seq_num,
    //         uid = uid,
    //         date = date,
    //         subject = subject,
    //         from_display_name = from_display_name,
    //         from_user = from_user,
    //         from_domain = from_domain,
    //         to_display_name = to_display_name,
    //         to_user = to_user,
    //         to_domain = to_domain,
    //         message_id = message_id,
    //         internal_date = internal_date,
    //         bytes = bytes,
    //         data = data,
    //     ))
    // }

    // (UID FLAGS RFC822.SIZE BODY.PEEK[] INTERNALDATE)
    pub fn fetch_uid_flags_RFC822_bodypeek(&self) -> Result<String>{
        let internal_date = self.internal_date()?;
        let data = &self.email_contents; // This is what the bytes refer to, the new line adds +2 though
        let seq_num = &self.seq;
        let uid = &self.uid;

        let bytes = data.len()+2;
        Ok(format!(
            "{seq_num} FETCH (UID {uid} FLAGS (\\RECENT) RFC822.SIZE 53000 BODY[] {{{bytes}}}\r\n\
            {data}\r\n INTERNALDATE \"{internal_date}\")\r\n",
            seq_num = seq_num,
            uid = uid,
            internal_date = internal_date,
            bytes = bytes,
            data = data,
        ))
    }
}

#[test]
fn test_internal_date(){
    let email = Email::new("1", "1", "test_emails/NoDisplayNames.eml").unwrap();
    let date = email.internal_date().unwrap();
    assert_eq!(date, "2021-Nov-23 11:26:52 +0000");
}

#[test]
fn test_to_header_displayname(){
    let email = Email::new("1", "1", "test_emails/DisplayNames.eml").unwrap();
    let (user,domain,display_name) = email.to_header().expect("Failed to Header");

    assert_eq!(user, "adam.test");
    assert_eq!(domain, "example.scot");
    assert_eq!(display_name, "Adam Test");
}
#[test]
fn test_to_header_no_displayname(){
    let email = Email::new("1", "1", "test_emails/NoDisplayNames.eml").unwrap();
    let (user,domain,display_name) = email.to_header().expect("Failed to Header");

    assert_eq!(user, "adam.test");
    assert_eq!(domain, "example.scot");
    assert_eq!(display_name, "NIL");
}