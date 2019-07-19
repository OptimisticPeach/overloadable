/*!
  This crate provides the ability to create overloadable functions in rust through
  the use of a macro.
  # Syntax:
  ```ignore
  # struct function_types;
  # struct optional_return_type;
  # trait constraints {}
  # const code_body: () = ();

  use overloadable::overloadable;
  overloadable!{
      #[doc = "Some meta attributes."]
      pub function_name as
      fn<OptionalTypeArgs>(function_params: function_types) -> optional_return_type where OptionalTypeArgs: constraints {
          code_body
      }
  }
  ```
  # What is produced
  Here is an example of the output produced by `overloadable`:
  ```ignore
  use overloadable::overloadable;
  use std::fmt::Debug;
  overloadable!{
      pub my_func as
      fn(x: usize, y: &str) -> f32 {
          (x * y.len()) as f32
      },
      fn<T>() where T: Debug {}
  }
  //Gives
  #[allow(non_camel_case_types)]
  pub struct my_func;
  impl Fn<(usize, &str,)> for my_func {
      extern "rust-call" fn call(&self, (x, y,): (usize, &str,)) -> f32 {
          {
              (x * y.len()) as f32
          }
      }
  }
  //The rest of the `Fn*` family
  impl<T> Fn<()> for my_func where T: Debug {
      extern "rust-call" fn call(&self, (): ()) -> () {
          {}
      }
  }
  //The rest of the `Fn*` family.
  ```
*/
extern crate proc_macro;
use self::proc_macro::TokenStream;
use proc_macro2::TokenStream as Tok2;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{
    bracketed, parenthesized, parse_macro_input,
    token::{Bracket, Paren},
    Block, Generics, Ident, Meta, Pat,
    ReturnType, Token, Type, TypeTuple, Visibility, WhereClause,
};

///
/// Overloadable function macro. Please read the top level documentation for this crate
/// for more information on this.
///
struct Overloadable {
    vis: Visibility,
    name: Ident,
    _as_keyword: Token![as],
    fns: Punctuated<ParsedFnDef, Token![,]>,
}

impl Parse for Overloadable {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            vis: input.parse()?,
            name: input.parse()?,
            _as_keyword: input.parse()?,
            fns: input.parse_terminated(ParsedFnDef::parse)?,
        })
    }
}

struct ParsedFnDef {
    meta: Vec<(Meta, Bracket)>,
    _func: Token![fn],
    gen: Option<Generics>,
    paren: Paren,
    params: Punctuated<(Pat, Token![:], Type), Token![,]>,
    ret: ReturnType,
    w_clause: Option<WhereClause>,
    code: Block,
}

fn parse_pattern_type_pair(input: ParseStream) -> Result<(Pat, Token![:], Type)> {
    Ok((input.parse()?, input.parse()?, input.parse()?))
}

impl Parse for ParsedFnDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut meta = Vec::new();
        while input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            let meta_content;
            let brackets = bracketed!(meta_content in input);
            meta.push((meta_content.parse()?, brackets))
        }
        let _func = input.parse::<Token![fn]>()?;
        let gen = if input.peek(Token![<]) {
            Some(input.parse::<Generics>()?)
        } else {
            None
        };
        let params_content;
        let paren = parenthesized!(params_content in input);
        let params = params_content.parse_terminated(parse_pattern_type_pair)?;
        let ret = input.parse()?;
        let w_clause = if input.peek(Token![where]) {
            Some(input.parse::<WhereClause>()?)
        } else {
            None
        };
        let code = input.parse()?;
        Ok(Self {
            meta,
            _func,
            gen,
            paren,
            params,
            ret,
            w_clause,
            code,
        })
    }
}

#[proc_macro]
pub fn overloadable(input: TokenStream) -> TokenStream {
    let Overloadable { vis, name, fns, .. } = parse_macro_input!(input as Overloadable);
    let name = &name;
    let struct_decl = quote_spanned! { name.span() =>
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        #vis struct #name;
    };
    let fn_decls: Vec<Tok2> = fns.iter().map(
        |ParsedFnDef {
            gen,
            params,
            ret,
            w_clause,
            code,
            paren,
            meta,
            ..
         }| {
            let ret = match ret {
                ReturnType::Type(_, ty) => *ty.clone(),
                ReturnType::Default => Type::Tuple(TypeTuple {paren_token: *paren, elems: Punctuated::new()}),
            };
            let mut param_types = Vec::new();
            let mut param_patterns = Vec::new();
            params.iter().for_each(|(pat, _, ty)| {param_types.push(ty); param_patterns.push(pat)});
            let pty = &param_types[..];
            let ppt = &param_patterns[..];
            let meta: Vec<Tok2> = meta.iter().map(|(m, b)| quote_spanned!(b.span => #[#m])).collect();
            let meta = &meta[..];
            quote!(
                impl#gen Fn<(#(#pty,)*)> for #name #w_clause {
                    #(#meta)*
                    extern "rust-call" fn call(&self, (#(#ppt,)*): (#(#pty,)*)) -> Self::Output {
                        #code
                    }
                }
                impl#gen FnOnce<(#(#pty,)*)> for #name #w_clause {
                    type Output = #ret;
                    #(#meta)*
                    extern "rust-call" fn call_once(self, x: (#(#pty,)*)) -> Self::Output {
                        self.call(x)
                    }
                }
                impl#gen FnMut<(#(#pty,)*)> for #name #w_clause {
                    #(#meta)*
                    extern "rust-call" fn call_mut(&mut self, x: (#(#pty,)*)) -> Self::Output {
                        self.call(x)
                    }
                }
            )
        }
    ).collect();

    let expanded = quote! {
        #struct_decl
        #(#fn_decls)*
    };
    TokenStream::from(expanded)
}
