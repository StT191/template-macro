
use proc_macro2::{TokenTree, Punct, Ident, Span, Delimiter};

use crate::*;


pub enum Action {
   Escape(Punct),
   Assign(Ident, Assign),
   Quote(Quote),
}


macro_rules! action {
   ($action:ident: $($attr:tt)*) => { Ok(Action::$action($($attr)*)) }
}

pub fn parse_action(mut span: Span, input: &mut TokenIter, env: &mut Env) -> Res<Action> {

   match next!(span, input) {

      TokenTree::Ident(ident) => match next!(span, input) {

         // assignment
         TokenTree::Punct(punct) if punct.as_char() == '=' => {
            action!(Assign: ident, parse_assign_value(span, input, env)?)
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
