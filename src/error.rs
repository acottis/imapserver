pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug)]
pub enum Error{
    IO(std::io::Error),
    UTF8(std::string::FromUtf8Error),
    CommandNotRecognised,
    FolderLookup(&'static str),
    InvalidToField,
    CantReadEmail,
    TimeDate(chrono::ParseError),
    IMAPCopyErr,
    TCPReadTimeout,
    NotAMonth,
}