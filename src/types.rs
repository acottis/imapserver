
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