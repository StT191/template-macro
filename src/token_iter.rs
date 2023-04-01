
use proc_macro2::{TokenStream, token_stream::IntoIter, TokenTree};

pub struct TokenIter {
   iters: Vec<IntoIter>,
}

impl TokenIter {
   pub fn push_in_front(&mut self, tokens: impl Into<TokenStream>) {
      self.iters.push(tokens.into().into_iter());
   }
}

impl From<TokenStream> for TokenIter {
   fn from(tokens: TokenStream) -> Self {
      Self { iters: vec![tokens.into_iter()] }
   }
}


impl Iterator for TokenIter {
   type Item = TokenTree;

   fn next(&mut self) -> Option<TokenTree> {
      while let Some(iter) = self.iters.last_mut() {
         match iter.next() {
            Some(token) => return Some(token),
            None => self.iters.pop(),
         };
      }
      None
   }
}