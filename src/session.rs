use crate::error::{Result, Error};
use crate::email::Email;
use chrono::prelude::{Utc, TimeZone};
use std::fs;

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
        let arg = msg.rsplit(r#" "#).next().unwrap().replace("\"", "").to_uppercase();
        if folders.contains(&arg){
            return Ok(vec![arg])
        }
        match arg.as_str() {
            ""      => return Ok(vec![r#""""#.into()]),
            "%/%"   => return Ok(vec![r#""""#.into()]),
            "%"     => return Ok(folders),
            "*"     => return Ok(folders),
            _       => return Ok(Vec::new()),
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
    /// Search UID
    /// 
    pub fn search(&self, msg: &str) -> Result<Vec<String>>{
        let date = msg.rsplitn(2," ").next().unwrap();

        let mut split_date = date.split("-");
        let day = split_date.next().unwrap();
        let month: crate::types::Month = split_date.next().unwrap().try_into().unwrap();
        let year = split_date.next().unwrap();
        let email_timestamp = Utc.ymd(year.parse().unwrap(), month as u32, day.parse().unwrap()).and_hms(0, 0, 0).timestamp();

        let path = format!("{}/mail/{}/Inbox", MAIL_ROOT, self.username.as_ref().unwrap());
        let dir = fs::read_dir(path).unwrap();

        let search_results: Vec<_> = dir.into_iter().filter(|f| {
            let file_date: f64 = f.as_ref().unwrap().file_name().into_string().unwrap().split("s.eml").next().unwrap().parse().unwrap();
            file_date > email_timestamp as f64
        }).collect();

        let uids: Vec<_> = search_results.into_iter().map(| f |{
            self.filename_to_uid(f.as_ref().unwrap())
        }).collect();

        Ok(uids)
    }
    /// Fetch UID
    /// 
    pub fn fetch_uid(&self, msg: &str) -> Result<Vec<String>>{
        // Holding Vector for result
        let mut emails: Vec<Email> = Vec::new();
        let mut responses: Vec<String> = Vec::new();
        let mut fetch_all = false;

        // Split it into two parts, sequence number(s) and args
        let mut split = msg.splitn(2, " ");
        let uid = split.next().unwrap();
        let args = split.next().unwrap();

        // Get the files in the Inbox
        let path = format!("{}/mail/{}/Inbox", MAIL_ROOT, self.username.as_ref().unwrap());
        let dir = fs::read_dir(path).unwrap();

        // Get the UID list
        let mut uids: Vec<String> = Vec::new(); 

        if uid.contains(",") {
            let mut uid_list = uid.split(",");
            while let Some(uid) = uid_list.next(){ 
                uids.push(uid.into());
            }
        }else if uid.contains(":"){
            fetch_all = true;
        }
        else{
            // Lookup one thing
            uids.push(uid.into());
        }
        
        for (index, file) in dir.enumerate(){
            let file_uid = self.filename_to_uid(file.as_ref().unwrap());
            if fetch_all == true{
                emails.push(Email::new(&file_uid, &index.to_string(), file.as_ref().unwrap().path()).unwrap());
            }
            else if uids.contains(&file_uid){
                emails.push(Email::new(&file_uid, &index.to_string(), file.as_ref().unwrap().path()).unwrap());
            }
        }

        // Format the emails
        for email in emails{
            responses.push(email.format_response(args))
        }
        // Parse Args
        Ok(responses)
    }
    /// Fetch (Non UID version)
    /// 
    pub fn fetch_seq(&self, msg: &str) -> Result<Vec<String>>{
        // Holding Vector for result
        let mut emails: Vec<Email> = Vec::new();
        let mut responses: Vec<String> = Vec::new();

        // Split it into two parts, sequence number(s) and args
        let mut split = msg.splitn(2, " ");
        let seq = split.next().unwrap();
        let args = split.next().unwrap();

        // Get the files in the Inbox
        let path = format!("{}/mail/{}/Inbox", MAIL_ROOT, self.username.as_ref().unwrap());
        let dir = fs::read_dir(path).unwrap();

        if seq.contains(",") {
            // Look up Vector
            emails.append(&mut self.fetch_list(seq, dir).unwrap());
        }
        else if seq.contains(":") {
            // Lookup Range
            emails.append(&mut self.fetch_range(seq, dir).unwrap());
        }
        else{
            // Lookup one thing
            emails.push(self.fetch_one(seq, dir).unwrap());
        }
            // Formate the
        for email in emails{
            responses.push(email.format_response(args))
        }
        
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
        let shorter_filename = filename.split_off(3);
        let uid_seed = shorter_filename.split("s.eml").next().unwrap().parse::<f64>().unwrap();
        ((uid_seed * 10f64) as usize).to_string()
    }
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
#[test]
fn fetch_uid_single(){
    let mut session = UserSession::new();
    session.authenticate("\"test@ashdown.scot\" tset");
    
    let res = session.fetch_uid("87142369,87142369 (UID FLAGS RFC822.SIZE BODY.PEEK[] INTERNALDATE)").unwrap();
    println!("{:#?}", res);
}
#[test]
fn search(){
    let mut session = UserSession::new();
    session.authenticate("\"test@ashdown.scot\" tset");
    
    let res = session.search("SINCE 04-Dec-2021").unwrap();
    println!("{:#?}", res);
}