use html5ever::{
  self as html,
  rcdom::{self, NodeData, RcDom},
  tendril::TendrilSink,
};
use regex::Regex;
use std::{
  io::{self, Write},
  string,
};

error_chain! {
  foreign_links {
    FromUtf8(string::FromUtf8Error);
    Io(io::Error);
  }
}

lazy_static! {
  static ref PARSE_OPTS: html::ParseOpts = html::ParseOpts {
    tree_builder: html::tree_builder::TreeBuilderOpts {
      drop_doctype: true,
      ..Default::default()
    },
    ..Default::default()
  };
}

// Used to fix the double-escaping the Reddit API does
pub fn unwrap(html: &str) -> Result<String> {
  fn visit(node: rcdom::Handle, out: &mut Vec<u8>) -> Result<()> {
    match node.data {
      NodeData::Text { ref contents } => write!(out, "{}", contents.borrow())?,
      _ => (),
    }

    for child in node.children.borrow().iter() {
      visit(child.clone(), out)?;
    }

    Ok(())
  }

  let opts = PARSE_OPTS.clone();

  let dom = html::parse_document(RcDom::default(), opts)
    .from_utf8()
    .read_from(&mut html.as_bytes())?;

  let mut ret = Vec::new();

  visit(dom.document, &mut ret)?;

  Ok(String::from_utf8(ret)?.trim_right().to_string())
}

pub fn pretty_unwrap(html: &str) -> Result<String> {
  #[derive(Debug)]
  enum NodeType {
    Block,
    Inline,
    Meta,
    Unknown(String),
  }

  #[derive(Debug)]
  enum Element {
    Text(String),
    Node(NodeType, Children),
    Ignore,
  }

  type Children = Vec<Element>;

  lazy_static! {
    static ref WHITESPACE_RE: Regex = Regex::new(r"\s+").unwrap();
  }

  fn visit_dom(node: rcdom::Handle) -> Result<Element> {
    let children: Result<Vec<_>> = node
      .children
      .borrow()
      .iter()
      .map(|c| visit_dom(c.clone()))
      .collect();

    if let Err(e) = children {
      return Err(e);
    }

    let mut children = children.unwrap();

    let children: Vec<_> = children
      .drain(..)
      .filter_map(|c| match c {
        Element::Ignore => None,
        e => Some(e),
      })
      .collect();

    Ok(match node.data {
      NodeData::Document => Element::Node(NodeType::Meta, children),
      NodeData::Doctype { .. } => Element::Ignore,
      NodeData::Text { ref contents } => {
        let text = contents.borrow();
        let text = WHITESPACE_RE.replace_all(&text, " ");

        // TODO: handle <pre> correctly
        Element::Text(text.into_owned())
      }
      NodeData::Comment { .. } => Element::Ignore,
      NodeData::Element { ref name, .. } => Element::Node(
        match name.local.to_string().as_str() {
          "div" => NodeType::Block,
          "p" => NodeType::Block,
          "ol" => NodeType::Block,
          "ul" => NodeType::Block,
          "li" => NodeType::Block,
          "blockquote" => NodeType::Block,
          "pre" => NodeType::Block,
          "h1" => NodeType::Block,
          "h2" => NodeType::Block,
          "h3" => NodeType::Block,
          "h4" => NodeType::Block,
          "h5" => NodeType::Block,
          "h6" => NodeType::Block,
          "br" => NodeType::Block,
          "hr" => NodeType::Block,
          "span" => NodeType::Inline,
          "em" => NodeType::Inline,
          "strong" => NodeType::Inline,
          "sub" => NodeType::Inline,
          "sup" => NodeType::Inline,
          "a" => NodeType::Inline,
          "del" => NodeType::Inline,
          "code" => NodeType::Inline,
          "html" => NodeType::Meta,
          "head" => NodeType::Meta,
          "body" => NodeType::Meta,
          s => NodeType::Unknown(s.to_string()),
        },
        children,
      ),
      NodeData::ProcessingInstruction { .. } => Element::Ignore,
    })
  }

  #[derive(Debug, Clone, Copy)]
  enum VisitState {
    BeginLine,
    MidLine,
  }

  fn visit_elt(el: &Element, state: VisitState) -> (String, VisitState) {
    match el {
      Element::Text(s) => {
        match state {
          VisitState::BeginLine => {
            let trimmed = s.trim_left();

            (
              trimmed.into(),
              if trimmed.len() > 0 {
                VisitState::MidLine
              } else {
                VisitState::BeginLine
              },
            )
          }
          VisitState::MidLine => (s.clone(), VisitState::MidLine),
        }
      }
      Element::Node(t, c) => {
        let mut children: Vec<String> = Vec::new();

        let mut state = state;

        match t {
          NodeType::Block => {
            match state {
              VisitState::BeginLine => (),
              VisitState::MidLine => children.push("\n".into()),
            }

            state = VisitState::BeginLine;
          }
          NodeType::Inline => (),
          NodeType::Meta => (),
          NodeType::Unknown(s) => children.push(format!("<{}> ", s)),
        }

        for child in c.iter() {
          let (s, new_state) = visit_elt(child, state);

          children.push(s);

          state = new_state;
        }

        // TODO: this is very not right, but it works well enough and I
        //       don't want to touch it anymore

        (children.concat().trim_right().into(), state)
      }
      Element::Ignore => unreachable!(),
    }
  }

  let opts = PARSE_OPTS.clone();

  let dom = html::parse_document(RcDom::default(), opts)
    .from_utf8()
    .read_from(&mut html.as_bytes())?;

  let elt = visit_dom(dom.document)?;

  // println!("{:#?}", elt);

  Ok(visit_elt(&elt, VisitState::BeginLine).0)
}
