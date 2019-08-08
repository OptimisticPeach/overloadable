/*!
  This crate provides the ability to create overloadable functions in rust through
  the use of a macro.
  # Syntax:
  ```ignore
  # #![feature(fn_traits, unboxed_closures, proc_macro_hygiene)]
  # struct function_types;
  # struct optional_return_type;
  # trait constraints {}
  # const code_body: () = ();
  use overloadable::overloadable;
  overloadable!{
      pub function_name as
      #[doc = "Some meta attributes."]
      fn<OptionalTypeArgs>(function_params: function_types) -> optional_return_type where OptionalTypeArgs: constraints {
          code_body
      }
  }
  ```
  # What is produced
  Here is an example of the output produced by `overloadable`:
  ```ignore
  # #![feature(fn_traits, unboxed_closures)]
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
  pub struct my_func_;
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

  Note that you cannot have functions with unused generic parameters due to the
  trait-implementing nature of this method.
*/
extern crate proc_macro;
use self::proc_macro::TokenStream;
use proc_macro2::TokenStream as Tok2;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    bracketed,
    parenthesized,
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Bracket, Paren},
    Block,
    Error,
    Generics,
    Ident,
    Meta,
    Pat,
    ReturnType,
    Token,
    Type,
    TypeTuple,
    Visibility,
    WhereClause,
};

struct OverloadableGlobal {
    vis: Visibility,
    name: Ident,
    _as_keyword: Token![as],
    fns: Punctuated<ParsedFnDef, Token![,]>,
}

impl Parse for OverloadableGlobal {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            vis: input.parse()?,
            name: input.parse()?,
            _as_keyword: input.parse()?,
            fns: input.parse_terminated(ParsedFnDef::parse)?,
        })
    }
}

struct OverloadableAssociated {
    vis: Visibility,
    struct_name: Ident,
    _colons: Token![::],
    name: Ident,
    _as: Token![as],
    fns: Punctuated<ParsedFnDef, Token![,]>,
}

impl Parse for OverloadableAssociated {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            vis: input.parse()?,
            struct_name: input.parse()?,
            _colons: input.parse()?,
            name: input.parse()?,
            _as: input.parse()?,
            fns: input.parse_terminated(ParsedFnDef::parse)?,
        })
    }
}

enum ThisDef {
    Explicit(
        Option<Token![mut]>,
        Token![self],
        Token![:],
        Type,
        Option<Token![,]>,
    ),
    Implicit(
        Option<Token![&]>,
        Option<Token![mut]>,
        Token![self],
        Option<Token![,]>,
    ),
}

impl Parse for ThisDef {
    fn parse(input: ParseStream) -> Result<Self> {
        if (input.peek2(Token![:]) || input.peek3(Token![:]))
            && (input.peek(Token![self]) || input.peek2(Token![self]))
        {
            //Explicit route
            let mutable = if input.peek(Token![mut]) {
                Some(input.parse::<Token![mut]>()?)
            } else {
                None
            };
            let this = input.parse::<Token![self]>()?;
            Ok(ThisDef::Explicit(
                mutable,
                this,
                input.parse()?,
                input.parse()?,
                input.parse()?,
            ))
        } else if input.peek(Token![self]) || input.peek2(Token![self]) || input.peek3(Token![self])
        {
            Ok(ThisDef::Implicit(
                input.parse()?,
                input.parse()?,
                input.parse()?,
                input.parse()?,
            ))
        } else {
            Err(input.error("Could not find self type!"))
        }
    }
}

impl ToTokens for ThisDef {
    fn to_tokens(&self, tokens: &mut Tok2) {
        match self {
            ThisDef::Explicit(mut_def, self_def, colon, ty, comma) => {
                mut_def.to_tokens(tokens);
                self_def.to_tokens(tokens);
                colon.to_tokens(tokens);
                ty.to_tokens(tokens);
                comma.to_tokens(tokens);
            }
            ThisDef::Implicit(and, mut_def, self_def, comma) => {
                and.to_tokens(tokens);
                mut_def.to_tokens(tokens);
                self_def.to_tokens(tokens);
                comma.to_tokens(tokens);
            }
        }
    }
}

impl ThisDef {
    pub fn is_sized_dependent(this: &Option<Self>) -> bool {
        if let Some(ThisDef::Implicit(None, ..)) = this { true }
        else { false }
    }
}

struct ParsedFnDef {
    meta: Vec<(Meta, Bracket)>,
    _func: Token![fn],
    gen: Option<Generics>,
    paren: Paren,
    this: Option<ThisDef>,
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
        let this = params_content.parse::<ThisDef>().ok();
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
            this,
            params,
            ret,
            w_clause,
            code,
        })
    }
}

fn gen_fn_decls<T: IntoIterator<Item = ParsedFnDef>>(fns: T, name: &Ident) -> Result<Tok2> {
    let fns: Vec<Tok2> = fns.into_iter().map(
        |ParsedFnDef {
             gen,
             params,
             ret,
             w_clause,
             code,
             paren,
             meta,
             this,
             ..
        }| {
            if this.is_some() {
                return Err(Error::new(paren.span, "This declaration cannot contain a `self`-style parameter."));
            }
            let ret = match ret {
                ReturnType::Type(_, ty) => *ty.clone(),
                ReturnType::Default => Type::Tuple(TypeTuple { paren_token: paren, elems: Punctuated::new() }),
            };
            let mut param_types = Vec::new();
            let mut param_patterns = Vec::new();
            params.iter().for_each(|(pat, _, ty)| {
                param_types.push(ty);
                param_patterns.push(pat)
            });
            let pty = &param_types[..];
            let ppt = &param_patterns[..];
            let meta: Vec<Tok2> = meta.iter().map(|(m, b)| quote_spanned!(b.span => #[#m])).collect();
            let meta = &meta[..];
            Ok(quote!(
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
            ))
        }
    ).collect::<Result<Vec<Tok2>>>()?;
    Ok(quote!(
        #(#fns)*
    ))
}

fn gen_trait_fn_decls<T: IntoIterator<Item = ParsedFnDef>>(
    fns: T,
    name: &Ident,
    struct_name: &Ident,
    vis: &Visibility,
) -> Result<Tok2> {
    let fns: Vec<Tok2> = fns
        .into_iter()
        .enumerate()
        .map(
            |(
                index,
                ParsedFnDef {
                    gen,
                    params,
                    ret,
                    w_clause,
                    code,
                    paren,
                    meta,
                    this,
                    ..
                },
            )| {
                let ret = match ret {
                    ReturnType::Type(_, ty) => *ty.clone(),
                    ReturnType::Default => Type::Tuple(TypeTuple {
                        paren_token: paren,
                        elems: Punctuated::new(),
                    }),
                };
                let mut trait_params = Vec::with_capacity(params.len());
                let mut impl_params = Vec::with_capacity(params.len());
                for (index, (lhs, _, rhs)) in params.iter().enumerate() {
                    let next_ident = Ident::new(&format!("_{}", index), lhs.span());
                    trait_params.push(quote!(#next_ident: #rhs));
                    impl_params.push(quote!(#lhs: #rhs));
                }
                let meta: Vec<Tok2> = meta
                    .iter()
                    .map(|(m, b)| quote_spanned!(b.span => #[#m]))
                    .collect();
                let meta = &meta[..];
                let trait_name = Ident::new(
                    &format!("{}Trait{}", struct_name, index),
                    struct_name.span(),
                );
                let sized_requirement = if ThisDef::is_sized_dependent(&this) {
                    quote!(Sized)
                } else { quote!() };
                Ok(quote!(
                    #vis trait #trait_name: #sized_requirement {
                        fn #name#gen(#this#(#trait_params),*) -> #ret #w_clause;
                    }
                    impl #trait_name for #struct_name {
                        #(#meta)*
                        fn #name#gen(#this#(#impl_params),*) -> #ret #w_clause {
                            #code
                        }
                    }
                ))
            },
        )
        .collect::<Result<Vec<Tok2>>>()?;

    Ok(quote!(
        #(#fns)*
    ))
}

///
/// Overloadable function macro. Please read the top level documentation for this crate
/// for more information on this.
///
/// ## Example:
/// ```
/// # #![feature(fn_traits, unboxed_closures, proc_macro_hygiene)]
/// # use std::fmt::{Debug, Display};
/// overloadable::overloadable! {
///     my_func as
///     fn(x: usize) -> usize {
///         x * 2
///     },
///     fn(x: &str) -> usize {
///         x.len()
///     },
///     fn<T: Debug>(x: T, y: T) -> String where T: Display {
///         format!("{:?}, {}", x, y)
///     },
///     fn<T: Debug + Display>((x, y): (T, T)) -> String {
///         my_func(x, y)
///     },
/// }
/// ```
///
#[proc_macro]
pub fn overloadable(input: TokenStream) -> TokenStream {
    let OverloadableGlobal { vis, name, fns, .. } = parse_macro_input!(input as OverloadableGlobal);
    let name = &name;
    let struct_decl = quote_spanned! { name.span() =>
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        #vis struct #name;
    };
    let fn_decls = gen_fn_decls(fns, name).unwrap();

    let expanded = quote! {
        #struct_decl
        #(#fn_decls)*
    };
    TokenStream::from(expanded)
}

///
/// Overloadable function macro for members. This allows you to have overloadable methods
/// and associated functions.
///
/// This has similar syntax to that of `overloadable`, except that it differs in that
/// the function name must be preceded by `StructName::`, for example:
///
/// ```
/// # #![feature(fn_traits, unboxed_closures, proc_macro_hygiene)]
/// struct Foo;
/// overloadable::overloadable_member!{
///     Foo::func_name as
///     fn() {},
///     fn(self) {},
///     fn(mut self, x: isize) {},
///     fn(&self, y: usize) {},
///     fn(self: Box<Self>, z: &str) {}
/// }
/// ```
///
/// ** NOTE **
/// This is internally implemented using custom traits, so to have this functionality
/// carry over, you must use a `use my_mod::*` to import all of the traits defined by
/// this macro.
///
#[proc_macro]
pub fn overloadable_member(input: TokenStream) -> TokenStream {
    let OverloadableAssociated {
        vis,
        struct_name,
        name,
        fns,
        ..
    } = parse_macro_input!(input as OverloadableAssociated);
    TokenStream::from(gen_trait_fn_decls(fns, &name, &struct_name, &vis).unwrap())
}
