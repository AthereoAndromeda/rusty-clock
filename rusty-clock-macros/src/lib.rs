use darling::{FromMeta, ast::NestedMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, Meta, parse::ParseStream};
// use procedural_macros_impl::add_fields_impl;
//

#[derive(FromMeta, Clone, Debug)]
// Struct to define the macro's configuration for adding fields and
// optionally getters and setters
struct AddFields {
    #[darling(multiple, rename = "fields")]
    add_fields: Vec<NewFieldDef>,

    #[darling(default)]
    getter: Option<IdentList>,
    #[darling(default)]
    setter: Option<IdentList>,
}

#[derive(FromMeta, Clone, Debug)]
// Struct definition for new fields that will be added dynamically
struct NewFieldDef {
    name: syn::Ident,
    ty: syn::Type,
}

// A struct to hold the parsed identifiers of getters and setters
#[derive(Debug, Clone)]
struct IdentList(Vec<Ident>);

impl darling::FromMeta for IdentList {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        if let syn::Meta::List(meta_list) = item {
            let tokens: proc_macro2::TokenStream = meta_list.clone().tokens;
            let list: IdentList =
                syn::parse2::<IdentList>(tokens).expect("Failed to parse identlist");
            Ok(list)
        } else {
            Err(darling::Error::custom("Expected list of identifiers"))
        }
    }
}

impl syn::parse::Parse for IdentList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut entries = Vec::<Ident>::new();

        if input.is_empty() {
            panic!("At least one field must be specified");
        }

        // Step 1: parse the "field" argument
        let field = input.parse::<syn::Ident>().expect("search field");
        if field.to_string().as_str() != "field" {
            panic!("wrong field name");
        }

        // Step 2: parse Token "="
        input.parse::<syn::Token![=]>()?;

        // Step 3: parse the field entries for setters and getters, separated by Token ','
        while !input.is_empty() {
            let field_entry: syn::Ident = if let Ok(field_entry) = input.parse::<syn::Ident>() {
                // parse identifier
                field_entry
            } else if let Ok(field_entry) = input.parse::<syn::LitStr>() {
                // parse String
                syn::Ident::new(field_entry.value().as_str(), proc_macro2::Span::call_site())
            } else {
                panic!("field_entry must be either a string literal or an identifier!");
            };

            if !input.is_empty() {
                input.parse::<syn::Token![,]>()?;
            }
            entries.push(field_entry);
        }
        // Step 4: Return the filled IdentList
        Ok(IdentList(entries))
    }
}

// #[proc_macro_attribute]
// pub fn add_fields(attr: TokenStream, item: TokenStream) -> TokenStream {
//     add_fields_impl(attr.into(), item.into()).into()
// }

#[proc_macro_derive(Funny)]
pub fn test(input: TokenStream) -> TokenStream {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
