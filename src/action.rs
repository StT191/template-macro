
use proc_macro2::{
   TokenTree, Punct, Ident, Literal, Group, Span, Delimiter,
   token_stream::{IntoIter as TokenIter},
};

use crate::*;


pub enum Modifier { None, Concat, First, Last, NotFirst, NotLast }

pub enum Quote {
   Block(Modifier, Group),
   Iter(Group, Group),
   Item(Group),
}

impl Quote {
   pub fn span(&self) -> Span {
      match self {
         Quote::Block(_, blk) => blk.span(), Quote::Iter(_, blk) => blk.span(), Quote::Item(gp) => gp.span()
      }
   }
}

pub enum Assign {
   Ident(Ident),
   Literal(Literal),
   Map(Group),
   List(Group),
   Quote(Quote),
}

impl Assign {
   pub fn span(&self) -> Span {
      match self {
         Assign::Ident(tk) => tk.span(), Assign::Literal(tk) => tk.span(), Assign::Map(tk) => tk.span(),
         Assign::List(tk) => tk.span(), Assign::Quote(tk) => tk.span(),
      }
   }
}

pub enum Action {
   Escape(Punct),
   Assign(Ident, Assign),
   Quote(Quote),
}


macro_rules! action {
   ($action:ident: $($attr:tt)*) => { Ok(Action::$action($($attr)*)) }
}


pub fn parse_assign_value(mut span: Span, input: &mut TokenIter) -> Res<Assign> {
   match next!(span, input) {

      TokenTree::Ident(id) => Ok(Assign::Ident(id)),
      TokenTree::Literal(lt) => Ok(Assign::Literal(lt)),
      TokenTree::Group(gp) if gp.delimiter() == Delimiter::Parenthesis => Ok(Assign::List(gp)),
      TokenTree::Group(gp) if gp.delimiter() == Delimiter::Brace => Ok(Assign::Map(gp)),

      TokenTree::Punct(pt) if pt.as_char() == '$' => match parse_action(span, input)? {
         Action::Quote(quote) => Ok(Assign::Quote(quote)),
         Action::Escape(escape) => err!(escape.span(), "unexpected token"),
         Action::Assign(_, assign) => err!(span.join(assign.span()).unwrap(), "unexpected assignment"),
      },

      _ => err!(span, "unexpected token"),
   }
}


pub fn parse_action(mut span: Span, input: &mut TokenIter) -> Res<Action> {

   match next!(span, input) {

      TokenTree::Ident(ident) => match next!(span, input) {

         // assignment
         TokenTree::Punct(punct) if punct.as_char() == '=' => {
            action!(Assign: ident, parse_assign_value(span, input)?)
         },

         // modified block quotes
         TokenTree::Group(blk) if blk.delimiter() == Delimiter::Brace => {
            let id = ident.to_string();
            if id == "first" { action!(Quote: Quote::Block(Modifier::First, blk)) }
            else if id == "last" { action!(Quote: Quote::Block(Modifier::Last, blk)) }
            else { err!(ident.span(), "unexpected token") }
         },

         _ => err!(span, "unexpected token"),
      },

      TokenTree::Punct(punct) => match punct.as_char() {

         // escaped
         '$' => action!(Escape: punct),

         // concat function
         '#' => {
            let blk = match_next!(span, input, Group(blk) if blk.delimiter() == Delimiter::Brace);
            action!(Quote: Quote::Block(Modifier::Concat, blk))
         },

         // not modifier
         '!' => {
            let id;
            match_next!(span, input, Ident(ident) if {
               id = ident.to_string();
               id == "first" || id == "last"
            });

            let blk = match_next!(span, input, Group(blk) if blk.delimiter() == Delimiter::Brace);

            action!(Quote: Quote::Block(
               if id == "first" { Modifier::NotFirst } else { Modifier::NotLast },
               blk,
            ))
         },

         // any other
         _ => err!(span, "unexpected token"),
      },

      // blocks
      TokenTree::Group(gp) => match gp.delimiter() {

         // block quote
         Delimiter::Brace => action!(Quote: Quote::Block(Modifier::None, gp)),

         // var qute
         Delimiter::Parenthesis => action!(Quote: Quote::Item(gp)),

         // bind block quote
         Delimiter::Bracket => {
            let blk = match_next!(span, input, Group(blk) if blk.delimiter() == Delimiter::Brace);
            action!(Quote: Quote::Iter(gp, blk))
         },

         _ => err!(span, "unexpected token"),
      },

      _ => err!(span, "unexpected token"),
   }
}
