
// return an error
macro_rules! err {
   ($span:expr, $err:expr) => {
      { return Err(($span, $err)) }
   };
}

// get the next token or return error
macro_rules! next {
   ($span:ident, $input:ident) => {
      if let Some(token) = $input.next() {
         #[allow(unused_assignments)]
         let _ = { $span = token.span() }; // make it a stmt to use the attribute
         token
      }
      else {
         err!($span, "unexpected end of input")
      }
   }
}

// get and match the next token or return an error
macro_rules! match_token {
   ($token:expr, $var:ident) => {
      match_token!($token, $var(_token))
   };
   ($token:expr, $var:ident($bind:ident) $($if:tt)*) => {
      match $token {
         TokenTree::$var($bind) $($if)* => $bind,
         _ => err!($token.span(), "unexpected token"),
      }
   };
}

// get and match the next token or return an error
macro_rules! match_next {
   ($span:ident, $input:ident, $var:ident) => {
      match_next!($span, $input, $var(_token))
   };
   ($span:ident, $input:ident, $var:ident($bind:ident) $($if:tt)*) => {
      match_token!(next!($span, $input), $var($bind) $($if)*)
   };
}