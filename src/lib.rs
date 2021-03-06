#![recursion_limit = "300"]
extern crate pest;
extern crate error_chain;
#[macro_use]
extern crate nom;
extern crate id_tree;
use id_tree::InsertBehavior::*;
use id_tree::*;
use nom::{anychar, space, alphanumeric};
use nom::IResult;
use std::str;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

// Move to HashSet
pub const SELF_CLOSING: [&str; 16] = ["area", "base", "br", "col", "command", "embed", "hr",
                                      "img", "input", "keygen", "link", "meta", "param", "source",
                                      "track", "wbr"];
trait HtmlDisplay {
    fn hat(&self) -> String;
    fn pants(&self) -> Option<String> {
        None
    }
}

type AttrMap = HashMap<String, String>;

type ContextCode = String;
type CodeRun = String;
type Text = String;

// this wont fly in the future gonna need make real structs
impl HtmlDisplay for Text {
    fn hat(&self) -> String {
        self.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct HamlNode {
    pub tag: String,
    pub attributes: Option<AttrMap>,
    pub id: Option<String>,
    pub context_lookup: Option<ContextCode>,
    pub contents: String,
    pub class: Vec<String>,
}

impl HtmlDisplay for HamlNode {
    fn hat(&self) -> String {
        let mut string = format!("<{}", self.tag);
        if let Some(id) = self.id.as_ref() {
            string.push_str(&format!(" id=\"{}\"", id));
        }
        if let Some(attrs) = self.attributes.as_ref() {
            for (k, v) in attrs {
                string.push_str(&format!(" {}=\"{}\"", k, v));
            }
        }
        if self.class.len() > 0 {
            string.push_str(&format!(" class=\"{}\"", self.class.join(" ")));
        }

        if SELF_CLOSING.iter().any(|&tag| tag == self.tag) {
            string.push_str(" />")
        } else {
            string.push_str(&format!(">{}", self.contents))
        }
        // context lookup
        string
    }

    fn pants(&self) -> Option<String> {
        if SELF_CLOSING.iter().any(|&tag| tag == self.tag) {
            None
        } else {
            Some(format!("</{}>", self.tag))
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
enum HamlCode {
    HamlNodeBlock(HamlNode),
    CodeBlock(CodeRun),
    ContextBlock(ContextCode),
    TextBlock(Text),
}


named!(code_run<CodeRun>,
       chain!(
           tag!("- ") ~
           data: many1!(anychar),
           || CodeRun::from(data.into_iter().collect::<String>())
           )
      );

named!(context_lookup<ContextCode>,
       chain!(
           tag!("= ") ~
           data: many1!(anychar),
           || ContextCode::from(data.into_iter().collect::<String>())
           )
      );
// TODO: move to an char
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

named!(doctype<&str>,
       chain!(
           tag!("!!!") ~
           data: map_res!( alphanumeric, std::str::from_utf8),
           || data
           )
      );
// TODO: -?[_a-zA-Z]+[_a-zA-Z0-9-]*
named!(tag_class<&str>,
       chain!(
           tag!(".") ~
           class: map_res!( alphanumeric, std::str::from_utf8),
           || class
           )
      );

// TODO: figure out real limit
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
// TODO: value should be more then alphanumeric
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

named!(attributes_list<AttrMap>,
       map!(
           ws!(
               delimited!(
                   alt!(tag!("{") | tag!("(")),
                   separated_list!(
                       tag!(","),
                       attribute_hash_key_value
                       ),
                       alt!(tag!("}") | tag!(")"))
                       )
              ),
              |tuple_vec|{
                  let mut h= AttrMap::new();
                  for (k, v) in tuple_vec {
                      h.insert(String::from(k), String::from(v));
                  }
                  h
              }
           )
      );

named!(only_classes<(Option<&str>,Option<&str>,Vec<String>)>,
do_parse!(
    class: many1!(tag_class) >>
    (None, None, class.into_iter().map(|text| String::from(text)).collect())
    )
);

named!(only_id<(Option<&str>,Option<&str>,Vec<String>)>,
do_parse!(
    id: tag_id >>
    class: many0!(tag_class) >>
    (None, Some(id), class.into_iter().map(|text| String::from(text)).collect())
    )
);

named!(only_tag<(Option<&str>,Option<&str>,Vec<String>)>,
do_parse!(
    tag: tag_named >>
    class: many0!(tag_class) >>
    (Some(tag), None, class.into_iter().map(|text| String::from(text)).collect())
    )
);
named!(only_tag2<(Option<&str>,Option<&str>,Vec<String>)>,
do_parse!(
    tag: tag_named >>
    (Some(tag), None, vec![])
    )
);

named!(full_set<(Option<&str>,Option<&str>,Vec<String>)>,
do_parse!(
    tag: tag_named >>
    id: tag_id >>
    class: many0!(tag_class) >>
    (Some(tag), Some(id), class.into_iter().map(|text| String::from(text)).collect())
    )
);

named!(html_tag<(HamlNode)>,
do_parse!(
    tag_id_class: alt!(
       complete!(full_set) |
       complete!(only_tag) |
       complete!(only_id)  |
       complete!(only_classes)
    ) >>
    attributes_list: opt!(complete!(attributes_list)) >>
    context_lookup: opt!(complete!(context_lookup)) >>
    contents: many0!(anychar) >>
    (HamlNode{tag: String::from(tag_id_class.0.unwrap_or("div")),
    id: tag_id_class.1.map(|text| String::from(text)),
    contents: contents.into_iter().collect::<String>(),
    attributes: attributes_list,
    context_lookup: context_lookup,
    class: tag_id_class.2,
    })
    )
);

named!(html_line<(Vec<&str>, HamlNode)>,
do_parse!(
    whitespace: many0!(map_res!(alt!(tag!(" ")|tag!("\t")), str::from_utf8)) >>
    node: html_tag >>
    (whitespace, node)
    )
);

named!(the_rest<(Text)>,
do_parse!(
    contents: many0!(anychar)>>
    (contents.into_iter().collect::<Text>())
    )
);

named!(haml_line<(Vec<&str>, HamlCode)>,
do_parse!(
    whitespace: many0!(map_res!(alt!(tag!(" ")|tag!("\t")), str::from_utf8)) >>
    line: alt!(
        // context before tag. tag can include context
        context_lookup => { |h| HamlCode::ContextBlock(h)      }   |
        code_run => { |h|       HamlCode::CodeBlock(h)      }   |
        html_tag => { |h|       HamlCode::HamlNodeBlock(h)  }   |
        the_rest => { |h|       HamlCode::TextBlock(h)      }
        )>> 
    (whitespace, line)
    )
);

struct HAMLParser {
    haml: String,
    nodes: Option<Tree<HamlCode>>,
}

impl HAMLParser {
    pub fn string_pre_order(buffer: String, tree: &Tree<HamlCode>, node_id: &NodeId) -> String {
        let node_ref = tree.get(node_id).unwrap();
        let (hat, pants) = match *node_ref.data() {
            HamlCode::HamlNodeBlock(ref node) => {
                (node.hat(), node.pants().unwrap_or("".to_string()))
            }
            HamlCode::ContextBlock(ref node) => {
                (node.hat(), node.pants().unwrap_or("".to_string()))
            }
            HamlCode::TextBlock(ref node) => (node.hat(), node.pants().unwrap_or("".to_string())),

            HamlCode::CodeBlock(ref node) => (node.hat(), node.pants().unwrap_or("".to_string())),
            _ => ("".to_string(), "".to_string()),
        };
        let parent_hat = format!("{}{}", buffer, hat);
        let parent_pants = format!("{}{}", buffer, pants);

        let childern_str = node_ref.children()
            .iter()
            .map(|child_id| {
                     let new_buff = format!("{}  ",buffer);
                     HAMLParser::string_pre_order(new_buff, tree, &child_id)
                 })
            .collect::<Vec<String>>()
            .join("\n");
        let mut list = vec![parent_hat];
        if childern_str.trim().len() > 0 {
            list.push(childern_str);
        }
        if parent_pants.trim().len() > 0 {
            list.push(parent_pants);
        }

        list.join("\n")
    }

    pub fn render(&self, context: Option<HashMap<String, Arc<fmt::Display>>>) -> String {
        let root = self.nodes
            .as_ref()
            .unwrap()
            .root_node_id()
            .unwrap()
            .clone();
        HAMLParser::string_pre_order("".to_string(), self.nodes.as_ref().unwrap(), &root)
    }

    pub fn parse(&mut self) -> Result<(), String> {
        fn parent_id(tree: &Tree<HamlCode>, current_node: &Option<NodeId>) -> Option<NodeId> {
            let parent = current_node.as_ref().unwrap().clone();
            match tree.get(&parent).unwrap().parent() {
                Some(parent_id) => Some(parent_id.clone()),
                None => None,
            }
        }
        // TODO: create a base node because the doctype and html can be the same level
        let mut tree: Tree<HamlCode> = Tree::new();
        let mut previous_depth = 0;
        let mut current_node: Option<NodeId> = None;
        let mut whitespacer: Option<String> = None;
        let mut was_self_closing = false;
        let mut was_tag_block = false;
        for line in self.haml.lines() {
            // println!("{}",line);
            let tag = haml_line(line.as_bytes());
            match tag {
                IResult::Done(_, (whitespace, haml_code)) => {
                    let current_depth = if whitespacer.is_some() {
                        // TODO: make sure they are the same type of spacing
                        // TODO: make sure its a valid count not 3 spaces
                        whitespace.len() / whitespacer.as_ref().unwrap().len()
                    } else if whitespace.len() != 0 && whitespacer.is_none() {
                        whitespacer = Some(whitespace.into_iter().collect::<String>());
                        1
                    } else {
                        0
                    };

                    let is_self_closing = match haml_code {
                        HamlCode::HamlNodeBlock(ref node) => {
                            !node.contents.trim_left().is_empty() ||
                            SELF_CLOSING.iter().any(|&tag| tag == node.tag)
                        }
                        _ => false,
                    };

                    let is_tag_block = match haml_code {
                        HamlCode::HamlNodeBlock(_) => true,
                        _ => false,
                    };

                    if current_node.is_some() {
                        if current_depth == previous_depth {
                            // support empty tags
                            if !was_self_closing && (was_tag_block && is_tag_block) {
                                match parent_id(&tree, &current_node) {
                                    Some(parent_id) => {
                                        let parent = parent_id.clone();
                                        current_node = Some(parent);
                                    }
                                    None => {
                                        return Err(format!("unwinding to far current depth {} new {}\n{}", 
                                                       previous_depth, current_depth, line))
                                    }
                                }
                            }
                            let child_node = Node::new(haml_code);
                            let last_child = tree.insert(child_node,
                                                         UnderNode(current_node.as_ref().unwrap()))
                                .unwrap();

                            if !is_self_closing {
                                current_node = Some(last_child);
                            }
                        } else if current_depth == previous_depth + 1 {
                            let child_node = Node::new(haml_code);
                            tree.insert(child_node, UnderNode(current_node.as_ref().unwrap()))
                                .unwrap();

                            if !is_self_closing {
                                let last_child = {
                                    let node = tree.get(current_node.as_ref().unwrap()).unwrap();
                                    // TODO remove clone?
                                    node.children()
                                        .last()
                                        .unwrap()
                                        .clone()
                                };
                                current_node = Some(last_child);
                            }
                        } else if current_depth > previous_depth {
                            return Err(format!("Jumped depth to far from {} to {}\n{}", previous_depth, current_depth,line));
                        } else {
                            // TODO remove clones
                            let mut parent_node = current_node.clone();
                            let mut depth_run = previous_depth - current_depth;
                            if !was_self_closing {
                                depth_run += 1;
                            }

                            for _ in 0..depth_run {
                                match parent_id(&tree, &parent_node) {
                                    Some(parent_node_id) => {
                                        parent_node = Some(parent_node_id.clone());
                                    }
                                    None => {
                                        return Err(format!("unwinding to far current depth {} new {}\n{}", 
                                                       previous_depth, current_depth, line))
                                    }
                                }
                            }

                            let child_node = Node::new(haml_code);
                            let last_child = tree.insert(child_node,
                                                         UnderNode(parent_node.as_ref().unwrap()))
                                .unwrap();
                            if !is_self_closing {
                                current_node = Some(last_child)
                            } else {
                                current_node = parent_node;
                            }
                        }
                    } else {
                        match haml_code {
                            HamlCode::HamlNodeBlock(_) => {
                                current_node = Some(tree.insert(Node::new(haml_code), AsRoot)
                                                        .unwrap());
                            }
                            _ => return Err(format!("No base node in stack found {:?}", haml_code)),
                        }
                    }
                    was_tag_block = is_tag_block;
                    was_self_closing = is_self_closing;
                    previous_depth = current_depth;
                }
                _ => {}
            }

        }
        self.nodes = Some(tree);
        // Deal with panics as errors
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //#![feature(trace_macros)]
    use nom::IResult;
    use super::*;
    fn print_pre_order(buffer: String, tree: &Tree<HamlCode>, node_id: &NodeId) {
        let node_ref = tree.get(node_id).unwrap();

        println!("{}{:?}", buffer, node_ref.data());

        for child_id in node_ref.children() {
            let new_buff = format!("{}  ",buffer);
            print_pre_order(new_buff, tree, &child_id);
        }
    }
    // TODO figure out real tests
    #[test]
    fn it_parser_scratch() {
        let haml = r#"
%ul
  %li.class1.class2
  %li#coderun.code= smoked_salt 
  %li
    - if true
      %br
    = salt_box 
  %li.inline Salt
  %li#subdata
    Pepper
"#;
        let mut parser = HAMLParser {
            nodes: None,
            haml: haml.to_string(),
        };
        parser.parse().unwrap();
        let root = parser.nodes
            .as_ref()
            .unwrap()
            .root_node_id()
            .unwrap()
            .clone();
        {
            print_pre_order("".to_string(), &parser.nodes.as_ref().unwrap(), &root);
        }
        println!("{}",parser.render(None));

    }

    #[test]
    fn it_parser_self_closing_tag() {
        let haml = r#"
%p
  .1
    %br
    .21
      .210
"#;
        let mut parser = HAMLParser {
            nodes: None,
            haml: haml.to_string(),
        };
        parser.parse().unwrap();
        let root = parser.nodes
            .as_ref()
            .unwrap()
            .root_node_id()
            .unwrap()
            .clone();
        print_pre_order("".to_string(), &parser.nodes.unwrap(), &root);

    }
    #[test]
    fn it_parser_empty_tag() {
        let haml = r#"
%p
  .1
    .20 
    .21
      .210
    .22
      .220
"#;
        let mut parser = HAMLParser {
            nodes: None,
            haml: haml.to_string(),
        };
        parser.parse().unwrap();
        let root = parser.nodes
            .as_ref()
            .unwrap()
            .root_node_id()
            .unwrap()
            .clone();
        print_pre_order("".to_string(), &parser.nodes.unwrap(), &root);

    }

    #[test]
    fn it_parser_self_closing() {
        let haml = r#"
%p
  .1
    .20 200
    .21
      .210
    .22
      .220
"#;
        let mut parser = HAMLParser {
            nodes: None,
            haml: haml.to_string(),
        };
        parser.parse().unwrap();
        let root = parser.nodes
            .as_ref()
            .unwrap()
            .root_node_id()
            .unwrap()
            .clone();
        print_pre_order("".to_string(), &parser.nodes.unwrap(), &root);

    }

    #[test]
    fn it_parses_haml_line_text_only() {
        let parsed_node = haml_line("prints".as_bytes());
        match parsed_node {
            IResult::Done(_, tup) => {
                assert_eq!(tup.1,  HamlCode::TextBlock("prints".to_string()));
            }
            _ => {
                assert_eq!(false, true);
            }
        }
    }


    #[test]
    fn it_parses_haml_line_context_block() {
        let parsed_node = haml_line("= prints".as_bytes());
        match parsed_node {
            IResult::Done(_, tup) => {
                assert_eq!(tup.1,  HamlCode::ContextBlock("prints".to_string()));
            }
            _ => {
                assert_eq!(false, true);
            }
        }
    }


    #[test]
    fn it_parses_haml_line_haml_tag() {
        let node = HamlNode {
            class: vec![],
            tag: "p".to_string(),
            id: None,
            context_lookup: None,
            contents: "".to_string(),
            attributes: None,
        };

        let parsed_node = haml_line("%p".as_bytes());
        match parsed_node {
            IResult::Done(_, tup) => {
                assert_eq!(tup.1,  HamlCode::HamlNodeBlock(node.clone()));
            }
            _ => {
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn it_parses_tag_id() {
        let empty = &b""[..];
        assert_eq!(tag_id("#id".as_bytes()), IResult::Done(empty, ("id")));
    }

    #[test]
    fn it_parses_tag_nammed() {
        let empty = &b""[..];
        assert_eq!(tag_named("%p".as_bytes()), IResult::Done(empty, ("p")));
    }

    #[test]
    fn it_parses_html_line() {
        let empty = &b""[..];

        let node = HamlNode {
            class: vec![],
            tag: "p".to_string(),
            id: None,
            context_lookup: None,
            contents: "".to_string(),
            attributes: None,
        };
        assert_eq!(html_line("%p".as_bytes()), IResult::Done(empty, (vec![], node.clone())));
        assert_eq!(html_line("\t\t%p".as_bytes()), IResult::Done(empty, (vec!["\t","\t"], node.clone())));
        assert_eq!(html_line("  %p".as_bytes()), IResult::Done(empty, (vec![" "," "], node)));
    }

    #[test]
    fn it_parses_html_tag() {
        let empty = &b""[..];

        let node = HamlNode {
            class: vec![],
            tag: "p".to_string(),
            id: None,
            context_lookup: None,
            contents: "".to_string(),
            attributes: None,
        };
        assert_eq!(html_tag("%p".as_bytes()), IResult::Done(empty, (node)));

        let node = HamlNode {
            class: vec![],
            tag: "p".to_string(),
            id: None,
            context_lookup: None,
            contents: " Yes sir".to_string(),
            attributes: None,
        };
        assert_eq!(html_tag("%p Yes sir".as_bytes()), IResult::Done(empty, (node)));

        let node = HamlNode {
            class: vec![],
            tag: "p".to_string(),
            contents: "".to_string(),
            context_lookup: None,
            id: Some("banana".to_string()),
            attributes: None,
        };
        assert_eq!(html_tag("%p#banana".as_bytes()), IResult::Done(empty, (node)));

        let node = HamlNode {
            class: vec!["pan".to_string(),"cakes".to_string()],
            tag: "p".to_string(),
            contents: "".to_string(),
            context_lookup: None,
            id: Some("banana".to_string()),
            attributes: None,
        };
        assert_eq!(html_tag("%p#banana.pan.cakes".as_bytes()), IResult::Done(empty, (node)));

        let mut attrs = AttrMap::new();
        attrs.insert("d".to_string(), "3".to_string());
        let node = HamlNode {
            class: vec![],
            tag: "p".to_string(),
            contents: "".to_string(),
            context_lookup: None,
            id: Some("banana".to_string()),
            attributes: Some(attrs.clone()),
        };
        assert_eq!(html_tag("%p#banana(:d=>3)".as_bytes()), IResult::Done(empty, (node)));

        let node = HamlNode {
            class: vec![],
            tag: "p".to_string(),
            contents: "".to_string(),
            context_lookup: None,
            id: None,
            attributes: Some(attrs.clone()),
        };
        assert_eq!(html_tag("%p(:d=>3)".as_bytes()), IResult::Done(empty, (node)));

        let node = HamlNode {
            class: vec![],
            tag: "p".to_string(),
            contents: "".to_string(),
            context_lookup: Some("banana".to_string()),
            id: None,
            attributes: Some(attrs.clone()),
        };
        assert_eq!(html_tag("%p(:d=>3)= banana".as_bytes()), IResult::Done(empty, (node)));
    }

    #[test]
    fn it_parses_attributes_not_allowed() {
        let empty = &b""[..];
        let mut attrs = AttrMap::new();
        attrs.insert("a".to_string(), "3".to_string());
        // should not allow this kind of set.
        assert_eq!(attributes_list("(:a => 3}".as_bytes()), IResult::Done(empty, attrs));
    }

    #[test]
    fn it_parses_attributes() {
        let empty = &b""[..];
        let mut attrs = AttrMap::new();
        assert_eq!(attributes_list("{}".as_bytes()), IResult::Done(empty,attrs.clone()));

        attrs.insert("a".to_string(), "3".to_string());
        assert_eq!(attributes_list("(:a=>3)".as_bytes()), IResult::Done(empty, attrs.clone()));

        assert_eq!(attributes_list("{:a=>\"3\"}".as_bytes()), IResult::Done(empty, attrs.clone()));

        assert_eq!(attributes_list("{:a=> '3'}".as_bytes()), IResult::Done(empty, attrs.clone()));

        attrs.insert("b".to_string(), "4".to_string());
        assert_eq!(attributes_list("{:a=>3, :b=>'4'}".as_bytes()), IResult::Done(empty, attrs));
    }
}
