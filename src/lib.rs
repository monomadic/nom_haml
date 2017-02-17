#[macro_use]
extern crate nom;
use nom::{IResult,digit, space, alpha, alphanumeric};
struct HAMLParser{
    haml: String,
}


named!(single_quote_string<&str>,
  delimited!(
    tag!("'"),
    map_res!(escaped!(call!(alphanumeric), '\\', is_a!("\'n\\")), std::str::from_utf8),
    tag!("'")
  )
);
named!(double_quote_string<&str>,
  delimited!(
    tag!("\""),
    map_res!(escaped!(call!(alphanumeric), '\\', is_a!("\"n\\")), std::str::from_utf8),
    tag!("\"")
  )
);


named!(tag_id<&str>,
  chain!(
    tag!("#") ~
    id: map_res!( alphanumeric, std::str::from_utf8),
    || id
  )
);
named!(tag_named<&str>,
  chain!(
    tag!("%") ~
    tag: map_res!( alphanumeric, std::str::from_utf8),
    || tag
  )
);


named!(attribute_hash_key_value<(&str, &str)>, 
       chain!(
           tag!(":") ~
           space? ~
           key: map_res!( alphanumeric, std::str::from_utf8) ~
           space? ~
           tag!("=>") ~
           space? ~
           value: alt!(single_quote_string | double_quote_string | map_res!( alphanumeric, std::str::from_utf8)) ,
           || (key,value)
        )
      );

named!(attributes_list<Vec<(&str, &str)>>, 
       dbg_dmp!(ws!(
            delimited!( 
                alt!(tag!("{") | tag!("(")),
                separated_list!(
                    tag!(","),
                    attribute_hash_key_value
                ),
                alt!(tag!("}") | tag!(")"))
                )
            ) 
    ));


named!(html_tag<(std::option::Option<&str>, std::option::Option<&str>, std::option::Option<std::vec::Vec<(&str, &str)>>)>,
       do_parse!(
               tag: opt!(complete!(tag_named)) >>
               id: opt!(complete!(tag_id)) >>
               attributes: opt!(complete!(attributes_list)) >>
               (tag, id, attributes)
               )
       );

#[cfg(test)]
mod tests {
    use nom::{IResult};
    use super::*;

    #[test]
    fn it_parses_tag_id() {
        let empty = &b""[..];
        //should not allow this kind of set.
        assert_eq!(tag_id("#id".as_bytes()), IResult::Done(empty, ("id")));
    }

    #[test]
    fn it_parses_tag_nammed() {
        let empty = &b""[..];
        //should not allow this kind of set.
        assert_eq!(tag_named("%p".as_bytes()), IResult::Done(empty, ("p")));
    }

    #[test]
    fn it_parses_html_tag() {
        let empty = &b""[..];
        //should not allow this kind of set.
        assert_eq!(html_tag("%p".as_bytes()), IResult::Done(empty, (Some("p"),None,None)));
        assert_eq!(html_tag("%p#banana".as_bytes()), IResult::Done(empty, (Some("p"),Some("banana"),None)));
        assert_eq!(html_tag("%p#banana(:d=>3)".as_bytes()), IResult::Done(empty, (Some("p"),Some("banana"), Some(vec![("d","3")]))));
        assert_eq!(html_tag("%p(:d=>3)".as_bytes()), IResult::Done(empty, (Some("p"),None,Some(vec![("d","3")]))));
    }
    #[test]
    fn it_parses_attributes_not_allowed() {
        let empty = &b""[..];
        //should not allow this kind of set.
        assert_eq!(attributes_list("(:a => 3}".as_bytes()), IResult::Done(empty, vec![("a","3")]));
    }

    #[test]
    fn it_parses_attributes() {
        let empty = &b""[..];
        assert_eq!(attributes_list("{}".as_bytes()), IResult::Done(empty, vec![]));
        assert_eq!(attributes_list("(:a=>3)".as_bytes()), IResult::Done(empty, vec![("a","3")]));
        assert_eq!(attributes_list("{:a=>\"3\"}".as_bytes()), IResult::Done(empty, vec![("a","3")]));
        assert_eq!(attributes_list("{:a=> '3'}".as_bytes()), IResult::Done(empty, vec![("a","3")]));
        assert_eq!(attributes_list("{:a=>3, :b=>'4'}".as_bytes()), IResult::Done(empty, vec![("a","3"),("b","4")]));
    }
}
