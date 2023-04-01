
use proc_macro2::{TokenStream, TokenTree, Ident, Group, Span};
use syn::parse_str;
use itertools::Either;
use std::rc::Rc;

use crate::*;


pub enum BlockModifier { Concat, First, Last, NotFirst, NotLast }
pub enum ItemModifier { None, Len }

pub enum Quote {
   Block(BlockModifier, Group),
   Iter(Group, Group),
   Item(ItemModifier, Group),
}

impl Quote {
   fn span(&self) -> Span {
      match self {
         Quote::Block(_, blk) => blk.span(), Quote::Iter(_, blk) => blk.span(), Quote::Item(_, gp) => gp.span()
      }
   }
}


fn parse_scoped_block(input: TokenStream, output: &mut TokenStream, env: &mut Env, iter_scope: Option<IterScope>) -> Res<()> {
   env.push_scope(iter_scope);
   let res = parse_block(input, output, env);
   env.pop_scope();
   res
}


pub fn parse_quote(span: Span, quote: Quote, output: &mut TokenStream, env: &mut Env) -> Res<()> {

   let span = span.join(quote.span()).unwrap();

   match quote {

      Quote::Block(modifier, block) => {

         use BlockModifier::{First, Last, NotFirst, NotLast, Concat};

         if matches!(modifier, First | Last | NotFirst | NotLast) {
            if let Some(scope) = env.get_iter_scope() {
               match modifier {
                  First => if !scope.first { return Ok(()) },
                  NotFirst => if scope.first { return Ok(()) },
                  Last => if !scope.last { return Ok(()) },
                  NotLast => if scope.last { return Ok(()) },
                  _ => unreachable!(),
               }
            } else if matches!(modifier, NotFirst | NotLast) {
               return Ok(());
            }
         }

         if let Concat = modifier {

            let mut collector = TokenStream::new();
            parse_scoped_block(block.stream(), &mut collector, env, None)?;

            let ident_str = collector.to_string().replace(" ", "");

            let mut ident = match parse_str::<Ident>(&ident_str) {
               Ok(ident) => ident,
               Err(_) => err!(block.span(), "this doesn't concatenate to an identifier"),
            };

            ident.set_span(span);

            output.extend(Some(TokenTree::from(ident)));
         }
         else {
            parse_scoped_block(block.stream(), output, env, None)?;
         }
      },

      Quote::Iter(path_group, block) => {

         let item = parse_item_path(path_group, env)?;

         if let Item::Map(map) = item.as_ref() {

            let item_iter = map.iter();
            let last = item_iter.len() - 1;

            for (i, (key, item)) in item_iter.enumerate() {

               let iter_scope = IterScope {
                  first: i == 0, last: i == last, index: i, key: key.to_string(), value: Rc::clone(item),
               };

               parse_scoped_block(block.stream(), output, env, Some(iter_scope))?;
            }
         }
         else {

            let item_iter = match item.as_ref() {
               Item::Ident(_) | Item::Literal(_) | Item::Stream(_) => {
                  Either::Left([Rc::clone(&item)].into_iter())
               },
               Item::List(list) => {
                  Either::Right(list.iter().cloned())
               },
               _ => unreachable!(),
            };
            let last = item_iter.len() - 1;

            for (i, item) in item_iter.enumerate() {

               let iter_scope = IterScope {
                  first: i == 0, last: i == last, index: i, key: i.to_string(), value: item,
               };

               parse_scoped_block(block.stream(), output, env, Some(iter_scope))?;
            }
         }
      },

      Quote::Item(modifier, path_group) => match modifier {

         ItemModifier::None => match parse_item_path(path_group, env)?.as_ref() {

            Item::Ident(ident) => {
               let mut ident = ident.clone();
               ident.set_span(span);
               output.extend(Some(TokenTree::from(ident)));
            },

            Item::Literal(literal) => {
               let mut literal = literal.clone();
               literal.set_span(span);
               output.extend(Some(TokenTree::from(literal)));
            },

            Item::Stream(stream) => output.extend(stream.clone().into_iter()),

            Item::List(_) => err!(span, "can not quote a list item"),
            Item::Map(_) => err!(span, "can not quote a map item"),
         },

         ItemModifier::Len => {

            let len = match parse_item_path(path_group, env)?.as_ref() {
               Item::Ident(_) | Item::Literal(_) | Item::Stream(_) => 1,
               Item::List(list) => list.len(),
               Item::Map(map) => map.len(),
            };

            let mut literal = Literal::usize_unsuffixed(len);
            literal.set_span(span);
            output.extend(Some(TokenTree::from(literal)));
         },
      }
   }

   Ok(())
}