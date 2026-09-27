use proc_macro::TokenStream;
use quote::quote;
use syn::{Error, Fields, GenericArgument, ItemStruct, PathArguments, Type};

/// Enforces the structural contract for a metric category aggregator.
///
/// A category aggregator must be a struct containing a named
/// `start_time: DateTime<Utc>` field. The input struct is returned
/// unchanged when the contract is satisfied.
#[proc_macro_attribute]
pub fn metric_category_aggregator(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as ItemStruct);

    match validate_start_time(&input) {
        Ok(()) => quote!(#input).into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn validate_start_time(input: &ItemStruct) -> Result<(), Error> {
    let Fields::Named(fields) = &input.fields else {
        return Err(Error::new_spanned(
            &input.ident,
            "metric category aggregator must be a struct with named fields",
        ));
    };

    let Some(field) = fields.named.iter().find(|field| {
        field
            .ident
            .as_ref()
            .is_some_and(|ident| ident == "start_time")
    }) else {
        return Err(Error::new_spanned(
            &input.ident,
            "metric category aggregator must contain a `start_time: DateTime<Utc>` field",
        ));
    };

    if !is_datetime_utc(&field.ty) {
        return Err(Error::new_spanned(
            &field.ty,
            "metric category aggregator `start_time` must have type `DateTime<Utc>`",
        ));
    }

    Ok(())
}

fn is_datetime_utc(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };

    let Some(datetime_segment) = type_path.path.segments.last() else {
        return false;
    };

    if datetime_segment.ident != "DateTime" {
        return false;
    }

    let PathArguments::AngleBracketed(arguments) = &datetime_segment.arguments else {
        return false;
    };

    let Some(GenericArgument::Type(Type::Path(utc_type))) = arguments.args.first() else {
        return false;
    };

    utc_type
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Utc")
}
