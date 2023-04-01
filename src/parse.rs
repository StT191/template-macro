
use proc_macro2::{TokenStream, TokenTree, Group, Ident, Literal};

use std::rc::Rc;
use std::str::FromStr;

use crate::*;



pub fn evaluate(input: TokenStream) -> Res<TokenStream> {

   let mut output = TokenStream::new();
   let mut scope = Env::new();
   scope.push_scope(None);

   match parse_block(input, &mut output, &mut scope) {
      Ok(()) => Ok(output),
      Err(err) => Err(err),
   }
}


pub fn parse_block(input: TokenStream, output: &mut TokenStream, env: &mut Env) -> Res<()> {

   let mut input = TokenIter::from(input);

   while let Some(token) = input.next() { match token {

      TokenTree::Punct(punct) => match punct.as_char() {

         // action signifier
         '$' => match parse_action(punct.span(), &mut input, env)? {

            Action::Escape(escaped) => output.extend(Some(TokenTree::from(escaped))),

            Action::Assign(id, assign) => {
               let item = parse_assign(assign, env)?;
               env.set_item(id, item);
            },

            Action::Quote(quote) => parse_quote(punct.span(), quote, output, env)?,
         },

         // any other
         _ => output.extend(Some(TokenTree::from(punct))),
      },

      TokenTree::Group(group) => {

         // parse recursively
         let mut collector = TokenStream::new();
         parse_block(group.stream(), &mut collector, env)?;

         let mut collect_group = Group::new(group.delimiter(), collector);
         collect_group.set_span(group.span());

         output.extend(Some(TokenTree::from(collect_group)));
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

      match token {

         TokenTree::Punct(punct) => match punct.as_char() {

            // access sub
            '.' if !needs_segment => needs_segment = true,

            // access bind
            '@' if item.is_none() && path.len() == 0 => {

               needs_segment = false;

               let mut id_span = span;
               let ident = match_next!(id_span, item_path, Ident);
               span = span.join(id_span).unwrap();

               let get = match ident.to_string().as_str() {
                  "value" => 0, "index" => 1, "key" => 2,
                  _ => err!(span, "unknown identifier"),
               };

               let scope = if let Some(scp) = env.get_iter_scope() { scp }
               else { match get {
                  0 => err!(span, "@value is only available in iterator blocks"),
                  1 => err!(span, "@index is only available in iterator blocks"),
                  _ => err!(span, "@key is only available in iterator blocks"),
               }};

               item = Some((span, match get {
                  0 => Rc::clone(&scope.value),
                  1 => Item::Literal(Literal::usize_unsuffixed(scope.index)).into(),
                  _ => Item::Ident(Ident::new(&scope.key, span)).into(),
               }));
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