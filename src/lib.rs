
// mods

type Res<T> = Result<T, (proc_macro2::Span, &'static str)>;

#[macro_use]
mod macros;
mod token_iter;   use token_iter::*;
mod env;          use env::*;
mod quotes;       use quotes::*;
mod assign;       use assign::*;
mod action;       use action::*;
mod parse;        use parse::*;


// exports

use proc_macro::{TokenStream as TokenStream1};
use proc_macro2::{TokenStream, TokenTree, Literal};
use quote::{quote, quote_spanned};


#[proc_macro]
pub fn template(input: TokenStream1) -> TokenStream1 {

   match evaluate(input.into()) {
      Ok(output) => output.into(),
      Err((span, err)) => {
         let string_lit = Literal::string(&err);
         quote_spanned!{span=>compile_error!(#string_lit)}.into()
      }
   }
}


#[proc_macro]
pub fn debug_template(input: TokenStream1) -> TokenStream1 {

   match evaluate(input.into()) {
      Ok(output) => {
         let debug = output.to_string();
         let string_lit = Literal::string(&debug);
         quote!{compile_error!(#string_lit)}.into()
      },
      Err((span, err)) => {
         let string_lit = Literal::string(&err);
         quote_spanned!{span=>compile_error!(#string_lit)}.into()
      }
   }
}


#[proc_macro]
pub fn debug_inplace(input: TokenStream1) -> TokenStream1 {

   let mut debug = String::new();

   for tree in input {
      debug.push_str(&format!("{:#?}\n", tree));
   }

   let string_lit = Literal::string(&debug);

   quote!{compile_error!(#string_lit)}.into()
}


#[proc_macro]
pub fn debug_string(input: TokenStream1) -> TokenStream1 {

   let mut debug = String::new();

   for tree in input {
      debug.push_str(&format!("{:#?}\n", tree));
   }

   let string_lit = Literal::string(&debug);

   TokenStream::from(TokenTree::from(string_lit)).into()
}
