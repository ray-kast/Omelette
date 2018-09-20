use schema::*;

#[derive(Insertable)]
#[table_name = "form_ids"]
pub struct FormId<'a> {
  pub norm: &'a str,
  pub id: i32,
}

#[derive(Queryable)]
pub struct FormIdQ {
  pub norm: String,
  pub id: i32,
}

#[derive(Insertable)]
#[table_name = "forms"]
pub struct Form<'a> {
  pub oid: i32,
  pub id: i32,
  pub blank: &'a str,
  pub full: &'a str,
}

#[derive(Queryable)]
pub struct FormQ {
  pub oid: i32,
  pub id: i32,
  pub blank: String,
  pub full: String,
}

#[derive(Insertable)]
#[table_name = "set_ids"]
pub struct SetId<'a> {
  pub key: &'a str,
  pub id: i32,
}

#[derive(Queryable)]
pub struct SetIdQ {
  pub key: String,
  pub id: i32,
}

#[derive(Insertable)]
#[table_name = "sets"]
pub struct Set<'a> {
  pub oid: i32,
  pub id: i32,
  pub norm: &'a str,
}

#[derive(Queryable)]
pub struct SetQ {
  pub oid: i32,
  pub id: i32,
  pub norm: String,
}

#[derive(Insertable)]
#[table_name = "set_keys"]
pub struct SetKey<'a> {
  pub oid: i32,
  pub len: i32,
  pub key: &'a str,
}

#[derive(Queryable)]
pub struct SetKeyQ {
  pub oid: i32,
  pub len: i32,
  pub key: String,
}
