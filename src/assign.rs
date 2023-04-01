
use proc_macro2::{TokenTree, Ident, Literal, Group, Span, Delimiter::{Parenthesis, Brace}};
use std::collections::HashMap;
use std::rc::Rc;

use crate::*;


pub enum Assign {
   Ident(Ident),
   Literal(Literal),
   Stream(Group),
   Item(Group),
   List(Group),
   Map(Group),
}

impl Assign {
   fn span(&self) -> Span {
      match self {
         Assign::Ident(tk) => tk.span(), Assign::Literal(tk) => tk.span(),
         Assign::Stream(gp) | Assign::Item(gp) | Assign::List(gp) | Assign::Map(gp) => gp.span(),
      }
   }
}


pub fn parse_assign_value(mut span: Span, input: &mut TokenIter, env: &mut Env) -> Res<Assign> {
   Ok(match next!(span, input) {

      TokenTree::Ident(id) => Assign::Ident(id),
      TokenTree::Literal(lt) => Assign::Literal(lt),
      TokenTree::Group(gp) if gp.delimiter() == Parenthesis => Assign::List(gp),

      TokenTree::Group(gp) if gp.delimiter() == Brace => {
         let mut tokens = gp.stream().into_iter();
         match next!(span, tokens) {
            // quoted tokenstream
            TokenTree::Group(mut blk) if blk.delimiter() == Brace => match tokens.next() {
               None => {
                  blk.set_span(span);
                  Assign::Stream(blk)
               },
               Some(tk) => err!(tk.span(), "unexpected token"),
            },
            // map
            _ => Assign::Map(gp),
         }
      },

      TokenTree::Punct(pt) if pt.as_char() == '@' => Assign::Item(
         match_next!(span, input, Group(gp) if gp.delimiter() == Parenthesis)
      ),

      TokenTree::Punct(pt) if pt.as_char() == '$' => match parse_action(span, input, env)? {

         Action::Quote(quote) => {
            let mut collector = TokenStream::new();
            parse_quote(span, quote, &mut collector, env)?;
            input.push_in_front(collector);
            parse_assign_value(span, input, env)?
         }

         Action::Escape(escape) => err!(escape.span(), "unexpected token"),
         Action::Assign(_, assign) => err!(span.join(assign.span()).unwrap(), "unexpected assignment"),
      },

      _ => err!(span, "unexpected token"),
   })
}



fn evaluate_scoped_block(input: TokenStream, env: &mut Env) -> Res<TokenStream> {
   let mut output = TokenStream::new();
   env.push_scope(None);
   let res = parse_block(input, &mut output, env);
   env.pop_scope();
   res.and(Ok(output))
}


pub fn parse_assign(assign: Assign, env: &mut Env) -> Res<Rc<Item>> {
   Ok(match assign {

      Assign::Ident(ident) => Item::Ident(ident).into(),

      Assign::Literal(lit) => Item::Literal(lit).into(),

      Assign::Stream(group) => {
         let collector = evaluate_scoped_block(group.stream(), env)?;
         Item::Stream(collector).into()
      },

      Assign::Item(group) => parse_item_path(group, env)?,

      Assign::List(group) => {

         let stream = evaluate_scoped_block(group.stream(), env)?;

         let mut list = Vec::new();

         if !stream.is_empty() {
            let mut tokens = stream.into();

            while let Ok(assign) = parse_assign_value(group.span(), &mut tokens, env) {

               let item = parse_assign(assign, env)?;
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

         let stream = evaluate_scoped_block(group.stream(), env)?;

         let mut map = HashMap::new();

         if !stream.is_empty() {
            let mut tokens: TokenIter = stream.into();

            while let Some(token) = tokens.next() {

               let mut span = token.span();

               let ident = match_token!(token, Ident).to_string();

               match_next!(span, tokens, Punct(pt) if pt.as_char() == ':');

               let assign = parse_assign_value(group.span(), &mut tokens, env)?;
               let item = parse_assign(assign, env)?;

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
