
use proc_macro2::{
   TokenStream, TokenTree, Group, Span, Ident, Literal,
   token_stream::{IntoIter as TokenIter},
};
use syn::parse_str;
use itertools::Either;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;

use crate::{Res, env::*, action::*};



pub fn evaluate(input: TokenStream) -> Res<TokenStream> {

   let mut input = input.into_iter();
   let mut output = TokenStream::new();
   let mut scope = Env::new();

   match parse_block(&mut input, &mut output, &mut scope) {
      Ok(()) => Ok(output),
      Err(err) => Err(err),
   }
}


fn parse_block(input: &mut TokenIter, output: &mut TokenStream, env: &mut Env) -> Res<()> {

   while let Some(token) = input.next() { match token {

      TokenTree::Punct(punct) => match punct.as_char() {

         // action signifier
         '$' => match parse_action(punct.span(), input)? {

            Action::Escape(escaped) => output.extend(Some(TokenTree::from(escaped))),

            Action::Assign(ident, assign) => {
               let item = parse_assign(punct.span(), assign, env)?;
               env.set_item(ident.to_string(), item);
            },

            Action::Quote(quote) => parse_quote(punct.span(), quote, output, env)?,
         },

         // any other
         _ => output.extend(Some(TokenTree::from(punct))),
      },

      TokenTree::Group(group) => {

         // parse recursively
         let mut sub_stream = TokenStream::new();
         parse_block(&mut group.stream().into_iter(), &mut sub_stream, env)?;

         output.extend(Some(TokenTree::from(
            Group::new(group.delimiter(), sub_stream)
         )));
      },

      other => output.extend(Some(other)),
   }}

   Ok(())
}


fn parse_assign(span: Span, assign: Assign, env: &mut Env) -> Res<Rc<Item>> {
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
         let stream = group.stream();
         let mut list = Vec::new();

         if !stream.is_empty() {
            let mut tokens = stream.into_iter();

            while let Ok(assign) = parse_assign_value(group.span(), &mut tokens) {

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
         let stream = group.stream();
         let mut map = HashMap::new();

         if !stream.is_empty() {
            let mut tokens = stream.into_iter();

            while let Some(token) = tokens.next() {

               let mut span = token.span();

               let ident = match_token!(token, Ident).to_string();

               match_next!(span, tokens, Punct(pt) if pt.as_char() == ':');

               let assign = parse_assign_value(group.span(), &mut tokens)?;
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


fn parse_quote(span: Span, quote: Quote, output: &mut TokenStream, env: &mut Env) -> Res<()> {

   let span = span.join(quote.span()).unwrap();

   match quote {

      Quote::Block(modifier, block) => {

         use Modifier::{First, Last, NotFirst, NotLast, Concat};

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

         env.push_scope(None);

         if let Concat = modifier {

            let mut collector = TokenStream::new();
            parse_block(&mut block.stream().into_iter(), &mut collector, env)?;

            let ident_str = collector.to_string().replace(" ", "");

            let mut ident = match parse_str::<Ident>(&ident_str) {
               Ok(ident) => ident,
               Err(_) => err!(span, "this doesn't evaluate to a valid identifier"),
            };

            ident.set_span(span);

            output.extend(Some(TokenTree::from(ident)));
         }
         else {
            parse_block(&mut block.stream().into_iter(), output, env)?;
         }

         env.pop_scope();
      },

      Quote::Iter(path_group, block) => {

         let item = parse_item_path(path_group, env)?;

         if let Item::Map(map) = item.as_ref() {

            let item_iter = map.iter();
            let last = item_iter.len() - 1;

            for (i, (key, item)) in item_iter.enumerate() {

               env.push_scope(Some(IterScope {
                  first: i == 0, last: i == last, index: i, key: key.to_string(), bind: Rc::clone(item),
               }));

               parse_block(&mut block.stream().into_iter(), output, env)?;

               env.pop_scope();
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

               env.push_scope(Some(IterScope {
                  first: i == 0, last: i == last, index: i, key: i.to_string(), bind: item,
               }));

               parse_block(&mut block.stream().into_iter(), output, env)?;

               env.pop_scope();
            }
         }
      },

      Quote::Item(path_group) => {
         match parse_item_path(path_group, env)?.as_ref() {
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
         }
      },
   }

   Ok(())
}


fn parse_item_path(item_path: Group, env: &mut Env) -> Res<Rc<Item>> {

   // let full_span = item_path.span();
   let mut span = item_path.span();
   let mut item_path = item_path.stream().into_iter();

   let mut path = Vec::new();
   let mut needs_segment = true;
   let mut item = None;

   loop {
      let token = if let Some(token) = item_path.next() { token }
      else if needs_segment { err!(span, "unexpected end of input") }
      else { break };

      span = token.span();
      // full_span = full_span.join(span).unwrap();

      match token {

         TokenTree::Punct(punct) => match punct.as_char() {

            // access sub
            '.' if !needs_segment => needs_segment = true,

            // access bind
            '@' if item.is_none() && path.len() == 0 => {

               needs_segment = false;
               let mut id_span = span;
               let id;

               let get = match item_path.next() {
                  None => 0,
                  Some(TokenTree::Ident(ident)) if {
                     id = ident.to_string();
                     id == "index" || ident == "key"
                  } => {

                     if let Some(rest) = item_path.next() {
                        err!(rest.span(), "unexpected token");
                     }

                     id_span = id_span.join(ident.span()).unwrap();

                     if id == "index" { 1 } else { 2 }
                  },
                  Some(TokenTree::Punct(punct)) if punct.as_char() == '.' => {
                     needs_segment = true;
                     span = punct.span();
                     0
                  },
                  Some(token) => err!(token.span(), "unexpected token"),
               };

               let scope = if let Some(scp) = env.get_iter_scope() { scp }
               else { match get {
                  0 => err!(id_span, "@ is only available in iterator blocks"),
                  1 => err!(id_span, "@index is only available in iterator blocks"),
                  _ => err!(id_span, "@key is only available in iterator blocks"),
               }};

               match get {
                  0 => item = Some((id_span, Rc::clone(&scope.bind))),
                  1 => return Ok(
                     Item::Literal(Literal::usize_unsuffixed(scope.index)).into()
                  ),
                  _ => return Ok(
                     Item::Ident(Ident::new(&scope.key, span)).into()
                  ),
               }
            },

            _ => err!(span, "unexpected token"),
         },

         TokenTree::Ident(ident) if needs_segment => {
            needs_segment = false;
            path.push(Segment { span, key: Key::String(ident.to_string()) });
         },

         TokenTree::Literal(lit) if needs_segment => match usize::from_str(&lit.to_string()) {
            Ok(index) => {
               needs_segment = false;
               path.push(Segment { span, key: Key::Index(index) });
            },
            Err(_) => err!(span, "unexpected token"),
         },

         _ => err!(span, "unexpected token"),
      }
   }

   if let Some((span, item)) = item {
      item.get_item(span, &path)
   } else {
      env.get_item(&path)
   }
}