use std::io::{Read};
use std::fs::metadata;
use chrono::{DateTime, Utc};
use crate::error::{Result, Error};
use crate::parse_email::ParseEmail;


#[derive(Debug, Default)]
pub struct UserSession{
    email: Option<String>,
    username: Option<String>,
    pub mail_count: usize,
    pub authenticated: bool,
    pub bad_attempts: u8,
}

/// This struct handles information about the connected session and methods that modify, controls access
/// 
impl UserSession{

    pub fn new() -> Self{
        Self::default()
    }

    pub fn authenticate(&mut self, creds: String) {
        let tmp = creds.replace(&['\"', '\r','\n'][..], "");
        let mut sc = tmp.rsplit(" ");   
        let pass = sc.next().unwrap_or("");
        let user = sc.next().unwrap_or("");
        println!("User: {}, Password: {}", user, pass);
        self.email = Some(user.to_string());
        self.username = Some(user.split("@").next().unwrap().to_string());
        self.authenticated = true;
    }

    pub fn count_emails(&mut self) -> Result<()>{
        let user =  self.username.as_ref().ok_or(Error::FolderLookup("Username Invalid"))?;
        let path_str = &format!("mail/{}/Inbox", user);
        let path = std::path::Path::new(path_str);
        self.mail_count = std::fs::read_dir(path).map_err(Error::IO)?.count();
        Ok(())
    }


    pub fn copy(&self, msg: String) -> Result<()>{
        Err(Error::IMAPCopyErr)
    }

    pub fn fetch_all(&self, msg: String) -> Result<String>{
        let mut b = vec![];
        let mut f = std::fs::File::open("mail/test/inbox/test.eml").unwrap();
        f.read_to_end(&mut b).unwrap();
        let res = format!("1 FETCH (BODY[] {{{}}}\r\n{}\r\n)\r\n", 
            b.len()+2, String::from_utf8(b).map_err(Error::UTF8)?);
        Ok(res)
    }

    pub fn fetch_info(&self, msg: String) -> Result<String>{
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
}
/// Takes the creation date of the file as the creation date and converts to an IMAP friendly format [String]
/// 
fn internal_date(path: impl AsRef<std::path::Path>) -> Result<String> {
    let metadata = metadata(path).map_err(Error::IO)?;
    let created_date = metadata.created().map_err(Error::IO)?;
    let dt: DateTime<Utc> = created_date.into();
    Ok(dt.format("%Y-%b-%d %H:%M:%S %z").to_string())
}

#[test]
fn test_internal_date(){

    let date = internal_date("test_emails/NoDisplayNames.eml").unwrap();
    assert_eq!(date, "2021-Nov-23 11:26:52 +0000");
}