
use proc_macro2::{TokenStream, TokenTree, Group, Ident, Literal};

use std::rc::Rc;
use std::str::FromStr;

use crate::*;



pub fn evaluate(input: TokenStream) -> Res<TokenStream> {

   let mut input = input.into();
   let mut output = TokenStream::new();
   let mut scope = Env::new();

   match parse_block(&mut input, &mut output, &mut scope) {
      Ok(()) => Ok(output),
      Err(err) => Err(err),
   }
}


pub fn parse_block(input: &mut TokenIter, output: &mut TokenStream, env: &mut Env) -> Res<()> {

   while let Some(token) = input.next() { match token {

      TokenTree::Punct(punct) => match punct.as_char() {

         // action signifier
         '$' => match parse_action(punct.span(), input, env)? {

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
         parse_block(&mut group.stream().into(), &mut sub_stream, env)?;

         output.extend(Some(TokenTree::from(
            Group::new(group.delimiter(), sub_stream)
         )));
      },

      other => output.extend(Some(other)),
   }}

   Ok(())
}


pub fn parse_item_path(item_path: Group, env: &mut Env) -> Res<Rc<Item>> {

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