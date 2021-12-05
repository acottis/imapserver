use std::io::{Read};
use std::fs;
use chrono::{DateTime, Utc};
use crate::error::{Result, Error};
use crate::email::Email;

static MAIL_ROOT: &'static str = "D:/MAILSERVER";

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
    /// Authenticates
    pub fn authenticate(&mut self, creds: &str) {
        let tmp = creds.replace(&['\"', '\r','\n'][..], "");
        let mut sc = tmp.rsplit(" ");   
        let pass = sc.next().unwrap_or("");
        let user = sc.next().unwrap_or("");
        println!("User: {}, Password: {}", user, pass);
        self.email = Some(user.to_string());
        self.username = Some(user.split("@").next().unwrap().to_string());
        self.authenticated = true;
    }
    /// Counts number of emails in Inbox
    /// 
    pub fn count_emails(&mut self) -> Result<()>{
        let user =  self.username.as_ref().ok_or(Error::FolderLookup("Username Invalid"))?;
        let path_str = &format!("{}/mail/{}/Inbox", MAIL_ROOT ,user);
        let path = std::path::Path::new(path_str);
        self.mail_count = std::fs::read_dir(path).map_err(Error::IO)?.count();
        Ok(())
    }
    /// Copy, not implemented
    pub fn copy(&self, msg: &str) -> Result<()>{
        todo!();
        //Err(Error::IMAPCopyErr)
    }
    /// Lists the folders in the mailbox for SELECT queries
    /// 
    pub fn list(&self, msg: &str) -> Result<Vec<String>>{
        let folders = self.get_folders().unwrap();
        let arg = msg.rsplit(r#" "#).next().unwrap().replace("\"", "");
        if folders.contains(&arg){
            return Ok(vec![arg])
        }
        if arg == ""{
            return Ok(vec![r#""""#.into()])
        }
        else{
            return Ok(folders)
        }
    }
    ///Gets all folders and puts them into array
    /// 
    fn get_folders(&self) -> Result<Vec<String>>{

        let path = format!("{}/mail/{}", MAIL_ROOT, self.username.as_ref().unwrap());
        let mut dir = fs::read_dir(path).unwrap();

        let mut folders: Vec<String> = vec![];
        while let Some(f) = dir.next(){
            let file = f.unwrap();
            if file.metadata().unwrap().is_dir() {
                let name = file.file_name().into_string().unwrap();
                folders.push(name);
            }
        }
        Ok(folders)
    }
    /// Takes the creation date of the file as the creation date and converts to an IMAP friendly format [String]
    /// 
    fn internal_date(&self, path: impl AsRef<std::path::Path>) -> Result<String> {
        let metadata = fs::metadata(path).map_err(Error::IO)?;
        let created_date = metadata.created().map_err(Error::IO)?;
        let dt: DateTime<Utc> = created_date.into();
        Ok(dt.format("%Y-%b-%d %H:%M:%S %z").to_string())
    }
    /// Fetch (Non UID version)
    /// 
    pub fn fetch_seq(&self, msg: &str) -> Result<Vec<Email>>{
        // Holding Vector for result
        let mut responses: Vec<Email> = Vec::new();

        // Split it into two parts, sequence number(s) and args
        let mut split = msg.splitn(2, " ");
        let seq = split.next().unwrap();
        let args = split.next();

        // Get the files in the Inbox
        let path = format!("{}/mail/{}/Inbox", MAIL_ROOT, self.username.as_ref().unwrap());
        let dir = fs::read_dir(path).unwrap();

        if seq.contains(",") {
            // Look up Vector
            responses.append(&mut self.fetch_list(seq, dir).unwrap());
        }
        else if seq.contains(":") {
            // Lookup Range
            responses.append(&mut self.fetch_range(seq, dir).unwrap());
        }
        else{
            // Lookup one thing
            responses.push(self.fetch_one(seq, dir).unwrap());
        }

        match args{
            _ => {}
        };

        // Parse Args
        Ok(responses)
    }
    /// Fetch one from Sequence number
    /// 
    fn fetch_one(&self, seq: &str, mut dir: fs::ReadDir) -> Result<Email> {
        let seq_num: usize = seq.parse().unwrap();
        let file = dir.nth(seq_num-1).unwrap().unwrap();
        let uid = self.filename_to_uid(&file);
        Ok(Email::new(&uid, &seq, file.path()).unwrap())
    }
    /// Fetch Range from Sequence number
    ///
    fn fetch_range(&self, seq: &str, dir: fs::ReadDir) -> Result<Vec<Email>> {
        let mut res: Vec<Email> = Vec::new();

        let mut split = seq.splitn(2, ":");
        let from = split.next().unwrap().parse().unwrap();
        let to = split.next().unwrap().parse().unwrap();

        for (index, file) in dir.enumerate(){
            let index = index + 1;
            if index < from { continue };
            if index > to { break };
            
            let uid = self.filename_to_uid(&file.as_ref().unwrap());
            res.push(Email::new(&uid, &index.to_string(), file.unwrap().path()).unwrap());
        }
        Ok(res)
    }
    /// Fetch list from Sequence number
    ///
    fn fetch_list(&self, seq: &str, dir: fs::ReadDir) -> Result<Vec<Email>> {
        let mut res: Vec<Email> = Vec::new();
        let mut seqs: Vec<String> = Vec::new();

        let mut seq_list = seq.splitn(2, " ").next().unwrap().split(",");
        let seq_count = seq_list.clone().count();
        
        while let Some(seq) = seq_list.next(){   
            seqs.push(seq.into());
        }

        for (index, file) in dir.enumerate(){
            let index = index + 1;
            if res.len() >= seq_count { break };
            if !seqs.contains(&index.to_string()) { continue };
            
            let uid = self.filename_to_uid(&file.as_ref().unwrap());
            res.push(Email::new(&uid, &index.to_string(), file.as_ref().unwrap().path()).unwrap());
        }
        Ok(res)
    }
    /// Converts a DirEnty to its UID using the filename which is based on date
    fn filename_to_uid(&self, file: &fs::DirEntry) -> String{
        let mut filename = file.file_name().into_string().unwrap();
        let shorter_filename = filename.split_off(1);
        let uid_seed = shorter_filename.split("s.eml").next().unwrap().parse::<f64>().unwrap();
        ((uid_seed * 10f64) as usize).to_string()
    }
}

#[test]
fn test_internal_date(){
    let session = UserSession::new();
    let date = session.internal_date("test_emails/NoDisplayNames.eml").unwrap();
    assert_eq!(date, "2021-Nov-23 11:26:52 +0000");
}
#[test]
fn fetch_seq_single(){
    let mut session = UserSession::new();
    session.authenticate("\"test@ashdown.scot\" tset");
    
    let res = session.fetch_seq("9 (UID)").unwrap();
    println!("{:#?}", res);
}
#[test]
fn fetch_seq_range(){
    let mut session = UserSession::new();
    session.authenticate("\"test@ashdown.scot\" tset");
    
    let res = session.fetch_seq("1:7 (UID)").unwrap();
    println!("{:#?}", res);
}
#[test]
fn fetch_seq_list(){
    let mut session = UserSession::new();
    session.authenticate("\"test@ashdown.scot\" tset");
    
    let res = session.fetch_seq("1,2,4,5 (UID)").unwrap();
    println!("{:#?}", res);
}
// #[test]
// fn test_fetch_info_range(){
//     let mut session = UserSession::new();
//     session.authenticate("\"test@ashdown.scot\" tset");

//     println!("{:#?}", session.fetch_info_from_seq("3:6 (UID FLAGS)").unwrap());
// }
// #[test]
// fn test_fetch_info_single(){
//     let mut session = UserSession::new();
//     session.authenticate("\"test@ashdown.scot\" tset");

//     println!("{:#?}", session.fetch_info_from_seq("1 (UID FLAGS)").unwrap());
// }
// #[test]
// fn test_fetch_all(){
//     let mut session = UserSession::new();
//     session.authenticate("\"test@ashdown.scot\" tset");

//     println!("{:#?}", session.fetch_info_from_seq("1 BODY.PEEK[]").unwrap());
// }
// #[test]
// fn test_list(){
//     let mut session = UserSession::new();
//     session.authenticate("\"test@ashdown.scot\" tset");

//     assert_eq!(session.list(r#"LIST "" "Inbox""#).unwrap(), vec!["Inbox".to_owned()])

// }
// #[test]
// fn test_get_uid(){
//     let mut session = UserSession::new();
//     session.authenticate("\"test@ashdown.scot\" tset");

//     println!("{:?}", session.get_uid(9).unwrap());
//     //assert_eq!(session.get_uid(r#"9 (UID)"#).unwrap(), 2);

// }
// #[test]
// fn test_fetch_parse(){
//     let mut session = UserSession::new();
//     session.authenticate("\"test@ashdown.scot\" tset");
//     session.get_uids_from_fetch("7570,8057,399641,448990,943731,946272,963704,970680 (UID FLAGS RFC822.SIZE BODY.PEEK[] INTERNALDATE)").unwrap();
// }
// #[test]
// fn test_fetching_uids(){
//     let mut session = UserSession::new();
//     session.authenticate("\"test@ashdown.scot\" tset");
//     let test = session.get_uids_from_fetch("7570,8057,399641,448990,943731,946272,963704,970680 (UID FLAGS RFC822.SIZE BODY.PEEK[] INTERNALDATE)").unwrap();
//     println!("{:?}", session.fetch_info_from_uid(test));
// }














    // pub fn fetch_all(&self, msg: &str) -> Result<String>{
    //     println!("Message: {}", msg);
    //     let sequence: usize = msg.splitn(2, " ").next().unwrap().parse().unwrap();

    //     let dir = format!("{}/mail/{}/inbox", MAIL_ROOT, self.username.as_ref().unwrap());
    //     let email = std::fs::read_dir(dir).unwrap().nth(sequence-1).unwrap().unwrap();
    //     println!("{:?}", email);

    //     let mut b = vec![];
    //     let mut f = std::fs::File::open(email.path()).unwrap();
    //     f.read_to_end(&mut b).unwrap();
    //     let res = format!("{} FETCH (BODY[] {{{}}}\r\n{}\r\n)\r\n",
    //         sequence, 
    //         b.len()+2, 
    //         String::from_utf8(b).map_err(Error::UTF8)?
    //     );
    //     Ok(res)
    // }

    // pub fn get_uids_from_fetch(&self, msg: &str) -> Result<Vec<String>>{
    //     // This is what the client intially asks for
    //     let mut uids: Vec<String> = vec![];
    //     let uids_string = msg.splitn(2," ").next().unwrap();
    //     let mut iter = uids_string.split(",");
    
    //     while let Some(uid) = iter.next(){
    //         uids.push(uid.into());
    //     }
    //     Ok(uids)
    // }

    // fn get_uid_from_filename(&self, f: &fs::DirEntry) -> usize{
    //     let mut file = f.file_name().into_string().unwrap();
    //     let shorter_filename = file.split_off(6);
    //     let uid_seed = shorter_filename.split("s.eml").next().unwrap().parse::<f64>().unwrap();
    //     (uid_seed * 100f64) as usize
    // }

    // pub fn fetch_info_from_uid(&self, uids: Vec<String>) -> Result<Vec<String>>{
    //     let mut responses = Vec::new();
    //     let dir = format!("{}/mail/{}/inbox", MAIL_ROOT, self.username.as_ref().unwrap());
    //     let files = std::fs::read_dir(dir).unwrap();

    //     for (i, f) in files.enumerate(){
    //         let i = i+1; // Adjust 0 to 1 index
    //         let uid = &self.get_uid_from_filename(&f.as_ref().unwrap()).to_string();
    //         println!("{}", uid);
    //         if !uids.contains(uid) { continue };
                        

    //         let path = f.map_err(Error::IO)?.path();
    //         // println!("{:?}", &path);
    //         let parser: ParseEmail = ParseEmail::new(&path)?;
    //         let (to_user, to_domain, to_display_name) = parser.to_header()?;
    //         let (from_user, from_domain, from_display_name) = parser.from_header()?;
    //         let date = parser.date_header()?;
    //         let subject = parser.subject_header()?;
    //         let internal_date = internal_date(&path)?;

    //         // TODO FIELDS
    //         //let message_id = "<CADkb2rHmFeg1D01=a=n9xFZ5LxqH5FsWHT_dPxMyK7O4v1EKUA@mail.gmail.com>";
    //         let message_id = "NIL";
    //         let data = ""; // This is what the bytes refer to, the new line adds +2 though
    //         let seq_num = i;
    //         let uid = self.get_uid(seq_num).unwrap();

    //         let bytes = data.len()+2;
    //         responses.push(format!(
    //             "{seq_num} FETCH (UID {uid} FLAGS (\\RECENT) ENVELOPE (\"{date}\" \"{subject}\" \
    //             ((\"{from_display_name}\" NIL \"{from_user}\" \"{from_domain}\")) NIL NIL ((\"{to_display_name}\" NIL \
    //             \"{to_user}\" \"{to_domain}\")) NIL NIL NIL {message_id}) INTERNALDATE \"{internal_date}\" \
    //             BODY[HEADER.FIELDS (References)] {{{bytes}}}\r\n{data}\r\n)\r\n",
    //             seq_num = seq_num,
    //             uid = uid,
    //             date = date,
    //             subject = subject,
    //             from_display_name = from_display_name,
    //             from_user = from_user,
    //             from_domain = from_domain,
    //             to_display_name = to_display_name,
    //             to_user = to_user,
    //             to_domain = to_domain,
    //             message_id = message_id,
    //             internal_date = internal_date,
    //             bytes = bytes,
    //             data = data,
    //         ));
    //     }
    //     Ok(responses)
    // }


    // pub fn fetch_info_from_seq(&self, msg: &str) -> Result<Vec<String>>{

    //     let from;
    //     let to;

    //     let mut responses = Vec::new();
    //     if msg.contains(":"){
    //         let mut range = msg.splitn(2," ").next().unwrap().split(":");
    //         from = range.next().unwrap().parse().unwrap();
    //         to = range.next().unwrap().parse().unwrap_or(std::usize::MAX);
    //     }else{
    //         from = msg.splitn(2, " ").next().unwrap().parse().unwrap();
    //         to = from;
    //     }

    //     let dir = format!("{}/mail/{}/inbox", MAIL_ROOT, self.username.as_ref().unwrap());
    //     let files = std::fs::read_dir(dir).unwrap();

    //     for (i, f) in files.enumerate(){
    //         let i = i+1; // Adjust 0 to 1 index
    //         if i < from { continue };
    //         if i > to { break };

    //         let path = f.map_err(Error::IO)?.path();
    //         // println!("{:?}", &path);
    //         let parser: ParseEmail = ParseEmail::new(&path)?;
    //         let (to_user, to_domain, to_display_name) = parser.to_header()?;
    //         let (from_user, from_domain, from_display_name) = parser.from_header()?;
    //         let date = parser.date_header()?;
    //         let subject = parser.subject_header()?;
    //         let internal_date = internal_date(&path)?;

    //         // TODO FIELDS
    //         //let message_id = "<CADkb2rHmFeg1D01=a=n9xFZ5LxqH5FsWHT_dPxMyK7O4v1EKUA@mail.gmail.com>";
    //         let message_id = "NIL";
    //         let data = ""; // This is what the bytes refer to, the new line adds +2 though
    //         let seq_num = i;
    //         let uid = self.get_uid(seq_num).unwrap();

    //         let bytes = data.len()+2;
    //         responses.push(format!(
    //             "{seq_num} FETCH (UID {uid} FLAGS (\\RECENT) ENVELOPE (\"{date}\" \"{subject}\" \
    //             ((\"{from_display_name}\" NIL \"{from_user}\" \"{from_domain}\")) NIL NIL ((\"{to_display_name}\" NIL \
    //             \"{to_user}\" \"{to_domain}\")) NIL NIL NIL {message_id}) INTERNALDATE \"{internal_date}\" \
    //             BODY[HEADER.FIELDS (References)] {{{bytes}}}\r\n{data}\r\n)\r\n",
    //             seq_num = seq_num,
    //             uid = uid,
    //             date = date,
    //             subject = subject,
    //             from_display_name = from_display_name,
    //             from_user = from_user,
    //             from_domain = from_domain,
    //             to_display_name = to_display_name,
    //             to_user = to_user,
    //             to_domain = to_domain,
    //             message_id = message_id,
    //             internal_date = internal_date,
    //             bytes = bytes,
    //             data = data,
    //         ));
    //     }
    //     Ok(responses)
    // }

    // /// Dont @ me, it works
    // pub fn get_uid(&self, seq: usize) -> Result<usize>{

    //     let path = format!("{}/mail/{}", MAIL_ROOT, self.username.as_ref().unwrap());
    //     let file = fs::read_dir(path).unwrap().nth(seq-1).unwrap();
    //     let mut filename = file.unwrap().file_name().into_string().unwrap();
    //     let shorter_filename = filename.split_off(6);
    //     let uid_seed = shorter_filename.split("s.eml").next().unwrap().parse::<f64>().unwrap();
    //     let uid = uid_seed * 100f64;
    //     Ok(uid as usize)
    // }