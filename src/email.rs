/// This struct is responsible for opening a email file from the file system then picking out the data that IMAP requires
/// 
use regex::Regex;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use crate::error::{Result,Error};

/// Struct containing the email contents
///
#[derive(Debug)] 
pub struct Email{
    uid: String,
    seq: String,
    email_contents: String,
}

impl Email{
    /// Creates a new struct and gets the contents of an email and stores as a [String]
    /// 
    pub fn new(uid: &str, seq: &str, email_path: impl AsRef<Path>) -> Result<Self> {
        let mut buf = vec![];
        let mut f = File::open(email_path).map_err(Error::IO)?;
        f.read_to_end(&mut buf).map_err(Error::IO)?;
        let email = String::from_utf8(buf).map_err(Error::UTF8)?;

        Ok(Self{
            uid: uid.to_owned(),
            seq: seq.to_owned(),
            email_contents: email,
        })
    }
    /// Parses the to field of an email
    /// 
    pub fn to_header(&self) -> Result<(String, String, String)> {

        let to = Regex::new(r"(?mi)^TO:.*").unwrap();

        let find_to = to.find(&self.email_contents);
        let line = find_to.map_or("", |m| m.as_str()).replace(&['<','>','\r'][..], "");
        let mut split = line.splitn(2, " ").last().unwrap().rsplitn(2, " "); // Remove To: and split email and displayname 

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
    /// Format a response with just the UID
    /// 
    fn format_response(&self) -> String{
        
        String::new()
    }
}