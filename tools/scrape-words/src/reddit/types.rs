use serde::{self, de};
use serde_value;
use std::{
  collections::HashMap, fmt, result::Result as StdResult, str::FromStr,
};

error_chain! {
  errors {
    BadVariant(of: String, expect: String) {
      description("bad enum variant"),
      display("bad variant of enum {} (expecting {})", of, expect),
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
  Listing,
  Comment,
  Account,
  Link,
  Message,
  Subreddit,
  Award,
  More,
}

struct KindVisitor;

error_chain! {
  types {
    ParseKindError, ParseKindErrorKind, ParseKindResultExt, ParseKindResult;
  }

  errors {
    NoMatch(s: String) {
      description("string matched nothing"),
      display("string '{}' matched nothing", s),
    }
  }
}

impl ToString for Kind {
  fn to_string(&self) -> String {
    match self {
      Kind::Listing => "Listing",
      Kind::Comment => "t1",
      Kind::Account => "t2",
      Kind::Link => "t3",
      Kind::Message => "t4",
      Kind::Subreddit => "t5",
      Kind::Award => "t6",
      Kind::More => "more",
    }.to_string()
  }
}

impl Into<String> for Kind {
  fn into(self) -> String {
    self.to_string()
  }
}

impl FromStr for Kind {
  type Err = ParseKindError;

  fn from_str(s: &str) -> ParseKindResult<Self> {
    match s {
      "Listing" => Ok(Kind::Listing),
      "t1" => Ok(Kind::Comment),
      "t2" => Ok(Kind::Account),
      "t3" => Ok(Kind::Link),
      "t4" => Ok(Kind::Message),
      "t5" => Ok(Kind::Subreddit),
      "t6" => Ok(Kind::Award),
      "more" => Ok(Kind::More),
      s => Err(ParseKindErrorKind::NoMatch(s.into()).into()),
    }
  }
}

impl serde::Serialize for Kind {
  fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

impl<'de> serde::Deserialize<'de> for Kind {
  fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    deserializer.deserialize_str(KindVisitor)
  }
}

impl<'de> de::Visitor<'de> for KindVisitor {
  type Value = Kind;

  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_str("a thing kind ID")
  }

  fn visit_str<E>(self, value: &str) -> StdResult<Self::Value, E>
  where
    E: de::Error,
  {
    Kind::from_str(value).map_err(|e| E::custom(e.to_string()))
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing {
  pub modhash: Option<String>,
  pub dist: Option<u32>,
  pub after: Option<String>,
  pub before: Option<String>,

  pub children: Vec<Thing>,

  #[serde(flatten)]
  pub other: HashMap<String, serde_value::Value>, // TODO: keep an eye on this
}

// TODO: this should have custom serialization
#[derive(Debug, Clone)]
pub enum CommentReplies {
  None,
  Some(Box<Thing>),
}

struct CommentRepliesVisitor;

impl serde::Serialize for CommentReplies {
  fn serialize<S>(&self, _: S) -> StdResult<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    panic!("not implemented")
  }
}

impl<'de> serde::Deserialize<'de> for CommentReplies {
  fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    deserializer.deserialize_any(CommentRepliesVisitor)
  }
}

impl<'de> de::Visitor<'de> for CommentRepliesVisitor {
  type Value = CommentReplies;

  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_str("a thing or an empty string")
  }

  fn visit_str<E>(self, s: &str) -> StdResult<Self::Value, E>
  where
    E: de::Error,
  {
    if s.len() == 0 {
      Ok(CommentReplies::None)
    } else {
      Err(de::Error::invalid_value(de::Unexpected::Str(s), &self))
    }
  }

  fn visit_map<A>(self, map: A) -> StdResult<Self::Value, A::Error>
  where
    A: de::MapAccess<'de>,
  {
    Ok(CommentReplies::Some(Box::new(ThingVisitor.visit_map(map)?)))
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
  // Subreddit info
  pub subreddit: String,
  pub subreddit_id: String,
  pub subreddit_name_prefixed: String,
  pub subreddit_type: String,

  // Author info
  pub author: String,
  pub author_fullname: Option<String>, // Wait, excuse me?
  pub author_flair_template_id: Option<String>,
  pub author_flair_css_class: Option<String>,
  pub author_flair_text_color: Option<String>,
  pub author_flair_background_color: Option<String>,
  pub author_flair_type: Option<String>,
  pub author_flair_text: Option<String>,
  pub author_flair_richtext: Option<Vec<serde_value::Value>>,
  pub is_submitter: bool,

  // Comment metadata
  pub id: String,
  pub name: String,
  pub permalink: String,
  pub score_hidden: bool,
  pub can_gild: bool,
  pub stickied: bool,
  pub archived: bool,
  pub distinguished: Option<String>,
  pub gilded: u32,
  pub collapsed: bool,
  pub collapsed_reason: Option<String>,
  // pub post_hint: Option<String>,

  // Comment info
  pub depth: u32,
  pub parent_id: String,
  pub link_id: String,
  pub created: f64,     // TODO
  pub created_utc: f64, // TODO
  pub edited: EditTime,
  pub body: String,
  pub body_html: String,
  // pub replies: CommentReplies,

  // Stats
  pub ups: i32,
  pub downs: u32,
  pub score: i32,
  pub controversiality: u32,

  // Moderation info
  pub banned_at_utc: (), // TODO
  pub banned_by: (),     // TODO
  pub mod_note: Option<String>,
  pub mod_reason_title: Option<String>,
  pub mod_reason_by: Option<String>,
  pub mod_reports: Vec<()>, // TODO
  pub num_reports: (),      // TODO
  pub report_reasons: (),   // TODO
  pub removal_reason: Option<String>,
  pub user_reports: Vec<String>,

  // User-specific
  pub saved: bool,
  pub likes: Option<bool>,

  // What?
  pub approved_at_utc: (),
  pub approved_by: (),
  pub can_mod_post: bool,
  pub no_follow: bool,
  pub send_replies: bool,

  #[serde(flatten)]
  pub other: HashMap<String, serde_value::Value>, // TODO: keep an eye on this

  pub replies: CommentReplies, // TODO: remove this
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
  #[serde(flatten)]
  pub other: HashMap<String, serde_value::Value>, // TODO: keep an eye on this
}

// TODO: this should have custom serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EditTime {
  Unedited(bool),
  Edited(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkPreviewImageSource {
  pub width: u32,
  pub height: u32,
  pub url: String,
  // #[serde(flatten)]
  // pub other: HashMap<String, serde_value::Value>, // TODO: keep an eye on this
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkPreviewImage {
  pub id: String,
  pub source: LinkPreviewImageSource,
  pub resolutions: Vec<LinkPreviewImageSource>,
  // #[serde(flatten)]
  // pub other: HashMap<String, serde_value::Value>, // TODO: keep an eye on this
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkPreview {
  pub enabled: bool,
  pub images: Vec<LinkPreviewImage>,
  // #[serde(flatten)]
  // pub other: HashMap<String, serde_value::Value>, // TODO: keep an eye on this
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
  // Subreddit info
  pub subreddit: String,
  pub subreddit_id: String,
  pub subreddit_name_prefixed: String,
  pub subreddit_type: String,
  pub subreddit_subscribers: u32,

  // Author info
  pub author: String,
  pub author_fullname: Option<String>, // Wait, excuse me?
  pub author_flair_template_id: Option<String>,
  pub author_flair_css_class: Option<String>,
  pub author_flair_text_color: Option<String>,
  pub author_flair_background_color: Option<String>,
  pub author_flair_type: Option<String>,
  pub author_flair_text: Option<String>,
  pub author_flair_richtext: Option<Vec<serde_value::Value>>,

  // Post metadata
  pub id: String,
  pub name: String,
  pub domain: String,
  pub permalink: String,
  pub hidden: bool,
  pub can_gild: bool,
  pub is_crosspostable: bool,
  pub is_self: bool,
  pub is_meta: bool,
  pub is_original_content: bool,
  pub over_18: bool,
  pub spoiler: bool,
  pub pinned: bool,
  pub stickied: bool,
  pub archived: bool,
  pub locked: bool,
  pub distinguished: Option<String>,
  pub gilded: u32,
  pub link_flair_template_id: Option<String>,
  pub link_flair_css_class: Option<String>,
  pub link_flair_text_color: Option<String>,
  pub link_flair_background_color: Option<String>,
  pub link_flair_type: String,
  pub link_flair_text: Option<String>,
  pub link_flair_richtext: Vec<serde_value::Value>, // TODO
  pub post_hint: Option<String>,

  // Post info
  pub title: String,
  pub created: f64,     // TODO
  pub created_utc: f64, // TODO
  pub edited: EditTime,
  pub selftext: String,
  pub selftext_html: Option<String>, // Wait, why is this nullable but selftext isn't?
  pub url: String,
  pub media_only: bool,
  pub is_video: bool,
  pub thumbnail: String,
  pub thumbnail_width: Option<u32>,
  pub thumbnail_height: Option<u32>,
  pub preview: Option<LinkPreview>,
  pub media: serde_value::Value,                               // TODO
  pub media_embed: HashMap<String, serde_value::Value>,        // TODO
  pub secure_media: serde_value::Value,                        // TODO
  pub secure_media_embed: HashMap<String, serde_value::Value>, // TODO

  // Stats
  pub view_count: (), // TODO
  pub ups: u32,
  pub downs: u32,
  pub score: i32,
  pub upvote_ratio: Option<f64>,
  pub hide_score: bool,
  pub num_comments: u32,
  pub num_crossposts: u32,

  // Moderation info
  pub banned_at_utc: (), // TODO
  pub banned_by: (),     // TODO
  pub mod_note: Option<String>,
  pub mod_reason_title: Option<String>,
  pub mod_reason_by: Option<String>,
  pub mod_reports: Vec<()>, // TODO
  pub num_reports: (),      // TODO
  pub report_reasons: (),   // TODO
  pub removal_reason: Option<String>,
  pub user_reports: Vec<String>,

  // User-specific
  pub clicked: bool,
  pub visited: bool, // TODO: how's this different from 'clicked'?
  pub likes: Option<bool>,
  pub saved: bool,

  // What?
  pub approved_at_utc: (),
  pub approved_by: (),
  pub can_mod_post: bool,
  pub category: (),
  pub content_categories: (),
  pub contest_mode: bool,
  pub is_reddit_media_domain: bool,
  pub no_follow: bool,
  pub parent_whitelist_status: Option<String>,
  pub pwls: Option<u32>,
  pub quarantine: bool,
  pub send_replies: bool,
  pub suggested_sort: Option<String>,
  pub whitelist_status: Option<String>,
  pub wls: Option<u32>,

  #[serde(flatten)]
  pub other: HashMap<String, serde_value::Value>, // TODO: keep an eye on this
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
  #[serde(flatten)]
  pub other: HashMap<String, serde_value::Value>, // TODO: keep an eye on this
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subreddit {
  #[serde(flatten)]
  pub other: HashMap<String, serde_value::Value>, // TODO: keep an eye on this
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Award {
  #[serde(flatten)]
  pub other: HashMap<String, serde_value::Value>, // TODO: keep an eye on this
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct More {
  pub id: String,
  pub name: String,
  pub depth: u32,
  pub parent_id: String,
  pub count: u32,
  pub children: Vec<serde_value::Value>, // TODO

  #[serde(flatten)]
  pub other: HashMap<String, serde_value::Value>, // TODO: keep an eye on this
}

#[derive(Debug, Clone)]
pub enum Thing {
  Listing(Listing),
  Comment(Comment),
  Account(Account),
  Link(Link),
  Message(Message),
  Subreddit(Subreddit),
  Award(Award),
  More(More),
}

struct ThingVisitor;

// TODO: implement TryInto for Thing

impl Thing {
  pub fn into_listing(self) -> Listing {
    if let Thing::Listing(r) = self {
      r
    } else {
      panic!("bad variant for enum Thing, expected Listing");
    }
  }

  pub fn into_comment(self) -> Comment {
    if let Thing::Comment(r) = self {
      r
    } else {
      panic!("bad variant for enum Thing, expected Comment");
    }
  }

  pub fn into_account(self) -> Account {
    if let Thing::Account(r) = self {
      r
    } else {
      panic!("bad variant for enum Thing, expected Account");
    }
  }

  pub fn into_link(self) -> Link {
    if let Thing::Link(r) = self {
      r
    } else {
      panic!("bad variant for enum Thing, expected Link");
    }
  }

  pub fn into_message(self) -> Message {
    if let Thing::Message(r) = self {
      r
    } else {
      panic!("bad variant for enum Thing, expected Message");
    }
  }

  pub fn into_subreddit(self) -> Subreddit {
    if let Thing::Subreddit(r) = self {
      r
    } else {
      panic!("bad variant for enum Thing, expected Subreddit");
    }
  }

  pub fn into_award(self) -> Award {
    if let Thing::Award(r) = self {
      r
    } else {
      panic!("bad variant for enum Thing, expected Award");
    }
  }

  pub fn try_into_listing(self) -> Result<Listing> {
    if let Thing::Listing(r) = self {
      Ok(r)
    } else {
      Err(ErrorKind::BadVariant("Thing".into(), "Listing".into()).into())
    }
  }

  pub fn try_into_comment(self) -> Result<Comment> {
    if let Thing::Comment(r) = self {
      Ok(r)
    } else {
      Err(ErrorKind::BadVariant("Thing".into(), "Comment".into()).into())
    }
  }

  pub fn try_into_account(self) -> Result<Account> {
    if let Thing::Account(r) = self {
      Ok(r)
    } else {
      Err(ErrorKind::BadVariant("Thing".into(), "Account".into()).into())
    }
  }

  pub fn try_into_link(self) -> Result<Link> {
    if let Thing::Link(r) = self {
      Ok(r)
    } else {
      Err(ErrorKind::BadVariant("Thing".into(), "Link".into()).into())
    }
  }

  pub fn try_into_message(self) -> Result<Message> {
    if let Thing::Message(r) = self {
      Ok(r)
    } else {
      Err(ErrorKind::BadVariant("Thing".into(), "Message".into()).into())
    }
  }

  pub fn try_into_subreddit(self) -> Result<Subreddit> {
    if let Thing::Subreddit(r) = self {
      Ok(r)
    } else {
      Err(ErrorKind::BadVariant("Thing".into(), "Subreddit".into()).into())
    }
  }

  pub fn try_into_award(self) -> Result<Award> {
    if let Thing::Award(r) = self {
      Ok(r)
    } else {
      Err(ErrorKind::BadVariant("Thing".into(), "Award".into()).into())
    }
  }
}

impl serde::Serialize for Thing {
  fn serialize<S>(&self, _: S) -> StdResult<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    panic!("not implemented")
  }
}

impl<'de> serde::Deserialize<'de> for Thing {
  fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    deserializer.deserialize_map(ThingVisitor)
  }
}

impl<'de> de::Visitor<'de> for ThingVisitor {
  type Value = Thing;

  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_str("a thing")
  }

  fn visit_map<A>(self, mut map: A) -> StdResult<Self::Value, A::Error>
  where
    A: de::MapAccess<'de>,
  {
    while let Some(key) = map.next_key::<String>()? {
      match key.as_str() {
        "kind" => {
          let kind: Kind = map.next_value()?;

          while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
              "data" => {
                return Ok(match kind {
                  Kind::Listing => Thing::Listing(map.next_value()?),
                  Kind::Comment => Thing::Comment(map.next_value()?),
                  Kind::Account => Thing::Account(map.next_value()?),
                  Kind::Link => Thing::Link(map.next_value()?),
                  Kind::Message => Thing::Message(map.next_value()?),
                  Kind::Subreddit => Thing::Subreddit(map.next_value()?),
                  Kind::Award => Thing::Award(map.next_value()?),
                  Kind::More => Thing::More(map.next_value()?),
                })
              }
              s => {
                println!("WARNING: ignoring unexpected field {}", s);
              }
            }
          }
        }
        "data" => {
          let data: serde_value::Value = map.next_value()?;

          while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
              "kind" => {
                let kind: Kind = map.next_value()?;

                return Ok(match kind {
                  Kind::Listing => {
                    Thing::Listing(data.deserialize_into().map_err(|e| {
                      <A::Error as de::Error>::custom(e.to_string())
                    })?)
                  }
                  Kind::Comment => {
                    Thing::Comment(data.deserialize_into().map_err(|e| {
                      <A::Error as de::Error>::custom(e.to_string())
                    })?)
                  }
                  Kind::Account => {
                    Thing::Account(data.deserialize_into().map_err(|e| {
                      <A::Error as de::Error>::custom(e.to_string())
                    })?)
                  }
                  Kind::Link => Thing::Link(data.deserialize_into().map_err(
                    |e| <A::Error as de::Error>::custom(e.to_string()),
                  )?),
                  Kind::Message => {
                    Thing::Message(data.deserialize_into().map_err(|e| {
                      <A::Error as de::Error>::custom(e.to_string())
                    })?)
                  }
                  Kind::Subreddit => {
                    Thing::Subreddit(data.deserialize_into().map_err(|e| {
                      <A::Error as de::Error>::custom(e.to_string())
                    })?)
                  }
                  Kind::Award => {
                    Thing::Award(data.deserialize_into().map_err(|e| {
                      <A::Error as de::Error>::custom(e.to_string())
                    })?)
                  }
                  Kind::More => {
                    Thing::More(data.deserialize_into().map_err(|e| {
                      <A::Error as de::Error>::custom(e.to_string())
                    })?)
                  }
                });
              }
              s => {
                println!("WARNING: ignoring unexpected field {}", s);
              }
            }
          }
        }
        s => {
          println!("WARNING: ignoring unknown field {}", s);
        }
      }
    }

    Err(<A::Error as de::Error>::custom(
      "failed to deserialize any fields of Thing",
    ))
  }
}
