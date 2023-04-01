
use proc_macro2::{TokenStream, Span, Ident, Literal};
use std::collections::HashMap;
use std::rc::Rc;

use crate::Res;


// item env
pub enum Item {
   Ident(Ident),
   Literal(Literal),
   Stream(TokenStream),
   List(Vec<Rc<Item>>),
   Map(HashMap<String, Rc<Item>>),
}

pub struct Env(Vec<Scope>);


// scope
pub struct IterScope {
   pub first: bool,
   pub last: bool,
   pub index: usize,
   pub key: String,
   pub value: Rc<Item>,
}

struct Scope {
   items: HashMap<String, Rc<Item>>,
   iter_scope: Option<IterScope>,
}

impl Scope {
   fn new(iter_scope: Option<IterScope>) -> Self {
      Self { items: HashMap::new(), iter_scope }
   }
}


// getter key
pub enum Key {
   String(String),
   Index(usize)
}


pub struct Segment { pub span: Span, pub key: Key }


impl Item {
   pub fn get_item(self: &Rc<Self>, mut span: Span, path: &[Segment]) -> Res<Rc<Item>> {

      let iter = path.into_iter();
      let mut last_item = self;

      for segm in iter {
         match last_item.as_ref() {

            Item::Ident(_) | Item::Literal(_) | Item::Stream(_) => {
               err!(span, "item is not indexable")
            },

            Item::List(list) => { match segm.key {
               Key::String(_) => {
                  err!(segm.span, "can't index list with an identifier")
               },
               Key::Index(index) => {
                  if let Some(item) = list.get(index) {
                     last_item = item;
                  }
                  else {
                     err!(segm.span, "item not found")
                  }
               },
            }},

            Item::Map(map) => { match segm.key {
               Key::Index(_) => {
                  err!(segm.span, "can't index map with an integer")
               },
               Key::String(ref key) => {
                  if let Some(item) = map.get(key) {
                     last_item = item;
                  }
                  else {
                     err!(segm.span, "item not found")
                  }
               },
            }},
         }

         span = span.join(segm.span).unwrap(); // should never fail
      }

      Ok(Rc::clone(last_item))
   }
}


// functionality
impl Env {

   pub fn new() -> Self {
      Env(Vec::with_capacity(2))
   }

   pub fn push_scope(&mut self, iter_scope: Option<IterScope>) {
      self.0.push(Scope::new(iter_scope));
   }

   pub fn pop_scope(&mut self) {
      self.0.pop();
   }

   pub fn set_item(&mut self, key: String, item: Rc<Item>) {
      self.0.last_mut().unwrap().items.insert(key, item); // should never fail
   }

   pub fn get_iter_scope(&self) -> Option<&IterScope> {
      self.0.iter().rev().find_map(|scope| scope.iter_scope.as_ref())
   }

   pub fn get_item(&self, path: &[Segment]) -> Res<Rc<Item>> {

      let first = &path[0]; // should never fail

      let span = first.span.clone();

      let key = match first.key {
         Key::String(ref key) => key,
         Key::Index(_) => err!(span, "can't index scope with an integer"),
      };

      let item = self.0.iter().rev()
         .find_map(|scope| scope.items.get(key))
         .ok_or_else(|| (span, "item not found"))?
      ;

      item.get_item(span, &path[1..])
   }
}
