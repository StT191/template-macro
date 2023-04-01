
use proc_macro2::{TokenTree, Punct, Span, Delimiter::{Parenthesis, Brace, Bracket}};

use crate::{*, BlockModifier::{Concat, First, Last, NotFirst, NotLast}};


pub enum Action {
   Escape(Punct),
   Assign(String, Assign),
   Quote(Quote),
}


macro_rules! ok_action {
   ($action:ident: $($attr:tt)*) => { Ok(Action::$action($($attr)*)) }
}


pub fn parse_action(mut span: Span, input: &mut TokenIter, env: &mut Env) -> Res<Action> {

   match next!(span, input) {

      TokenTree::Ident(ident) => {

         let id = ident.to_string();

         match next!(span, input) {

            // assignment
            TokenTree::Punct(punct) if punct.as_char() == ':' => {
               ok_action!(Assign: id, parse_assign_value(span, input, env)?)
            },

            // modified block quotes
            TokenTree::Group(blk) if blk.delimiter() == Brace => match id.as_str() {
               "first" => ok_action!(Quote: Quote::Block(First, blk)),
               "last" => ok_action!(Quote: Quote::Block(Last, blk)),
               "concat_ident" => ok_action!(Quote: Quote::Block(Concat, blk)),
               _ => err!(ident.span(), "unknown modifier"),
            },

            // item len function
            TokenTree::Group(gp) if gp.delimiter() == Parenthesis => match id.as_str() {
               "len" => ok_action!(Quote: Quote::Item(ItemModifier::Len, gp)),
               _ => err!(ident.span(), "unknown modifier"),
            },

            _ => err!(span, "unexpected token"),
         }
      },

      TokenTree::Punct(punct) => match punct.as_char() {

         // escaped
         '$' => ok_action!(Escape: punct),

         // concat function
         '#' => {
            let blk = match_next!(span, input, Group(blk) if blk.delimiter() == Brace);
            ok_action!(Quote: Quote::Block(Concat, blk))
         },

         // not modifier
         '!' => {
            let before = span.clone();
            let id = match_next!(span, input, Ident(ident)).to_string();

            if id != "first" && id != "last" {
               err!(before.join(span).unwrap(), "unknown modifier")
            }

            let blk = match_next!(span, input, Group(blk) if blk.delimiter() == Brace);

            ok_action!(Quote: Quote::Block(
               if id == "first" { NotFirst } else { NotLast },
               blk,
            ))
         },

         // any other
         _ => err!(span, "unexpected token"),
      },

      // blocks
      TokenTree::Group(gp) => match gp.delimiter() {

         // item quote
         Parenthesis => ok_action!(Quote: Quote::Item(ItemModifier::None, gp)),

         // bind block quote
         Bracket => {
            let blk = match_next!(span, input, Group(blk) if blk.delimiter() == Brace);
            ok_action!(Quote: Quote::Iter(gp, blk))
         },

         _ => err!(span, "unexpected token"),
      },

      _ => err!(span, "unexpected token"),
   }
}
