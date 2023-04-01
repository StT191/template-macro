
use proc_macro2::{TokenTree, Ident, Literal, Group, Span, Delimiter};
use std::collections::HashMap;
use std::rc::Rc;

use crate::*;


pub enum Assign {
   Ident(Ident),
   Literal(Literal),
   Quote(Quote),
   List(Group),
   Map(Group),
}

impl Assign {
   pub fn span(&self) -> Span {
      match self {
         Assign::Ident(tk) => tk.span(), Assign::Literal(tk) => tk.span(), Assign::Map(tk) => tk.span(),
         Assign::List(tk) => tk.span(), Assign::Quote(tk) => tk.span(),
      }
   }
}


pub fn parse_assign_value(mut span: Span, input: &mut TokenIter, env: &mut Env) -> Res<Assign> {
   match next!(span, input) {

      TokenTree::Ident(id) => Ok(Assign::Ident(id)),
      TokenTree::Literal(lt) => Ok(Assign::Literal(lt)),
      TokenTree::Group(gp) if gp.delimiter() == Delimiter::Parenthesis => Ok(Assign::List(gp)),
      TokenTree::Group(gp) if gp.delimiter() == Delimiter::Brace => Ok(Assign::Map(gp)),

      TokenTree::Punct(pt) if pt.as_char() == '$' => match parse_action(span, input, env)? {

         Action::Quote(quote) => {
            let mut collector = TokenStream::new();
            parse_quote(span, quote, &mut collector, env)?;
            input.push_in_front(collector);
            parse_assign_value(span, input, env)
         }

         Action::Escape(escape) => err!(escape.span(), "unexpected token"),
         Action::Assign(_, assign) => err!(span.join(assign.span()).unwrap(), "unexpected assignment"),
      },

      _ => err!(span, "unexpected token"),
   }
}


pub fn parse_assign(span: Span, assign: Assign, env: &mut Env) -> Res<Rc<Item>> {
   Ok(match assign {

      Assign::Ident(ident) => Item::Ident(ident).into(),

      Assign::Literal(lit) => Item::Literal(lit).into(),

      Assign::Quote(quote) => match quote {

         Quote::Item(group) => parse_item_path(group, env)?,

         _ => {
            let mut collector = TokenStream::new();
            parse_quote(span, quote, &mut collector, env)?;
            Item::Stream(collector).into()
         },
      },

      Assign::List(group) => {

         let mut tokens = group.stream().into();
         let mut stream = TokenStream::new();
         parse_block(&mut tokens, &mut stream, env)?;

         let mut list = Vec::new();

         if !stream.is_empty() {
            let mut tokens = stream.into();

            while let Ok(assign) = parse_assign_value(group.span(), &mut tokens, env) {

               let item = parse_assign(assign.span(), assign, env)?;
               list.push(item);

               match tokens.next() {
                  None => break,
                  Some(TokenTree::Punct(pt)) if pt.as_char() == ',' => continue,
                  Some(other) => err!(other.span(), "unexpected token"),
               }
            }
         }

         Item::List(list).into()
      },

      Assign::Map(group) => {

         let mut tokens = group.stream().into();
         let mut stream = TokenStream::new();
         parse_block(&mut tokens, &mut stream, env)?;

         let mut map = HashMap::new();

         if !stream.is_empty() {
            let mut tokens: TokenIter = stream.into();

            while let Some(token) = tokens.next() {

               let mut span = token.span();

               let ident = match_token!(token, Ident).to_string();

               match_next!(span, tokens, Punct(pt) if pt.as_char() == ':');

               let assign = parse_assign_value(group.span(), &mut tokens, env)?;
               let item = parse_assign(assign.span(), assign, env)?;

               map.insert(ident, item);

               match tokens.next() {
                  None => break,
                  Some(TokenTree::Punct(pt)) if pt.as_char() == ',' => continue,
                  Some(other) => err!(other.span(), "unexpected token"),
               }
            }
         }

         Item::Map(map).into()
      },

   })
}
