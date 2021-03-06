
#[allow(dead_code)]
#[derive(PartialEq)]
pub enum Response{
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
pub enum Command{
    Capability,
    Authenticate,
    Fetch,
    Unrecognised,
    Login,
    List,
    Lsub,
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
            "LSUB" => Command::Lsub,
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

pub enum Month{
    Jan = 1,
    Feb = 2,
    Mar = 3,
    Apr = 4,
    May = 5,
    Jun = 6,
    Jul = 7,
    Aug = 8,
    Sep = 9,
    Oct = 10,
    Nov = 11,
    Dec = 12,
}

impl TryFrom<&str> for Month{
    type Error = crate::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error>{
        Ok(match s {
            "Jan" => Month::Jan,
            "Feb" => Month::Feb,
            "Mar" => Month::Mar,
            "Apr" => Month::Apr,
            "May" => Month::May,
            "Jun" => Month::Jun,
            "Jul" => Month::Jul,
            "Aug" => Month::Aug,
            "Sep" => Month::Sep,
            "Oct" => Month::Oct,
            "Nov" => Month::Nov,
            "Dec" => Month::Dec,
            _ => return Err(crate::Error::NotAMonth)
        })
    }
}