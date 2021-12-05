use crate::email::Email;
#[test]
fn email_parse_from_no_displaynames(){
    let parser = Email::new("1", "1", "test_emails/NoDisplayNames.eml").unwrap();
    let (user, domain, display) = parser.from_header().unwrap();
    assert_eq!(user, "adam.bar", "User is wrong");
    assert_eq!(domain, "foo.com", "Domain is wrong");
    assert_eq!(display, "NIL", "Display Name is wrong");
}
#[test]
fn email_parse_from_displaynames(){
    let parser = Email::new("1", "1", "test_emails/DisplayNames.eml").unwrap();
    let (user, domain, display) = parser.from_header().unwrap();
    assert_eq!(user, "adam.bar", "User is wrong");
    assert_eq!(domain, "foo.com", "Domain is wrong");
    assert_eq!(display, "Adam the Rusty", "Display Name is wrong");
}
#[test]
fn email_parse_to_displaynames(){
    let parser = Email::new("1", "1", "test_emails/DisplayNames.eml").unwrap();
    let (user, domain, display) = parser.to_header().unwrap();
    assert_eq!(user, "adam.test", "User is wrong");
    assert_eq!(domain, "example.scot", "Domain is wrong");
    assert_eq!(display, "Adam Test", "Display Name is wrong");
}
#[test]
fn email_parse_to_no_displaynames(){
    let parser = Email::new("1", "1", "test_emails/NoDisplayNames.eml").unwrap();
    let (user, domain, display) = parser.to_header().unwrap();
    assert_eq!(user, "adam.test", "User is wrong");
    assert_eq!(domain, "example.scot", "Domain is wrong");
    assert_eq!(display, "NIL", "Display Name is wrong");
}
#[test]
fn email_parse_date(){
    let parser = Email::new("1", "1", "test_emails/NoDisplayNames.eml").unwrap();
    let date = parser.date_header().unwrap();
    assert_eq!(date, "Tue, 23 Nov 2021 16:56:32 +0000", "Date is wrong");

}
#[test]
fn email_parse_subject(){
    let parser = Email::new("1", "1", "test_emails/NoDisplayNames.eml").unwrap();
    let subject = parser.subject_header().unwrap();
    assert_eq!(subject, "Testing Email");
}